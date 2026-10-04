//! Trusted static bootstrap fixtures and post-quiescence verifier. Not scheduler lifetime.
#[cfg(all(feature = "boot-payload", feature = "machine-events"))]
use crate::event;
#[cfg(all(feature = "boot-payload", feature = "machine-events"))]
use crate::scheduler;
use crate::scheduler::task::{CONTEXT_EXITED, CONTEXT_FAULTED};
use crate::{
    cpu,
    cpu::context::{
        Context, ESR_DATA_ABORT_LOWER, ESR_INSTRUCTION_ABORT_LOWER, ESR_SVC64, ESR_SYSREG_TRAP,
        ESR_UNKNOWN,
    },
    memory, percpu,
    platform::config,
    process::{ImageFormat, Origin, Registry, Spec},
};
use core::sync::atomic::Ordering;
const TASKS: usize = config::USER_PROCESSES_PER_CPU;
#[cfg(feature = "boot-payload")]
const PAYLOAD_MAX_SLICES: usize = 512;
// SAFETY: INV-USER-IMAGE: trusted position-independent EL0 fixture, copied into owned RX pages.
core::arch::global_asm!(include_str!("boot_workload.S"));
unsafe extern "C" {
    static user_image_start: u8;
    static user_image_end: u8;
}
const PAIR_WORDS: usize = 2;
#[cfg(feature = "boot-payload")]
const PAYLOAD_RUN_TIMEOUT: crate::time::Duration = crate::time::Duration::from_secs(2);
const MAX_SLICES: usize = 16;
const REQUIRED_SLICES: usize = 3;
const REQUIRED_WORKER_SLICES: usize = 5;
const MODE_WORKER: u64 = 0;
const MODE_READ: u64 = 1;
const MODE_WRITE: u64 = 2;
const MODE_EXECUTE: u64 = 3;
const MODE_PRIVILEGED: u64 = 4;
const MARKER_BASE: u64 = 0x4b4f_4c00;
const SIMD_LOW: u64 = 0x5555_5555_5555_5555;
const SIMD_HIGH: u64 = 0x6666_6666_6666_6666;
const REG_ID: usize = 21;
const REG_MARKER: usize = 20;
const REG_COUNTER: usize = 19;
const LAST_SIMD: usize = Context::ZERO.simd.len() - 1;
const DATA_TAG_WORD: usize = 2;
const FPCR_ROUND_SHIFT: u32 = 22;
const FPSR_INEXACT_SHIFT: u32 = 4;
const ROUNDING_MODE_MASK: u64 = 3;
const ENTRY_ARGUMENTS: usize = 5;

