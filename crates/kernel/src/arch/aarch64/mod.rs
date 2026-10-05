use core::arch::{asm, global_asm};
pub mod context;
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
const TCR_ASID_16_BIT: u64 = 1 << 36;
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
read_reg!(daif, "daif");
read_reg!(active_root, "ttbr0_el1");
const PSTATE_IRQ_MASK: u64 = 1 << 7;
pub fn irq_masked() -> bool {
    daif() & PSTATE_IRQ_MASK != 0
}
read_reg!(mpidr, "mpidr_el1");
read_reg!(id_aa64mmfr0, "id_aa64mmfr0_el1");
pub fn asid_bits() -> Option<u8> {
    match (id_aa64mmfr0() >> 4) & 0xf {
        0 => Some(8),
        2 => Some(16),
        _ => None,
    }
}
read_reg!(user_esr, "esr_el1");
read_reg!(user_far, "far_el1");
/// Query current stage-1 EL0 permissions without dereferencing user bytes.
pub(crate) fn user_translation(
    address: usize,
    write: bool,
) -> Result<(), kernel_core::user_copy::Error> {
    const PAR_FAULT: u64 = 1;
    const PAR_STATUS_SHIFT: u32 = 1;
    const PAR_STATUS_MASK: u64 = 0x3f;
    let par: u64;
    // SAFETY: INV-USER-COPY: masked synchronous current-task section; AT uses
    // EL0 permissions and current retained TTBR. PAR is read after ISB, no
    // mapping writer/root switch/IRQ can intervene on this CPU.
    unsafe {
        if write {
            asm!("at s1e0w, {address}", address = in(reg) address, options(nostack));
        } else {
            asm!("at s1e0r, {address}", address = in(reg) address, options(nostack));
        }
        asm!("isb", "mrs {par}, par_el1", par = out(reg) par, options(nostack));
    }
    if par & PAR_FAULT == 0 {
        return Ok(());
    }
    let status = (par >> PAR_STATUS_SHIFT) & PAR_STATUS_MASK;
    Err(if (12..=15).contains(&status) {
        kernel_core::user_copy::Error::PermissionDenied
    } else {
        kernel_core::user_copy::Error::UserFault { copied: 0 }
    })
}
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
    // Keep the compiler memory clobber: protected accesses must not cross DAIF changes.
    unsafe {
        asm!("msr daifset, #{mask}", "isb", mask=const DAIF_IRQ_MASK, options(nostack));
    }
}
pub fn unmask() {
    // SAFETY: INV-IRQ: called only after GIC, source and vectors are initialized.
    // Pending handlers can observe memory; this is also a compiler ordering boundary.
    unsafe {
        asm!("msr daifclr, #{mask}", "isb", mask=const DAIF_IRQ_MASK, options(nostack));
    }
}
/// Wait with IRQ masked until an interrupt is pending, then service it before
/// returning to the masked owner-local continuation. Pending SGIs close the
/// recheck-to-WFI race; the physical timer supplies finite deferred retries.
pub fn ipc_idle() {
    assert!(irq_masked());
    crate::sync::assert_scheduler_unlocked();
    assert!(
        !crate::percpu::current()
            .scheduler_borrow
            .load(core::sync::atomic::Ordering::Acquire)
    );
    // SAFETY: INV-IPC-IDLE: native permanent EL1 stack/root, initialized GIC and
    // vectors, no scheduler/object/copy permit spans WFI or IRQ delivery. IRQ
    // only flags timer/SGI work; all endpoint accesses remain deferred/masked.
    unsafe {
        asm!("wfi", "msr daifclr, #{mask}", "isb", "msr daifset, #{mask}", "isb",
            mask=const DAIF_IRQ_MASK, options(nostack));
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
    crate::asid::initialize(asid_bits());
    let control = sctlr()
        | SCTLR_MMU_ENABLE
        | SCTLR_DATA_CACHE_ENABLE
        | SCTLR_INSTRUCTION_CACHE_ENABLE
        | SCTLR_WRITE_EXECUTE_NEVER;
    // SAFETY: INV-MMU: root is aligned initialized owned tables; current PC/SP identity mapped.
    unsafe {
        asm!("dsb sy", "msr mair_el1, {mair}", "msr tcr_el1, {tcr}", "msr ttbr0_el1, {root}", "isb", "tlbi vmalle1", "dsb sy", "isb", "msr sctlr_el1, {control}", "isb", mair=in(reg) MAIR_DEVICE_NGNRNE_NORMAL_WB, tcr=in(reg) (TCR_T0SZ_39_BIT | TCR_IRGN0_WB | TCR_ORGN0_WB | TCR_SH0_INNER | TCR_EPD1 | TCR_IPS_40_BIT | if asid_bits() == Some(16) { TCR_ASID_16_BIT } else { 0 }), root=in(reg) root, control=in(reg) control, options(nostack));
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

/// # Safety
/// Root owns live aligned tables and preserves this CPU's kernel code/stack mappings.
/// IRQ is masked; caller retains the address space until all CPUs leave it. No ASID reuse
/// without completed local invalidation. The foundation pins each space to one CPU.
pub unsafe fn activate_root(root: u64) {
    #[cfg(feature = "kernel-tests")]
    crate::asid::record_switch();
    #[cfg(feature = "kernel-tests")]
    crate::asid::record_full_tlbi();
    // SAFETY: INV-USER-TTBR: native ASID zero, caller retains the live root; this is the explicit full-flush baseline and quiescent return path.
    unsafe {
        asm!("dsb ish", "msr ttbr0_el1, {}", "isb", "tlbi vmalle1", "dsb ish", "isb", in(reg) root, options(nostack));
    }
}

/// Switch between fixed-affinity process roots. ASID-tagged switches need no TLBI.
pub unsafe fn activate_user_root(root: u64, asid: u16) {
    if asid == 0 {
        // Unsupported hardware and the explicit baseline preserve the original contract.
        unsafe {
            activate_root(root);
        }
        return;
    }
    #[cfg(feature = "kernel-tests")]
    crate::asid::record_switch();
    let ttbr = crate::asid::ttbr(root, asid);
    // SAFETY: INV-USER-TTBR: the pinned root owns live tables and ASID lease; IRQ is masked.
    unsafe {
        asm!("dsb ish", "msr ttbr0_el1, {}", "isb", in(reg) ttbr, options(nostack));
    }
    #[cfg(feature = "kernel-tests")]
    assert_eq!(
        (active_root() >> 48) as u16,
        asid,
        "hardware TTBR ASID readback"
    );
}

/// Enter the native root while retaining other pinned ASID translations. ASID
/// zero hardware fallback keeps the full-flush correctness baseline.
pub unsafe fn activate_native_root(root: u64) {
    if !crate::asid::enabled() {
        unsafe {
            activate_root(root);
        }
        return;
    }
    #[cfg(feature = "kernel-tests")]
    crate::asid::record_switch();
    // SAFETY: INV-USER-RETIRE: native ASID zero maps the permanent kernel stack/code.
    unsafe {
        asm!("dsb ish", "msr ttbr0_el1, {}", "isb", in(reg) root, options(nostack));
    }
}

pub fn local_invalidate_asid(asid: u16) {
    let operand = (u64::from(asid)) << 48;
    // SAFETY: INV-ASID-RETIRE: pinned owner CPU has stopped using this lease; local TLBI completes before retirement acknowledgement.
    #[cfg(not(feature = "asid-reuse-negative"))]
    unsafe {
        asm!("dsb ish", "tlbi aside1, {}", "dsb ish", "isb", in(reg) operand, options(nostack));
    }
    #[cfg(feature = "asid-reuse-negative")]
    let _ = operand;
}
pub fn publish_instructions(start: usize, end: usize) {
    clean_boot(start, end);
    // SAFETY: INV-USER-IMAGE: completed code writes to PoC before global inner-shareable I-cache invalidation.
    unsafe {
        asm!("ic ialluis", "dsb ish", "isb", options(nostack));
    }
}
