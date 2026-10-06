//! Owned split-syscall continuation. Preparation/copy requires scheduler scope;
//! endpoint execution and reservation completion require that scope to be gone.
use crate::{cpu::context::Context, handles::Namespace, scheduler::task::Task, user_copy::Access};
use core::sync::atomic::{AtomicBool, Ordering};
use kernel_core::{
    domain,
    handles::{Handle, Kind},
    ipc::{
        self, Error, Reference,
        wire::{Input, Operation, Output},
    },
    process::ProcessId,
};

static WAIT_REGISTRATION_OMITTED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "ipc-wait-recheck-negative")]
pub(crate) fn wait_registration_omitted() -> bool {
    WAIT_REGISTRATION_OMITTED.load(Ordering::Acquire)
}

pub(crate) const REQUEST: u16 = 0xa0;
pub(crate) struct Prepared {
    caller: ProcessId,
    domain: domain::Reference,
    input: Input,
    endpoint: Option<Reference>,
    output: usize,
    output_capacity: usize,
    wait: Option<ipc::WaitKey>,
}
impl Prepared {
    pub(crate) fn blocking(&self) -> bool {
        matches!(self.input.operation, Operation::Receive | Operation::Wait)
    }
}
pub(crate) enum Action {
    Returned(u64),
    Receive(ipc::Delivery),
    Collect(ipc::Collection),
    Blocked {
        key: ipc::WaitKey,
        target: ipc::WaitTarget,
    },
}
pub(crate) enum Failure {
    State(Error),
    Contended,
}
impl Failure {
    fn storage(error: Error) -> Self {
        if error == Error::Busy {
            Self::Contended
        } else {
            Self::State(error)
        }
    }
    pub(crate) fn retry(&self, blocking: bool) -> bool {
        matches!(self, Self::Contended) || (blocking && matches!(self, Self::State(Error::Busy)))
    }
    pub(crate) fn status(self) -> u64 {
        match self {
            Self::State(error) => status(error),
            Self::Contended => status(Error::Busy),
        }
    }
}
/// Executing identity is derived from the current namespace and immutable task,
/// never a principal selector in the copied request. All owned values survive
/// dropping the scope; neither Access nor a namespace borrow escapes.
pub(crate) fn prepare(
    task: &Task,
    namespace: &Namespace,
    frame: &Context,
) -> Result<Prepared, u64> {
    let caller = namespace.owner().ok_or(5_u64)?;
    if caller.slot() != task.id() || caller.generation() != task.process_generation() {
        return Err(4);
    }
    let length = usize::try_from(frame.gpr[1]).map_err(|_| 1_u64)?;
    let snapshot = Access::current(task)
        .and_then(|access| {
            access.copy_from_user::<{ ipc::wire::MAX_FRAME }>(frame.gpr[0] as usize, length)
        })
        .map_err(|_| 8_u64)?;
    let input = Input::decode(snapshot.bytes()).map_err(status)?;
    let endpoint = if matches!(
        input.operation,
        Operation::Submit
            | Operation::Receive
            | Operation::Commit
            | Operation::Reply
            | Operation::Shutdown
    ) {
        let retained = namespace
            .lookup(caller, Handle::decode(input.selector), Kind::Endpoint)
            .map_err(crate::handles::status)?;
        let reference = if input.operation == Operation::Submit {
            retained.endpoint_sender()
        } else {
            retained.endpoint_receiver(caller)
        }
        .map_err(crate::handles::status)?;
        Some(reference.try_clone().map_err(status)?)
    } else {
        None
    };
    let domain = namespace.domain().map_err(crate::handles::status)?.clone();
    let wait = if matches!(input.operation, Operation::Receive | Operation::Wait) {
        Some(ipc::WaitKey::new(caller).map_err(status)?)
    } else {
        None
    };
    Ok(Prepared {
        caller,
        domain,
        input,
        endpoint,
        output: frame.gpr[2] as usize,
        output_capacity: usize::try_from(frame.gpr[3]).map_err(|_| 1_u64)?,
        wait,
    })
}
pub(crate) fn execute(prepared: &Prepared, now: u64) -> Result<Action, Failure> {
    let input = &prepared.input;
    if cfg!(feature = "native-ipc-negative")
        && input.operation == Operation::Submit
        && input.id >= 10
    {
        return Err(Failure::State(Error::Invalid));
    }
    if let Some(endpoint) = &prepared.endpoint {
        super::storage::with(endpoint, |state| match input.operation {
            Operation::Submit => state
                .submit(
                    endpoint,
                    &prepared.domain,
                    input.id,
                    input.deadline,
                    input.payload.clone(),
                    now,
                )
                .map(|id| Action::Returned(id.receipt())),
            Operation::Receive => match state.reserve_receive(prepared.caller, now) {
                Ok(delivery) => Ok(Action::Receive(delivery)),
                Err(Error::Busy) => {
                    let key = prepared.wait.expect("receive wait identity prepared");
                    if state.wait_readable(prepared.caller, key)? {
                        return Err(Error::Busy);
                    }
                    if cfg!(feature = "ipc-wait-recheck-negative") {
                        WAIT_REGISTRATION_OMITTED.store(true, Ordering::Release);
                    }
                    Ok(Action::Blocked {
                        key,
                        target: ipc::WaitTarget::Readable(endpoint.id()),
                    })
                }
                Err(error) => Err(error),
            },
            Operation::Commit => state
                .commit(prepared.caller, ipc::ServiceToken::decode(input.id), now)
                .map(|()| Action::Returned(0)),
            Operation::Reply => state
                .reply(
                    prepared.caller,
                    ipc::ServiceToken::decode(input.id),
                    input.payload.clone(),
                    now,
                )
                .map(|outcome| Action::Returned(outcome.encode() as u64)),
            Operation::Shutdown => {
                state.shutdown();
                Ok(Action::Returned(0))
            }
            _ => Err(Error::Invalid),
        })
        .map_err(Failure::storage)?
        .map_err(Failure::State)
    } else {
        let (endpoint, id) =
            super::storage::receipt(prepared.caller, input.selector).map_err(Failure::storage)?;
        super::storage::with(&endpoint, |state| {
            state.expire(now);
            match input.operation {
                Operation::Collect => state
                    .reserve_collect(prepared.caller, id)
                    .map(Action::Collect),
                Operation::Cancel => state
                    .cancel(prepared.caller, id, now)
                    .map(|outcome| Action::Returned(outcome.encode() as u64)),
                Operation::Abandon => state
                    .abandon(prepared.caller, id)
                    .map(|()| Action::Returned(0)),
                Operation::Wait => {
                    let key = prepared.wait.expect("terminal wait identity prepared");
                    if state.wait_terminal(prepared.caller, id, key)? {
                        Ok(Action::Returned(0))
                    } else {
                        if cfg!(feature = "ipc-wait-recheck-negative") {
                            WAIT_REGISTRATION_OMITTED.store(true, Ordering::Release);
                        }
                        Ok(Action::Blocked {
                            key,
                            target: ipc::WaitTarget::Terminal(endpoint.id(), id),
                        })
                    }
                }
                _ => Err(Error::Invalid),
            }
        })
        .map_err(Failure::storage)?
        .map_err(Failure::State)
    }
}
/// Copy-out alone, inside a reacquired exact executing task scope. Caller must
/// retain Action until endpoint reservation completion has actually succeeded.
pub(crate) fn copy(prepared: &Prepared, task: &Task, action: &Action) -> Result<usize, u64> {
    if task.id() != prepared.caller.slot()
        || task.process_generation() != prepared.caller.generation()
    {
        return Err(4);
    }
    let output = match action {
        Action::Receive(delivery) => Output::received(
            delivery.token,
            delivery.client_id,
            delivery.deadline,
            &delivery.payload,
        ),
        Action::Collect(collection) => Output::result(
            collection.client_id,
            collection.outcome,
            &collection.payload,
        ),
        Action::Returned(_) | Action::Blocked { .. } => return Ok(0),
    };
    if output.bytes().len() > prepared.output_capacity {
        return Err(1);
    }
    Access::current(task)
        .and_then(|access| access.copy_to_user(prepared.output, output.bytes()))
        .map_err(|_| 8_u64)?;
    Ok(output.bytes().len())
}
/// Nonblocking completion outside scheduler scope. Busy retains the owned action
/// for a deferred continuation; dropping it on Busy would leak a reservation.
pub(crate) fn finish(action: &Action, copied: bool) -> Result<(), Error> {
    match action {
        Action::Receive(delivery) => super::storage::with(delivery.endpoint(), |state| {
            state.finish_receive(delivery, copied)
        })?,
        Action::Collect(collection) => super::storage::with(collection.endpoint(), |state| {
            state.finish_collect(collection, copied)
        })?,
        Action::Returned(_) | Action::Blocked { .. } => Ok(()),
    }
}
pub(crate) fn status(error: Error) -> u64 {
    match error {
        Error::Invalid => 1,
        Error::Stale => 2,
        Error::Denied => 11,
        Error::Exhausted => 12,
        Error::Unsupported => 13,
        Error::Busy => 14,
        Error::AlreadyTerminal | Error::ChargeAlreadyReleased => 15,
        Error::Expired => 16,
    }
}
