use core::arch::{asm, global_asm};
pub mod page;
#[cfg(feature = "kernel-tests")]
pub const INSTRUCTION_BYTES: usize = 4;
pub const CURRENT_EL_SHIFT: u32 = 2;
pub const CURRENT_EL1: u64 = 1 << 2;
#[cfg(feature = "negative-test")]
pub const CURRENT_EL2: u64 = 2 << 2;
pub const SCTLR_MMU_ENABLE: u64 = 1;
const SCTLR_DATA_CACHE_ENABLE: u64 = 1 << 2;
const SCTLR_INSTRUCTION_CACHE_ENABLE: u64 = 1 << 12;
const SCTLR_WRITE_EXECUTE_NEVER: u64 = 1 << 19;
const MAIR_DEVICE_NGNRNE_NORMAL_WB: u64 = 0xff00;
const TCR_T0SZ_39_BIT: u64 = 25;
const TCR_IRGN0_WB: u64 = 1 << 8;
const TCR_ORGN0_WB: u64 = 1 << 10;
const TCR_SH0_INNER: u64 = 3 << 12;
const TCR_EPD1: u64 = 1 << 23;
const TCR_IPS_40_BIT: u64 = 2 << 32;
const DAIF_IRQ_MASK: u8 = 2;
const CNTP_CTL_ENABLE: u64 = 1;
const ICC_SRE_ENABLE: u64 = 1;
const ICC_PMR_ALL_PRIORITIES: u64 = 255;
const ICC_IGRPEN1_ENABLE: u64 = 1;
const PSCI_SYSTEM_OFF: u64 = 0x84000008;
const PSCI_CPU_ON: u64 = 0xc4000003;
const PSCI_CPU_OFF: u64 = 0x84000002;
const PSCI_AFFINITY_INFO: u64 = 0xc4000004;
const CACHE_LINE_BYTES: usize = 64; // Cortex-A57 platform pin.
const AFFINITY_LEVEL_BITS: u32 = 8;
const AFFINITY_LEVEL_MASK: u64 = u8::MAX as u64;
const MPIDR_AFF1_SHIFT: u32 = AFFINITY_LEVEL_BITS;
const MPIDR_AFF2_SHIFT: u32 = 2 * AFFINITY_LEVEL_BITS;
const MPIDR_AFF3_SHIFT: u32 = 32;
const SGI_AFF1_SHIFT: u32 = 16;
const SGI_INTID_SHIFT: u32 = 24;
const SGI_AFF2_SHIFT: u32 = 32;
const SGI_AFF3_SHIFT: u32 = 48;
const SGI_TARGET_COUNT: u64 = 16;
pub const PSCI_SUCCESS: i64 = 0;
pub const PSCI_AFFINITY_ON: i64 = 0;
pub const PSCI_AFFINITY_OFF: i64 = 1;
#[cfg(feature = "kernel-tests")]
pub const ESR_EC_SHIFT: u32 = 26;
#[cfg(feature = "kernel-tests")]
pub const ESR_EC_DATA_ABORT_CURRENT_EL: u64 = 0x25;
#[cfg(feature = "kernel-tests")]
pub const ESR_EC_BRK: u64 = 0x3c;
#[cfg(feature = "kernel-tests")]
pub const ESR_EC_INSTRUCTION_ABORT_CURRENT_EL: u64 = 0x21;
#[cfg(feature = "kernel-tests")]
pub const ESR_FAULT_STATUS_MASK: u64 = 0x3f;
#[cfg(feature = "kernel-tests")]
pub const ESR_TRANSLATION_FAULT_L3: u64 = 7;
#[cfg(feature = "kernel-tests")]
pub const ESR_PERMISSION_FAULT_L3: u64 = 15;
// SAFETY: INV-ENTRY and INV-VECTOR, reviewed assembly owns startup and exception ABI.
global_asm!(include_str!("entry.S"));

