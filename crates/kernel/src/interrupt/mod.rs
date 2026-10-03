use crate::{arch::aarch64 as cpu, hal::Registers};
use core::sync::atomic::{AtomicUsize, Ordering};
use kernel_core::platform::Description;
const GICD_CTLR: usize = 0x0000;
const GICD_CTLR_RWP: u32 = 1 << 31;
const GICD_CTLR_ARE_NS: u32 = 1 << 4;
const GICD_CTLR_ENABLE_G1_NS: u32 = 1 << 1;
const GICR_FRAME_SIZE: usize = 0x20000;
const GICR_CTLR: usize = 0x0000;
const GICR_CTLR_RWP: u32 = 1 << 3;
const GICR_TYPER_AFFINITY: usize = 0x000c;
const GICR_TYPER_LOW: usize = 0x0008;
const GICR_TYPER_LAST: u32 = 1 << 4;
const GICR_TYPER_VLPIS: u32 = 1 << 1;
const GICR_IPRIORITYR_SGI_WORD: usize = 0x10400;
const GICR_AFF3_SHIFT: u32 = 24;
const MPIDR_AFF3_SHIFT: u32 = 32;
const AFFINITY_LOW_LEVELS_MASK: u64 = 0xffffff;
const GICR_WAKER: usize = 0x0014;
const GICR_WAKER_PROCESSOR_SLEEP: u32 = 1 << 1;
const GICR_WAKER_CHILDREN_ASLEEP: u32 = 1 << 2;
const GICR_ICENABLER0: usize = 0x10180;
const GICR_ICPENDR0: usize = 0x10280;
const GICR_IGROUPR0: usize = 0x10080;
const GICR_ICFGR1: usize = 0x10c04;
const GICR_TIMER_TRIGGER_MASK: u32 = 3 << 28;
const GICR_IPRIORITYR_TIMER_WORD: usize = 0x1041c;
const GICR_DEFAULT_PRIORITIES: u32 = 0x80808080;
const GICR_ISENABLER0: usize = 0x10100;
#[cfg(feature = "kernel-tests")]
const TEST_FPCR_ALTERNATE_ROUNDING: u64 = 1 << 22;
#[cfg(feature = "kernel-tests")]
const TEST_FPSR_INEXACT: u64 = 1 << 4;
#[cfg(feature = "kernel-tests")]
const TEST_SIMD_LOW_CLOBBER: u8 = 0xaa;
#[cfg(feature = "kernel-tests")]
const TEST_SIMD_HIGH_CLOBBER: u8 = 0xbb;
const GIC_SPURIOUS_INTID: u64 = 1023;
// Bounded register reads, not a duration measured by the counter.
const GIC_PROGRESS_POLL_LIMIT: usize = 1_000_000;
static REDIST_BASE: AtomicUsize = AtomicUsize::new(0);
static REDIST_SIZE: AtomicUsize = AtomicUsize::new(0);
pub fn delivered() -> &'static core::sync::atomic::AtomicU64 {
    &crate::percpu::current().timers
}
pub fn initialize(d: &Description) {
    crate::percpu::primary_only();
    // SAFETY: INV-GIC: validated GICv3 register regions, IRQ masked; sole CPU configuration owner.
    let distributor =
        unsafe { Registers::new(d.distributor.base as usize, d.distributor.size as usize) };
    distributor.write(GICD_CTLR, 0);
    wait(|| distributor.read(GICD_CTLR) & GICD_CTLR_RWP == 0);
    REDIST_SIZE.store(d.redistributor.size as usize, Ordering::Release);
    REDIST_BASE.store(d.redistributor.base as usize, Ordering::Release);
    distributor.write(GICD_CTLR, GICD_CTLR_ARE_NS | GICD_CTLR_ENABLE_G1_NS);
    wait(|| distributor.read(GICD_CTLR) & GICD_CTLR_RWP == 0);
    initialize_local();
}
pub fn initialize_local() {
    let base = REDIST_BASE.load(Ordering::Acquire);
    let size = REDIST_SIZE.load(Ordering::Acquire);
    assert!(base != 0);
    let affinity = cpu::affinity();
    let compressed =
        ((affinity >> MPIDR_AFF3_SHIFT) << GICR_AFF3_SHIFT) | (affinity & AFFINITY_LOW_LEVELS_MASK);
    let mut found = None;
    for offset in (0..size).step_by(GICR_FRAME_SIZE) {
        if offset + GICR_FRAME_SIZE > size {
            break;
        }
        // SAFETY: INV-GIC-AFFINITY: validated bounded redistributor region; each CPU owns only matching GICR_TYPER affinity.
        let frame = unsafe { Registers::new(base + offset, GICR_FRAME_SIZE) };
        assert_eq!(
            frame.read(GICR_TYPER_LOW) & GICR_TYPER_VLPIS,
            0,
            "unsupported redistributor stride"
        );
        if u64::from(frame.read(GICR_TYPER_AFFINITY)) == compressed {
            found = Some(frame);
            break;
        }
        if frame.read(GICR_TYPER_LOW) & GICR_TYPER_LAST != 0 {
            break;
        }
    }
    let redist = found.expect("missing CPU redistributor affinity");
    redist.write(
        GICR_WAKER,
        redist.read(GICR_WAKER) & !GICR_WAKER_PROCESSOR_SLEEP,
    );
    wait(|| redist.read(GICR_WAKER) & GICR_WAKER_CHILDREN_ASLEEP == 0);
    redist.write(GICR_ICENABLER0, u32::MAX);
    redist.write(GICR_ICPENDR0, u32::MAX);
    redist.write(GICR_IGROUPR0, u32::MAX);
    let cfg = redist.read(GICR_ICFGR1) & !GICR_TIMER_TRIGGER_MASK;
    redist.write(GICR_ICFGR1, cfg);
    redist.write(GICR_IPRIORITYR_TIMER_WORD, GICR_DEFAULT_PRIORITIES);
    redist.write(GICR_IPRIORITYR_SGI_WORD, GICR_DEFAULT_PRIORITIES);
    redist.write(
        GICR_ISENABLER0,
        (1 << crate::platform::PHYSICAL_TIMER_IRQ) | (1 << crate::smp::IPI),
    );
    wait(|| redist.read(GICR_CTLR) & GICR_CTLR_RWP == 0);
    cpu::cpu_interface();
}
fn wait(mut ready: impl FnMut() -> bool) {
    for _ in 0..GIC_PROGRESS_POLL_LIMIT {
        if ready() {
            return;
        }
    }
    panic!("GIC progress timeout");
}
#[unsafe(no_mangle)]
pub extern "C" fn interrupt_entry() -> u64 {
    let entry_ticks = cpu::ticks();
    let id = cpu::acknowledge();
    if id == GIC_SPURIOUS_INTID {
        return entry_ticks;
    }
    if id == u64::from(crate::smp::IPI) {
        crate::smp::on_ipi();
        cpu::end_irq(id);
        return entry_ticks;
    }
    if id != u64::from(crate::platform::PHYSICAL_TIMER_IRQ) {
        crate::fatal_exception(0, id, 0);
    }
    cpu::timer_stop();
    #[cfg(feature = "kernel-tests")]
    // SAFETY: INV-PROBE: test-only context negative control; vector entry must preserve compiler-usable SIMD state.
    unsafe {
        core::arch::asm!("movi v0.16b, #{low}","movi v31.16b, #{high}","msr fpcr,{fpcr}","msr fpsr,{fpsr}",low=const TEST_SIMD_LOW_CLOBBER,high=const TEST_SIMD_HIGH_CLOBBER,fpcr=in(reg) TEST_FPCR_ALTERNATE_ROUNDING,fpsr=in(reg) TEST_FPSR_INEXACT,out("v0") _,out("v31") _,options(nostack));
    }
    delivered().fetch_add(1, Ordering::Release);
    crate::scheduler::on_timer();
    cpu::end_irq(id);
    entry_ticks
}
