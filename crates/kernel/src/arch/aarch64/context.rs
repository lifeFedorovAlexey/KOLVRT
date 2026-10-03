//! Architecture frame and lower-EL exception boundary, independent of boot fixtures.
use crate::{cpu, scheduler};
const GPRS: usize = 32;
const SIMD_REGISTERS: usize = 32;
const PAIR_WORDS: usize = 2;
const SAVED_REGISTER_BYTES: usize = (GPRS + SIMD_REGISTERS * PAIR_WORDS + 2) * 8;
const USER_META_WORDS: usize = 4;
pub(crate) const USER_EL0T: u64 = 0;
pub(crate) const PSTATE_MODE_MASK: u64 = 0x1f; // Includes the AArch32 nRW bit.
pub(crate) const ESR_CLASS_SHIFT: u64 = 26;
pub(crate) const ESR_SVC64: u64 = 0x15;
pub(crate) const ESR_DATA_ABORT_LOWER: u64 = 0x24;
pub(crate) const ESR_INSTRUCTION_ABORT_LOWER: u64 = 0x20;
pub(crate) const ESR_SYSREG_TRAP: u64 = 0x18;
pub(crate) const ESR_UNKNOWN: u64 = 0;
pub(crate) const SVC_IMMEDIATE_MASK: u64 = u16::MAX as u64;
const TRAP_IRQ: u64 = 1;
#[cfg(feature = "user-context-negative")]
const NEGATIVE_SAVED_REGISTER: usize = 20;
#[cfg(feature = "user-context-negative")]
pub(crate) fn corrupt_saved(context: &mut Context) {
    // One-way corruption cannot cancel itself across an even number of preemptions.
    context.gpr[NEGATIVE_SAVED_REGISTER] = 0;
}
#[cfg(feature = "scheduler-context-negative")]
const NEGATIVE_EL1H_MODE: u64 = 5;
#[cfg(feature = "scheduler-context-negative")]
const NEGATIVE_AARCH32_MODE: u64 = 1 << 4;
#[cfg(feature = "scheduler-context-negative")]
pub(crate) fn corrupt_mode(context: &mut Context) {
    context.pstate = if cfg!(feature = "scheduler-aarch32-negative") {
        NEGATIVE_AARCH32_MODE
    } else if cfg!(feature = "scheduler-user-irq-negative") {
        super::PSTATE_IRQ_MASK
    } else {
        NEGATIVE_EL1H_MODE
    };
}
fn require_user_context(context: &Context) {
    let valid = valid_user_context(context);
    #[cfg(feature = "scheduler-context-negative")]
    if !valid {
        crate::event!(
            "{{\"event\":\"scheduler-reject\",\"status\":\"fail\",\"error\":\"InvalidUserContext\"}}"
        );
    }
    assert!(valid, "invalid AArch64 EL0 context or masked user IRQ");
}
pub(crate) fn valid_user_context(context: &Context) -> bool {
    context.pstate & (PSTATE_MODE_MASK | super::PSTATE_IRQ_MASK) == USER_EL0T
}
pub(crate) enum Trap {
    Irq,
    Sync {
        class: u64,
        operation: u16,
        far: usize,
    },
}
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub(crate) struct Context {
    pub(crate) gpr: [u64; GPRS],
    pub(crate) simd: [[u64; PAIR_WORDS]; SIMD_REGISTERS],
    pub(crate) fpcr: u64,
    pub(crate) fpsr: u64,
    pub(crate) pc: u64,
    pub(crate) pstate: u64,
    pub(crate) sp: u64,
    pub(crate) tpidr: u64,
}
const _: () = {
    assert!(core::mem::offset_of!(Context, simd) == GPRS * core::mem::size_of::<u64>());
    assert!(
        core::mem::offset_of!(Context, fpcr)
            == (GPRS + SIMD_REGISTERS * PAIR_WORDS) * core::mem::size_of::<u64>()
    );
    assert!(core::mem::offset_of!(Context, pc) == SAVED_REGISTER_BYTES);
    assert!(
        core::mem::offset_of!(Context, pstate)
            == SAVED_REGISTER_BYTES + core::mem::size_of::<u64>()
    );
    assert!(
        core::mem::offset_of!(Context, sp)
            == SAVED_REGISTER_BYTES + PAIR_WORDS * core::mem::size_of::<u64>()
    );
    assert!(
        core::mem::offset_of!(Context, tpidr)
            == SAVED_REGISTER_BYTES + (USER_META_WORDS - 1) * core::mem::size_of::<u64>()
    );
    assert!(
        core::mem::size_of::<Context>()
            == SAVED_REGISTER_BYTES + USER_META_WORDS * core::mem::size_of::<u64>()
    );
};
impl Context {
    pub(crate) const ZERO: Self = Self {
        gpr: [0; GPRS],
        simd: [[0; PAIR_WORDS]; SIMD_REGISTERS],
        fpcr: 0,
        fpsr: 0,
        pc: 0,
        pstate: USER_EL0T,
        sp: 0,
        tpidr: 0,
    };
}

unsafe extern "C" {
    fn run_user(context: *const Context, resume: *mut usize);
}
/// # Safety
/// Retained root/context and live permanent kernel stack; no scheduler borrow or
/// exclusion permit survives this call. Assembly restores the native ABI before return.
pub(crate) unsafe fn enter(context: &Context, resume: *mut usize) {
    require_user_context(context);
    assert!(cpu::irq_masked());
    assert!(
        !crate::percpu::current()
            .scheduler_borrow
            .load(core::sync::atomic::Ordering::Acquire),
        "scheduler borrow across user entry"
    );
    crate::sync::assert_scheduler_unlocked();
    // SAFETY: INV-USER-CONTEXT: checked exact frame ABI and caller's retained root;
    // assembly publishes resume SP using STLR before any lower-EL return.
    unsafe {
        run_user(context, resume);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn user_trap(
    frame: *mut Context,
    kind: u64,
    entry_ticks: u64,
) -> usize {
    assert!(cpu::irq_masked(), "user exception IRQ contract");
    let trap = if kind == TRAP_IRQ {
        Trap::Irq
    } else {
        let esr = cpu::user_esr();
        let class = esr >> ESR_CLASS_SHIFT;
        Trap::Sync {
            class,
            operation: (esr & SVC_IMMEDIATE_MASK) as u16,
            // FAR is not defined for SVC/sysreg/unknown exceptions. Never expose
            // the stale address left by another process's earlier abort.
            far: if class == ESR_DATA_ABORT_LOWER || class == ESR_INSTRUCTION_ABORT_LOWER {
                cpu::user_far() as usize
            } else {
                0
            },
        }
    };
    // SAFETY: INV-USER-CONTEXT: lower-EL vector supplies the complete aligned frame
    // on the CPU's permanent EL1 stack; the mutable borrow ends before ERET.
    let resume = scheduler::trap(unsafe { &mut *frame }, trap, entry_ticks);
    if resume == 0 {
        // SAFETY: INV-USER-CONTEXT: same live vector frame, scheduler borrow ended;
        // validate the selected return state before the assembly ERET boundary.
        require_user_context(unsafe { &*frame });
    }
    resume
}
