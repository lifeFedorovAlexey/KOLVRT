#[cfg(feature = "kernel-tests")]
use crate::hal::Registers;
use kernel_core::platform::{Description, FDT_HEADER_BYTES, FDT_TOTAL_SIZE_OFFSET, discover};
pub mod config;
pub use config::PHYSICAL_TIMER_IRQ;
pub use config::RAM_BASE;
pub const RAM_SIZE: u64 = config::RAM_BYTES as u64;
const MAX_BOOT_DESCRIPTION_BYTES: usize = 0x200000;
pub fn discover_boot() -> Description {
    assert_eq!(config::ACTIVE_CPUS, kernel_core::platform::MAX_BOOT_CPUS);
    assert_eq!(config::CONFIGURED_CPUS, config::ACTIVE_CPUS);
    // SAFETY: INV-FDT: pinned ELF boot places immutable DTB at RAM start, first 40 bytes RAM.
    let header = unsafe { core::slice::from_raw_parts(RAM_BASE as *const u8, FDT_HEADER_BYTES) };
    let size = u32::from_be_bytes(
        header[FDT_TOTAL_SIZE_OFFSET..FDT_TOTAL_SIZE_OFFSET + core::mem::size_of::<u32>()]
            .try_into()
            .unwrap(),
    ) as usize;
    assert!((FDT_HEADER_BYTES..=MAX_BOOT_DESCRIPTION_BYTES).contains(&size));
    // SAFETY: INV-FDT: checked extent ends before loaded kernel; firmware/CPU1 never mutates it.
    let bytes = unsafe { core::slice::from_raw_parts(RAM_BASE as *const u8, size) };
    let d = discover(bytes).expect("invalid boot description");
    assert_eq!(d.ram.base, RAM_BASE as u64);
    assert_eq!(d.ram.size, RAM_SIZE);
    for r in [d.uart, d.distributor, d.redistributor] {
        assert!(r.base % config::PAGE_BYTES as u64 == 0 && r.end().unwrap() <= RAM_BASE as u64);
    }
    let regions = [d.uart, d.distributor, d.redistributor];
    for (i, a) in regions.iter().enumerate() {
        for b in &regions[i + 1..] {
            assert!(a.end().unwrap() <= b.base || b.end().unwrap() <= a.base);
        }
    }
    assert_eq!(d.timer_irq, PHYSICAL_TIMER_IRQ);
    assert!(
        d.psci_smc && d.cpu_count == config::ACTIVE_CPUS,
        "SMP platform contract"
    );
    d
}
#[cfg(feature = "kernel-tests")]
pub fn uart(d: &Description) -> Registers {
    // SAFETY: INV-MMIO: discovered and platform-validated PL011; identity device mapping.
    unsafe { Registers::new(d.uart.base as usize, d.uart.size as usize) }
}
