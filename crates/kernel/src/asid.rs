use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use crate::{percpu, platform::config};

const PER_CPU: usize = config::USER_PROCESSES_PER_CPU;
const CPUS: usize = config::ACTIVE_CPUS;
const NO_ASID: u32 = u32::MAX;
static ASID_LIMIT: AtomicU32 = AtomicU32::new(0);
static NEXT_EPOCH: AtomicU64 = AtomicU64::new(1);
#[cfg(feature = "kernel-tests")]
static SWITCHES: [AtomicU64; CPUS] = [const { AtomicU64::new(0) }; CPUS];
#[cfg(feature = "kernel-tests")]
static FULL_TLBI: [AtomicU64; CPUS] = [const { AtomicU64::new(0) }; CPUS];
#[cfg(feature = "kernel-tests")]
static ASID_TLBI: [AtomicU64; CPUS] = [const { AtomicU64::new(0) }; CPUS];
#[cfg(feature = "kernel-tests")]
static REUSES: [AtomicU64; CPUS] = [const { AtomicU64::new(0) }; CPUS];
struct LeaseSlot {
    asid: AtomicU32,
    epoch: AtomicU64,
    allocated: AtomicBool,
    retired: AtomicBool,
}
impl LeaseSlot {
    const fn new() -> Self {
        Self {
            asid: AtomicU32::new(NO_ASID),
            epoch: AtomicU64::new(0),
            allocated: AtomicBool::new(false),
            retired: AtomicBool::new(false),
        }
    }
}
static LEASES: [[LeaseSlot; PER_CPU + 1]; CPUS] = [
    [const { LeaseSlot::new() }; PER_CPU + 1],
    [const { LeaseSlot::new() }; PER_CPU + 1],
];

/// Initialize from ID_AA64MMFR0_EL1.ASIDBits on the boot CPU. A homogeneous
/// SMP system is required by the platform contract; unknown encodings fall back.
pub fn initialize(bits: Option<u8>) {
    #[cfg(not(feature = "asid-baseline"))]
    let count = match bits {
        Some(8) => 1 << 8,
        Some(16) => 1 << 16,
        _ => 0,
    };
    #[cfg(feature = "asid-baseline")]
    let count = {
        let _ = bits;
        0
    };
    ASID_LIMIT.store(count, Ordering::Release);
}

pub fn enabled() -> bool {
    ASID_LIMIT.load(Ordering::Acquire) > 1
}

/// An address-space tag is a root/ASID/epoch identity. ASID zero is reserved
/// for the native root and the measured ASID-zero baseline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lease {
    pub asid: u16,
    pub epoch: u64,
    pub owner: usize,
    pub slot: usize,
}
#[derive(Clone, Copy, Debug, Default)]
#[cfg(feature = "kernel-tests")]
pub struct Counters {
    pub switches: u64,
    pub full_tlbi: u64,
    pub asid_tlbi: u64,
    pub reuses: u64,
}
#[cfg(feature = "kernel-tests")]
pub fn counters() -> [Counters; CPUS] {
    core::array::from_fn(|cpu| Counters {
        switches: SWITCHES[cpu].load(Ordering::Acquire),
        full_tlbi: FULL_TLBI[cpu].load(Ordering::Acquire),
        asid_tlbi: ASID_TLBI[cpu].load(Ordering::Acquire),
        reuses: REUSES[cpu].load(Ordering::Acquire),
    })
}
#[cfg(feature = "kernel-tests")]
pub fn record_switch() {
    SWITCHES[percpu::id()].fetch_add(1, Ordering::Relaxed);
}
#[cfg(feature = "kernel-tests")]
pub fn record_full_tlbi() {
    FULL_TLBI[percpu::id()].fetch_add(1, Ordering::Relaxed);
}
#[cfg(all(feature = "kernel-tests", not(feature = "asid-reuse-negative")))]
pub fn record_asid_tlbi(owner: usize) {
    ASID_TLBI[owner].fetch_add(1, Ordering::Relaxed);
}
impl Lease {
    pub const NATIVE: Self = Self {
        asid: 0,
        epoch: 0,
        owner: 0,
        slot: 0,
    };
}

/// Reserve a nonzero hardware ASID for a process pinned to `owner`. Candidate
/// ids are checked against all live leases on that CPU; ASIDs are PE-local.
pub fn allocate(process_slot: usize, owner: usize) -> Lease {
    assert!(
        percpu::id() == percpu::BOOT_CPU,
        "ASID allocation coordinator"
    );
    assert!(owner < CPUS && process_slot / PER_CPU == owner);
    assert_eq!(process_slot / PER_CPU, owner);
    try_allocate_at(owner, process_slot % PER_CPU).expect("ASID pool exhausted")
}

