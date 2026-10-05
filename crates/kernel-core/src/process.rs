//! Narrow process identity/lifetime protocol. Identity confers no authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessId {
    slot: usize,
    generation: u64,
}
impl ProcessId {
    pub fn slot(self) -> usize {
        self.slot
    }
    pub fn generation(self) -> u64 {
        self.generation
    }
    #[cfg(feature = "ipc-wake-generation-negative")]
    pub fn with_generation_for_test(self, generation: u64) -> Self {
        Self {
            slot: self.slot,
            generation,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Free,
    Creating,
    Prepared,
    Admitted,
    Completed,
    Reclaiming,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreationStep {
    Slot,
    Frames,
    Space,
    Context,
    Commit,
}
pub const CREATION_STEPS: [CreationStep; 5] = [
    CreationStep::Slot,
    CreationStep::Frames,
    CreationStep::Space,
    CreationStep::Context,
    CreationStep::Commit,
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Exited(u64),
    Faulted { class: u64, address: usize },
    BudgetExpired,
    CreationFailed(CreationStep),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    AlreadyOwned,
    RollbackLeak,
    ForeignCpu,
    IrqEnabled,
    Unauthorized,
    Capacity,
    GenerationExhausted,
    Stale,
    Transition,
    NotQuiescent,
    InvalidImage,
    Allocation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completion {
    pub id: ProcessId,
    pub reason: Reason,
}
#[derive(Clone, Copy)]
struct Slot {
    generation: u64,
    state: State,
    reason: Option<Reason>,
}
impl Slot {
    const FREE: Self = Self {
        generation: 0,
        state: State::Free,
        reason: None,
    };
}
pub struct Table<const N: usize> {
    slots: [Slot; N],
}
impl<const N: usize> Default for Table<N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<const N: usize> Table<N> {
    pub const fn new() -> Self {
        Self {
            slots: [Slot::FREE; N],
        }
    }
    pub fn reserve(&mut self, range: core::ops::Range<usize>) -> Result<ProcessId, Error> {
        if range.start >= range.end || range.end > N {
            return Err(Error::ForeignCpu);
        }
        let slot = range
            .clone()
            .find(|&i| self.slots[i].state == State::Free && self.slots[i].generation != u64::MAX)
            .ok_or_else(|| {
                if range.clone().any(|i| self.slots[i].state == State::Free) {
                    Error::GenerationExhausted
                } else {
                    Error::Capacity
                }
            })?;
        let record = &mut self.slots[slot];
        record.generation = record
            .generation
            .checked_add(1)
            .ok_or(Error::GenerationExhausted)?;
        record.state = State::Creating;
        record.reason = None;
        Ok(ProcessId {
            slot,
            generation: record.generation,
        })
    }
    fn get(&self, id: ProcessId) -> Result<&Slot, Error> {
        let slot = self.slots.get(id.slot).ok_or(Error::Stale)?;
        if slot.generation != id.generation || slot.state == State::Free {
            return Err(Error::Stale);
        }
        Ok(slot)
    }
    pub fn state(&self, id: ProcessId) -> Result<State, Error> {
        Ok(self.get(id)?.state)
    }
    fn transition(&mut self, id: ProcessId, from: State, to: State) -> Result<(), Error> {
        if self.get(id)?.state != from {
            return Err(Error::Transition);
        }
        self.slots[id.slot].state = to;
        Ok(())
    }
    pub fn prepared(&mut self, id: ProcessId) -> Result<(), Error> {
        self.transition(id, State::Creating, State::Prepared)
    }
    pub fn start(&mut self, id: ProcessId) -> Result<(), Error> {
        self.transition(id, State::Prepared, State::Admitted)
    }
    pub fn rollback(&mut self, id: ProcessId, step: CreationStep) -> Result<Completion, Error> {
        self.transition(id, State::Creating, State::Free)?;
        Ok(Completion {
            id,
            reason: Reason::CreationFailed(step),
        })
    }
    /// Caller must derive quiescence from acquired scheduler detachment, never a deadline.
    pub fn complete(&mut self, id: ProcessId, reason: Reason, detached: bool) -> Result<(), Error> {
        if !detached {
            return Err(Error::NotQuiescent);
        }
        if matches!(reason, Reason::CreationFailed(_)) {
            return Err(Error::Transition);
        }
        self.transition(id, State::Admitted, State::Completed)?;
        self.slots[id.slot].reason = Some(reason);
        Ok(())
    }
    pub fn completion(&self, id: ProcessId) -> Result<Completion, Error> {
        let slot = self.get(id)?;
        if slot.state != State::Completed {
            return Err(Error::Transition);
        }
        Ok(Completion {
            id,
            reason: slot.reason.ok_or(Error::Transition)?,
        })
    }
    pub fn reclaim(&mut self, id: ProcessId, quiescent: bool) -> Result<(), Error> {
        if !quiescent {
            return Err(Error::NotQuiescent);
        }
        self.transition(id, State::Completed, State::Reclaiming)
    }
    pub fn released(&mut self, id: ProcessId) -> Result<(), Error> {
        self.transition(id, State::Reclaiming, State::Free)
    }
    pub fn validate_completion(&self, completion: Completion) -> Result<(), Error> {
        if self.completion(completion.id)? != completion {
            return Err(Error::Stale);
        }
        Ok(())
    }
    pub fn live(&self) -> usize {
        self.slots.iter().filter(|s| s.state != State::Free).count()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_terminal_detachment_and_exact_reuse() {
        let mut t = Table::<1>::new();
        let old = t.reserve(0..1).unwrap();
        assert_eq!(t.start(old), Err(Error::Transition));
        t.prepared(old).unwrap();
        t.start(old).unwrap();
        assert_eq!(t.start(old), Err(Error::Transition));
        assert_eq!(t.reclaim(old, true), Err(Error::Transition));
        assert_eq!(
            t.complete(old, Reason::Exited(7), false),
            Err(Error::NotQuiescent)
        );
        t.complete(old, Reason::Exited(7), true).unwrap();
        let completion = t.completion(old).unwrap();
        assert_eq!(
            t.complete(old, Reason::Exited(7), true),
            Err(Error::Transition)
        );
        assert_eq!(t.reclaim(old, false), Err(Error::NotQuiescent));
        t.reclaim(old, true).unwrap();
        t.released(old).unwrap();
        assert_eq!(t.released(old), Err(Error::Stale));
        let new = t.reserve(0..1).unwrap();
        assert_eq!(new.slot(), old.slot());
        assert_ne!(new.generation(), old.generation());
        assert_eq!(t.start(old), Err(Error::Stale));
        assert_eq!(t.validate_completion(completion), Err(Error::Stale));
    }
    #[test]
    fn every_rollback_burns_identity_and_exhaustion_never_overwrites() {
        let mut t = Table::<1>::new();
        for step in CREATION_STEPS {
            let id = t.reserve(0..1).unwrap();
            assert_eq!(t.reserve(0..1), Err(Error::Capacity));
            assert_eq!(
                t.rollback(id, step).unwrap().reason,
                Reason::CreationFailed(step)
            );
            assert_eq!(t.live(), 0);
            assert_eq!(t.state(id), Err(Error::Stale));
        }
        t.slots[0].generation = u64::MAX;
        assert_eq!(t.reserve(0..1), Err(Error::GenerationExhausted));
        assert_eq!(t.reserve(1..2), Err(Error::ForeignCpu));
    }
}
