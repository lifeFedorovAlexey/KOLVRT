//! Synchronous current-process byte boundary. No Rust references to user RAM.
#![cfg_attr(not(feature = "kernel-tests"), allow(dead_code))] // Foundation; no public pointer SVC yet.
use crate::{cpu, memory, percpu, scheduler::task::Task};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
pub(crate) use kernel_core::user_copy::{Error, MAX_BYTES};
const USER_END: usize = memory::USER_BASE + 0x20_0000;
const PAGE_BYTES: usize = crate::platform::config::PAGE_BYTES;
#[cfg(feature = "diagnostics")]
struct Diagnostics {
    attempts: AtomicUsize,
    failures: AtomicUsize,
    recovered: AtomicUsize,
    address: AtomicUsize,
    length: AtomicUsize,
    reason: AtomicUsize,
    esr: core::sync::atomic::AtomicU64,
}
#[cfg(feature = "diagnostics")]
static DIAGNOSTICS: [Diagnostics; crate::platform::config::ACTIVE_CPUS] = [const {
    Diagnostics {
        attempts: AtomicUsize::new(0),
        failures: AtomicUsize::new(0),
        recovered: AtomicUsize::new(0),
        address: AtomicUsize::new(0),
        length: AtomicUsize::new(0),
        reason: AtomicUsize::new(0),
        esr: core::sync::atomic::AtomicU64::new(0),
    }
};
    crate::platform::config::ACTIVE_CPUS];
fn diagnose(address: usize, length: usize, result: Result<(), Error>) -> Result<(), Error> {
    #[cfg(feature = "diagnostics")]
    {
        let d = &DIAGNOSTICS[percpu::id()];
        d.attempts.fetch_add(1, Ordering::Relaxed);
        if let Err(error) = result {
            d.failures.fetch_add(1, Ordering::Relaxed);
            d.address.store(address, Ordering::Relaxed);
            d.length.store(length, Ordering::Relaxed);
            let reason = match error {
                Error::TooLarge => 1,
                Error::InvalidRange => 2,
                Error::PermissionDenied => 3,
                Error::UserFault { .. } => 4,
                Error::StaleContext => 5,
            };
            d.reason.store(reason, Ordering::Relaxed);
        }
    }
    #[cfg(not(feature = "diagnostics"))]
    let _ = (address, length);
    result
}
struct Recovery {
    active: AtomicBool,
    start: AtomicUsize,
    end: AtomicUsize,
}
static RECOVERY: [Recovery; crate::platform::config::ACTIVE_CPUS] = [const {
    Recovery {
        active: AtomicBool::new(false),
        start: AtomicUsize::new(0),
        end: AtomicUsize::new(0),
    }
};
    crate::platform::config::ACTIVE_CPUS];
