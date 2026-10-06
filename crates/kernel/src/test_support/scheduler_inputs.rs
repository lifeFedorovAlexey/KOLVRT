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
    panic!("forbidden scheduler input was accepted");
}