fn try_allocate_at(owner: usize, slot: usize) -> Option<Lease> {
    if !enabled() {
        return Some(Lease {
            asid: 0,
            epoch: 0,
            owner,
            slot,
        });
    }
    let cell = &LEASES[owner][slot];
    if cell.allocated.load(Ordering::Acquire) {
        return None;
    }
    let limit = ASID_LIMIT.load(Ordering::Acquire);
    #[cfg(feature = "kernel-tests")]
    let limit = limit.min(5);
    let mut selected = None;
    'candidate: for candidate in 1..limit {
        for live in &LEASES[owner] {
            if live.allocated.load(Ordering::Acquire)
                && live.asid.load(Ordering::Acquire) == candidate
            {
                continue 'candidate;
            }
        }
        selected = Some(candidate);
        break;
    }
    let asid = selected?;
    let epoch = NEXT_EPOCH.fetch_add(1, Ordering::AcqRel);
    assert_ne!(epoch, u64::MAX, "ASID epoch exhausted");
    #[cfg(feature = "kernel-tests")]
    if cell.epoch.load(Ordering::Relaxed) != 0 {
        REUSES[owner].fetch_add(1, Ordering::Relaxed);
    }
    cell.epoch.store(epoch, Ordering::Relaxed);
    cell.asid.store(asid, Ordering::Relaxed);
    cell.retired.store(false, Ordering::Relaxed);
    cell.allocated.store(true, Ordering::Release);
    Some(Lease {
        asid: asid as u16,
        epoch,
        owner,
        slot,
    })
}

/// Called only by the pinned owner CPU after its last use of this root. The
/// local ASID invalidation must complete before `retired` becomes observable.
pub fn retire(lease: Lease) {
    if lease.asid == 0 {
        return;
    }
    assert_eq!(
        percpu::id(),
        lease.owner,
        "ASID retirement on non-owner CPU"
    );
    let cell = &LEASES[lease.owner][lease.slot];
    assert!(cell.allocated.load(Ordering::Acquire));
    assert_eq!(cell.asid.load(Ordering::Acquire), u32::from(lease.asid));
    assert_eq!(cell.epoch.load(Ordering::Acquire), lease.epoch);
    if cell.retired.load(Ordering::Acquire) {
        return;
    }
    crate::cpu::local_invalidate_asid(lease.asid);
    #[cfg(all(feature = "kernel-tests", not(feature = "asid-reuse-negative")))]
    record_asid_tlbi(lease.owner);
    cell.retired.store(true, Ordering::Release);
}

/// Return a fully retired lease. `unpublished` is reserved for transactional
/// construction rollback before a root can ever be activated.
pub fn release(lease: Lease, unpublished: bool) {
    assert_eq!(percpu::id(), percpu::BOOT_CPU, "ASID release coordinator");
    if lease.asid == 0 {
        return;
    }
    let cell = &LEASES[lease.owner][lease.slot];
    assert!(cell.allocated.load(Ordering::Acquire));
    assert_eq!(cell.asid.load(Ordering::Acquire), u32::from(lease.asid));
    assert_eq!(cell.epoch.load(Ordering::Acquire), lease.epoch);
    assert!(
        unpublished || cell.retired.load(Ordering::Acquire),
        "ASID reused before local TLBI acknowledgement"
    );
    cell.allocated.store(false, Ordering::Release);
    cell.asid.store(NO_ASID, Ordering::Relaxed);
}

pub fn ttbr(root: u64, asid: u16) -> u64 {
    // TTBR.ASID is always bits [63:48]. In 8-bit mode its upper eight
    // bits are reserved/ignored, so the tag still starts at bit 48.
    (root & 0x0000_ffff_ffff_ffff) | (u64::from(asid) << 48)
}

#[cfg(feature = "kernel-tests")]
pub fn exhaustion_probe(owner: usize) -> bool {
    // The test pool is explicitly limited to four user ASIDs; reserve all four,
    // observe exhaustion, retire/release one lease, then prove safe reuse.
    if !enabled() {
        return true;
    }
    let Some(lease) = try_allocate_at(owner, PER_CPU) else {
        return true;
    };
    release(lease, true);
    false
}
