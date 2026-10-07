use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};
// Attempt bound for this locking implementation; not a timed wait.
const LOCK_PROGRESS_ATTEMPTS: usize = 1_000_000;
#[cfg(target_os = "none")]
pub fn assert_scheduler_unlocked() {
    assert_eq!(
        crate::percpu::current()
            .ordinary_locks
            .load(Ordering::Acquire),
        0,
        "scheduler lock order contract"
    );
}
/// Ordinary kernel code only; IRQ and exception handlers must not acquire this lock.
/// Never nest locks, allocate, or call callbacks while holding a guard.
/// Atomic exclusion/publication supports ordinary code on both active AArch64 CPUs.
/// QEMU tests exercise handoff; hardware weak-memory and fairness remain unproven.
/// The owner must run and release within LOCK_PROGRESS_ATTEMPTS; failure panics,
/// rather than promising fairness or waiting indefinitely. IRQs are not masked here.
pub struct Lock<T> {
    held: AtomicBool,
    value: UnsafeCell<T>,
}
// SAFETY: INV-LOCK: Acquire/Release excludes access; T:Send; IRQ handler never takes this lock.
unsafe impl<T: Send> Sync for Lock<T> {}
impl<T> Lock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            held: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }
    pub fn try_lock(&self) -> Option<Guard<'_, T>> {
        #[cfg(target_os = "none")]
        {
            assert!(
                !crate::percpu::current()
                    .scheduler_borrow
                    .load(Ordering::Acquire),
                "lock inside scheduler ownership contract"
            );
        }
        if self.held.load(Ordering::Relaxed) {
            return None;
        }
        #[cfg(target_os = "none")]
        assert_eq!(
            crate::percpu::current()
                .ordinary_locks
                .load(Ordering::Acquire),
            0,
            "ordinary lock nesting contract"
        );
        self.held
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| {
                #[cfg(target_os = "none")]
                crate::percpu::current()
                    .ordinary_locks
                    .fetch_add(1, Ordering::AcqRel);
                Guard {
                    lock: self,
                    marker: core::marker::PhantomData,
                }
            })
    }
    pub fn lock(&self) -> Guard<'_, T> {
        for _ in 0..LOCK_PROGRESS_ATTEMPTS {
            if let Some(g) = self.try_lock() {
                return g;
            }
            core::hint::spin_loop();
        }
        panic!("lock progress bound exceeded");
    }
}
pub struct Guard<'a, T> {
    lock: &'a Lock<T>,
    marker: core::marker::PhantomData<(&'a mut T, *mut ())>,
}
impl<T> Deref for Guard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: INV-LOCK: guard holds exclusive ownership until Drop; reference tied to guard.
        unsafe { &*self.lock.value.get() }
    }
}
impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: INV-LOCK: exclusive guard cannot be duplicated, mutable borrow bounds lifetime.
        unsafe { &mut *self.lock.value.get() }
    }
}
impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        #[cfg(target_os = "none")]
        assert_eq!(
            crate::percpu::current()
                .ordinary_locks
                .fetch_sub(1, Ordering::AcqRel),
            1,
            "ordinary lock nesting contract"
        );
        self.lock.held.store(false, Ordering::Release);
    }
}
