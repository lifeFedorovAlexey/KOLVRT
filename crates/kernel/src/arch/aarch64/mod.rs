use core::arch::{asm, global_asm};
pub mod page;
#[cfg(feature = "kernel-tests")]
pub const INSTRUCTION_BYTES: usize = 4;
pub const CURRENT_EL_SHIFT: u32 = 2;
#[cfg(feature = "kernel-tests")]
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
    // SAFETY: INV-IRQ: single active CPU; masking cannot release an IRQ-owned resource.
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
pub fn invalidate() {
    // SAFETY: INV-TLB: only CPU0 participates; store publication precedes global invalidation.
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
            options(nostack)
        );
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