fn context_valid(frame: &Context, id: usize) -> bool {
    frame.gpr[REG_COUNTER] == 0
        || (frame.gpr[REG_MARKER] == MARKER_BASE + id as u64
            && frame.gpr[REG_ID] == id as u64
            && frame.simd[0] == [SIMD_LOW; PAIR_WORDS]
            && frame.simd[LAST_SIMD] == [SIMD_HIGH; PAIR_WORDS]
            && frame.fpcr == (id as u64 & ROUNDING_MODE_MASK) << FPCR_ROUND_SHIFT
            && frame.fpsr == (id as u64 & 1) << FPSR_INEXACT_SHIFT
            && frame.tpidr == id as u64)
}
#[derive(Clone, Copy)]
pub struct Evidence {
    pub processes: usize,
    pub switches: usize,
    pub faults: usize,
    pub workers: usize,
    pub reclaimed: bool,
    pub owners_released: bool,
    pub fault_checks: [bool; config::USER_PROCESSES],
}
pub fn exercise(p: &mut memory::Physical, processes: &mut Registry) -> Evidence {
    percpu::primary_only();
    cpu::mask();
    assert_eq!(memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire), 0);
    let before = p.available();
    let image_start = &raw const user_image_start as usize;
    let image_end = &raw const user_image_end as usize;
    assert!(image_end > image_start);
    // SAFETY: INV-USER-IMAGE: linked immutable position-independent trusted image extent.
    let image =
        unsafe { core::slice::from_raw_parts(image_start as *const u8, image_end - image_start) };
    #[cfg(feature = "user-retirement-negative")]
    {
        let frame = p.allocate(memory::USER_SPACE_PAGES, 1).unwrap();
        let space = memory::UserSpace::new(&frame, 0, image);
        core::mem::forget(space);
        p.release(frame);
        panic!("retained user frame release accepted");
    }
    let mut contexts = [Context::ZERO; config::USER_PROCESSES];
    let mut expectations = [(0, 0); config::USER_PROCESSES];
    for owner in 0..config::ACTIVE_CPUS {
        for (index, task) in contexts[owner * TASKS..(owner + 1) * TASKS]
            .iter_mut()
            .enumerate()
        {
            let id = owner * TASKS + index;
            let (mode, target, expected_class) = match (owner, index) {
                (_, 0) => (MODE_WORKER, 0, ESR_SVC64),
                (percpu::BOOT_CPU, 1) => {
                    (MODE_READ, memory::kernel_bounds().0, ESR_DATA_ABORT_LOWER)
                }
                (percpu::BOOT_CPU, 2) => (
                    MODE_READ,
                    memory::UserSpace::alias((id + 1) % config::USER_PROCESSES),
                    ESR_DATA_ABORT_LOWER,
                ),
                (percpu::BOOT_CPU, _) => (MODE_WRITE, memory::USER_CODE, ESR_DATA_ABORT_LOWER),
                (_, 1) => (MODE_READ, memory::USER_GUARD, ESR_DATA_ABORT_LOWER),
                (_, 2) => (MODE_EXECUTE, memory::USER_DATA, ESR_INSTRUCTION_ABORT_LOWER),
                (_, _) => (MODE_PRIVILEGED, 0, ESR_UNKNOWN),
            };
            let mut context = Context::ZERO;
            context.pc = memory::USER_CODE as u64;
            context.sp = memory::USER_STACK_TOP as u64;
            context.tpidr = id as u64;
            context.gpr[..ENTRY_ARGUMENTS].copy_from_slice(&[
                id as u64,
                memory::USER_DATA as u64,
                target as u64,
                mode,
                MARKER_BASE + id as u64,
            ]);
            *task = context;
            expectations[id] = (expected_class, target);
        }
    }
    assert_eq!(processes.live(), 0, "fixture requires empty registry");
    let identities: [_; config::USER_PROCESSES] = core::array::from_fn(|id| {
        let identity = processes
            .create(
                p,
                Origin::Bootstrap,
                Spec {
                    image,
                    image_format: ImageFormat::RawFixture,
                    context: contexts[id],
                    owner: id / TASKS,
                    entry: memory::USER_CODE,
                    slice_limit: Some(MAX_SLICES),
                },
                None,
            )
            .unwrap_or_else(|failure| crate::process::reject(failure.error));
        assert_eq!(identity.slot(), id);
        processes.start(identity).unwrap();
        identity
    });
    // The fixture waits for measured timer slices before exit/fault. Its
    // per-process slice limit bounds work; a wall-clock deadline can expire on
    // a descheduled QEMU host before those observations exist. The host runner
    // still rejects a stalled emulator after its independent timeout.
    let completed = processes.dispatch(None);
    let mut evidence = Evidence {
        processes: config::USER_PROCESSES,
        switches: 0,
        faults: 0,
        workers: 0,
        reclaimed: false,
        owners_released: completed.owners_released,
        fault_checks: [false; config::USER_PROCESSES],
    };
    evidence.switches = completed.switches;
    for task in &completed.tasks {
        let (expected_class, expected_far) = expectations[task.id];
        assert!(
            context_valid(&task.context, task.id),
            "user register context lost"
        );
        // SAFETY: INV-USER-RETIRE: all CPUs acquired done, no user/kernel page writer;
        // retained frame's initialized tag is read before any space/charge is retired.
        let tag = unsafe {
            core::ptr::read_volatile(
                (processes.data_address(identities[task.id]).unwrap() as *const u64)
                    .add(DATA_TAG_WORD),
            )
        };
        assert_eq!(
            tag,
            MARKER_BASE + task.id as u64,
            "user address-space alias leaked"
        );
        assert!(
            task.slices >= REQUIRED_SLICES,
            "user task not timer serviced: id={} state={} slices={} fault={:#x}",
            task.id,
            task.state,
            task.slices,
            task.fault_class
        );
        assert!(
            task.context.gpr[REG_COUNTER] > 0,
            "user task made no progress"
        );
        if expected_class == ESR_SVC64 {
            assert_eq!(task.state, CONTEXT_EXITED, "EL0 worker failed");
            assert!(task.slices >= REQUIRED_WORKER_SLICES);
            assert_eq!(
                task.peer_faults_at_exit,
                TASKS - 1,
                "worker exited before peers faulted"
            );
            assert_eq!(task.context.gpr[0], 1, "user stack context lost");
            evidence.workers += 1;
        } else {
            assert_eq!(task.state, CONTEXT_FAULTED, "EL0 expected fault missing");
            assert!(
                task.fault_class == expected_class
                    || (expected_class == ESR_UNKNOWN && task.fault_class == ESR_SYSREG_TRAP),
                "wrong user fault class"
            );
            evidence.faults += 1;
            evidence.fault_checks[task.id] = true;
            if expected_far != 0 {
                assert_eq!(
                    task.fault_far, expected_far,
                    "unexpected user fault address"
                );
            }
        }
    }
    for identity in identities {
        processes.reclaim(p, identity).unwrap();
    }
    evidence.reclaimed = p.available() == before;
    assert!(evidence.reclaimed, "EL0 frame leak");
    assert!(evidence.owners_released, "EL0 executing owner retained");
    evidence
}

