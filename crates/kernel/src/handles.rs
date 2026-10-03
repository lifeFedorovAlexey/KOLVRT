//! Narrow process-local reference boundary. No authority or delegation.
use crate::{cpu::context::Context, scheduler::task::Task, user_copy::Access};
use kernel_core::handles::{Error, Handle, Kind};
pub(crate) const CAPACITY: usize = 8;
pub(crate) type Namespace = kernel_core::handles::Namespace<CAPACITY>;
pub(crate) const REQUEST: u16 = 0x80;
fn status(error: Error) -> u64 {
    match error {
        Error::Invalid => 1,
        Error::Stale => 2,
        Error::WrongType => 3,
        Error::ForeignProcess => 4,
        Error::Inactive => 5,
        Error::Capacity => 6,
        Error::GenerationExhausted => 7,
    }
}
pub(crate) fn call(
    task: &mut Task,
    handles: &mut Namespace,
    frame: &mut Context,
    operation: u16,
) -> bool {
    #[cfg(feature = "kernel-tests")]
    if testing::call(task, handles, frame, operation) {
        return true;
    }
    if operation != REQUEST {
        return false;
    }
    // Trusted owner attribution precedes copying; no user-supplied process selector.
    let Some(caller) = handles.owner() else {
        frame.gpr[0] = 5;
        return true;
    };
    if caller.slot() != task.id() || caller.generation() != task.process_generation() {
        frame.gpr[0] = 4;
        return true;
    }
    let snapshot = match Access::current(task)
        .and_then(|access| access.copy_from_user::<32>(frame.gpr[0] as usize, 32))
    {
        Ok(s) => s,
        Err(_) => {
            frame.gpr[0] = 8;
            return true;
        }
    };
    let bytes = snapshot.bytes();
    let word = |i: usize| {
        u64::from_le_bytes(
            bytes[i * 8..(i + 1) * 8]
                .try_into()
                .expect("bounded wire word"),
        )
    };
    if word(0) != 1 {
        frame.gpr[0] = 1;
        return true;
    }
    let handle = Handle::decode(word(2));
    let kind = match word(3) {
        1 => Kind::Event,
        2 => Kind::Completion,
        _ => {
            frame.gpr[0] = 1;
            return true;
        }
    };
    frame.gpr[0] = match word(1) {
        1 => handles
            .lookup(caller, handle, kind)
            .map(|_| 0)
            .unwrap_or_else(status),
        2 => handles
            .close(caller, handle)
            .map(|_| 0)
            .unwrap_or_else(status),
        _ => 1,
    };
    true
}
#[cfg(feature = "kernel-tests")]
pub(crate) mod testing;
