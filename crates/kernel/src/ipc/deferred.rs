//! Bounded deadline/death/wake drainage, outside scheduler storage and IRQ.
//! Endpoint exclusion spans only core state and finite atomic mailbox operations;
//! the actual SGI is sent after every endpoint permit is released.
use kernel_core::{
    ipc::{MAX_WAITERS, mailbox::Mailbox},
    process::ProcessId,
};
static MAILBOXES: [Mailbox; crate::scheduler::CAPACITY] =
    [const { Mailbox::new() }; crate::scheduler::CAPACITY];

pub(crate) struct Poll {
    pub deadline: Option<u64>,
    pub quiescent: bool,
    /// A busy cell retains all its work. A timer/deferred retry is required;
    /// false may never be interpreted as endpoint or session quiescence.
    pub complete: bool,
}
pub(crate) fn mailbox(slot: usize) -> &'static Mailbox {
    &MAILBOXES[slot]
}
pub(crate) fn mailboxes_quiescent() -> bool {
    MAILBOXES.iter().all(Mailbox::quiescent)
}
pub(crate) fn poll(deaths: &[Option<ProcessId>]) -> Poll {
    let now = crate::cpu::ticks();
    let mut deadline: Option<u64> = None;
    let mut notify = 0_usize;
    let mut mailbox_busy = false;
    let mut quiescent = true;
    let complete = super::storage::each(|endpoint| {
        for id in deaths.iter().flatten() {
            endpoint.process_death(*id);
        }
        endpoint.expire(now);
        if let Some(next) = endpoint.next_deadline() {
            deadline = Some(deadline.map_or(next, |current| current.min(next)));
        }
        let mut wakes = [None; MAX_WAITERS];
        for (slot, wake) in wakes.iter_mut().zip(endpoint.pending_wakes()) {
            *slot = Some(wake);
        }
        for wake in wakes.into_iter().flatten() {
            let target = wake.key.process().slot();
            if cfg!(feature = "ipc-wake-publication-negative")
                && target / crate::scheduler::TASKS != endpoint.reference().cpu()
            {
                continue;
            }
            assert!(
                target < MAILBOXES.len(),
                "IPC process outside fixed scheduler capacity"
            );
            let mailbox = &MAILBOXES[target];
            if mailbox.acknowledged(wake.key.sequence()) {
                endpoint
                    .acknowledge_wake(wake.key)
                    .expect("exclusive exact retained wake");
                mailbox
                    .finish_acknowledgement(wake.key.sequence())
                    .expect("single source acknowledgement");
            } else {
                match mailbox.publish(wake.key.sequence()) {
                    Ok(true) => notify |= 1 << (target / crate::scheduler::TASKS),
                    Ok(false) => {}
                    Err(kernel_core::ipc::Error::Busy) => mailbox_busy = true,
                    Err(error) => panic!("invalid retained wake sequence: {error:?}"),
                }
            }
        }
        quiescent &= endpoint.outstanding() == 0 && endpoint.waiter_count() == 0;
    });
    for cpu in 0..crate::platform::config::ACTIVE_CPUS {
        if cpu != crate::percpu::id() && notify & (1 << cpu) != 0 {
            crate::smp::ping(cpu);
        }
    }
    Poll {
        deadline,
        complete: complete && !mailbox_busy,
        quiescent: complete && quiescent && mailboxes_quiescent(),
    }
}
