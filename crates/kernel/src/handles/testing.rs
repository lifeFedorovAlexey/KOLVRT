//! Actual current-task EL0 exercise; resource creation is trusted fixture only.
use super::*;
use crate::{
    memory,
    process::{Origin, Reason, Spec},
    user_copy,
};
use core::sync::atomic::{AtomicU64, Ordering};
use kernel_core::{
    handles::{CreationError, Namespace as Table},
    process::{Completion, Table as Processes},
    wait::Event,
};
static LAST: [AtomicU64; crate::platform::config::ACTIVE_CPUS] =
    [const { AtomicU64::new(0) }; crate::platform::config::ACTIVE_CPUS];
pub(super) fn call(
    task: &mut Task,
    table: &mut Namespace,
    frame: &mut Context,
    operation: u16,
) -> bool {
    if operation != 0x81 && operation != 0x82 {
        return false;
    }
    let caller = table.owner().expect("admitted namespace");
    let access = Access::current(task).expect("current caller");
    let address = frame.gpr[1] as usize;
    if operation == 0x82 {
        let a = table.create_event(caller, Event::new(), |h| {
            access.copy_to_user(address + 64, &h.encode().to_le_bytes())
        });
        let b = table.create_completion(
            caller,
            Completion {
                id: caller,
                reason: Reason::Exited(17),
            },
            |h| access.copy_to_user(address + 72, &h.encode().to_le_bytes()),
        );
        frame.gpr[0] = u64::from(a.is_ok() && b.is_ok());
        return true;
    }
    let old = Handle::decode(LAST[crate::percpu::id()].load(Ordering::Acquire));
    let stale_process = table.lookup(caller, old, Kind::Event).is_err();
    let failed = table.create_event(caller, Event::new(), |h| {
        access.copy_to_user(memory::USER_GUARD - 4, &h.encode().to_le_bytes())
    });
    let rollback = matches!(
        failed,
        Err(CreationError::Publication(user_copy::Error::UserFault {
            copied: 0
        }))
    ) && table.live() == 0;
    let h = table
        .create_event(caller, Event::new(), |_| Ok::<_, ()>(()))
        .unwrap();
    let mut identities = Processes::<2>::new();
    let a = identities.reserve(0..1).unwrap();
    let b = identities.reserve(1..2).unwrap();
    let foreign = if a.slot() == caller.slot() { b } else { a };
    let owner = table.lookup(foreign, h, Kind::Event).err() == Some(Error::ForeignProcess);
    let kind = table.lookup(caller, h, Kind::Completion).err() == Some(Error::WrongType);
    let retained = {
        let r = table.lookup(caller, h, Kind::Event).unwrap();
        let e = r.event().unwrap();
        e.signal() && !e.register()
    };
    table.close(caller, h).unwrap();
    let double = table.close(caller, h) == Err(Error::Stale);
    let fresh = table
        .create_event(caller, Event::new(), |_| Ok::<_, ()>(()))
        .unwrap();
    let generation = fresh != h && table.lookup(caller, h, Kind::Event).err() == Some(Error::Stale);
    table.close(caller, fresh).unwrap();
    let mut exhaustion = Table::<1, 2>::new();
    exhaustion.bind(caller).unwrap();
    for _ in 0..2 {
        let h = exhaustion
            .create_event(caller, Event::new(), |_| Ok::<_, ()>(()))
            .unwrap();
        exhaustion.close(caller, h).unwrap();
    }
    let wrap = exhaustion
        .create_event(caller, Event::new(), |_| Ok::<_, ()>(()))
        .err()
        == Some(CreationError::Handle(Error::GenerationExhausted));
    let mut filled = [Handle::decode(0); CAPACITY];
    for h in &mut filled {
        *h = table
            .create_event(caller, Event::new(), |_| Ok::<_, ()>(()))
            .unwrap();
    }
    let capacity = table
        .create_event(caller, Event::new(), |_| Ok::<_, ()>(()))
        .err()
        == Some(CreationError::Handle(Error::Capacity));
    for h in filled {
        table.close(caller, h).unwrap();
    }
    let mut stress = true;
    for _ in 0..64 {
        let h = table
            .create_event(caller, Event::new(), |_| Ok::<_, ()>(()))
            .unwrap();
        table.close(caller, h).unwrap();
        stress &= table.lookup(caller, h, Kind::Event).is_err();
    }
    let h = table
        .create_event(caller, Event::new(), |h| {
            access.copy_to_user(address + 64, &h.encode().to_le_bytes())
        })
        .unwrap();
    LAST[crate::percpu::id()].store(h.encode(), Ordering::Release);
    frame.gpr[0] = u64::from(
        stale_process
            && rollback
            && owner
            && kind
            && retained
            && double
            && generation
            && wrap
            && capacity
            && stress,
    );
    true
}
unsafe extern "C" {
    static handle_foreign_start: u8;
    static handle_foreign_end: u8;
    static handle_image_start: u8;
    static handle_image_end: u8;
}
pub(crate) fn exercise(
    physical: &mut memory::Physical,
    registry: &mut crate::process::Registry,
    mut report: impl FnMut(&str, bool),
) {
    let before = physical.available();
    let start = &raw const handle_image_start as usize;
    let end = &raw const handle_image_end as usize;
    assert!(end > start && end - start <= crate::platform::config::PAGE_BYTES);
    // SAFETY: INV-USER-IMAGE: immutable trusted linker fixture, checked page bound.
    let image = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
    let foreign_start = &raw const handle_foreign_start as usize;
    let foreign_end = &raw const handle_foreign_end as usize;
    assert!(
        foreign_end > foreign_start
            && foreign_end - foreign_start <= crate::platform::config::PAGE_BYTES
    );
    // SAFETY: INV-USER-IMAGE: bounded immutable linked foreign-handle fixture.
    let foreign_image = unsafe {
        core::slice::from_raw_parts(foreign_start as *const u8, foreign_end - foreign_start)
    };
    let foreign_ids: [_; crate::platform::config::ACTIVE_CPUS] = core::array::from_fn(|owner| {
        let mut context = Context::ZERO;
        context.pc = memory::USER_CODE as u64;
        context.sp = memory::USER_STACK_TOP as u64;
        registry
            .create(
                physical,
                Origin::Bootstrap,
                Spec {
                    image: foreign_image,
                    context,
                    owner,
                    entry: memory::USER_CODE,
                    slice_limit: None,
                },
                None,
            )
            .unwrap()
    });
    let foreign_handle = registry.seed_handle(foreign_ids[0]);
    for (owner, &id) in foreign_ids.iter().enumerate() {
        registry.handle_input(id, foreign_handle.encode(), if owner == 0 { 0 } else { 2 });
        registry.start(id).unwrap();
    }
    registry.dispatch(Some(crate::time::Duration::from_secs(5)));
    report(
        "handle_cross_process_reference_isolation",
        foreign_ids
            .iter()
            .all(|&id| registry.completion(id).unwrap().reason == Reason::Exited(1)),
    );
    for id in foreign_ids {
        registry.reclaim(physical, id).unwrap();
    }
    let mut passed = true;
    for round in 0..8 {
        let ids: [_; crate::platform::config::ACTIVE_CPUS] = core::array::from_fn(|owner| {
            let mut context = Context::ZERO;
            context.pc = memory::USER_CODE as u64;
            context.sp = memory::USER_STACK_TOP as u64;
            context.gpr[2] = u64::from(round % 2 == 1);
            let id = registry
                .create(
                    physical,
                    Origin::Bootstrap,
                    Spec {
                        image,
                        context,
                        owner,
                        entry: memory::USER_CODE,
                        slice_limit: None,
                    },
                    None,
                )
                .unwrap();
            registry.start(id).unwrap();
            id
        });
        registry.dispatch(Some(crate::time::Duration::from_secs(5)));
        for id in ids {
            let reason = registry.completion(id).unwrap().reason;
            passed &= if round % 2 == 0 {
                reason == Reason::Exited(1)
            } else {
                matches!(reason, Reason::Faulted { .. })
            };
            passed &= registry.handle_state(id) == (0, false);
            registry.reclaim(physical, id).unwrap();
        }
    }
    report("handle_el0_identity_type_generation_and_lifetime", passed);
    report(
        "handle_exit_fault_cleanup_and_process_reuse",
        registry.live() == 0 && physical.available() == before,
    );
}
