//! Copied caller-local lookup, close, attenuated transfer and Event revocation.
use crate::{cpu::context::Context, scheduler::task::Task, user_copy::Access};
use kernel_core::handles::{Error, Handle, Kind, Rights};
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
        Error::Rights => 9,
        Error::ReferenceExhausted => 10,
        Error::Revoked => 11,
    }
}
pub(crate) fn call(
    task: &mut Task,
    namespaces: &mut [Namespace; crate::scheduler::TASKS],
    current: usize,
    frame: &mut Context,
    operation: u16,
) -> bool {
    #[cfg(feature = "kernel-tests")]
    if testing::call(task, &mut namespaces[current], frame, operation) {
        return true;
    }
    if operation != REQUEST {
        return false;
    }
    // Trusted owner attribution precedes copying; no user-supplied process selector.
    let Some(caller) = namespaces[current].owner() else {
        frame.gpr[0] = 5;
        return true;
    };
    if caller.slot() != task.id() || caller.generation() != task.process_generation() {
        frame.gpr[0] = 4;
        return true;
    }
    let snapshot = match Access::current(task)
        .and_then(|access| access.copy_from_user::<48>(frame.gpr[0] as usize, 48))
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
    if word(1) == 3 {
        let target_slot = match usize::try_from(word(3)) {
            Ok(slot) => slot,
            Err(_) => {
                frame.gpr[0] = 1;
                return true;
            }
        };
        if target_slot / crate::scheduler::TASKS != task.id() / crate::scheduler::TASKS {
            frame.gpr[0] = status(Error::ForeignProcess);
            return true;
        }
        let receiver_index = target_slot % crate::scheduler::TASKS;
        if receiver_index == current {
            frame.gpr[0] = status(Error::Invalid);
            return true;
        }
        let Some(receiver_owner) = namespaces[receiver_index].owner() else {
            frame.gpr[0] = status(Error::Inactive);
            return true;
        };
        if receiver_owner.slot() != target_slot || receiver_owner.generation() != word(4) {
            frame.gpr[0] = status(Error::Stale);
            return true;
        }
        let requested = if word(5) <= u8::MAX as u64 {
            Rights::from_bits(word(5) as u8)
        } else {
            None
        };
        let Some(requested) = requested else {
            frame.gpr[0] = status(Error::Rights);
            return true;
        };
        let result = if current < receiver_index {
            let (before, after) = namespaces.split_at_mut(receiver_index);
            before[current].transfer(caller, handle, &mut after[0], receiver_owner, requested)
        } else {
            let (before, after) = namespaces.split_at_mut(current);
            after[0].transfer(
                caller,
                handle,
                &mut before[receiver_index],
                receiver_owner,
                requested,
            )
        };
        frame.gpr[0] = result.map(Handle::encode).unwrap_or_else(status);
        return true;
    }
    if word(1) == 4 {
        if word(3) != 0 || word(4) != 0 || word(5) != 0 {
            frame.gpr[0] = 1;
            return true;
        }
        frame.gpr[0] = namespaces[current]
            .revoke(caller, handle)
            .map(|_| 0)
            .unwrap_or_else(status);
        return true;
    }
    let kind = match word(3) {
        1 => Kind::Event,
        2 => Kind::Completion,
        _ => {
            frame.gpr[0] = 1;
            return true;
        }
    };
    if word(4) > u8::MAX as u64 || word(5) != 0 {
        frame.gpr[0] = 1;
        return true;
    }
    let required = match Rights::from_bits(word(4) as u8) {
        Some(rights) => rights,
        None => {
            frame.gpr[0] = status(Error::Rights);
            return true;
        }
    };
    let handles = &mut namespaces[current];
    frame.gpr[0] = match word(1) {
        1 => handles
            .lookup_with_rights(caller, handle, kind, required)
            .map(|_| 0)
            .unwrap_or_else(status),
        2 => match handles.lookup(caller, handle, kind) {
            Err(error) => status(error),
            Ok(_) => handles
                .close(caller, handle)
                .map(|_| 0)
                .unwrap_or_else(status),
        },
        _ => 1,
    };
    true
}
#[cfg(feature = "kernel-tests")]
pub(crate) mod testing;