macro_rules! read_reg {
    ($name:ident, $reg:literal) => {
        pub fn $name() -> u64 { let value;
            // SAFETY: INV-REG: privileged EL1 architectural read, no memory access.
            unsafe { asm!(concat!("mrs {}, ", $reg), out(reg) value, options(nomem, nostack)); } value
        }
    };
}
read_reg!(el, "CurrentEL");
read_reg!(ticks, "cntpct_el0");
read_reg!(frequency, "cntfrq_el0");
read_reg!(sctlr, "sctlr_el1");
read_reg!(acknowledge, "S3_0_C12_C12_0");
read_reg!(mpidr, "mpidr_el1");
pub fn affinity() -> u64 {
    mpidr() & kernel_core::platform::MPIDR_AFFINITY_MASK
}
pub fn stack_pointer() -> usize {
    let sp;
    // SAFETY: INV-PERCPU: observes this CPU's current aligned stack, never dereferences it.
    unsafe {
        asm!("mov {}, sp", out(reg) sp, options(nomem, nostack));
    }
    sp
}
pub fn barrier() {
    // SAFETY: INV-TLB: complete published memory effects before signalling/completion.
    unsafe {
        asm!("dsb ish", "isb", options(nostack));
    }
}
pub fn local_invalidate() {
    // SAFETY: INV-SHOOTDOWN: calling CPU is at a reader-quiescent boundary; all prior loads finish before acknowledgement.
    unsafe {
        asm!(
            "dsb ish",
            "tlbi vmalle1",
            "dsb ish",
            "isb",
            options(nostack)
        );
    }
}
pub fn clean_boot(start: usize, end: usize) {
    // SAFETY: INV-BOOT-PUBLISH: CPU0-owned linked RAM, Cortex-A57 64-byte lines; clean tables/publication to PoC for MMU-off secondary.
    unsafe {
        for at in (start & !(CACHE_LINE_BYTES - 1)..end).step_by(CACHE_LINE_BYTES) {
            asm!("dc cvac, {}", in(reg) at, options(nostack));
        }
        asm!("dsb sy", options(nostack));
    }
}
fn psci(function: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    let result;
    // SAFETY: INV-CPUON: validated SMC conduit; PSCI/SMCCC preserve x4-x18 for these calls, x0-x3 declared clobbered.
    unsafe {
        asm!("smc #0", inlateout("x0") function => result, inlateout("x1") arg1 => _, inlateout("x2") arg2 => _, inlateout("x3") arg3 => _, options(nostack));
    }
    result
}
pub fn start_cpu(affinity: u64, entry: usize, root: u64) -> i64 {
    psci(PSCI_CPU_ON, affinity, entry as u64, root)
}
pub fn affinity_state(affinity: u64) -> i64 {
    psci(PSCI_AFFINITY_INFO, affinity, 0, 0)
}
pub fn cpu_off() -> ! {
    let _ = psci(PSCI_CPU_OFF, 0, 0, 0);
    loop {
        core::hint::spin_loop();
    }
}
pub fn send_sgi(affinity: u64, id: u8) {
    assert!(
        u64::from(id) < SGI_TARGET_COUNT && (affinity & AFFINITY_LEVEL_MASK) < SGI_TARGET_COUNT,
        "SGI target range"
    );
    let value = ((affinity >> MPIDR_AFF3_SHIFT & AFFINITY_LEVEL_MASK) << SGI_AFF3_SHIFT)
        | ((affinity >> MPIDR_AFF2_SHIFT & AFFINITY_LEVEL_MASK) << SGI_AFF2_SHIFT)
        | (u64::from(id) << SGI_INTID_SHIFT)
        | ((affinity >> MPIDR_AFF1_SHIFT & AFFINITY_LEVEL_MASK) << SGI_AFF1_SHIFT)
        | (1 << (affinity & AFFINITY_LEVEL_MASK));
    // SAFETY: INV-SGI: affinity is a validated participating CPU; Group1 SGI, published mailbox before device signal.
    unsafe {
        asm!("dsb ishst", "msr S3_0_C12_C11_5, {}", "isb", in(reg) value, options(nostack));
    }
}
pub fn vectors() {
    unsafe extern "C" {
        static vectors: u8;
    }
    // SAFETY: INV-VECTOR: linked 2 KiB-aligned vector table with kernel lifetime.
    unsafe {
        asm!("msr vbar_el1, {}", "isb", in(reg) &raw const vectors, options(nostack));
    }
}
pub fn mask() {
    // SAFETY: INV-IRQ: affects only the calling CPU; masking cannot release an IRQ-owned resource.
    unsafe {
        asm!("msr daifset, #{mask}", "isb", mask=const DAIF_IRQ_MASK, options(nomem, nostack));
    }
}
pub fn unmask() {
    // SAFETY: INV-IRQ: called only after GIC, source and vectors are initialized.
    unsafe {
        asm!("msr daifclr, #{mask}", "isb", mask=const DAIF_IRQ_MASK, options(nomem, nostack));
    }
}
pub fn timer(deadline: u64) {
    // SAFETY: INV-TIMER: deadline in the current physical-counter epoch; PPI initialized.
    unsafe {
        asm!("msr cntp_cval_el0, {}", "msr cntp_ctl_el0, {control}", "isb", in(reg) deadline, control=in(reg) CNTP_CTL_ENABLE, options(nostack));
    }
}
pub fn timer_stop() {
    // SAFETY: INV-TIMER: masks/deasserts level source before GIC EOI.
    unsafe {
        asm!("msr cntp_ctl_el0, xzr", "isb", options(nomem, nostack));
    }
}
pub fn end_irq(id: u64) {
    // SAFETY: INV-IRQ: exact acknowledged INTID, EOImode=0 drops priority and deactivates.
    unsafe {
        asm!("dsb sy", "msr S3_0_C12_C12_1, {}", "isb", in(reg) id, options(nostack));
    }
}
pub fn cpu_interface() {
    // SAFETY: INV-IRQ: nonsecure Group 1, redistributor initialized, IRQs still masked.
    unsafe {
        asm!("dsb sy", "msr S3_0_C12_C12_5,{sre}", "isb", "msr S3_0_C4_C6_0,{priority}", "msr S3_0_C12_C12_4,xzr", "msr S3_0_C12_C12_3,xzr", "msr S3_0_C12_C12_7,{enable}", "isb", sre=in(reg) ICC_SRE_ENABLE, priority=in(reg) ICC_PMR_ALL_PRIORITIES, enable=in(reg) ICC_IGRPEN1_ENABLE, options(nostack));
    }
}
pub fn enable_mmu(root: u64) {
    let control = sctlr()
        | SCTLR_MMU_ENABLE
        | SCTLR_DATA_CACHE_ENABLE
        | SCTLR_INSTRUCTION_CACHE_ENABLE
        | SCTLR_WRITE_EXECUTE_NEVER;
    // SAFETY: INV-MMU: root is aligned initialized owned tables; current PC/SP identity mapped.
    unsafe {
        asm!("dsb sy", "msr mair_el1, {mair}", "msr tcr_el1, {tcr}", "msr ttbr0_el1, {root}", "isb", "tlbi vmalle1", "dsb sy", "isb", "msr sctlr_el1, {control}", "isb", mair=in(reg) MAIR_DEVICE_NGNRNE_NORMAL_WB, tcr=in(reg) (TCR_T0SZ_39_BIT | TCR_IRGN0_WB | TCR_ORGN0_WB | TCR_SH0_INNER | TCR_EPD1 | TCR_IPS_40_BIT), root=in(reg) root, control=in(reg) control, options(nostack));
    }
}
pub fn poweroff() -> ! {
    // SAFETY: INV-PSCI: pinned virt exposes PSCI SMC, SYSTEM_OFF has no returned lifetime.
    unsafe {
        asm!("smc #0", in("x0") PSCI_SYSTEM_OFF, options(nostack));
    }
    loop {
        core::hint::spin_loop();
    }
}