unsafe extern "C" {
    fn user_copy_in(destination: *mut u8, source: usize, length: usize) -> usize;
    fn user_copy_out(destination: usize, source: *const u8, length: usize) -> usize;
    static user_copy_load: u8;
    static user_copy_store: u8;
    static user_copy_resume: u8;
}
/// Owned initialized snapshot. A failed input never publishes partial bytes.
pub(crate) struct Snapshot<const N: usize> {
    bytes: [u8; N],
    length: usize,
}
impl<const N: usize> Snapshot<N> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.length]
    }
}
/// Borrowed executing task prevents mutation/exit while copy runs. Not Send/Sync.
pub(crate) struct Access<'a> {
    task: &'a Task,
    owner: core::marker::PhantomData<*mut ()>,
}
impl<'a> Access<'a> {
    pub(super) fn current(task: &'a Task) -> Result<Self, Error> {
        if !crate::scheduler::copy_context(task) {
            return Err(Error::StaleContext);
        }
        Ok(Self {
            task,
            owner: core::marker::PhantomData,
        })
    }
    fn validate(&self, address: usize, length: usize, write: bool) -> Result<(), Error> {
        if !crate::scheduler::copy_context(self.task) {
            return Err(Error::StaleContext);
        }
        kernel_core::user_copy::range(address, length, memory::USER_BASE, USER_END)?;
        if length == 0 {
            return Ok(());
        }
        let last = (address + length - 1) / PAGE_BYTES;
        for page in address / PAGE_BYTES..=last {
            cpu::user_translation(page * PAGE_BYTES, write)?;
        }
        Ok(())
    }
    pub fn copy_from_user<const N: usize>(
        &self,
        address: usize,
        length: usize,
    ) -> Result<Snapshot<N>, Error> {
        if N > MAX_BYTES || length > N {
            return Err(Error::TooLarge);
        }
        diagnose(address, length, self.validate(address, length, false))?;
        self.read_snapshot(address, length)
    }
    fn read_snapshot<const N: usize>(
        &self,
        address: usize,
        length: usize,
    ) -> Result<Snapshot<N>, Error> {
        assert!(length <= N && N <= MAX_BYTES);
        let mut snapshot = Snapshot {
            bytes: [0; N],
            length,
        };
        let _guard = CopyGuard::new(address, length)?;
        // SAFETY: INV-USER-COPY: initialized disjoint bounded kernel storage, checked
        // current root/generation/EL0 permissions, immutable retained mappings;
        // only the exact LDTRB instruction recovers an architectural user fault.
        let copied = unsafe { user_copy_in(snapshot.bytes.as_mut_ptr(), address, length) };
        if copied != length {
            return Err(Error::UserFault { copied });
        }
        Ok(snapshot)
    }
    pub fn copy_to_user(&self, address: usize, bytes: &[u8]) -> Result<(), Error> {
        diagnose(
            address,
            bytes.len(),
            self.validate(address, bytes.len(), true),
        )?;
        self.write_bytes(address, bytes)
    }
    fn write_bytes(&self, address: usize, bytes: &[u8]) -> Result<(), Error> {
        let _guard = CopyGuard::new(address, bytes.len())?;
        // SAFETY: INV-USER-COPY: initialized Rust byte slice, checked bounded writable
        // current user range; no struct/padding conversion. Exact STTRB recovery
        // returns the completed prefix and never reports a partial write as success.
        let copied = unsafe { user_copy_out(address, bytes.as_ptr(), bytes.len()) };
        if copied != bytes.len() {
            return Err(Error::UserFault { copied });
        }
        Ok(())
    }
}
struct CopyGuard;
impl CopyGuard {
    fn new(address: usize, length: usize) -> Result<Self, Error> {
        let recovery = &RECOVERY[percpu::id()];
        if recovery.active.load(Ordering::Acquire) {
            return Err(Error::StaleContext);
        }
        recovery.start.store(address, Ordering::Relaxed);
        recovery.end.store(address + length, Ordering::Relaxed);
        recovery.active.store(true, Ordering::Release);
        Ok(Self)
    }
}
impl Drop for CopyGuard {
    fn drop(&mut self) {
        RECOVERY[percpu::id()]
            .active
            .store(false, Ordering::Release);
    }
}
/// Recovery cannot admit arbitrary EL1 faults, instruction aborts or kernel-side accesses.
pub(crate) fn recover(esr: u64, far: u64, pc: u64) -> Option<u64> {
    const DATA_ABORT_CURRENT: u64 = 0x25;
    const FAULT_STATUS_MASK: u64 = 0x3f;
    const WRITE: u64 = 1 << 6;
    const FAR_INVALID: u64 = 1 << 10;
    let r = &RECOVERY[percpu::id()];
    let load = &raw const user_copy_load as u64;
    let store = &raw const user_copy_store as u64;
    if esr >> cpu::ESR_EC_SHIFT != DATA_ABORT_CURRENT
        || !(4..=15).contains(&(esr & FAULT_STATUS_MASK))
        || esr & FAR_INVALID != 0
        || !((pc == load && esr & WRITE == 0) || (pc == store && esr & WRITE != 0))
        || !r.active.load(Ordering::Acquire)
        || !(r.start.load(Ordering::Relaxed)..r.end.load(Ordering::Relaxed))
            .contains(&(far as usize))
    {
        return None;
    }
    #[cfg(feature = "diagnostics")]
    {
        DIAGNOSTICS[percpu::id()]
            .recovered
            .fetch_add(1, Ordering::Relaxed);
        DIAGNOSTICS[percpu::id()].esr.store(esr, Ordering::Relaxed);
    }
    Some(&raw const user_copy_resume as u64)
}

#[cfg(feature = "kernel-tests")]
pub(crate) mod testing;
