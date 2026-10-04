//! Concrete Event-notification admission for one same-CPU service, not general IPC.
use crate::{
    cpu::context::Context,
    handles::{Namespace, status},
    scheduler::task::Task,
    user_copy::Access,
};
use kernel_core::{
    domain::{Charge, ChargeKind},
    handles::{Error, Handle},
    process::ProcessId,
    wait::AcceptedSignal,
};
pub(crate) const REQUEST: u16 = 0x90;
pub(crate) struct Work {
    service: ProcessId,
    sequence: u64,
    signal: AcceptedSignal,
    consumer: Charge,
    _queue: Charge,
    _service: Charge,
}
#[derive(Clone, Copy)]
enum Outcome {
    Pending,
    Completed,
    CancelledBeforeEffect,
}
#[derive(Clone, Copy)]
struct Receipt {
    consumer: ProcessId,
    sequence: u64,
    outcome: Outcome,
}
/// Current same-CPU notification storage. A later IPC design must rederive its
/// queue/terminal arbiter rather than treating this pilot encoding as authority.
pub(crate) struct Queue {
    entries: [Option<Work>; crate::scheduler::TASKS],
    receipts: [Option<Receipt>; crate::scheduler::TASKS],
    sequences: [u64; crate::scheduler::TASKS],
}
impl Queue {
    pub const EMPTY: Self = Self {
        entries: [const { None }; crate::scheduler::TASKS],
        receipts: [None; crate::scheduler::TASKS],
        sequences: [0; crate::scheduler::TASKS],
    };
    fn terminal(&mut self, consumer: ProcessId, sequence: u64, outcome: Outcome) {
        let index = consumer.slot() % crate::scheduler::TASKS;
        if let Some(receipt) = self.receipts[index].as_mut()
            && receipt.consumer == consumer
            && receipt.sequence == sequence
        {
            receipt.outcome = outcome;
        }
    }
    pub fn close_service(&mut self, index: usize) {
        if let Some(work) = self.entries[index].take() {
            self.terminal(
                work.consumer.domain().owner(),
                work.sequence,
                Outcome::CancelledBeforeEffect,
            );
        }
    }
    pub fn close_consumer(&mut self, index: usize) {
        self.receipts[index] = None;
    }
}
impl core::ops::Index<usize> for Queue {
    type Output = Option<Work>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}
impl core::ops::IndexMut<usize> for Queue {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}
fn charge(domain: &kernel_core::domain::Reference, kind: ChargeKind) -> Result<Charge, Error> {
    domain.charge(kind).map_err(|e| {
        if e == kernel_core::domain::Error::Exhausted {
            Error::Budget
        } else {
            Error::Denied
        }
    })
}
pub(crate) fn call(
    task: &Task,
    tables: &mut [Namespace; crate::scheduler::TASKS],
    queues: &mut Queue,
    current: usize,
    frame: &mut Context,
    operation: u16,
) -> bool {
    #[cfg(feature = "kernel-tests")]
    if testing::helper(&tables[current], frame, operation) {
        return true;
    }
    if operation != REQUEST {
        return false;
    }
    let result = (|| {
        let caller = tables[current].owner().ok_or(Error::Denied)?;
        if caller.slot() != task.id() || caller.generation() != task.process_generation() {
            return Err(Error::Denied);
        }
        tables[current]
            .domain()?
            .validate(caller)
            .map_err(|_| Error::Denied)?;
        let snapshot = Access::current(task)
            .and_then(|a| a.copy_from_user::<48>(frame.gpr[0] as usize, 48))
            .map_err(|_| Error::Invalid)?;
        let word = |i: usize| {
            u64::from_le_bytes(
                snapshot.bytes()[i * 8..(i + 1) * 8]
                    .try_into()
                    .expect("bounded word"),
            )
        };
        if word(0) != 1 {
            return Err(Error::Invalid);
        }
        if (word(3) != 0 || word(4) != 0 || word(5) != 0)
            && !cfg!(feature = "capability-scope-negative")
        {
            return Err(Error::Denied);
        }
        match word(1) {
            4 => {
                let (service, signal) =
                    tables[current].admit_signal(caller, Handle::decode(word(2)))?;
                if service.slot() / crate::scheduler::TASKS != task.id() / crate::scheduler::TASKS
                    || service == caller
                {
                    return Err(Error::Denied);
                }
                let receiver = service.slot() % crate::scheduler::TASKS;
                if tables[receiver].owner() != Some(service) {
                    return Err(Error::Denied);
                }
                if queues[receiver].is_some() || queues.receipts[current].is_some() {
                    return Err(Error::Budget);
                }
                let sequence = queues.sequences[current]
                    .checked_add(1)
                    .ok_or(Error::GenerationExhausted)?;
                let consumer = charge(tables[current].domain()?, ChargeKind::Request)?;
                let queue = charge(tables[receiver].domain()?, ChargeKind::Queue)?;
                let server = charge(tables[receiver].domain()?, ChargeKind::Request)?;
                queues.sequences[current] = sequence;
                queues.receipts[current] = Some(Receipt {
                    consumer: caller,
                    sequence,
                    outcome: Outcome::Pending,
                });
                frame.gpr[1] = sequence;
                queues[receiver] = Some(Work {
                    service,
                    sequence,
                    signal,
                    consumer,
                    _queue: queue,
                    _service: server,
                });
                Ok(0)
            }
            5 => {
                tables[current].revoke(caller, Handle::decode(word(2)))?;
                Ok(0)
            }
            6 => Err(Error::Denied), // There is no unprivileged grant issuer.
            7 => {
                if word(2) != 0 {
                    return Err(Error::Denied);
                }
                let (pages, handles, queue, requests) = tables[current].domain()?.usage();
                frame.gpr[1] = pages as u64;
                frame.gpr[2] = handles as u64;
                frame.gpr[3] = queue as u64;
                frame.gpr[4] = requests as u64;
                Ok(0)
            }
            8 | 9 => {
                if word(2) != 0 {
                    return Err(Error::Denied);
                }
                let work = queues[current].as_ref().ok_or(Error::Denied)?;
                if work.service != caller {
                    return Err(Error::Denied);
                }
                if word(1) == 8 {
                    frame.gpr[1] = u64::from(work.consumer.domain().closing());
                    frame.gpr[2] = u64::from(work.consumer.domain().usage().3);
                    Ok(1)
                } else {
                    let work = queues[current].take().unwrap();
                    let consumer = work.consumer.domain().owner();
                    let sequence = work.sequence;
                    // Effect is bound to the admitted consumer target, never a service-private handle.
                    work.signal.finish();
                    queues.terminal(consumer, sequence, Outcome::Completed);
                    Ok(0)
                }
            }
            10 => {
                let receipt = queues.receipts[current].as_ref().ok_or(Error::Denied)?;
                if receipt.consumer != caller || receipt.sequence != word(2) {
                    return Err(Error::Denied);
                }
                let status = match receipt.outcome {
                    Outcome::Pending => 0,
                    Outcome::Completed => 1,
                    Outcome::CancelledBeforeEffect => 2,
                };
                if status != 0 {
                    queues.receipts[current] = None;
                }
                Ok(status)
            }
            _ => Err(Error::Invalid),
        }
    })();
    frame.gpr[0] = result.unwrap_or_else(status);
    true
}
#[cfg(feature = "kernel-tests")]
pub(crate) mod testing;
