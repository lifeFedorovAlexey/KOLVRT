use crate::{cpu, platform::config};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use kernel_core::platform::Description;
pub const OFF: usize = 0;
pub const ONLINE: usize = 1;
#[cfg(feature = "kernel-tests")]
pub const WORKING: usize = 2;
pub const QUIESCENT: usize = 3;
pub const FAILED: usize = 4;
pub const BOOT_CPU: usize = 0;
pub const SECONDARY_CPU: usize = 1;
const UNBOUND: u64 = u64::MAX;
pub struct Cpu {
    pub affinity: AtomicU64,
    pub stack: AtomicUsize,
    pub state: AtomicUsize,
    pub timers: AtomicU64,
    pub ipis: AtomicU64,
    pub tlb_ack: AtomicU64,
    #[cfg(feature = "kernel-tests")]
    pub expected_pc: AtomicU64,
    #[cfg(feature = "kernel-tests")]
    pub expected_resume: AtomicU64,
    #[cfg(feature = "kernel-tests")]
    pub fault_esr: AtomicU64,
    #[cfg(feature = "kernel-tests")]
    pub fault_far: AtomicU64,
}
impl Cpu {
    const fn new() -> Self {
        Self {
            affinity: AtomicU64::new(UNBOUND),
            stack: AtomicUsize::new(0),
            state: AtomicUsize::new(OFF),
            timers: AtomicU64::new(0),
            ipis: AtomicU64::new(0),
            tlb_ack: AtomicU64::new(0),
            #[cfg(feature = "kernel-tests")]
            expected_pc: AtomicU64::new(0),
            #[cfg(feature = "kernel-tests")]
            expected_resume: AtomicU64::new(0),
            #[cfg(feature = "kernel-tests")]
            fault_esr: AtomicU64::new(0),
            #[cfg(feature = "kernel-tests")]
            fault_far: AtomicU64::new(0),
        }
    }
}
pub static CPUS: [Cpu; config::ACTIVE_CPUS] = [const { Cpu::new() }; config::ACTIVE_CPUS];
pub fn initialize(d: &Description) {
    let primary = cpu::affinity();
    assert!(d.cpu_affinities[..d.cpu_count].contains(&primary));
    let secondary = *d.cpu_affinities[..d.cpu_count]
        .iter()
        .find(|&&a| a != primary)
        .unwrap();
    CPUS[BOOT_CPU].affinity.store(primary, Ordering::Release);
    CPUS[SECONDARY_CPU]
        .affinity
        .store(secondary, Ordering::Release);
    CPUS[BOOT_CPU]
        .stack
        .store(cpu::stack_pointer(), Ordering::Release);
    CPUS[BOOT_CPU].state.store(ONLINE, Ordering::Release);
}
pub fn id() -> usize {
    try_id().expect("unbound CPU affinity")
}
pub fn try_id() -> Option<usize> {
    let affinity = cpu::affinity();
    CPUS.iter()
        .position(|c| c.affinity.load(Ordering::Acquire) == affinity)
}
pub fn is_secondary() -> bool {
    let primary = CPUS[BOOT_CPU].affinity.load(Ordering::Acquire);
    primary != UNBOUND && cpu::affinity() != primary
}
pub fn current() -> &'static Cpu {
    &CPUS[id()]
}
pub fn primary_only() {
    assert_eq!(id(), BOOT_CPU, "CPU0-owned resource");
}
