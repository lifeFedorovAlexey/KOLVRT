//! Coalescing event latch for one retained waiter. Slot lifetime and authority
//! belong to the caller; reset is permitted only after publishers are quiescent.
use core::sync::atomic::{AtomicU8, Ordering};
const WAITING: u8 = 1;
const PENDING: u8 = 2;
const REVOKED: u8 = 4;
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
        self.admit_signal().unwrap_or(false)
    }
    /// Admit a new publisher unless revocation has already linearized. The CAS
    /// makes signal admission and revoke a single ordered decision.
    pub fn admit_signal(&self) -> Result<bool, ()> {
        let mut state = self.state.load(Ordering::Acquire);
        loop {
            if state & REVOKED != 0 {
                return Err(());
            }
            match self.state.compare_exchange_weak(
                state,
                state | PENDING,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(state & PENDING == 0),
                Err(next) => state = next,
            }
        }
    }
    /// Prevent future publisher admission while preserving already-pending work.
    pub fn revoke(&self) -> bool {
        self.state.fetch_or(REVOKED, Ordering::AcqRel) & REVOKED == 0
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
                .compare_exchange_weak(
                    state,
                    state & REVOKED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
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

/// Fixed-capacity shared event storage for delegated native handles. The
/// reference count owns the slot across namespaces; its generation prevents a
/// stale owner from observing a later occupant after the final release.
const SHARED_EVENT_SLOTS: usize = 512;
const MAX_SHARED_REFERENCES: usize = 1024;
const RESERVED_REFS: usize = usize::MAX;
struct SharedSlot {
    generation: core::sync::atomic::AtomicU64,
    references: core::sync::atomic::AtomicUsize,
    state: AtomicU8,
}
impl SharedSlot {
    const fn new() -> Self {
        Self {
            generation: core::sync::atomic::AtomicU64::new(0),
            references: core::sync::atomic::AtomicUsize::new(0),
            state: AtomicU8::new(0),
        }
    }
}
static SHARED_SLOTS: [SharedSlot; SHARED_EVENT_SLOTS] =
    [const { SharedSlot::new() }; SHARED_EVENT_SLOTS];

/// A bounded, clonable event target. Clones share one latch and retain its
/// storage until the final handle or accepted operation releases it.
pub struct SharedEvent {
    slot: u16,
    generation: u64,
}
impl SharedEvent {
    pub fn try_new() -> Option<Self> {
        for (index, slot) in SHARED_SLOTS.iter().enumerate() {
            if slot
                .references
                .compare_exchange(0, RESERVED_REFS, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                continue;
            }
            let Some(generation) = slot.generation.load(Ordering::Relaxed).checked_add(1) else {
                // A wrapped identity is quarantined permanently.
                continue;
            };
            slot.state.store(0, Ordering::Relaxed);
            slot.generation.store(generation, Ordering::Release);
            slot.references.store(1, Ordering::Release);
            return Some(Self {
                slot: index as u16,
                generation,
            });
        }
        None
    }
    fn storage(&self) -> &SharedSlot {
        let slot = &SHARED_SLOTS[self.slot as usize];
        assert_eq!(slot.generation.load(Ordering::Acquire), self.generation);
        assert_ne!(slot.references.load(Ordering::Acquire), 0);
        slot
    }
    pub fn try_clone(&self) -> Option<Self> {
        let slot = self.storage();
        let mut references = slot.references.load(Ordering::Acquire);
        loop {
            if references == 0 || references == RESERVED_REFS || references >= MAX_SHARED_REFERENCES
            {
                return None;
            }
            let next = references.checked_add(1)?;
            match slot.references.compare_exchange_weak(
                references,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Some(Self {
                        slot: self.slot,
                        generation: self.generation,
                    });
                }
                Err(observed) => references = observed,
            }
        }
    }
    pub fn signal(&self) -> bool {
        self.admit_signal().unwrap_or(false)
    }
    pub fn admit_signal(&self) -> Result<bool, ()> {
        let state = &self.storage().state;
        let mut observed = state.load(Ordering::Acquire);
        loop {
            if observed & REVOKED != 0 {
                return Err(());
            }
            match state.compare_exchange_weak(
                observed,
                observed | PENDING,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(observed & PENDING == 0),
                Err(next) => observed = next,
            }
        }
    }
    pub fn revoke(&self) -> bool {
        self.storage().state.fetch_or(REVOKED, Ordering::AcqRel) & REVOKED == 0
    }
    pub fn register(&self) -> bool {
        self.storage().state.fetch_or(WAITING, Ordering::AcqRel);
        !self.consume()
    }
    pub fn consume(&self) -> bool {
        let state = &self.storage().state;
        let mut observed = state.load(Ordering::Acquire);
        while observed & (WAITING | PENDING) == WAITING | PENDING {
            match state.compare_exchange_weak(
                observed,
                observed & REVOKED,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(next) => observed = next,
            }
        }
        false
    }
}
impl Drop for SharedEvent {
    fn drop(&mut self) {
        let slot = &SHARED_SLOTS[self.slot as usize];
        assert_eq!(slot.generation.load(Ordering::Acquire), self.generation);
        let previous = slot.references.fetch_sub(1, Ordering::AcqRel);
        assert!(previous > 0 && previous != RESERVED_REFS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revoke_rejects_future_admission_without_erasing_pending_work() {
        let event = Event::new();
        assert_eq!(event.admit_signal(), Ok(true));
        assert!(event.revoke());
        assert!(!event.revoke());
        assert!(event.consume());
        assert_eq!(event.admit_signal(), Err(()));
        assert!(!event.signal());
    }

    #[test]
    fn shared_alias_observes_revocation_after_owner_close() {
        let owner = SharedEvent::try_new().unwrap();
        let alias = owner.try_clone().unwrap();
        assert_eq!(alias.admit_signal(), Ok(true));
        owner.revoke();
        drop(owner);
        assert_eq!(alias.admit_signal(), Err(()));
        assert!(alias.consume());
    }
}
