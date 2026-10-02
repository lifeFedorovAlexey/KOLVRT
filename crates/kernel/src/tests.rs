use crate::time::Duration;
use crate::{cpu, event, interrupt, memory, percpu, platform, smp, sync};
use alloc::{boxed::Box, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};
static TESTS_REPORTED: AtomicUsize = AtomicUsize::new(0);
const UART_PID0: usize = 0xfe0;
const UART_PID1: usize = 0xfe4;
const UART_PID_MASK: u32 = 0xff;
const UART_PID0_PL011: u32 = 0x11;
const UART_PID1_PL011: u32 = 0x10;
const TEST_TIMER_DELAY: Duration = Duration::from_millis(1);
const TEST_MASK_INTERVAL: Duration = Duration::from_millis(10);
const TEST_IRQ_TIMEOUT: Duration = Duration::from_millis(500);
const HEAP_TEST_PATTERN: u64 = 0xa5;
const HEAP_TEST_WORDS: usize = 64;
const HEAP_TEST_ELEMENTS: u64 = 256;
const PAGE_TEST_PATTERN: u64 = 0x0123_4567_89ab_cdef;
const LOCK_TEST_INITIAL: u64 = 1;
const LOCK_TEST_INCREMENT: u64 = 2;
const LOCK_MEASUREMENT_WARMUP: usize = 8;
const LOCK_MEASUREMENT_SAMPLES: usize = 64;
const IPI_REPETITIONS: usize = 32;
const SMP_WORD_PATTERN: u64 = 0x534d505f4b4f4c; // ASCII SMP_KOL.
use kernel_core::platform::Description;
unsafe extern "C" {
    fn probe_read(address: usize) -> u64;
    fn probe_write(address: usize, value: u64);
    fn probe_break();
    fn probe_execute(address: usize);
    fn probe_simd_irq(counter: *const u64, deadline: u64) -> u64;
    static probe_execute_resume: u8;
    static probe_read_pc: u8;
    static probe_write_pc: u8;
    static probe_break_pc: u8;
}
fn report(name: &str, passed: bool) {
    event!(
        "{{\"event\":\"test\",\"name\":\"{}\",\"status\":\"{}\"}}",
        name,
        if passed { "pass" } else { "fail" }
    );
    if !passed {
        crate::diagnostics::status("FAIL", "test", format_args!("{}", name));
    }
    assert!(passed, "kernel test failed: {}", name);
    TESTS_REPORTED.fetch_add(1, Ordering::Relaxed);
}
fn expected(pc: usize) {
    percpu::current().fault_esr.store(0, Ordering::Release);
    percpu::current().fault_far.store(0, Ordering::Release);
    percpu::current()
        .expected_resume
        .store((pc + cpu::INSTRUCTION_BYTES) as u64, Ordering::Release);
    percpu::current()
        .expected_pc
        .store(pc as u64, Ordering::Release);
}
fn wait_irq() -> bool {
    let end = crate::time::deadline_after(TEST_IRQ_TIMEOUT);
    while cpu::ticks() < end {
        if interrupt::delivered().load(Ordering::Acquire) > 0 {
            return true;
        }
    }
    false
}
pub fn remote_fault(address: usize) {
    expected(&raw const probe_read_pc as usize);
    // SAFETY: INV-PROBE: secondary's private probe registration; deliberately retired mapping, resumes at exact registered PC.
    unsafe {
        probe_read(address);
    }
    assert_eq!(
        percpu::current().fault_far.load(Ordering::Acquire),
        address as u64
    );
}
fn multicore(p: &mut memory::Physical) {
    use percpu::{BOOT_CPU, SECONDARY_CPU};
    use smp::experiment as remote;
    let first = &percpu::CPUS[BOOT_CPU];
    let second = &percpu::CPUS[SECONDARY_CPU];
    report(
        "smp_secondary_boot",
        second.state.load(Ordering::Acquire) == percpu::ONLINE
            && cpu::affinity_state(second.affinity.load(Ordering::Acquire))
                == cpu::PSCI_AFFINITY_ON,
    );
    report(
        "smp_cpu_identity",
        first.affinity.load(Ordering::Acquire) != second.affinity.load(Ordering::Acquire)
            && percpu::id() == BOOT_CPU,
    );
    unsafe extern "C" {
        static __secondary_stack_bottom: u8;
        static __secondary_stack_top: u8;
    }
    let stack = second.stack.load(Ordering::Acquire);
    report(
        "smp_separate_stacks",
        stack >= &raw const __secondary_stack_bottom as usize
            && stack < &raw const __secondary_stack_top as usize
            && first.stack.load(Ordering::Acquire) < &raw const __secondary_stack_bottom as usize,
    );
    let primary_ipis = first.ipis.load(Ordering::Acquire);
    let secondary_ipis = second.ipis.load(Ordering::Acquire);
    smp::ping(SECONDARY_CPU);
    smp::wait(
        || second.ipis.load(Ordering::Acquire) == secondary_ipis + 1,
        "IPI forward timeout",
    );
    report(
        "smp_ipi_forward",
        first.ipis.load(Ordering::Acquire) == primary_ipis,
    );
    cpu::unmask();
    // SAFETY: INV-REMOTE-READER: CPU0 test retains mapped immutable data through completion/retirement; FAULT is an exact registered probe, control commands carry zero.
    unsafe {
        remote::submit(remote::REPLY, 0);
    }
    remote::complete(remote::REPLY);
    smp::wait(
        || first.ipis.load(Ordering::Acquire) > primary_ipis,
        "IPI reply timeout",
    );
    cpu::mask();
    report(
        "smp_ipi_reverse",
        first.ipis.load(Ordering::Acquire) == primary_ipis + 1,
    );
    let before = second.ipis.load(Ordering::Acquire);
    for step in 0..IPI_REPETITIONS {
        smp::ping(SECONDARY_CPU);
        smp::wait(
            || second.ipis.load(Ordering::Acquire) > before + step as u64,
            "repeated IPI timeout",
        );
    }
    report(
        "smp_ipi_repeated",
        second.ipis.load(Ordering::Acquire) == before + IPI_REPETITIONS as u64,
    );
    *remote::LOCK_DATA.lock() = (0, remote::PUBLICATION_XOR);
    // SAFETY: INV-REMOTE-READER: CPU0 test retains mapped immutable data through completion/retirement; FAULT is an exact registered probe, control commands carry zero.
    unsafe {
        remote::submit(remote::LOCK, 0);
    }
    for _ in 0..remote::LOCK_ITERATIONS {
        remote::increment();
    }
    remote::complete(remote::LOCK);
    let pair = *remote::LOCK_DATA.lock();
    report(
        "smp_lock_publication",
        pair.0 == remote::LOCK_ITERATIONS * percpu::CPUS.len() as u64
            && pair.1 == pair.0 ^ remote::PUBLICATION_XOR,
    );
    let a = p.allocate(1, 1).unwrap();
    let address = a.address();
    let mut mapping = memory::map(&a, memory::DYNAMIC_BASE, true, false).unwrap();
    mapping.write_word(SMP_WORD_PATTERN).unwrap();
    cpu::barrier();
    // SAFETY: INV-REMOTE-READER: CPU0 test retains mapped immutable data through completion/retirement; FAULT is an exact registered probe, control commands carry zero.
    unsafe {
        remote::submit(remote::READ, mapping.address());
    }
    remote::complete(remote::READ);
    report(
        "smp_mapping_visible",
        remote::RESULT.load(Ordering::Acquire) == SMP_WORD_PATTERN,
    );
    remote::GATE.store(false, Ordering::Release);
    // SAFETY: INV-REMOTE-READER: CPU0 test retains mapped immutable data through completion/retirement; FAULT is an exact registered probe, control commands carry zero.
    unsafe {
        remote::submit(remote::HOLD, mapping.address());
    }
    smp::wait(
        || remote::ENTERED.load(Ordering::Acquire),
        "reader entry timeout",
    );
    let retirement = mapping.retire();
    report(
        "smp_retirement_pending",
        !retirement.acknowledged() && !p.reclaimable(&a),
    );
    #[cfg(feature = "retirement-negative")]
    {
        core::mem::forget(retirement);
        p.release(a);
        panic!("premature retirement release accepted");
    }
    #[cfg(not(feature = "retirement-negative"))]
    {
        let alternative = p.allocate(1, 1).unwrap();
        report("smp_no_premature_reuse", alternative.address() != address);
        p.release(alternative);
        remote::GATE.store(true, Ordering::Release);
        remote::complete(remote::HOLD);
        drop(retirement);
        report(
            "smp_remote_ack",
            p.reclaimable(&a) && second.tlb_ack.load(Ordering::Acquire) > 0,
        );
        // SAFETY: INV-REMOTE-READER: CPU0 test retains mapped immutable data through completion/retirement; FAULT is an exact registered probe, control commands carry zero.
        unsafe {
            remote::submit(remote::FAULT, memory::DYNAMIC_BASE);
        }
        remote::complete(remote::FAULT);
        report(
            "smp_remote_tlb_invalidation",
            remote::RESULT.load(Ordering::Acquire) & cpu::ESR_FAULT_STATUS_MASK
                == cpu::ESR_TRANSLATION_FAULT_L3,
        );
        p.release(a);
        let reused = p.allocate(1, 1).unwrap();
        report(
            "smp_safe_reuse",
            reused.address() == address && p.reclaimable(&reused),
        );
        p.release(reused);
    }
    let first_timers = first.timers.load(Ordering::Acquire);
    let second_timers = second.timers.load(Ordering::Acquire);
    // SAFETY: INV-REMOTE-READER: CPU0 test retains mapped immutable data through completion/retirement; FAULT is an exact registered probe, control commands carry zero.
    unsafe {
        remote::submit(remote::TIMER, 0);
    }
    cpu::timer(crate::time::deadline_after(TEST_TIMER_DELAY));
    cpu::unmask();
    smp::wait(
        || first.timers.load(Ordering::Acquire) > first_timers,
        "primary SMP timer timeout",
    );
    remote::complete(remote::TIMER);
    cpu::mask();
    report(
        "smp_simultaneous_timers",
        first.timers.load(Ordering::Acquire) > first_timers
            && second.timers.load(Ordering::Acquire) > second_timers,
    );
    report(
        "smp_percpu_independent",
        first.stack.load(Ordering::Acquire) != second.stack.load(Ordering::Acquire)
            && second.fault_far.load(Ordering::Acquire) == memory::DYNAMIC_BASE as u64,
    );
    #[cfg(feature = "secondary-panic-test")]
    {
        // SAFETY: INV-REMOTE-READER: CPU0 test retains mapped immutable data through completion/retirement; FAULT is an exact registered probe, control commands carry zero.
        unsafe {
            remote::submit(remote::PANIC, 0);
        }
        remote::complete(remote::PANIC);
        panic!("secondary panic not detected");
    }
    let users = crate::scheduler::exercise(p);
    report(
        "el0_processes",
        users.processes == platform::config::USER_PROCESSES,
    );
    report("el0_timer_switches", users.switches > users.processes);
    report(
        "el0_user_stacks",
        users.workers == platform::config::ACTIVE_CPUS,
    );
    report(
        "el0_memory_isolation",
        users.faults == users.processes - users.workers,
    );
    report(
        "el0_fault_containment",
        users.workers == platform::config::ACTIVE_CPUS && users.faults > 0,
    );
    report("el0_quiescent_reclamation", users.reclaimed);
    report(
        "el0_context_preservation",
        users.workers == platform::config::ACTIVE_CPUS,
    );
    report("el0_smp_ownership", users.owners_released);
    const PRIMARY_KERNEL_PROBE: usize = 1;
    const PRIMARY_FOREIGN_PROBE: usize = 2;
    const PRIMARY_CODE_PROBE: usize = 3;
    const SECONDARY_GUARD_PROBE: usize = platform::config::USER_PROCESSES_PER_CPU + 1;
    const SECONDARY_EXECUTE_PROBE: usize = platform::config::USER_PROCESSES_PER_CPU + 2;
    const SECONDARY_PRIVILEGED_PROBE: usize = platform::config::USER_PROCESSES_PER_CPU + 3;
    report(
        "el0_kernel_memory_rejected",
        users.fault_checks[PRIMARY_KERNEL_PROBE],
    );
    report(
        "el0_foreign_memory_rejected",
        users.fault_checks[PRIMARY_FOREIGN_PROBE],
    );
    report(
        "el0_code_write_rejected",
        users.fault_checks[PRIMARY_CODE_PROBE],
    );
    report("el0_stack_guard", users.fault_checks[SECONDARY_GUARD_PROBE]);
    report(
        "el0_data_execute_rejected",
        users.fault_checks[SECONDARY_EXECUTE_PROBE],
    );
    report(
        "el0_privileged_instruction_rejected",
        users.fault_checks[SECONDARY_PRIVILEGED_PROBE],
    );
    // SAFETY: INV-REMOTE-READER: bounded control work has no borrowed pointer; shutdown drains it before CPU_OFF.
    unsafe {
        remote::submit(remote::LOCK, 0);
    }
    smp::shutdown();
    report(
        "smp_orderly_shutdown",
        second.state.load(Ordering::Acquire) == percpu::QUIESCENT
            && cpu::affinity_state(second.affinity.load(Ordering::Acquire))
                == cpu::PSCI_AFFINITY_OFF
            && remote::DONE.load(Ordering::Acquire) == remote::LOCK
            && remote::LOCK_DATA.lock().0
                == remote::LOCK_ITERATIONS * (percpu::CPUS.len() as u64 + 1),
    );
}
pub fn run(d: &Description, p: &mut memory::Physical) {
    TESTS_REPORTED.store(0, Ordering::Relaxed);
    report("boot_el1", cpu::el() == cpu::CURRENT_EL1);
    let uart = platform::uart(d);
    report(
        "uart_mmio",
        uart.read(UART_PID0) & UART_PID_MASK == UART_PID0_PL011
            && uart.read(UART_PID1) & UART_PID_MASK == UART_PID1_PL011,
    );
    expected(&raw const probe_break_pc as usize);
    // SAFETY: INV-PROBE: deliberate BRK at exact registered instruction, test build only.
    unsafe {
        probe_break();
    }
    report(
        "exception_vectors",
        percpu::current().fault_esr.load(Ordering::Acquire) >> cpu::ESR_EC_SHIFT == cpu::ESR_EC_BRK,
    );
    report(
        "physical_discovery",
        d.ram.base == platform::RAM_BASE as u64 && d.ram.size == platform::RAM_SIZE,
    );
    let before = p.available();
    let a = p.allocate(1, 1).unwrap();
    let address = a.address();
    let b = p.allocate(1, 1).unwrap();
    report(
        "physical_allocator",
        a.address() != b.address() && p.available() == before - 2,
    );
    p.release(b);
    p.release(a);
    report("allocation_free", p.available() == before);
    let a = p.allocate(1, 1).unwrap();
    report("physical_reuse", a.address() == address);
    report(
        "physical_exhaustion",
        p.allocate(platform::config::PHYSICAL_PAGES, 1).is_none()
            && p.allocate(0, 1).is_none()
            && p.available() == before - 1,
    );
    let mapping = memory::map(&a, memory::DYNAMIC_BASE, true, false).unwrap();
    // SAFETY: INV-PROBE: newly allocated owned frame, mapped alias, aligned u64 access.
    unsafe {
        probe_write(mapping.address(), PAGE_TEST_PATTERN);
    }
    // SAFETY: INV-PROBE: same live frame, read initialized bytes through physical identity alias.
    let value = unsafe { probe_read(a.address()) };
    report("page_map", value == PAGE_TEST_PATTERN);
    report(
        "invalid_mapping_rejection",
        memory::map(&a, memory::DYNAMIC_BASE + 1, true, false).is_err()
            && memory::map(&a, memory::DYNAMIC_BASE, true, true).is_err()
            && memory::map(&a, memory::DYNAMIC_BASE, true, false).is_err(),
    );
    mapping.unmap();
    expected(&raw const probe_read_pc as usize);
    // SAFETY: INV-PROBE: deliberate unmapped access, exact test PC permits bounded recovery.
    unsafe {
        probe_read(memory::DYNAMIC_BASE);
    }
    report(
        "page_unmap",
        percpu::current().fault_esr.load(Ordering::Acquire) & cpu::ESR_FAULT_STATUS_MASK
            == cpu::ESR_TRANSLATION_FAULT_L3
            && percpu::current().fault_far.load(Ordering::Acquire) == memory::DYNAMIC_BASE as u64,
    );
    let mapping = memory::map(&a, memory::DYNAMIC_BASE, false, false).unwrap();
    expected(&raw const probe_write_pc as usize);
    // SAFETY: INV-PROBE: deliberate write-permission fault in test-only probe.
    unsafe {
        probe_write(mapping.address(), 0);
    }
    report(
        "permissions",
        percpu::current().fault_esr.load(Ordering::Acquire) & cpu::ESR_FAULT_STATUS_MASK
            == cpu::ESR_PERMISSION_FAULT_L3,
    );
    expected(mapping.address());
    percpu::current()
        .expected_resume
        .store(&raw const probe_execute_resume as u64, Ordering::Release);
    // SAFETY: INV-PROBE: execute-never page; test handler resumes at exact assembly recovery label.
    unsafe {
        probe_execute(mapping.address());
    }
    report(
        "execute_never",
        percpu::current().fault_esr.load(Ordering::Acquire) >> cpu::ESR_EC_SHIFT
            == cpu::ESR_EC_INSTRUCTION_ABORT_CURRENT_EL
            && percpu::current().fault_esr.load(Ordering::Acquire) & cpu::ESR_FAULT_STATUS_MASK
                == cpu::ESR_PERMISSION_FAULT_L3,
    );
    mapping.unmap();
    p.release(a);
    report("mmu", cpu::sctlr() & cpu::SCTLR_MMU_ENABLE != 0);
    let boxed = Box::new([HEAP_TEST_PATTERN; HEAP_TEST_WORDS]);
    let mut v = Vec::new();
    for i in 0..HEAP_TEST_ELEMENTS {
        v.push(i);
    }
    let expected_sum = HEAP_TEST_ELEMENTS * (HEAP_TEST_ELEMENTS - 1) / 2;
    report(
        "heap",
        boxed.iter().all(|&word| word == HEAP_TEST_PATTERN)
            && v.len() == HEAP_TEST_ELEMENTS as usize
            && v.iter().sum::<u64>() == expected_sum,
    );
    drop(v);
    drop(boxed);
    let layout =
        core::alloc::Layout::from_size_align(memory::HEAP_BYTES + 1, memory::HEAP_UNIT_BYTES)
            .unwrap();
    // SAFETY: INV-HEAP: explicit oversized layout exercises allocation failure; no non-null result is discarded.
    let exhausted =
        core::hint::black_box(unsafe { alloc::alloc::alloc(core::hint::black_box(layout)) });
    report("heap_exhaustion", exhausted.is_null());
    let lock = sync::Lock::new(LOCK_TEST_INITIAL);
    let mut guard = lock.lock();
    *guard += LOCK_TEST_INCREMENT;
    report("locking_exclusion", lock.try_lock().is_none());
    drop(guard);
    report(
        "locking_release",
        *lock.lock() == LOCK_TEST_INITIAL + LOCK_TEST_INCREMENT,
    );
    let start = cpu::ticks();
    while cpu::ticks() == start {
        core::hint::spin_loop();
    }
    report(
        "monotonic_time",
        cpu::frequency() > 0 && cpu::ticks() > start,
    );
    interrupt::delivered().store(0, Ordering::Release);
    cpu::mask();
    cpu::timer(crate::time::deadline_after(TEST_TIMER_DELAY));
    let stop = crate::time::deadline_after(TEST_MASK_INTERVAL);
    while cpu::ticks() < stop {
        core::hint::spin_loop();
    }
    report(
        "interrupt_masking",
        interrupt::delivered().load(Ordering::Acquire) == 0,
    );
    cpu::unmask();
    let delivered = wait_irq();
    cpu::mask();
    report("interrupt_delivery", delivered);
    interrupt::delivered().store(0, Ordering::Release);
    cpu::timer(crate::time::deadline_after(TEST_TIMER_DELAY));
    cpu::unmask();
    let delivered = wait_irq();
    cpu::mask();
    report("timer_rearm", delivered);
    interrupt::delivered().store(0, Ordering::Release);
    cpu::timer(crate::time::deadline_after(TEST_TIMER_DELAY));
    cpu::unmask();
    // SAFETY: INV-PROBE: aligned AtomicU64 loaded with LDAR, bounded physical-counter wait; vector must undo IRQ-side SIMD/FP clobber.
    let context = unsafe {
        probe_simd_irq(
            interrupt::delivered().as_ptr(),
            crate::time::deadline_after(TEST_IRQ_TIMEOUT),
        )
    };
    cpu::mask();
    report("irq_simd_context", context == 1);
    let mut samples = [0u64; LOCK_MEASUREMENT_SAMPLES];
    for i in 0..LOCK_MEASUREMENT_WARMUP + LOCK_MEASUREMENT_SAMPLES {
        let start = cpu::ticks();
        let g = lock.lock();
        core::hint::black_box(*g);
        drop(g);
        let elapsed = cpu::ticks() - start;
        if i >= LOCK_MEASUREMENT_WARMUP {
            samples[i - LOCK_MEASUREMENT_WARMUP] = elapsed;
        }
    }
    let raw_samples = samples;
    let [median, p95, p99] = kernel_core::quantiles(&mut samples).unwrap();
    event!(
        "{{\"event\":\"measurement\",\"scope\":\"lock_uncontended\",\"units\":\"timer_ticks\",\"frequency\":{},\"warmup\":{},\"iterations\":{},\"median\":{},\"p95\":{},\"p99\":{},\"samples\":{:?}}}",
        cpu::frequency(),
        LOCK_MEASUREMENT_WARMUP,
        LOCK_MEASUREMENT_SAMPLES,
        median,
        p95,
        p99,
        raw_samples
    );
    multicore(p);
    #[cfg(feature = "negative-test")]
    report("negative_control", cpu::el() == cpu::CURRENT_EL2);
    #[cfg(feature = "panic-test")]
    panic!("panic reporting negative control");
    #[cfg(not(any(feature = "negative-test", feature = "panic-test")))]
    crate::diagnostics::status(
        "OK",
        "tests",
        format_args!(
            "{} kernel checks passed",
            TESTS_REPORTED.load(Ordering::Relaxed)
        ),
    );
    #[cfg(not(any(feature = "negative-test", feature = "panic-test")))]
    event!(
        "{{\"event\":\"suite\",\"status\":\"pass\",\"tests\":{}}}",
        TESTS_REPORTED.load(Ordering::Relaxed)
    );
}
