//! Nonblocking admission protocol. The kernel supplies observed CPU/IRQ state;
//! this safe model grants exclusion, not authority to dereference kernel storage.
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Phase {
    Idle,
    Preparing,
    Admitted,
    Running,
    Done,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    ForeignCpu,
    IrqEnabled,
    Reentry,
    StaleGeneration,
    WrongPhase,
    NotQuiescent,
    GenerationExhausted,
}
pub struct Ownership {
    owner: usize,
    coordinator: usize,
    phase: AtomicUsize,
    generation: AtomicU64,
    borrowed: AtomicBool,
}
impl Ownership {
    pub const fn new(owner: usize, coordinator: usize) -> Self {
        Self {
            owner,
            coordinator,
            phase: AtomicUsize::new(Phase::Idle as usize),
            generation: AtomicU64::new(0),
            borrowed: AtomicBool::new(false),
        }
    }
    pub fn phase(&self) -> Phase {
        match self.phase.load(Ordering::Acquire) {
            value if value == Phase::Idle as usize => Phase::Idle,
            value if value == Phase::Preparing as usize => Phase::Preparing,
            value if value == Phase::Admitted as usize => Phase::Admitted,
            value if value == Phase::Running as usize => Phase::Running,
            value if value == Phase::Done as usize => Phase::Done,
            _ => unreachable!("invalid scheduler phase"),
        }
    }
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }
    pub fn completed(&self) -> bool {
        self.phase() == Phase::Done && !self.borrowed.load(Ordering::Acquire)
    }
    fn gate(&self, cpu: usize, expected: usize, masked: bool) -> Result<Access<'_>, Error> {
        if cpu != expected {
            return Err(Error::ForeignCpu);
        }
        if !masked {
            return Err(Error::IrqEnabled);
        }
        self.borrowed
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| Error::Reentry)?;
        Ok(Access { ownership: self })
    }
    fn checked(
        &self,
        cpu: usize,
        expected: usize,
        masked: bool,
        generation: u64,
        phase: Phase,
    ) -> Result<Access<'_>, Error> {
        let access = self.gate(cpu, expected, masked)?;
        if self.generation() != generation {
            return Err(Error::StaleGeneration);
        }
        if self.phase() != phase {
            return Err(Error::WrongPhase);
        }
        Ok(access)
    }
    pub fn prepare(&self, cpu: usize, masked: bool, generation: u64) -> Result<Access<'_>, Error> {
        let access = self.gate(cpu, self.coordinator, masked)?;
        if !matches!(self.phase(), Phase::Idle | Phase::Done) {
            return Err(Error::WrongPhase);
        }
        let next = self
            .generation()
            .checked_add(1)
            .ok_or(Error::GenerationExhausted)?;
        if generation != next {
            return Err(Error::StaleGeneration);
        }
        self.generation.store(generation, Ordering::Relaxed);
        self.phase
            .store(Phase::Preparing as usize, Ordering::Release);
        Ok(access)
    }
    pub fn publish(&self, cpu: usize, masked: bool, generation: u64) -> Result<(), Error> {
        let _access = self.checked(cpu, self.coordinator, masked, generation, Phase::Preparing)?;
        self.phase
            .store(Phase::Admitted as usize, Ordering::Release);
        Ok(())
    }
    pub fn start(&self, cpu: usize, masked: bool, generation: u64) -> Result<(), Error> {
        let _access = self.checked(cpu, self.owner, masked, generation, Phase::Admitted)?;
        self.phase.store(Phase::Running as usize, Ordering::Release);
        Ok(())
    }
    pub fn mutate(&self, cpu: usize, masked: bool, generation: u64) -> Result<Access<'_>, Error> {
        self.checked(cpu, self.owner, masked, generation, Phase::Running)
    }
    pub fn inspect(&self, cpu: usize, masked: bool, generation: u64) -> Result<Access<'_>, Error> {
        self.checked(cpu, self.coordinator, masked, generation, Phase::Done)
    }
    pub fn complete(
        &self,
        cpu: usize,
        masked: bool,
        generation: u64,
        quiescent: bool,
    ) -> Result<(), Error> {
        let _access = self.checked(cpu, self.owner, masked, generation, Phase::Running)?;
        if !quiescent {
            return Err(Error::NotQuiescent);
        }
        self.phase.store(Phase::Done as usize, Ordering::Release);
        Ok(())
    }
}
/// Private noncloneable permit: no waiting or nested access. Release publishes
/// completed access; phase transitions and inspection use acquire ordering.
pub struct Access<'a> {
    ownership: &'a Ownership,
}
impl Drop for Access<'_> {
    fn drop(&mut self) {
        self.ownership.borrowed.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    const COORDINATOR: usize = 0;
    const OWNER: usize = 1;
    const FIRST: u64 = 1;
    fn running() -> Ownership {
        let state = Ownership::new(OWNER, COORDINATOR);
        drop(state.prepare(COORDINATOR, true, FIRST).unwrap());
        state.publish(COORDINATOR, true, FIRST).unwrap();
        state.start(OWNER, true, FIRST).unwrap();
        state
    }
    #[test]
    fn foreign_cpu_and_unmasked_access_are_rejected() {
        let state = running();
        assert!(matches!(
            state.mutate(COORDINATOR, true, FIRST),
            Err(Error::ForeignCpu)
        ));
        assert!(matches!(
            state.mutate(OWNER, false, FIRST),
            Err(Error::IrqEnabled)
        ));
        assert!(matches!(
            state.prepare(OWNER, true, FIRST + 1),
            Err(Error::ForeignCpu)
        ));
    }
    #[test]
    fn reentry_does_not_wait_or_issue_a_second_permit() {
        let state = running();
        let permit = state.mutate(OWNER, true, FIRST).unwrap();
        assert!(matches!(
            state.mutate(OWNER, true, FIRST),
            Err(Error::Reentry)
        ));
        assert_eq!(
            state.complete(OWNER, true, FIRST, true),
            Err(Error::Reentry)
        );
        drop(permit);
        assert!(state.mutate(OWNER, true, FIRST).is_ok());
    }
    #[test]
    fn live_reset_inspection_duplicate_start_and_early_completion_fail() {
        let state = running();
        assert_eq!(state.start(OWNER, true, FIRST), Err(Error::WrongPhase));
        assert!(matches!(
            state.prepare(COORDINATOR, true, FIRST + 1),
            Err(Error::WrongPhase)
        ));
        assert!(matches!(
            state.inspect(COORDINATOR, true, FIRST),
            Err(Error::WrongPhase)
        ));
        assert_eq!(
            state.complete(OWNER, true, FIRST, false),
            Err(Error::NotQuiescent)
        );
        assert_eq!(state.phase(), Phase::Running);
    }
    #[test]
    fn completed_reader_excludes_reset_and_stale_handles_cannot_touch_reuse() {
        let state = running();
        state.complete(OWNER, true, FIRST, true).unwrap();
        let reader = state.inspect(COORDINATOR, true, FIRST).unwrap();
        assert!(matches!(
            state.prepare(COORDINATOR, true, FIRST + 1),
            Err(Error::Reentry)
        ));
        drop(reader);
        drop(state.prepare(COORDINATOR, true, FIRST + 1).unwrap());
        state.publish(COORDINATOR, true, FIRST + 1).unwrap();
        state.start(OWNER, true, FIRST + 1).unwrap();
        assert!(matches!(
            state.mutate(OWNER, true, FIRST),
            Err(Error::StaleGeneration)
        ));
        assert_eq!(
            state.complete(OWNER, true, FIRST, true),
            Err(Error::StaleGeneration)
        );
    }
    #[test]
    fn generation_wrap_never_revalidates_old_handles() {
        let state = Ownership::new(OWNER, COORDINATOR);
        state.generation.store(u64::MAX, Ordering::Relaxed);
        assert!(matches!(
            state.prepare(COORDINATOR, true, 0),
            Err(Error::GenerationExhausted)
        ));
    }
    #[test]
    fn concurrent_attempt_cannot_borrow_or_publish_while_owner_is_active() {
        let state = running();
        let permit = state.mutate(OWNER, true, FIRST).unwrap();
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    assert!(matches!(
                        state.mutate(OWNER, true, FIRST),
                        Err(Error::Reentry)
                    ));
                    assert_eq!(
                        state.complete(OWNER, true, FIRST, true),
                        Err(Error::Reentry)
                    );
                })
                .join()
                .unwrap();
        });
        drop(permit);
        state.complete(OWNER, true, FIRST, true).unwrap();
        assert!(state.completed());
        let reader = state.inspect(COORDINATOR, true, FIRST).unwrap();
        assert!(!state.completed());
        drop(reader);
        assert!(state.completed());
    }
}
