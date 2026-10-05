//! Fixed two-CPU coordination, not a scheduler. CPU0 serializes admission and PTE updates.
//! IRQ never waits or takes locks. CPU1 acknowledges TLBI only between bounded work items.
use crate::{cpu, percpu, time};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use percpu::SECONDARY_CPU;
pub const IPI: u8 = 1;
const COORDINATION_TIMEOUT: time::Duration = time::Duration::from_secs(2);
static READY: AtomicBool = AtomicBool::new(false);
static STOP: AtomicBool = AtomicBool::new(false);
static TLB_REQUEST: AtomicU64 = AtomicU64::new(0);
static TLB_SIGNAL: AtomicU64 = AtomicU64::new(0);
#[cfg(all(feature = "kernel-tests", not(feature = "retirement-negative")))]
static REMOTE_TLBI_COMPLETED: AtomicU64 = AtomicU64::new(0);

/// Test-only execution witness, distinct from the secondary's acknowledgement.
/// A translation fault alone cannot prove TLBI ran: cache eviction can also remove it.
#[cfg(all(feature = "kernel-tests", not(feature = "retirement-negative")))]
pub fn remote_tlbi_completed(generation: u64) -> bool {
    generation > 0 && REMOTE_TLBI_COMPLETED.load(Ordering::Acquire) >= generation
}
#[cfg(feature = "kernel-tests")]
pub mod experiment {
    use super::*;
    pub const LOCK: u64 = 1;
    pub const READ: u64 = 2;
    pub const HOLD: u64 = 3;
    pub const FAULT: u64 = 4;
    pub const PANIC: u64 = 5;
    pub const TIMER: u64 = 6;
    pub const REPLY: u64 = 7;
    pub const PROCESS_CONTEXT: u64 = 8;
    pub const LOCK_ITERATIONS: u64 = 4096;
    pub const PUBLICATION_XOR: u64 = 0x4b4f4c; // ASCII KOL.
    const TIMER_DELAY: time::Duration = time::Duration::from_millis(1);
    pub static COMMAND: AtomicU64 = AtomicU64::new(0);
    pub static DONE: AtomicU64 = AtomicU64::new(0);
    pub static ADDRESS: AtomicU64 = AtomicU64::new(0);
    pub static RESULT: AtomicU64 = AtomicU64::new(0);
    pub static GATE: AtomicBool = AtomicBool::new(false);
    pub static ENTERED: AtomicBool = AtomicBool::new(false);
    pub static LOCK_DATA: crate::sync::Lock<(u64, u64)> = crate::sync::Lock::new((0, 0));
    /// # Safety
    /// READ/HOLD require a retained immutable mapped word until completion or acknowledged
    /// retirement; no new reader may be admitted during retirement. FAULT is an exact
    /// test-only probe. Other commands must use a zero address. CPU0 is the sole producer.
    pub unsafe fn submit(command: u64, address: usize) {
        percpu::primary_only();
        assert!(!STOP.load(Ordering::Acquire), "secondary admission stopped");
        if command == READ || command == HOLD {
            assert!(
                !crate::memory::retirement_pending(),
                "remote reader admission during retirement"
            );
        }
        assert_eq!(COMMAND.load(Ordering::Acquire), 0);
        DONE.store(0, Ordering::Release);
        ENTERED.store(false, Ordering::Release);
        ADDRESS.store(address as u64, Ordering::Release);
        COMMAND.store(command, Ordering::Release);
        ping(SECONDARY_CPU);
    }
    pub fn complete(command: u64) {
        wait(
            || DONE.load(Ordering::Acquire) == command,
            "secondary work timeout",
        );
    }
    pub fn increment() {
        let mut pair = LOCK_DATA.lock();
        assert_eq!(pair.1, pair.0 ^ PUBLICATION_XOR);
        pair.0 += 1;
        pair.1 = pair.0 ^ PUBLICATION_XOR;
    }
    pub fn execute(command: u64) {
        match command {
            LOCK => {
                for _ in 0..LOCK_ITERATIONS {
                    increment();
                }
            }
            READ | HOLD => {
                let address = ADDRESS.load(Ordering::Acquire) as usize;
                // SAFETY: INV-REMOTE-READER: test driver retains Mapping/Frame until DONE or shootdown acknowledgement; aligned initialized borrowed word.
                RESULT.store(
                    unsafe { core::ptr::read_volatile(address as *const u64) },
                    Ordering::Release,
                );
                ENTERED.store(true, Ordering::Release);
                if command == HOLD {
                    wait(|| GATE.load(Ordering::Acquire), "reader gate timeout");
                }
            }
            FAULT => {
                crate::tests::remote_fault(ADDRESS.load(Ordering::Acquire) as usize);
                RESULT.store(
                    percpu::current().fault_esr.load(Ordering::Acquire),
                    Ordering::Release,
                );
            }
            PANIC => panic!("secondary panic control"),
            REPLY => ping(percpu::BOOT_CPU),
            PROCESS_CONTEXT => RESULT.store(
                u64::from(
                    crate::process::context_contract()
                        == Err(kernel_core::process::Error::ForeignCpu),
                ),
                Ordering::Release,
            ),
            TIMER => {
                let before = percpu::current().timers.load(Ordering::Acquire);
                cpu::timer(time::deadline_after(TIMER_DELAY));
                wait(
                    || percpu::current().timers.load(Ordering::Acquire) > before,
                    "secondary timer timeout",
                );
            }
            _ => panic!("unknown secondary experiment"),
        }
    }
}
pub fn wait(ready: impl FnMut() -> bool, reason: &str) {
    wait_optional(ready, Some(COORDINATION_TIMEOUT), reason);
}
/// Native completion can have no workload deadline. Timeout never authorizes reclaim.
/// Callers wait without a scheduler borrow/ordinary lock and require admitted work
/// to eventually terminate for return; existing boot coordination keeps its bound.
pub fn wait_optional(
    mut ready: impl FnMut() -> bool,
    timeout: Option<time::Duration>,
    reason: &str,
) {
    assert!(
        !percpu::current().scheduler_borrow.load(Ordering::Acquire),
        "scheduler borrow across wait"
    );
    crate::sync::assert_scheduler_unlocked();
    let deadline = timeout.map(time::deadline_after);
    loop {
        assert_ne!(
            percpu::CPUS[SECONDARY_CPU].state.load(Ordering::Acquire),
            percpu::FAILED,
            "secondary CPU failure"
        );
        if ready() {
            return;
        }
        assert!(
            deadline.is_none_or(|limit| cpu::ticks() < limit),
            "{reason}"
        );
        core::hint::spin_loop();
    }
}
pub fn ping(id: usize) {
    cpu::send_sgi(percpu::CPUS[id].affinity.load(Ordering::Acquire), IPI);
}
pub fn on_ipi() {
    crate::scheduler::on_ipi();
    let id = percpu::id();
    percpu::current().ipis.fetch_add(1, Ordering::Release);
    if id == SECONDARY_CPU {
        TLB_SIGNAL.store(TLB_REQUEST.load(Ordering::Acquire), Ordering::Release);
    }
}
pub fn start(root: u64) {
    percpu::primary_only();
    unsafe extern "C" {
        fn secondary_entry();
    }
    READY.store(true, Ordering::Release);
    let (start, end) = crate::memory::kernel_bounds();
    cpu::clean_boot(start, end);
    assert_eq!(
        cpu::start_cpu(
            percpu::CPUS[SECONDARY_CPU].affinity.load(Ordering::Acquire),
            secondary_entry as *const () as usize,
            root
        ),
        cpu::PSCI_SUCCESS,
        "PSCI CPU_ON failed"
    );
    wait(
        || percpu::CPUS[SECONDARY_CPU].state.load(Ordering::Acquire) == percpu::ONLINE,
        "CPU1 boot timeout",
    );
    assert_eq!(
        cpu::affinity_state(percpu::CPUS[SECONDARY_CPU].affinity.load(Ordering::Acquire)),
        cpu::PSCI_AFFINITY_ON
    );
}
#[unsafe(no_mangle)]
pub extern "C" fn secondary_main(root: u64) -> ! {
    cpu::enable_mmu(root);
    cpu::vectors();
    assert_eq!(cpu::el(), cpu::CURRENT_EL1);
    assert_ne!(cpu::sctlr() & cpu::SCTLR_MMU_ENABLE, 0);
    assert!(READY.load(Ordering::Acquire));
    assert_eq!(percpu::id(), SECONDARY_CPU);
    let local = percpu::current();
    local.stack.store(cpu::stack_pointer(), Ordering::Release);
    crate::interrupt::initialize_local();
    cpu::timer_stop();
    local.state.store(percpu::ONLINE, Ordering::Release);
    cpu::unmask();
    loop {
        // IRQ only publishes the requested generation; no active reader is acknowledged from IRQ.
        let requested = TLB_SIGNAL.load(Ordering::Acquire);
        if local.tlb_ack.load(Ordering::Acquire) < requested {
            #[cfg(not(feature = "remote-tlbi-negative"))]
            {
                cpu::local_invalidate();
                #[cfg(all(feature = "kernel-tests", not(feature = "retirement-negative")))]
                REMOTE_TLBI_COMPLETED.store(requested, Ordering::Release);
            }
            #[cfg(not(feature = "shootdown-negative"))]
            local.tlb_ack.store(requested, Ordering::Release);
        }
        #[cfg(feature = "kernel-tests")]
        {
            let command = experiment::COMMAND.load(Ordering::Acquire);
            if command != 0 {
                local.state.store(percpu::WORKING, Ordering::Release);
                experiment::execute(command);
                local.state.store(percpu::ONLINE, Ordering::Release);
                experiment::COMMAND.store(0, Ordering::Release);
                experiment::DONE.store(command, Ordering::Release);
            }
        }
        crate::scheduler::poll_secondary();
        if STOP.load(Ordering::Acquire) {
            cpu::mask();
            cpu::timer_stop();
            cpu::local_invalidate();
            local.state.store(percpu::QUIESCENT, Ordering::Release);
            cpu::cpu_off();
        }
        core::hint::spin_loop();
    }
}
pub fn request_invalidation() -> u64 {
    percpu::primary_only();
    let old = TLB_REQUEST.load(Ordering::Acquire);
    assert_eq!(
        percpu::CPUS[SECONDARY_CPU].tlb_ack.load(Ordering::Acquire),
        old,
        "one outstanding retirement"
    );
    let generation = old.checked_add(1).expect("TLB generation exhausted");
    cpu::local_invalidate();
    let state = percpu::CPUS[SECONDARY_CPU].state.load(Ordering::Acquire);
    assert_ne!(state, percpu::FAILED, "secondary CPU failure");
    if state != percpu::OFF && state != percpu::QUIESCENT {
        TLB_REQUEST.store(generation, Ordering::Release);
        ping(SECONDARY_CPU);
        generation
    } else {
        old
    }
}
pub fn acknowledged(generation: u64) -> bool {
    percpu::CPUS[SECONDARY_CPU].tlb_ack.load(Ordering::Acquire) >= generation
}
pub fn finish_invalidation(generation: u64) {
    wait(
        || acknowledged(generation),
        "remote TLB acknowledgement timeout",
    );
    cpu::barrier();
}
pub fn shutdown() {
    percpu::primary_only();
    STOP.store(true, Ordering::Release);
    ping(SECONDARY_CPU);
    wait(
        || percpu::CPUS[SECONDARY_CPU].state.load(Ordering::Acquire) == percpu::QUIESCENT,
        "secondary quiescence timeout",
    );
    wait(
        || {
            cpu::affinity_state(percpu::CPUS[SECONDARY_CPU].affinity.load(Ordering::Acquire))
                == cpu::PSCI_AFFINITY_OFF
        },
        "secondary CPU_OFF timeout",
    );
    cpu::barrier();
}
pub fn secondary_failure() -> ! {
    cpu::mask();
    cpu::timer_stop();
    percpu::CPUS[SECONDARY_CPU]
        .state
        .store(percpu::FAILED, Ordering::Release);
    cpu::cpu_off();
}
