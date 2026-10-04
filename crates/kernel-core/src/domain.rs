//! One-process domains with explicit immutable bootstrap limits. No policy interpreter.
use crate::process::ProcessId;
mod pool;
use core::sync::atomic::Ordering;
use pool::Lease;
const CLOSING: u64 = 1 << 63;
const FIELD: u64 = 0xffff;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DomainId(ProcessId, u16, u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub memory_pages: usize,
    pub handles: u16,
    pub queue: u16,
    pub requests: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Denied,
    Exhausted,
    Invalid,
}
/// A retained identity, never constructed from an EL0-supplied integer.
#[derive(Clone)]
pub struct Reference(Lease);
pub struct Owner(Reference);
pub struct Memory(Reference);
#[derive(Clone, Copy)]
pub enum ChargeKind {
    Handle,
    Queue,
    Request,
}
impl ChargeKind {
    fn shift(self) -> u32 {
        match self {
            Self::Handle => 0,
            Self::Queue => 16,
            Self::Request => 32,
        }
    }
}
pub struct Charge {
    reference: Reference,
    kind: ChargeKind,
}
impl Owner {
    /// Bootstrap supplies limits; kernel only validates and enforces them.
    pub fn new(process: ProcessId, limits: Limits, pages: usize) -> Result<(Self, Memory), Error> {
        if pages == 0 {
            return Err(Error::Invalid);
        }
        if pages > limits.memory_pages && !cfg!(feature = "domain-budget-negative") {
            return Err(Error::Exhausted);
        }
        let r = Reference(Lease::new(process, limits, pages).ok_or(Error::Exhausted)?);
        Ok((Self(r.clone()), Memory(r)))
    }
    pub fn reference(&self) -> Reference {
        self.0.clone()
    }
    pub fn close(&self) {
        self.0.close();
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.close();
    }
}
impl Drop for Memory {
    fn drop(&mut self) {
        assert_ne!(self.0.0.pages.swap(0, Ordering::AcqRel), 0);
    }
}
impl Reference {
    pub fn id(&self) -> DomainId {
        self.0.id
    }
    pub fn owner(&self) -> ProcessId {
        self.0.id.0
    }
    pub fn limits(&self) -> Limits {
        self.0.limits
    }
    pub fn closing(&self) -> bool {
        self.0.state.load(Ordering::Acquire) & CLOSING != 0
    }
    pub fn close(&self) {
        if !cfg!(feature = "domain-teardown-negative") {
            self.0.state.fetch_or(CLOSING, Ordering::AcqRel);
        }
    }
    pub fn validate(&self, process: ProcessId) -> Result<(), Error> {
        if self.owner() != process && !cfg!(feature = "domain-identity-negative") {
            return Err(Error::Denied);
        }
        if self.closing() && !cfg!(feature = "domain-teardown-negative") {
            return Err(Error::Denied);
        }
        Ok(())
    }
    pub fn usage(&self) -> (usize, u16, u16, u16) {
        let s = self.0.state.load(Ordering::Acquire);
        (
            self.0.pages.load(Ordering::Acquire),
            (s & FIELD) as u16,
            ((s >> 16) & FIELD) as u16,
            ((s >> 32) & FIELD) as u16,
        )
    }
    pub fn charge(&self, kind: ChargeKind) -> Result<Charge, Error> {
        let shift = kind.shift();
        let limit = match kind {
            ChargeKind::Handle => self.0.limits.handles,
            ChargeKind::Queue => self.0.limits.queue,
            ChargeKind::Request => self.0.limits.requests,
        };
        let mut s = self.0.state.load(Ordering::Acquire);
        loop {
            if s & CLOSING != 0 && !cfg!(feature = "domain-teardown-negative") {
                return Err(Error::Denied);
            }
            let used = (s >> shift) & FIELD;
            if used == FIELD
                || used >= u64::from(limit) && !cfg!(feature = "domain-budget-negative")
            {
                return Err(Error::Exhausted);
            }
            match self.0.state.compare_exchange_weak(
                s,
                s + (1 << shift),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Ok(Charge {
                        reference: self.clone(),
                        kind,
                    });
                }
                Err(next) => s = next,
            }
        }
    }
}
impl Charge {
    pub fn domain(&self) -> &Reference {
        &self.reference
    }
}
impl Drop for Charge {
    fn drop(&mut self) {
        let shift = self.kind.shift();
        let s = self
            .reference
            .0
            .state
            .fetch_sub(1 << shift, Ordering::AcqRel);
        assert_ne!(
            (s >> shift) & FIELD,
            0,
            "domain charge released exactly once"
        );
    }
}

pub fn live_domains() -> usize {
    pool::live()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn process() -> ProcessId {
        crate::process::Table::<1>::new().reserve(0..1).unwrap()
    }
    #[test]
    fn quotas_survive_close_and_accepted_work_then_release() {
        let id = process();
        let limits = Limits {
            memory_pages: 4,
            handles: 1,
            queue: 1,
            requests: 1,
        };
        assert!(matches!(Owner::new(id, limits, 5), Err(Error::Exhausted)));
        let (owner, memory) = Owner::new(id, limits, 4).unwrap();
        let r = owner.reference();
        let h = r.charge(ChargeKind::Handle).unwrap();
        let q = r.charge(ChargeKind::Queue).unwrap();
        let work = r.charge(ChargeKind::Request).unwrap();
        assert!(matches!(
            r.charge(ChargeKind::Request),
            Err(Error::Exhausted)
        ));
        assert_eq!(r.usage(), (4, 1, 1, 1));
        owner.close();
        assert_eq!(r.validate(id), Err(Error::Denied));
        assert!(matches!(r.charge(ChargeKind::Handle), Err(Error::Denied)));
        drop(owner);
        drop(h);
        drop(q);
        drop(memory);
        assert_eq!(work.domain().usage(), (0, 0, 0, 1));
        drop(work);
        assert_eq!(r.usage(), (0, 0, 0, 0));
    }
    #[test]
    fn teardown_serializes_with_concurrent_admission() {
        let id = process();
        let (owner, memory) = Owner::new(
            id,
            Limits {
                memory_pages: 1,
                handles: 1,
                queue: 1,
                requests: 1,
            },
            1,
        )
        .unwrap();
        let r = owner.reference();
        let other = r.clone();
        std::thread::scope(|s| {
            let admission = s.spawn(move || other.charge(ChargeKind::Request));
            owner.close();
            if let Ok(work) = admission.join().unwrap() {
                assert_eq!(work.domain().id(), r.id());
                drop(work);
            }
        });
        assert!(matches!(r.charge(ChargeKind::Request), Err(Error::Denied)));
        drop(memory);
        assert_eq!(r.usage(), (0, 0, 0, 0));
    }
}
