//! Coalescing event latch for one retained waiter. Slot lifetime and authority
//! belong to the caller; reset is permitted only after publishers are quiescent.
use core::sync::atomic::{AtomicU8, Ordering};
const WAITING: u8 = 1;
const PENDING: u8 = 2;
pub struct Event {
    state: AtomicU8,
}
impl Default for Event {
    fn default() -> Self {
        Self::new()
    }
}
impl Event {
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(0),
        }
    }
    /// Publish retained work before signaling. Repeated signals coalesce.
    pub fn signal(&self) -> bool {
        self.state.fetch_or(PENDING, Ordering::AcqRel) & PENDING == 0
    }
    /// Publish registration first, then recheck the pending condition.
    /// True means the caller must block and recheck before leaving its owner.
    pub fn register(&self) -> bool {
        self.state.fetch_or(WAITING, Ordering::AcqRel);
        !self.consume()
    }
    /// Only the waiter/coordinator consumes a registered notification.
    pub fn consume(&self) -> bool {
        let mut state = self.state.load(Ordering::Acquire);
        while state & (WAITING | PENDING) == WAITING | PENDING {
            match self
                .state
                .compare_exchange_weak(state, 0, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return true,
                Err(next) => state = next,
            }
        }
        false
    }
    /// Caller must exclude registration and publishers across lifetime reuse.
    pub fn reset(&self) {
        self.state.store(0, Ordering::Release);
    }
}
