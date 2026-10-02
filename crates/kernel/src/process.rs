//! Bootstrap-only native process owner. No EL0 create endpoint or authority model.
use crate::{cpu, memory, percpu, platform::config, scheduler, time};
use core::sync::atomic::{AtomicBool, Ordering};
static REGISTRY_CREATED: AtomicBool = AtomicBool::new(false);
use cpu::context::Context;
use kernel_core::process::Table;
pub(crate) use kernel_core::process::{Completion, CreationStep, Error, ProcessId, Reason, State};
use scheduler::task::{Admission, CONTEXT_EXITED, CONTEXT_FAULTED, CONTEXT_TIMED_OUT};
pub(crate) const CAPACITY: usize = scheduler::CAPACITY;
const OWNER_CAPACITY: usize = scheduler::TASKS;
#[derive(Clone, Copy)]
pub(crate) enum Origin {
    Bootstrap,
    El0,
}
#[derive(Clone, Copy)]
pub(crate) struct Spec<'a> {
    pub image: &'a [u8],
    pub context: Context,
    pub owner: usize,
    pub entry: usize,
    /// Explicit workload limit; None means normal exit/fault determines lifetime.
    pub slice_limit: Option<usize>,
}
#[derive(Debug)]
pub(crate) struct CreationFailure {
    pub error: Error,
    pub completion: Option<Completion>,
}
struct Object {
    id: ProcessId,
    space: memory::OwnedUserSpace,
    context: Context,
    slice_limit: Option<usize>,
    slices: usize,
    observations: crate::execution::Observations,
}
/// Mutable CPU0-owned value, not global shared storage. Owned frames make it
/// non-Send/non-Sync. Callers retain it independently of dispatch/workload lifetime.
pub(crate) struct Registry {
    table: Table<CAPACITY>,
    objects: [Option<Object>; CAPACITY],
}
pub(crate) fn context_contract() -> Result<(), Error> {
    if percpu::id() != percpu::BOOT_CPU {
        return Err(Error::ForeignCpu);
    }
    if !cpu::irq_masked() {
        return Err(Error::IrqEnabled);
    }
    crate::sync::assert_scheduler_unlocked();
    Ok(())
}
fn trace(id: ProcessId, state: State, step: Option<CreationStep>) {
    #[cfg(feature = "diagnostics")]
    crate::diagnostics::status(
        "DEBUG",
        "process",
        format_args!(
            "slot={} generation={} owner={} state={state:?} rollback={step:?}",
            id.slot(),
            id.generation(),
            id.slot() / OWNER_CAPACITY
        ),
    );
    #[cfg(not(feature = "diagnostics"))]
    let _ = (id, state, step);
}
impl Registry {
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(|error| reject(error))
    }
    pub fn try_new() -> Result<Self, Error> {
        context_contract()?;
        REGISTRY_CREATED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| Error::AlreadyOwned)?;
        Ok(Self {
            table: Table::new(),
            objects: core::array::from_fn(|_| None),
        })
    }

    pub fn create(
        &mut self,
        physical: &mut memory::Physical,
        origin: Origin,
        spec: Spec<'_>,
        fail_at: Option<CreationStep>,
    ) -> Result<ProcessId, CreationFailure> {
        self.create_inner(physical, origin, spec, fail_at)
            .map_err(|(error, completion)| CreationFailure { error, completion })
    }
    fn create_inner(
        &mut self,
        physical: &mut memory::Physical,
        origin: Origin,
        spec: Spec<'_>,
        fail_at: Option<CreationStep>,
    ) -> Result<ProcessId, (Error, Option<Completion>)> {
        context_contract().map_err(|e| (e, None))?;
        if !matches!(origin, Origin::Bootstrap) {
            return Err((Error::Unauthorized, None));
        }
        if memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire) != 0 {
            return Err((Error::NotQuiescent, None));
        }
        let payload = spec.entry == config::USER_PAYLOAD_BASE;
        if spec.owner >= config::ACTIVE_CPUS
            || spec.image.is_empty()
            || !(spec.entry == memory::USER_CODE || payload)
            || spec.image.len()
                > if payload {
                    config::USER_PAYLOAD_BYTES
                } else {
                    config::PAGE_BYTES
                }
            || spec.context.pc != spec.entry as u64
            || spec.context.sp != memory::USER_STACK_TOP as u64
            || !cpu::context::valid_user_context(&spec.context)
            || spec.slice_limit == Some(0)
        {
            return Err((Error::InvalidImage, None));
        }
        let range = spec.owner * OWNER_CAPACITY..(spec.owner + 1) * OWNER_CAPACITY;
        let id = self.table.reserve(range).map_err(|e| (e, None))?;
        trace(id, State::Creating, None);
        let mut frame = None;
        let mut space = None;
        let transaction = (|| {
            if fail_at == Some(CreationStep::Slot) {
                return Err((Error::Allocation, CreationStep::Slot));
            }
            frame = Some(
                physical
                    .allocate(
                        memory::USER_SPACE_PAGES
                            + if payload {
                                spec.image.len().div_ceil(config::PAGE_BYTES)
                            } else {
                                0
                            },
                        1,
                    )
                    .ok_or((Error::Allocation, CreationStep::Frames))?,
            );
            if fail_at == Some(CreationStep::Frames) {
                return Err((Error::Allocation, CreationStep::Frames));
            }
            space = Some(memory::OwnedUserSpace::new(
                frame.take().unwrap(),
                id.slot(),
                spec.image,
                spec.entry,
            ));
            if fail_at == Some(CreationStep::Space) {
                return Err((Error::Allocation, CreationStep::Space));
            }
            let context = spec.context;
            if fail_at == Some(CreationStep::Context) {
                return Err((Error::Allocation, CreationStep::Context));
            }
            if fail_at == Some(CreationStep::Commit) {
                return Err((Error::Allocation, CreationStep::Commit));
            }
            self.objects[id.slot()] = Some(Object {
                id,
                space: space.take().unwrap(),
                context,
                slice_limit: spec.slice_limit,
                slices: 0,
                observations: crate::execution::Observations::ZERO,
            });
            self.table.prepared(id).expect("transaction state");
            Ok(())
        })();
        if let Err((error, step)) = transaction {
            if let Some(space) = space {
                #[cfg(not(feature = "process-rollback-negative"))]
                space.reclaim(physical);
                #[cfg(feature = "process-rollback-negative")]
                core::mem::forget(space);
            }
            if let Some(frame) = frame {
                physical.release(frame);
            }
            let completion = self
                .table
                .rollback(id, step)
                .expect("creation rollback state");
            trace(id, State::Free, Some(step));
            return Err((error, Some(completion)));
        }
        trace(id, State::Prepared, None);
        Ok(id)
    }
    pub fn start(&mut self, id: ProcessId) -> Result<(), Error> {
        context_contract()?;
        self.table.start(id)?;
        trace(id, State::Admitted, None);
        Ok(())
    }
    pub fn state(&self, id: ProcessId) -> Result<State, Error> {
        context_contract()?;
        self.table.state(id)
    }
    pub fn live(&self) -> usize {
        context_contract().expect("process inspection owner");
        self.table.live()
    }
    pub fn data_address(&self, id: ProcessId) -> Result<usize, Error> {
        context_contract()?;
        self.table.state(id)?;
        Ok(self.objects[id.slot()]
            .as_ref()
            .ok_or(Error::Stale)?
            .space
            .data_address())
    }
    /// Internal synchronous completion driver, not a public wait ABI. No default
    /// workload deadline. The future service loop may schedule another dispatch.
    pub fn dispatch(&mut self, timeout: Option<time::Duration>) -> scheduler::Completed {
        self.dispatch_inner(timeout, false)
    }
    /// Preemptible scheduling step: surviving processes stay Admitted and retain
    /// their full context/accounting. No deadline is used to make dispatch return.
    pub fn step(&mut self) -> scheduler::Completed {
        self.dispatch_inner(None, true)
    }
    fn dispatch_inner(
        &mut self,
        timeout: Option<time::Duration>,
        step: bool,
    ) -> scheduler::Completed {
        context_contract().expect("process dispatch owner");
        let tasks = core::array::from_fn(|slot| {
            let object = self.objects[slot].as_ref()?;
            if self.table.state(object.id).ok()? != State::Admitted {
                return None;
            }
            Some(Admission {
                identity: object.id,
                space: &object.space,
                context: object.context,
                slice_budget: object.slice_limit,
                slices: object.slices,
                observations: object.observations,
            })
        });
        let completed = if step {
            scheduler::step(&tasks)
        } else {
            scheduler::dispatch(&tasks, timeout)
        };
        for result in completed.tasks.iter().filter(|r| r.process_generation != 0) {
            let object = self.objects[result.id]
                .as_mut()
                .expect("admitted process retained");
            assert_eq!(
                object.id.generation(),
                result.process_generation,
                "stale scheduler process completion"
            );
            object.context = result.context;
            object.slices = result.slices;
            object.observations = result.observations;
            if step && result.state == scheduler::task::CONTEXT_READY {
                assert!(scheduler::detached(object.id));
                continue;
            }
            let reason = match result.state {
                CONTEXT_EXITED => Reason::Exited(result.context.gpr[0]),
                CONTEXT_FAULTED => Reason::Faulted {
                    class: result.fault_class,
                    address: result.fault_far,
                },
                CONTEXT_TIMED_OUT => Reason::BudgetExpired,
                _ => panic!("nonterminal process completion"),
            };
            let detached = scheduler::detached(object.id);
            self.table
                .complete(object.id, reason, detached)
                .unwrap_or_else(|error| reject(error));
            object.context = result.context;
            trace(object.id, State::Completed, None);
        }
        completed
    }
    #[cfg(feature = "kernel-tests")]
    pub fn repeat_completion(&mut self, id: ProcessId) -> Result<(), Error> {
        context_contract()?;
        let completion = self.table.completion(id)?;
        self.table
            .complete(id, completion.reason, scheduler::detached(id))
    }
    pub fn completion(&self, id: ProcessId) -> Result<Completion, Error> {
        context_contract()?;
        self.table.completion(id)
    }
    pub fn validate_completion(&self, completion: Completion) -> Result<(), Error> {
        context_contract()?;
        self.table.validate_completion(completion)
    }
    pub fn reclaim(&mut self, physical: &mut memory::Physical, id: ProcessId) -> Result<(), Error> {
        context_contract()?;
        self.table.state(id)?;
        let quiescent = scheduler::detached(id);
        self.table.reclaim(id, quiescent)?;
        trace(id, State::Reclaiming, None);
        let object = self.objects[id.slot()].take().ok_or(Error::Stale)?;
        assert_eq!(object.id, id);
        object.space.reclaim(physical);
        self.table.released(id)?;
        trace(id, State::Free, None);
        Ok(())
    }
}
pub(crate) fn reject(error: Error) -> ! {
    #[cfg(feature = "process-contract-negative")]
    crate::event!(
        "{{\"event\":\"process-reject\",\"status\":\"fail\",\"error\":\"{:?}\"}}",
        error
    );
    panic!("process contract rejection: {error:?}")
}

impl Drop for Registry {
    fn drop(&mut self) {
        assert_eq!(self.table.live(), 0, "live process registry abandonment");
    }
}
