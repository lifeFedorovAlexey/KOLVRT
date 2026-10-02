use crate::time::Duration;
use crate::{
    EXPECTED_PC, EXPECTED_RESUME, FAULT_ESR, FAULT_FAR, cpu, interrupt, log, memory, platform, sync,
};
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
    log!(
        "{{\"event\":\"test\",\"name\":\"{}\",\"status\":\"{}\"}}\n",
        name,
        if passed { "pass" } else { "fail" }
    );
    assert!(passed, "kernel test failed: {}", name);
    TESTS_REPORTED.fetch_add(1, Ordering::Relaxed);
}
fn expected(pc: usize) {
    FAULT_ESR.store(0, Ordering::Release);
    FAULT_FAR.store(0, Ordering::Release);
    EXPECTED_RESUME.store((pc + cpu::INSTRUCTION_BYTES) as u64, Ordering::Release);
    EXPECTED_PC.store(pc as u64, Ordering::Release);
}
fn wait_irq() -> bool {
    let end = crate::time::deadline_after(TEST_IRQ_TIMEOUT);
    while cpu::ticks() < end {
        if interrupt::DELIVERED.load(Ordering::Acquire) > 0 {
            return true;
        }
    }
    false
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
        FAULT_ESR.load(Ordering::Acquire) >> cpu::ESR_EC_SHIFT == cpu::ESR_EC_BRK,
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
        FAULT_ESR.load(Ordering::Acquire) & cpu::ESR_FAULT_STATUS_MASK
            == cpu::ESR_TRANSLATION_FAULT_L3
            && FAULT_FAR.load(Ordering::Acquire) == memory::DYNAMIC_BASE as u64,
    );
    let mapping = memory::map(&a, memory::DYNAMIC_BASE, false, false).unwrap();
    expected(&raw const probe_write_pc as usize);
    // SAFETY: INV-PROBE: deliberate write-permission fault in test-only probe.
    unsafe {
        probe_write(mapping.address(), 0);
    }
    report(
        "permissions",
        FAULT_ESR.load(Ordering::Acquire) & cpu::ESR_FAULT_STATUS_MASK
            == cpu::ESR_PERMISSION_FAULT_L3,
    );
    expected(mapping.address());
    EXPECTED_RESUME.store(&raw const probe_execute_resume as u64, Ordering::Release);
    // SAFETY: INV-PROBE: execute-never page; test handler resumes at exact assembly recovery label.
    unsafe {
        probe_execute(mapping.address());
    }
    report(
        "execute_never",
        FAULT_ESR.load(Ordering::Acquire) >> cpu::ESR_EC_SHIFT
            == cpu::ESR_EC_INSTRUCTION_ABORT_CURRENT_EL
            && FAULT_ESR.load(Ordering::Acquire) & cpu::ESR_FAULT_STATUS_MASK
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
    interrupt::DELIVERED.store(0, Ordering::Release);
    cpu::mask();
    cpu::timer(crate::time::deadline_after(TEST_TIMER_DELAY));
    let stop = crate::time::deadline_after(TEST_MASK_INTERVAL);
    while cpu::ticks() < stop {
        core::hint::spin_loop();
    }
    report(
        "interrupt_masking",
        interrupt::DELIVERED.load(Ordering::Acquire) == 0,
    );
    cpu::unmask();
    let delivered = wait_irq();
    cpu::mask();
    report("interrupt_delivery", delivered);
    interrupt::DELIVERED.store(0, Ordering::Release);
    cpu::timer(crate::time::deadline_after(TEST_TIMER_DELAY));
    cpu::unmask();
    let delivered = wait_irq();
    cpu::mask();
    report("timer_rearm", delivered);
    interrupt::DELIVERED.store(0, Ordering::Release);
    cpu::timer(crate::time::deadline_after(TEST_TIMER_DELAY));
    cpu::unmask();
    // SAFETY: INV-PROBE: aligned AtomicU64 loaded with LDAR, bounded physical-counter wait; vector must undo IRQ-side SIMD/FP clobber.
    let context = unsafe {
        probe_simd_irq(
            interrupt::DELIVERED.as_ptr(),
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
    log!(
        "{{\"event\":\"measurement\",\"scope\":\"lock_uncontended\",\"units\":\"timer_ticks\",\"frequency\":{},\"warmup\":{},\"iterations\":{},\"median\":{},\"p95\":{},\"p99\":{},\"samples\":{:?}}}\n",
        cpu::frequency(),
        LOCK_MEASUREMENT_WARMUP,
        LOCK_MEASUREMENT_SAMPLES,
        median,
        p95,
        p99,
        raw_samples
    );
    #[cfg(feature = "negative-test")]
    report("negative_control", cpu::el() == cpu::CURRENT_EL2);
    #[cfg(feature = "panic-test")]
    panic!("panic reporting negative control");
    #[cfg(not(any(feature = "negative-test", feature = "panic-test")))]
    log!(
        "{{\"event\":\"suite\",\"status\":\"pass\",\"tests\":{}}}\n",
        TESTS_REPORTED.load(Ordering::Relaxed)
    );
}
