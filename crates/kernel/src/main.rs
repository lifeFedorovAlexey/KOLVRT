#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
extern crate alloc;
mod asid;
mod arch {
    pub mod aarch64;
}
mod boot_workload;
mod diagnostics;
mod execution;
mod hal;
mod handles;
mod interrupt;
mod ipc;
#[cfg(feature = "ipc-benchmark")]
mod ipc_benchmark;
mod ipc_workload;
mod memory;
mod percpu;
mod platform;
mod process;
mod process_workload;
mod scheduler;
mod security;
mod smp;
mod supervision;
mod supervision_workload;
mod sync;
#[cfg(feature = "kernel-tests")]
mod tests;
mod time;
mod user_copy;
use arch::aarch64 as cpu;
const BOOT_MEMORY_PATTERN: u64 = 0x4b4f4c565254; // ASCII "KOLVRT".
const KIBIBYTE_BYTES: usize = 1024;
const MEBIBYTE_BYTES: u64 = (KIBIBYTE_BYTES * KIBIBYTE_BYTES) as u64;
#[cfg(not(feature = "kernel-tests"))]
const BOOT_TIMER_DELAY: time::Duration = time::Duration::from_millis(10);
#[cfg(not(feature = "kernel-tests"))]
const BOOT_IRQ_TIMEOUT: time::Duration = time::Duration::from_secs(1);
use core::sync::atomic::Ordering;
// Keep failure probes behind an ordinary call boundary: a probe returning is
// an explicit failure, while its panic must not make the rest of boot code
// syntactically unreachable in the control build.
#[cfg(feature = "ownership-test")]
fn ownership_control(d: &kernel_core::platform::Description) {
    let _foreign_owner = memory::Physical::new(d);
    panic!("second physical owner was accepted");
}
#[cfg(feature = "retained-mapping-test")]
fn retained_mapping_control(physical: &mut memory::Physical) {
    let frame = physical
        .allocate(1, 1)
        .expect("retained mapping control allocation");
    let mapping = memory::map(&frame, memory::DYNAMIC_BASE, true, false).unwrap();
    core::mem::forget(mapping);
    physical.release(frame);
    panic!("retained mapping release was accepted");
}
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    let d = platform::discover_boot();
    percpu::initialize(&d);
    diagnostics::initialize(d.uart.base as usize);
    diagnostics::boot_banner();
    log!(
        "KOLVRT | {} | AArch64 | EL{}\n",
        if cfg!(feature = "diagnostics") {
            "DEV"
        } else {
            "PROD"
        },
        cpu::el() >> cpu::CURRENT_EL_SHIFT
    );
    diagnostics::status("OK", "console", format_args!("UART ready"));
    cpu::vectors();
    let mut physical = memory::Physical::new(&d);
    diagnostics::status(
        "OK",
        "memory",
        format_args!(
            "{} MiB RAM; {} free pages ({} KiB/page)",
            d.ram.size / MEBIBYTE_BYTES,
            physical.available(),
            platform::config::PAGE_BYTES / KIBIBYTE_BYTES
        ),
    );
    diagnostics::status("OK", "exceptions", format_args!("vectors installed"));
    memory::initialize_mmu(&d);
    memory::initialize_heap(&mut physical);
    let mut processes = process::Registry::new();
    interrupt::initialize(&d);
    smp::start(memory::table_root());
    let frame = physical.allocate(1, 1).expect("boot memory validation");
    let mut mapping =
        memory::map(&frame, memory::DYNAMIC_BASE, true, false).expect("boot page mapping");
    assert_eq!(mapping.read_word(), 0);
    mapping.write_word(BOOT_MEMORY_PATTERN).unwrap();
    assert_eq!(mapping.read_word(), BOOT_MEMORY_PATTERN);
    mapping.unmap();
    physical.release(frame);
    assert!(physical.available() > 0);
    diagnostics::status(
        "OK",
        "protection",
        format_args!("MMU enabled; W^X enforced; heap initialized"),
    );
    diagnostics::status(
        "OK",
        "interrupts",
        format_args!("GICv3 and physical timer initialized"),
    );
    diagnostics::status(
        "OK",
        "SMP",
        format_args!("{} CPUs participating", platform::config::ACTIVE_CPUS),
    );
    #[cfg(feature = "ownership-test")]
    ownership_control(&d);
    #[cfg(feature = "retained-mapping-test")]
    retained_mapping_control(&mut physical);
    #[cfg(feature = "kernel-tests")]
    tests::run(&d, &mut physical, &mut processes);
    #[cfg(not(feature = "kernel-tests"))]
    {
        assert!(supervision_workload::exercise(
            &mut physical,
            &mut processes
        ));
        let users = boot_workload::exercise(&mut physical, &mut processes);
        event!(
            "{{\"event\":\"el0\",\"status\":\"pass\",\"processes\":{},\"workers\":{},\"faults\":{},\"switches\":{},\"reclaimed\":{}}}",
            users.processes,
            users.workers,
            users.faults,
            users.switches,
            users.reclaimed
        );
        diagnostics::status(
            "OK",
            "EL0",
            format_args!(
                "{} processes; {} contained faults; {}",
                users.processes,
                users.faults,
                if users.reclaimed {
                    "resources reclaimed"
                } else {
                    "resources retained"
                }
            ),
        );
        process_workload::exercise(&mut physical, &mut processes, |name, passed| {
            if !passed {
                event!("{{\"event\":\"test\",\"name\":\"{name}\",\"status\":\"fail\"}}");
                diagnostics::status("FAIL", "process", format_args!("{name}"));
            }
            assert!(passed, "process workload check failed: {name}");
        });
        ipc_workload::exercise(&mut physical, &mut processes, |name, passed| {
            if !passed {
                diagnostics::status("FAIL", "IPC", format_args!("{name}"));
            }
            assert!(passed, "IPC workload check failed: {name}");
        });
        #[cfg(feature = "ipc-benchmark")]
        ipc_benchmark::exercise(&mut physical, &mut processes);
        #[cfg(feature = "boot-payload")]
        boot_workload::payload(
            &mut physical,
            &mut processes,
            include_bytes!(env!("KOLVRT_BOOT_PAYLOAD")),
        );
        cpu::timer(time::deadline_after(BOOT_TIMER_DELAY));
        cpu::unmask();
        let deadline = time::deadline_after(BOOT_IRQ_TIMEOUT);
        while interrupt::delivered().load(Ordering::Acquire) == 0 && cpu::ticks() < deadline {
            core::hint::spin_loop();
        }
        cpu::mask();
        assert!(interrupt::delivered().load(Ordering::Acquire) > 0);
        let before = percpu::CPUS[percpu::SECONDARY_CPU]
            .ipis
            .load(Ordering::Acquire);
        smp::ping(percpu::SECONDARY_CPU);
        smp::wait(
            || {
                percpu::CPUS[percpu::SECONDARY_CPU]
                    .ipis
                    .load(Ordering::Acquire)
                    > before
            },
            "boot IPI timeout",
        );
        diagnostics::status(
            "OK",
            "verification",
            format_args!("timer interrupt and secondary IPI confirmed"),
        );
        smp::shutdown();
        diagnostics::status(
            "OK",
            "shutdown",
            format_args!("secondary CPU shutdown verified"),
        );
        event!(
            "{{\"event\":\"boot\",\"status\":\"pass\",\"el\":1,\"timer_irq\":true,\"active_cpus\":{},\"secondary_shutdown_verified\":true,\"ipc_runs\":8,\"ipc_deadline_runs\":4,\"ipc_lifetime_runs\":28,\"ipc_requester_death_runs\":12,\"ipc_authority_runs\":4,\"ipc_payload_runs\":8,\"ipc_queue_runs\":8}}",
            platform::config::ACTIVE_CPUS
        );
    }
    let dropped = diagnostics::dropped();
    if dropped != 0 {
        diagnostics::status(
            "WARN",
            "output",
            format_args!("{} records lost; evidence may be incomplete", dropped),
        );
    }
    diagnostics::status(
        "OK",
        "boot",
        format_args!("validation complete; powering off (not a running system)"),
    );
    cpu::timer_stop();
    cpu::mask();
    cpu::poweroff();
}
#[unsafe(no_mangle)]
pub extern "C" fn synchronous(esr: u64, far: u64, pc: u64) -> u64 {
    if let Some(resume) = user_copy::recover(esr, far, pc) {
        return resume;
    }
    #[cfg(feature = "kernel-tests")]
    {
        let ec = esr >> cpu::ESR_EC_SHIFT;
        let local = percpu::current();
        if local
            .expected_pc
            .compare_exchange(pc, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            && pc != 0
            && (ec == cpu::ESR_EC_DATA_ABORT_CURRENT_EL
                || ec == cpu::ESR_EC_BRK
                || ec == cpu::ESR_EC_INSTRUCTION_ABORT_CURRENT_EL)
        {
            local.fault_esr.store(esr, Ordering::Release);
            local.fault_far.store(far, Ordering::Release);
            return local.expected_resume.load(Ordering::Acquire);
        }
    }
    fatal_exception(esr, far, pc)
}
#[unsafe(no_mangle)]
pub extern "C" fn fatal_exception(esr: u64, far: u64, pc: u64) -> ! {
    if percpu::is_secondary() {
        smp::secondary_failure();
    }
    cpu::mask();
    cpu::timer_stop();
    event!(
        "{{\"event\":\"fatal\",\"status\":\"fail\",\"esr\":{}}}",
        esr
    );
    diagnostics::status(
        "FAIL",
        "exception",
        format_args!("unhandled synchronous exception"),
    );
    #[cfg(feature = "diagnostics")]
    diagnostics::status(
        "DEBUG",
        "exception",
        format_args!("ESR={:#x} FAR={:#x} ELR={:#x}", esr, far, pc),
    );
    #[cfg(not(feature = "diagnostics"))]
    let _ = (far, pc);
    cpu::poweroff();
}
#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    if percpu::is_secondary() {
        smp::secondary_failure();
    }
    cpu::mask();
    cpu::timer_stop();
    event!(
        "{{\"event\":\"panic\",\"status\":\"fail\",\"line\":{}}}",
        info.location().map_or(0, |location| location.line())
    );
    if let Some(location) = info.location() {
        diagnostics::status(
            "FAIL",
            "panic",
            format_args!("kernel halted at {}:{}", location.file(), location.line()),
        );
    } else {
        diagnostics::status(
            "FAIL",
            "panic",
            format_args!("kernel halted; location unavailable"),
        );
    }
    #[cfg(feature = "diagnostics")]
    if let Some(location) = info.location() {
        diagnostics::status(
            "DEBUG",
            "panic",
            format_args!(
                "{}:{}: {}",
                location.file(),
                location.line(),
                info.message()
            ),
        );
    } else {
        diagnostics::status("DEBUG", "panic", format_args!("{}", info.message()));
    }
    #[cfg(not(feature = "diagnostics"))]
    let _ = info;
    cpu::poweroff();
}
