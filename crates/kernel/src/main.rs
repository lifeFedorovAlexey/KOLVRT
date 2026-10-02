#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
extern crate alloc;
mod arch {
    pub mod aarch64;
}
mod diagnostics;
mod hal;
mod interrupt;
mod memory;
mod platform;
mod sync;
#[cfg(feature = "kernel-tests")]
mod tests;
mod time;
use arch::aarch64 as cpu;
const BOOT_MEMORY_PATTERN: u64 = 0x4b4f4c565254; // ASCII "KOLVRT".
#[cfg(not(feature = "kernel-tests"))]
const BOOT_TIMER_DELAY: time::Duration = time::Duration::from_millis(10);
#[cfg(not(feature = "kernel-tests"))]
const BOOT_IRQ_TIMEOUT: time::Duration = time::Duration::from_secs(1);
#[cfg(feature = "kernel-tests")]
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
#[cfg(feature = "kernel-tests")]
static EXPECTED_PC: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "kernel-tests")]
static EXPECTED_RESUME: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "kernel-tests")]
static FAULT_ESR: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "kernel-tests")]
static FAULT_FAR: AtomicU64 = AtomicU64::new(0);
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    let d = platform::discover_boot();
    diagnostics::initialize(d.uart.base as usize);
    log!(
        "KOLVRT EL{} UART online\n",
        cpu::el() >> cpu::CURRENT_EL_SHIFT
    );
    cpu::vectors();
    let mut physical = memory::Physical::new(&d);
    #[cfg(feature = "diagnostics")]
    log!(
        "boot: RAM={:#x}+{:#x} free_pages={} vectors installed\n",
        d.ram.base,
        d.ram.size,
        physical.available()
    );
    memory::initialize_mmu(&d);
    memory::initialize_heap(&mut physical);
    interrupt::initialize(&d);
    let frame = physical.allocate(1, 1).expect("boot memory validation");
    let mut mapping =
        memory::map(&frame, memory::DYNAMIC_BASE, true, false).expect("boot page mapping");
    assert_eq!(mapping.read_word(), 0);
    mapping.write_word(BOOT_MEMORY_PATTERN).unwrap();
    assert_eq!(mapping.read_word(), BOOT_MEMORY_PATTERN);
    mapping.unmap();
    physical.release(frame);
    assert!(physical.available() > 0);
    #[cfg(feature = "diagnostics")]
    log!("boot: MMU W^X heap GICv3 physical timer ready; active_cpus=1\n");
    #[cfg(feature = "kernel-tests")]
    tests::run(&d, &mut physical);
    #[cfg(feature = "ownership-test")]
    {
        let _foreign_owner = memory::Physical::new(&d);
        panic!("second physical owner was accepted");
    }
    #[cfg(feature = "retained-mapping-test")]
    {
        let frame = physical
            .allocate(1, 1)
            .expect("retained mapping control allocation");
        let mapping = memory::map(&frame, memory::DYNAMIC_BASE, true, false).unwrap();
        core::mem::forget(mapping);
        physical.release(frame);
        panic!("retained mapping release was accepted");
    }
    #[cfg(not(feature = "kernel-tests"))]
    {
        cpu::timer(time::deadline_after(BOOT_TIMER_DELAY));
        cpu::unmask();
        let deadline = time::deadline_after(BOOT_IRQ_TIMEOUT);
        while interrupt::DELIVERED.load(Ordering::Acquire) == 0 && cpu::ticks() < deadline {
            core::hint::spin_loop();
        }
        cpu::mask();
        assert!(interrupt::DELIVERED.load(Ordering::Acquire) > 0);
        log!("{{\"event\":\"boot\",\"status\":\"pass\",\"el\":1,\"timer_irq\":true}}\n");
    }
    cpu::timer_stop();
    cpu::mask();
    cpu::poweroff();
}
#[unsafe(no_mangle)]
pub extern "C" fn synchronous(esr: u64, far: u64, pc: u64) -> u64 {
    #[cfg(feature = "kernel-tests")]
    {
        let ec = esr >> cpu::ESR_EC_SHIFT;
        if EXPECTED_PC
            .compare_exchange(pc, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            && pc != 0
            && (ec == cpu::ESR_EC_DATA_ABORT_CURRENT_EL
                || ec == cpu::ESR_EC_BRK
                || ec == cpu::ESR_EC_INSTRUCTION_ABORT_CURRENT_EL)
        {
            FAULT_ESR.store(esr, Ordering::Release);
            FAULT_FAR.store(far, Ordering::Release);
            return EXPECTED_RESUME.load(Ordering::Acquire);
        }
    }
    fatal_exception(esr, far, pc)
}
#[unsafe(no_mangle)]
pub extern "C" fn fatal_exception(esr: u64, far: u64, pc: u64) -> ! {
    cpu::mask();
    cpu::timer_stop();
    log!(
        "{{\"event\":\"fatal\",\"status\":\"fail\",\"esr\":{}}}\n",
        esr
    );
    #[cfg(feature = "diagnostics")]
    log!("exception: FAR={:#x} ELR={:#x}\n", far, pc);
    #[cfg(not(feature = "diagnostics"))]
    let _ = (far, pc);
    cpu::poweroff();
}
#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    cpu::mask();
    cpu::timer_stop();
    log!(
        "{{\"event\":\"panic\",\"status\":\"fail\"}}\nPANIC: {}\n",
        info
    );
    cpu::poweroff();
}