/// Run a trusted boot-selected opaque EL0 image. No adapter, route or profile types
/// cross this boundary. Immutable code is retained until both native roots are restored.
#[cfg(feature = "boot-payload")]
pub fn payload(p: &mut memory::Physical, processes: &mut Registry, image: &[u8]) {
    percpu::primary_only();
    cpu::mask();
    assert_eq!(memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire), 0);
    let before = p.available();
    let contexts: [Context; config::USER_PROCESSES] = core::array::from_fn(|id| {
        let mut context = Context::ZERO;
        context.pc = config::USER_PAYLOAD_BASE as u64;
        context.sp = memory::USER_STACK_TOP as u64;
        context.tpidr = id as u64;
        context.gpr[0] = id as u64;
        context
    });
    assert_eq!(
        processes.live(),
        0,
        "payload fixture requires empty registry"
    );
    let identities: [_; config::USER_PROCESSES] = core::array::from_fn(|id| {
        let identity = processes
            .create(
                p,
                Origin::Bootstrap,
                Spec {
                    image,
                    image_format: ImageFormat::RawFixture,
                    context: contexts[id],
                    owner: id / TASKS,
                    entry: config::USER_PAYLOAD_BASE,
                    slice_limit: Some(PAYLOAD_MAX_SLICES),
                },
                None,
            )
            .unwrap_or_else(|failure| crate::process::reject(failure.error));
        assert_eq!(identity.slot(), id);
        processes.start(identity).unwrap();
        identity
    });
    let completed = processes.dispatch(Some(PAYLOAD_RUN_TIMEOUT));
    #[cfg(feature = "machine-events")]
    for task in &completed.tasks {
        for offset in (0..task.report_len).step_by(scheduler::REPORT_CHUNK_WORDS) {
            let (words, length) = completed.report_chunk(task.id, offset);
            event!(
                "{{\"event\":\"user-report\",\"id\":{},\"offset\":{},\"words\":{:?}}}",
                task.id,
                offset,
                &words[..length]
            );
        }
        let process = identities
            .get(task.id)
            .expect("completed task has an admitted process identity");
        assert_eq!(process.slot(), task.id);
        assert_eq!(process.generation(), task.process_generation);
        let resident_pages = processes
            .resident_pages(*process)
            .expect("completed process retains its address-space charge");
        event!(
            "{{\"event\":\"user-result\",\"id\":{},\"state\":{},\"exit\":{},\"fault\":{},\"slices\":{},\"length\":{},\"native_attempts\":{},\"process_slot\":{},\"process_generation\":{},\"owner_cpu\":{},\"resident_pages\":{},\"el0_residency_ticks\":{},\"native_window_service_ticks\":{},\"counter_frequency_hz\":{}}}",
            task.id,
            task.state,
            task.context.gpr[0],
            task.fault_class,
            task.slices,
            task.report_len,
            task.native_attempts,
            process.slot(),
            process.generation(),
            task.id / scheduler::TASKS,
            resident_pages,
            task.el0_residency_ticks,
            task.native_window_service_ticks,
            cpu::frequency()
        );
    }
    for identity in identities {
        processes.reclaim(p, identity).unwrap();
    }
    assert_eq!(p.available(), before, "payload frame leak");
    assert!(completed.owners_released);
}
