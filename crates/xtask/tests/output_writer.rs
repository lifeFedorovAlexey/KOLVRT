use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
static WRITES: AtomicUsize = AtomicUsize::new(0);
static TX_FULL: AtomicBool = AtomicBool::new(false);
static SECONDARY: AtomicBool = AtomicBool::new(false);
mod percpu {
    pub fn is_secondary() -> bool {
        super::SECONDARY.load(super::Ordering::Relaxed)
    }
}
mod hal {
    pub struct Registers;
    impl Registers {
        // Same constructor contract as target HAL; this host mock accesses no MMIO.
        pub unsafe fn new(_base: usize, _size: usize) -> Self {
            Self
        }
        pub fn read(&self, _offset: usize) -> u32 {
            if super::TX_FULL.load(super::Ordering::Relaxed) {
                1 << 5
            } else {
                0
            }
        }
        pub fn write(&self, _offset: usize, _value: u32) {
            super::WRITES.fetch_add(1, super::Ordering::Relaxed);
        }
    }
}
#[path = "../../kernel/src/diagnostics/mod.rs"]
#[allow(unexpected_cfgs)] // Target feature is not a feature of the host xtask package.
mod diagnostics;

#[test]
fn writer_rejects_oversize_before_mmio_and_bounds_unavailable_output() {
    diagnostics::initialize(1);
    diagnostics::boot_banner();
    WRITES.store(0, Ordering::Relaxed);
    let oversized = "x".repeat(4097);
    diagnostics::print(format_args!("{}", oversized));
    assert_eq!(WRITES.load(Ordering::Relaxed), 0);
    assert_eq!(diagnostics::dropped(), 1);
    diagnostics::status("OK", "test", format_args!("bounded"));
    let writes = WRITES.load(Ordering::Relaxed);
    assert!(writes > 0);
    TX_FULL.store(true, Ordering::Relaxed);
    diagnostics::print(format_args!("stalled"));
    assert_eq!(WRITES.load(Ordering::Relaxed), writes);
    assert_eq!(diagnostics::dropped(), 2);
    TX_FULL.store(false, Ordering::Relaxed);
    SECONDARY.store(true, Ordering::Relaxed);
    diagnostics::print(format_args!("wrong CPU"));
    assert_eq!(WRITES.load(Ordering::Relaxed), writes);
    assert_eq!(diagnostics::dropped(), 3);
    SECONDARY.store(false, Ordering::Relaxed);
    diagnostics::initialize(0);
    diagnostics::print(format_args!("unavailable"));
    assert_eq!(diagnostics::dropped(), 4);
}
