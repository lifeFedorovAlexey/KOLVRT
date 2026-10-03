//! Only module allowed to dereference scheduler UnsafeCell storage.
use super::State;
#[cfg(feature = "machine-events")]
use super::TASKS;
use crate::{cpu, percpu};
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
#[cfg(feature = "machine-events")]
use kernel_core::execution as abi;
use kernel_core::scheduling::ownership::{Error, Ownership, Phase};
fn reject(error: Error) -> ! {
    #[cfg(feature = "scheduler-contract-negative")]
    crate::event!(
        "{{\"event\":\"scheduler-reject\",\"status\":\"fail\",\"error\":\"{:?}\"}}",
        error
    );
    panic!("scheduler ownership rejection: {error:?}");
}

pub(super) struct Local {
    state: UnsafeCell<State>,
    #[cfg(feature = "machine-events")]
    reports: UnsafeCell<[[u64; abi::REPORT_WORDS]; TASKS]>,
    ownership: Ownership,
    pub preempt: AtomicBool,
    pub resume_sp: AtomicUsize,
}
// SAFETY: INV-RUNQUEUE: every storage access owns the nonblocking acquire/release
// permit and checks observed CPU, IRQ mask, phase and generation. HRTB closures
// cannot return storage references; no permit spans ERET, waits or completion.
unsafe impl Sync for Local {}
/// Per-CPU scope also prohibits acquiring an ordinary lock from a storage callback.
struct Scope(core::marker::PhantomData<*mut ()>);
impl Scope {
    fn enter() -> Self {
        assert_eq!(
            percpu::current().scheduler_borrow.compare_exchange(
                false,
                true,
                Ordering::AcqRel,
                Ordering::Acquire
            ),
            Ok(false),
            "nested scheduler ownership contract"
        );
        Self(core::marker::PhantomData)
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        assert!(
            percpu::current()
                .scheduler_borrow
                .swap(false, Ordering::Release)
        );
    }
}
impl Local {
    pub const fn new(owner: usize) -> Self {
        Self {
            state: UnsafeCell::new(State::ZERO),
            #[cfg(feature = "machine-events")]
            reports: UnsafeCell::new([[0; abi::REPORT_WORDS]; TASKS]),
            ownership: Ownership::new(owner, percpu::BOOT_CPU),
            preempt: AtomicBool::new(false),
            resume_sp: AtomicUsize::new(0),
        }
    }
    pub fn phase(&self) -> Phase {
        self.ownership.phase()
    }
    pub fn generation(&self) -> u64 {
        self.ownership.generation()
    }
    pub fn completed(&self) -> bool {
        self.ownership.completed()
    }
    pub fn prepare(&self, generation: u64, state: State) {
        crate::sync::assert_scheduler_unlocked();
        let _access = self
            .ownership
            .prepare(percpu::id(), cpu::irq_masked(), generation)
            .unwrap_or_else(|error| reject(error));
        let _scope = Scope::enter();
        // SAFETY: INV-RUNQUEUE: exclusive preparing permit, observed coordinator,
        // IRQ masked; acquired prior completion excludes any admitted owner.
        unsafe {
            *self.state.get() = state;
        }
    }
    pub fn publish(&self, generation: u64) {
        crate::sync::assert_scheduler_unlocked();
        self.ownership
            .publish(percpu::id(), cpu::irq_masked(), generation)
            .unwrap_or_else(|error| reject(error));
    }
    pub fn start(&self, generation: u64) -> Result<(), Error> {
        crate::sync::assert_scheduler_unlocked();
        self.ownership
            .start(percpu::id(), cpu::irq_masked(), generation)
    }
    pub fn with<R>(
        &self,
        generation: u64,
        operation: impl for<'a> FnOnce(&'a mut State) -> R,
    ) -> R {
        crate::sync::assert_scheduler_unlocked();
        let _access = self
            .ownership
            .mutate(percpu::id(), cpu::irq_masked(), generation)
            .unwrap_or_else(|error| reject(error));
        let _scope = Scope::enter();
        // SAFETY: INV-RUNQUEUE: local observed CPU/IRQ/Running/generation checked,
        // exclusive permit rejects nested/concurrent entry; closure cannot leak a borrow.
        unsafe { operation(&mut *self.state.get()) }
    }
    #[cfg(feature = "machine-events")]
    pub fn with_reports<R>(
        &self,
        generation: u64,
        operation: impl for<'a> FnOnce(&'a mut State, &'a mut [[u64; abi::REPORT_WORDS]; TASKS]) -> R,
    ) -> R {
        crate::sync::assert_scheduler_unlocked();
        let _access = self
            .ownership
            .mutate(percpu::id(), cpu::irq_masked(), generation)
            .unwrap_or_else(|error| reject(error));
        let _scope = Scope::enter();
        // SAFETY: same exclusive per-CPU scheduler permit as state access protects both
        // the task queue and its index-aligned, owner-private report storage.
        unsafe { operation(&mut *self.state.get(), &mut *self.reports.get()) }
    }
    pub fn inspect<R>(&self, generation: u64, operation: impl for<'a> FnOnce(&'a State) -> R) -> R {
        self.quiescent(generation, |state| operation(state))
    }
    #[cfg(feature = "machine-events")]
    pub fn inspect_reports<R>(
        &self,
        generation: u64,
        operation: impl for<'a> FnOnce(&'a State, &'a [[u64; abi::REPORT_WORDS]; TASKS]) -> R,
    ) -> R {
        crate::sync::assert_scheduler_unlocked();
        let _access = self
            .ownership
            .inspect(percpu::id(), cpu::irq_masked(), generation)
            .unwrap_or_else(|error| reject(error));
        let _scope = Scope::enter();
        // SAFETY: acquired quiescent permit excludes execution and mutation for both
        // the task queue and report storage until this callback returns.
        unsafe { operation(&*self.state.get(), &*self.reports.get()) }
    }
    /// Coordinator-only editing after acquired completion, with the same exclusive
    /// permit as inspection. Used to unlink roots before process completion publication.
    pub fn quiescent<R>(
        &self,
        generation: u64,
        operation: impl for<'a> FnOnce(&'a mut State) -> R,
    ) -> R {
        crate::sync::assert_scheduler_unlocked();
        let _access = self
            .ownership
            .inspect(percpu::id(), cpu::irq_masked(), generation)
            .unwrap_or_else(|error| reject(error));
        let _scope = Scope::enter();
        // SAFETY: INV-RUNQUEUE: acquired Done and exclusive coordinator permit;
        // no owner can execute or change phase while borrowed; HRTB forbids escaped refs.
        unsafe { operation(&mut *self.state.get()) }
    }
    pub fn complete(&self, generation: u64, quiescent: bool) {
        crate::sync::assert_scheduler_unlocked();
        self.ownership
            .complete(percpu::id(), cpu::irq_masked(), generation, quiescent)
            .unwrap_or_else(|error| reject(error));
    }
    #[cfg(feature = "scheduler-contract-negative")]
    pub fn controls(&self, generation: u64) {
        #[cfg(feature = "scheduler-context-negative")]
        self.with(generation, |state| {
            crate::cpu::context::corrupt_mode(&mut state.tasks[0].context)
        });
        #[cfg(feature = "scheduler-start-negative")]
        self.start(generation).unwrap_or_else(|error| reject(error));
        #[cfg(feature = "scheduler-task-negative")]
        self.with(generation, |state| state.tasks[0].corrupt_generation());
        #[cfg(feature = "scheduler-foreign-negative")]
        super::LOCALS[percpu::SECONDARY_CPU].with(generation, |_| ());
        #[cfg(feature = "scheduler-reentry-negative")]
        self.with(generation, |_| super::on_timer());
        #[cfg(feature = "scheduler-stale-negative")]
        self.with(generation - 1, |_| ());
        #[cfg(feature = "scheduler-reset-negative")]
        self.prepare(generation + 1, State::ZERO);
        #[cfg(feature = "scheduler-inspect-negative")]
        self.inspect(generation, |_| ());
        #[cfg(feature = "scheduler-complete-negative")]
        self.complete(generation, false);
        #[cfg(feature = "scheduler-lock-negative")]
        {
            let lock = crate::sync::Lock::new(());
            let _held = lock.lock();
            self.with(generation, |_| ());
        }
        #[cfg(feature = "scheduler-inner-lock-negative")]
        self.with(generation, |_| {
            let lock = crate::sync::Lock::new(());
            let _held = lock.lock();
        });
        #[cfg(feature = "scheduler-irq-negative")]
        {
            crate::cpu::unmask();
            self.with(generation, |_| ());
        }
    }
}
