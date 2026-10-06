//! Test callers of the actual private scheduler adapter; no replacement implementation.
use super::*;
pub(crate) fn forbidden_input() {
    #[cfg(feature = "scheduler-inner-lock-input")]
    {
        static LOCAL: Local = Local::new(percpu::BOOT_CPU);
        let local = &LOCAL;
        local.prepare(1, |_| {});
        local.publish(1);
        local.start(1).unwrap();
        let lock = crate::sync::Lock::new(0);
        local.with(1, |_| {
            drop(lock.try_lock());
        });
    }
    #[cfg(feature = "scheduler-lock-input")]
    {
        static LOCAL: Local = Local::new(percpu::BOOT_CPU);
        let local = &LOCAL;
        let lock = crate::sync::Lock::new(0);
        let _guard = lock.lock();
        local.prepare(1, |_| {});
    }
    #[cfg(feature = "scheduler-task-input")]
    {
        let mut input = Task::ZERO;
        input.bind_queue(1);
        validate_task(&input, 0, 2);
    }
    #[cfg(feature = "scheduler-owner-input")]
    {
        static LOCAL: Local = Local::new(percpu::BOOT_CPU);
        LOCAL.prepare(1, |input| {
            input.tasks[0].bind_queue(1);
            input.tasks[0].state = CONTEXT_READY;
            input.tasks[1].state = CONTEXT_RUNNING;
        });
        LOCAL.publish(1);
        LOCAL.start(1).unwrap();
        LOCAL.with(1, |input| {
            choose(input, 1);
        });
    }

    #[cfg(feature = "ipc-storage-scope-input")]
    {
        static LOCAL: Local = Local::new(percpu::BOOT_CPU);
        LOCAL.prepare(1, |_| {});
        LOCAL.publish(1);
        LOCAL.start(1).unwrap();
        LOCAL.with(1, |_| {
            crate::ipc::deferred::poll(&[]);
        });
    }
    #[cfg(any(
        feature = "ipc-duplicate-ready-input",
        feature = "ipc-wrong-process-wake-input"
    ))]
    {
        let mut identities = kernel_core::process::Table::<2>::new();
        let caller = identities.reserve(0..1).unwrap();
        let foreign = identities.reserve(1..2).unwrap();
        // This fixture never enters EL0 or activates a translation root.
        let lease = crate::asid::Lease {
            asid: 0,
            epoch: 0,
            owner: 0,
            slot: 0,
        };
        let mut input = Task::new(
            caller.slot(),
            caller.generation(),
            0,
            lease,
            Context::ZERO,
            None,
        );
        #[cfg(all(
            feature = "ipc-duplicate-ready-input",
            not(feature = "ipc-wrong-process-wake-input")
        ))]
        let key = kernel_core::ipc::WaitKey::new(caller).unwrap();
        #[cfg(feature = "ipc-wrong-process-wake-input")]
        let key = kernel_core::ipc::WaitKey::new(foreign).unwrap();
        #[cfg(not(feature = "ipc-wrong-process-wake-input"))]
        let _ = foreign;
        input.publish_ipc_ready(key);
    }
    #[cfg(feature = "ipc-blocked-reclaim-input")]
    {
        let mut input = Task::ZERO;
        input.state = CONTEXT_BLOCKED;
        input.unlink_ipc();
    }
    panic!("forbidden scheduler input was accepted");
}
