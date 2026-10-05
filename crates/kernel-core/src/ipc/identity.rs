//! Concrete endpoint retention and one revoke/admission gate. No request storage here.
use super::Error;
use crate::process::ProcessId;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub const ENDPOINTS: usize = 8;
const RESERVED: usize = usize::MAX;
const MAX_REFERENCES: usize = 1024;
const REVOKED: u64 = 1 << 63;
const CLOSED: u64 = 1 << 62;
const SERIAL: u64 = CLOSED - 1;

struct Control {
    generation: AtomicU64,
    references: AtomicUsize,
    gate: AtomicU64,
}
impl Control {
    const fn new() -> Self {
        Self {
            generation: AtomicU64::new(0),
            references: AtomicUsize::new(0),
            gate: AtomicU64::new(0),
        }
    }
}
static CONTROLS: [Control; ENDPOINTS] = [const { Control::new() }; ENDPOINTS];
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static WAIT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndpointId {
    slot: usize,
    generation: u64,
}
impl EndpointId {
    /// Kernel pool selector only. Endpoint numbers are never an EL0 admission path.
    pub fn slot(self) -> usize {
        self.slot
    }
    pub fn generation(self) -> u64 {
        self.generation
    }
}

/// Private fields exclude construction from user numbers. Every value owns one pool lease.
pub struct Reference {
    id: EndpointId,
    service: ProcessId,
    cpu: usize,
}
impl Reference {
    pub(super) fn create(service: ProcessId, cpu: usize) -> Result<Self, Error> {
        for (slot, control) in CONTROLS.iter().enumerate() {
            if control
                .references
                .compare_exchange(0, RESERVED, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                continue;
            }
            let Some(generation) = control.generation.load(Ordering::Relaxed).checked_add(1) else {
                // Exhausted identity stays reserved permanently, never wrapped/rebound.
                continue;
            };
            control.gate.store(0, Ordering::Relaxed);
            control.generation.store(generation, Ordering::Release);
            control.references.store(1, Ordering::Release);
            return Ok(Self {
                id: EndpointId { slot, generation },
                service,
                cpu,
            });
        }
        Err(Error::Exhausted)
    }
    fn control(&self) -> &Control {
        let control = &CONTROLS[self.id.slot];
        assert_eq!(
            control.generation.load(Ordering::Acquire),
            self.id.generation
        );
        assert_ne!(control.references.load(Ordering::Acquire), 0);
        control
    }
    pub fn id(&self) -> EndpointId {
        self.id
    }
    pub fn service(&self) -> ProcessId {
        self.service
    }
    pub fn cpu(&self) -> usize {
        self.cpu
    }
    pub fn references(&self) -> usize {
        self.control().references.load(Ordering::Acquire)
    }
    pub fn try_clone(&self) -> Result<Self, Error> {
        let control = self.control();
        control
            .references
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n > 0 && n < MAX_REFERENCES).then(|| n + 1)
            })
            .map_err(|_| Error::Exhausted)?;
        Ok(Self {
            id: self.id,
            service: self.service,
            cpu: self.cpu,
        })
    }
    pub fn open(&self) -> bool {
        self.control().gate.load(Ordering::Acquire) & (REVOKED | CLOSED) == 0
    }
    pub fn closed(&self) -> bool {
        self.control().gate.load(Ordering::Acquire) & CLOSED != 0
    }
    /// Caller-local REVOKE lookup must precede this concrete operation.
    pub fn revoke(&self) -> bool {
        if cfg!(feature = "ipc-revoke-negative") {
            return true;
        }
        self.control().gate.fetch_or(REVOKED, Ordering::AcqRel) & REVOKED == 0
    }
    pub(super) fn close(&self) {
        self.control().gate.fetch_or(CLOSED, Ordering::AcqRel);
    }
    /// Called only after space/charge preparation under endpoint synchronization.
    /// This CAS is the admission/revoke linearization; failures publish no request.
    pub(super) fn admit(&self) -> Result<(), Error> {
        let control = self.control();
        let mut observed = control.gate.load(Ordering::Acquire);
        // SENDs serialize in endpoint storage; delegated aliases can be installed
        // under namespace ownership concurrently. Bound contention rather than spin.
        for _ in 0..32 {
            if observed & (REVOKED | CLOSED) != 0 {
                return Err(Error::Denied);
            }
            if observed & SERIAL == SERIAL {
                return Err(Error::Exhausted);
            }
            match control.gate.compare_exchange_weak(
                observed,
                observed + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(next) => observed = next,
            }
        }
        Err(Error::Busy)
    }
    /// A pre-revoke transfer can publish an already-revoked alias, never a renewed grant.
    pub fn admit_delegation(&self) -> Result<(), Error> {
        self.admit()
    }
}
impl Drop for Reference {
    fn drop(&mut self) {
        let previous = self.control().references.fetch_sub(1, Ordering::AcqRel);
        assert!(previous > 0 && previous != RESERVED);
    }
}

pub(super) fn next_request() -> Result<u64, Error> {
    next(&REQUEST_SEQUENCE)
}
pub(super) fn next_wait() -> Result<u64, Error> {
    WAIT_SEQUENCE
        .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
            n.checked_add(1).filter(|next| *next <= u64::MAX >> 1)
        })
        .map(|previous| previous + 1)
        .map_err(|_| Error::Exhausted)
}
pub(super) fn next(sequence: &AtomicU64) -> Result<u64, Error> {
    sequence
        .try_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
        .map(|previous| previous + 1)
        .map_err(|_| Error::Exhausted)
}
