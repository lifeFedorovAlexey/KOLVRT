//! Bounded fixed-affinity EL0 foundation. Each CPU owns its ready queue and trap state.
//! No queue locks, IRQ allocation, migration, IPC, capabilities or service model.
#[cfg(feature = "boot-payload")]
use crate::event;
use crate::{cpu, memory, percpu, platform::config, time};
use alloc::vec::Vec;
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use kernel_core::execution as abi;

const TASKS: usize = config::USER_PROCESSES_PER_CPU;
const GPRS: usize = 32;
const SIMD_REGISTERS: usize = 32;
const PAIR_WORDS: usize = 2;
const SAVED_REGISTER_BYTES: usize =
    (GPRS + SIMD_REGISTERS * PAIR_WORDS + PAIR_WORDS) * core::mem::size_of::<u64>();
const USER_META_WORDS: usize = 4;
const QUANTUM: time::Duration = time::Duration::from_millis(1);
const RUN_TIMEOUT: time::Duration = time::Duration::from_secs(2);
const MAX_SLICES: usize = 16;
const REQUIRED_SLICES: usize = 3;
const REQUIRED_WORKER_SLICES: usize = 5;
const NO_TASK: usize = TASKS;
const TRAP_IRQ: u64 = 1;
const ESR_CLASS_SHIFT: u32 = 26;
const ESR_SVC64: u64 = 0x15;
const ESR_DATA_ABORT_LOWER: u64 = 0x24;
const ESR_INSTRUCTION_ABORT_LOWER: u64 = 0x20;
const ESR_SYSREG_TRAP: u64 = 0x18;
const ESR_UNKNOWN: u64 = 0;
const SVC_IMMEDIATE_MASK: u64 = u16::MAX as u64;
const FINISHED_TRAP: u64 = 0x4e;
const USER_EL0T: u64 = 0;
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
const LAST_SIMD: usize = SIMD_REGISTERS - 1;
const CONTEXT_READY: usize = 0;
const CONTEXT_RUNNING: usize = 1;
const CONTEXT_EXITED: usize = 2;
const CONTEXT_FAULTED: usize = 3;
const CONTEXT_TIMED_OUT: usize = 4;
const DATA_TICKS_WORD: usize = 1;
const DATA_TAG_WORD: usize = 2;
const FPCR_ROUND_SHIFT: u32 = 22;
const FPSR_INEXACT_SHIFT: u32 = 4;
const ROUNDING_MODE_MASK: u64 = 3;
const ENTRY_ARGUMENTS: usize = 5;
const PSTATE_MODE_MASK: u64 = 0xf;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub(crate) struct Context {
    gpr: [u64; GPRS],
    simd: [[u64; PAIR_WORDS]; SIMD_REGISTERS],
    fpcr: u64,
    fpsr: u64,
    pc: u64,
    pstate: u64,
    sp: u64,
    tpidr: u64,
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
    const ZERO: Self = Self {
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
#[derive(Clone, Copy)]
struct Task {
    context: Context,
    root: u64,
    data: usize,
    state: usize,
    slices: usize,
    fault_class: u64,
    expected_class: u64,
    expected_far: usize,
    id: usize,
    fault_far: usize,
    context_ok: bool,
    peer_faults_at_exit: usize,
    fixture: bool,
    slice_budget: usize,
    observations: crate::execution::Observations,
    #[cfg(feature = "machine-events")]
    report: [u64; abi::REPORT_WORDS],
    #[cfg(feature = "machine-events")]
    report_len: usize,
}
impl Task {
    const ZERO: Self = Self {
        context: Context::ZERO,
        root: 0,
        data: 0,
        state: CONTEXT_READY,
        slices: 0,
        fault_class: 0,
        expected_class: 0,
        expected_far: 0,
        id: 0,
        fault_far: 0,
        context_ok: true,
        peer_faults_at_exit: 0,
        fixture: false,
        slice_budget: MAX_SLICES,
        observations: crate::execution::Observations::ZERO,
        #[cfg(feature = "machine-events")]
        report: [0; abi::REPORT_WORDS],
        #[cfg(feature = "machine-events")]
        report_len: 0,
    };
}
struct State {
    tasks: [Task; TASKS],
    current: usize,
    switches: usize,
    deadline: u64,
}
impl State {
    const ZERO: Self = Self {
        tasks: [Task::ZERO; TASKS],
        current: NO_TASK,
        switches: 0,
        deadline: 0,
    };
}
struct Local {
    state: UnsafeCell<State>,
    phase: AtomicUsize,
    preempt: AtomicBool,
    resume_sp: AtomicUsize,
}
// SAFETY: INV-RUNQUEUE: CPU0 writes before release launch; afterwards only the indexed
// CPU accesses State, with IRQ masked and no references surviving run_user. CPU0 reads
// only after acquire done; reset only after every CPU returns to native root. No migration.
unsafe impl Sync for Local {}
impl Local {
    const fn new() -> Self {
        Self {
            state: UnsafeCell::new(State::ZERO),
            phase: AtomicUsize::new(PHASE_IDLE),
            preempt: AtomicBool::new(false),
            resume_sp: AtomicUsize::new(0),
        }
    }
}
static LOCALS: [Local; config::ACTIVE_CPUS] = [const { Local::new() }; config::ACTIVE_CPUS];
static NATIVE_ROOT: AtomicU64 = AtomicU64::new(0);
static START: AtomicBool = AtomicBool::new(false);
static ARRIVED: AtomicUsize = AtomicUsize::new(0);
static SESSION: AtomicBool = AtomicBool::new(false);
const PHASE_IDLE: usize = 0;
const PHASE_ADMITTED: usize = 1;
const PHASE_RUNNING: usize = 2;
const PHASE_DONE: usize = 3;
const NO_CPU: usize = usize::MAX;
static RUNNING_OWNER: [AtomicUsize; config::USER_PROCESSES] =
    [const { AtomicUsize::new(NO_CPU) }; config::USER_PROCESSES];
fn acquire_process(id: usize) {
    assert_eq!(id / TASKS, percpu::id(), "wrong process affinity");
    assert_eq!(
        RUNNING_OWNER[id].compare_exchange(
            NO_CPU,
            percpu::id(),
            Ordering::AcqRel,
            Ordering::Acquire
        ),
        Ok(NO_CPU),
        "process already running"
    );
}
fn release_process(id: usize) {
    assert_eq!(
        RUNNING_OWNER[id].swap(NO_CPU, Ordering::AcqRel),
        percpu::id()
    );
}
/// # Safety
/// Same retained live-root contract as activate_root; used only for pinned user roots.
unsafe fn activate(root: u64) {
    #[cfg(feature = "user-root-negative")]
    let root = {
        let _ = root;
        NATIVE_ROOT.load(Ordering::Acquire)
    };
    // SAFETY: INV-USER-TTBR: caller retains the root and mask/stack preconditions.
    unsafe {
        cpu::activate_root(root);
    }
}
unsafe extern "C" {
    fn run_user(context: *const Context, resume_sp: *mut usize);
    static user_image_start: u8;
    static user_image_end: u8;
}

pub fn on_timer() {
    let local = &LOCALS[percpu::id()];
    if local.phase.load(Ordering::Acquire) == PHASE_RUNNING {
        local.preempt.store(true, Ordering::Release);
    }
}
pub fn poll_secondary() {
    if LOCALS[percpu::SECONDARY_CPU].phase.load(Ordering::Acquire) == PHASE_ADMITTED {
        run_local();
        cpu::unmask();
    }
}
fn choose(state: &mut State) -> Option<usize> {
    let ready: [bool; TASKS] =
        core::array::from_fn(|index| state.tasks[index].state == CONTEXT_READY);
    let current = (state.current != NO_TASK).then_some(state.current);
    if let Some(index) = kernel_core::scheduling::next_ready(current, &ready) {
        assert!(
            state.tasks.iter().all(|t| t.state != CONTEXT_RUNNING),
            "duplicate running task"
        );
        state.tasks[index].state = CONTEXT_RUNNING;
        state.current = index;
        state.switches += 1;
        return Some(index);
    }
    None
}
fn run_local() {
    cpu::mask();
    let local = &LOCALS[percpu::id()];
    if local
        .phase
        .compare_exchange(
            PHASE_ADMITTED,
            PHASE_RUNNING,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
    {
        return;
    }
    // SAFETY: INV-RUNQUEUE: this CPU exclusively owns state after launch; IRQ masked.
    let first = unsafe {
        let state = &mut *local.state.get();
        let index = choose(state).expect("empty ready queue");
        state.tasks[index]
    };
    acquire_process(first.id);
    // Both CPUs rendezvous before first EL0 entry, so the test covers concurrent queues.
    ARRIVED.fetch_or(1 << percpu::id(), Ordering::AcqRel);
    crate::smp::wait(
        || ARRIVED.load(Ordering::Acquire) == (1 << config::ACTIVE_CPUS) - 1,
        "EL0 CPU rendezvous timeout",
    );
    if percpu::id() == percpu::BOOT_CPU {
        START.store(true, Ordering::Release);
    } else {
        crate::smp::wait(|| START.load(Ordering::Acquire), "EL0 start timeout");
    }
    // SAFETY: INV-USER-TTBR: driver retains all spaces; root preserves kernel stack/code;
    // local ownership, masked IRQ, no Rust State reference spans exception execution.
    unsafe {
        activate(first.root);
    }
    cpu::timer(time::deadline_after(QUANTUM));
    // SAFETY: INV-USER-CONTEXT: exact statically checked frame ABI, live native stack;
    // assembly saves its resume SP with STLR, and restores full kernel ABI before return.
    unsafe {
        run_user(&raw const first.context, local.resume_sp.as_ptr());
    }
    assert_eq!(cpu::el(), cpu::CURRENT_EL1);
    memory::USER_EXECUTION_ACTIVE.fetch_sub(1, Ordering::AcqRel);
    local.phase.store(PHASE_DONE, Ordering::Release);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn user_trap(frame: *mut Context, kind: u64) -> usize {
    let local = &LOCALS[percpu::id()];
    assert_eq!(local.phase.load(Ordering::Acquire), PHASE_RUNNING);
    // SAFETY: INV-USER-CONTEXT and INV-RUNQUEUE: assembly supplies complete aligned frame
    // on this CPU's EL1 stack; exceptions mask IRQ, no mutable reference survives ERET.
    let (frame, state) = unsafe { (&mut *frame, &mut *local.state.get()) };
    assert!(state.current < TASKS);
    let current = state.current;
    let peer_faults = state
        .tasks
        .iter()
        .filter(|t| t.state == CONTEXT_FAULTED)
        .count();
    let task = &mut state.tasks[current];
    assert_eq!(task.state, CONTEXT_RUNNING);
    assert_eq!(frame.pstate & PSTATE_MODE_MASK, USER_EL0T);
    // An interrupt can arrive before the image has initialized markers. Check them only
    // once it has published progress, without trusting that progress as authority.
    if task.fixture && frame.gpr[REG_COUNTER] != 0 {
        task.context_ok &= frame.gpr[REG_MARKER] == MARKER_BASE + task.id as u64
            && frame.gpr[REG_ID] == task.id as u64
            && frame.simd[0] == [SIMD_LOW; PAIR_WORDS]
            && frame.simd[LAST_SIMD] == [SIMD_HIGH; PAIR_WORDS]
            && frame.fpcr == (task.id as u64 & ROUNDING_MODE_MASK) << FPCR_ROUND_SHIFT
            && frame.fpsr == (task.id as u64 & 1) << FPSR_INEXACT_SHIFT
            && frame.tpidr == task.id as u64;
    }
    task.context = *frame;
    if kind == TRAP_IRQ {
        if !local.preempt.swap(false, Ordering::AcqRel) {
            return 0;
        }
        task.slices += 1;
        task.observations.service_timer(task.slices);
        // SAFETY: INV-USER-SPACE: this CPU alone accesses its task's retained data page;
        // user execution is suspended, other tasks/CPUs cannot map this page at EL0.
        if task.fixture {
            unsafe {
                core::ptr::write_volatile(
                    (task.data as *mut u64).add(DATA_TICKS_WORD),
                    task.slices as u64,
                );
            }
        }
        task.state = if task.slices >= task.slice_budget {
            CONTEXT_TIMED_OUT
        } else {
            CONTEXT_READY
        };
        #[cfg(feature = "user-context-negative")]
        {
            task.context.gpr[REG_MARKER] ^= 1;
        }
    } else {
        let esr = cpu::user_esr();
        let class = esr >> ESR_CLASS_SHIFT;
        if class == ESR_SVC64 && native_call(task, frame, (esr & SVC_IMMEDIATE_MASK) as u16) {
            task.context = *frame;
            // Bounded synchronous native request: no locks, allocation or retained user
            // pointers; current task identity came from the owned runqueue, not registers.
            return 0;
        }
        if class == ESR_SVC64 && esr & SVC_IMMEDIATE_MASK == FINISHED_TRAP {
            task.state = CONTEXT_EXITED;
            task.peer_faults_at_exit = peer_faults;
        } else {
            task.state = CONTEXT_FAULTED;
            task.fault_class = class;
            task.fault_far = cpu::user_far() as usize;
        }
    }
    if cpu::ticks() >= state.deadline {
        for task in &mut state.tasks {
            if task.state == CONTEXT_READY || task.state == CONTEXT_RUNNING {
                task.state = CONTEXT_TIMED_OUT;
            }
        }
    }
    release_process(state.tasks[current].id);
    if let Some(next) = choose(state) {
        acquire_process(state.tasks[next].id);
        *frame = state.tasks[next].context;
        // SAFETY: INV-USER-TTBR: next pinned live root; completed flush precedes ERET.
        unsafe {
            activate(state.tasks[next].root);
        }
        cpu::timer(time::deadline_after(QUANTUM));
        0
    } else {
        cpu::timer_stop();
        // SAFETY: INV-USER-RETIRE: no ready/running process remains on this CPU;
        // native root and local TLBI precede release completion and frame reclamation.
        unsafe {
            cpu::activate_root(NATIVE_ROOT.load(Ordering::Acquire));
        }
        local.resume_sp.load(Ordering::Acquire)
    }
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
pub fn exercise(p: &mut memory::Physical) -> Evidence {
    percpu::primary_only();
    cpu::mask();
    assert_eq!(memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire), 0);
    assert_eq!(
        SESSION.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire),
        Ok(false),
        "overlapping user workload sessions"
    );
    let before = p.available();
    let mut frames: Vec<_> = (0..config::USER_PROCESSES)
        .map(|_| {
            p.allocate(memory::USER_SPACE_PAGES, 1)
                .expect("user allocation exhausted")
        })
        .collect();
    let image_start = &raw const user_image_start as usize;
    let image_end = &raw const user_image_end as usize;
    assert!(image_end > image_start);
    // SAFETY: INV-USER-IMAGE: linked immutable position-independent trusted image extent.
    let image =
        unsafe { core::slice::from_raw_parts(image_start as *const u8, image_end - image_start) };
    let spaces: Vec<_> = frames
        .iter()
        .enumerate()
        .map(|(id, frame)| memory::UserSpace::new(frame, id, image))
        .collect();
    #[cfg(feature = "user-retirement-negative")]
    if SESSION.load(Ordering::Acquire) {
        core::mem::forget(spaces);
        p.release(frames.pop().unwrap());
        panic!("retained user frame release accepted");
    }
    START.store(false, Ordering::Release);
    ARRIVED.store(0, Ordering::Release);
    NATIVE_ROOT.store(memory::table_root(), Ordering::Release);
    for (owner, local) in LOCALS.iter().enumerate() {
        assert!(matches!(
            local.phase.load(Ordering::Acquire),
            PHASE_IDLE | PHASE_DONE
        ));
        local.phase.store(PHASE_IDLE, Ordering::Release);
        // SAFETY: INV-RUNQUEUE: no launch yet; prior execution (if any) acquired complete.
        let state = unsafe { &mut *local.state.get() };
        *state = State::ZERO;
        state.deadline = time::deadline_after(RUN_TIMEOUT);
        for (index, task) in state.tasks.iter_mut().enumerate() {
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
            *task = Task {
                context,
                root: spaces[id].root(),
                data: spaces[id].data_address(),
                state: CONTEXT_READY,
                slices: 0,
                fault_class: 0,
                expected_class,
                expected_far: target,
                id,
                fault_far: 0,
                context_ok: true,
                peer_faults_at_exit: 0,
                fixture: true,
                ..Task::ZERO
            };
        }
        local.preempt.store(false, Ordering::Release);
    }
    memory::USER_EXECUTION_ACTIVE.store(config::ACTIVE_CPUS, Ordering::Release);
    for local in &LOCALS {
        local.phase.store(PHASE_ADMITTED, Ordering::Release);
    }
    crate::smp::ping(percpu::SECONDARY_CPU);
    run_local();
    crate::smp::wait(
        || LOCALS[percpu::SECONDARY_CPU].phase.load(Ordering::Acquire) == PHASE_DONE,
        "EL0 secondary completion timeout",
    );
    assert_eq!(memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire), 0);
    let mut evidence = Evidence {
        processes: config::USER_PROCESSES,
        switches: 0,
        faults: 0,
        workers: 0,
        reclaimed: false,
        owners_released: RUNNING_OWNER
            .iter()
            .all(|owner| owner.load(Ordering::Acquire) == NO_CPU),
        fault_checks: [false; config::USER_PROCESSES],
    };
    for local in &LOCALS {
        assert_eq!(local.phase.load(Ordering::Acquire), PHASE_DONE);
        // SAFETY: INV-RUNQUEUE: acquired completion, no admission or subsequent writer.
        let state = unsafe { &*local.state.get() };
        evidence.switches += state.switches;
        for task in &state.tasks {
            assert!(task.context_ok, "user register context lost");
            // SAFETY: INV-USER-RETIRE: all CPUs acquired done, no user/kernel page writer;
            // retained frame's initialized tag is read before any space/charge is retired.
            let tag =
                unsafe { core::ptr::read_volatile((task.data as *const u64).add(DATA_TAG_WORD)) };
            assert_eq!(
                tag,
                MARKER_BASE + task.id as u64,
                "user address-space alias leaked"
            );
            assert!(
                task.slices >= REQUIRED_SLICES,
                "user task not timer serviced"
            );
            assert!(
                task.context.gpr[REG_COUNTER] > 0,
                "user task made no progress"
            );
            if task.expected_class == ESR_SVC64 {
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
                    task.fault_class == task.expected_class
                        || (task.expected_class == ESR_UNKNOWN
                            && task.fault_class == ESR_SYSREG_TRAP),
                    "wrong user fault class"
                );
                evidence.faults += 1;
                evidence.fault_checks[task.id] = true;
                if task.expected_far != 0 {
                    assert_eq!(
                        task.fault_far, task.expected_far,
                        "unexpected user fault address"
                    );
                }
            }
        }
    }
    drop(spaces);
    for frame in frames.drain(..) {
        p.release(frame);
    }
    evidence.reclaimed = p.available() == before;
    assert!(evidence.reclaimed, "EL0 frame leak");
    assert!(evidence.owners_released, "EL0 executing owner retained");
    SESSION.store(false, Ordering::Release);
    evidence
}

#[cfg(feature = "boot-payload")]
const PAYLOAD_MAX_SLICES: usize = 512;
fn native_call(task: &mut Task, frame: &mut Context, operation: u16) -> bool {
    if task.observations.call(operation, &mut frame.gpr) {
        return true;
    }
    match operation {
        abi::SLICES => frame.gpr[0] = task.slices as u64,
        abi::CLOCK => {
            frame.gpr[0] = cpu::ticks();
            frame.gpr[1] = cpu::frequency();
        }
        #[cfg(feature = "machine-events")]
        abi::REPORT => {
            if task.report_len == task.report.len() {
                frame.gpr[0] = abi::FULL;
            } else {
                task.report[task.report_len] = frame.gpr[0];
                task.report_len += 1;
                frame.gpr[0] = abi::OK;
            }
        }
        _ => return false,
    }
    true
}

/// Run a trusted boot-selected opaque EL0 image. No adapter, route or profile types
/// cross this boundary. Immutable code is retained until both native roots are restored.
#[cfg(feature = "boot-payload")]
pub fn payload(p: &mut memory::Physical, image: &[u8]) {
    percpu::primary_only();
    cpu::mask();
    assert_eq!(memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire), 0);
    assert_eq!(
        SESSION.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire),
        Ok(false)
    );
    let before = p.available();
    let pages = memory::USER_SPACE_PAGES + image.len().div_ceil(config::PAGE_BYTES);
    let frames: Vec<_> = (0..config::USER_PROCESSES)
        .map(|_| p.allocate(pages, 1).expect("payload memory budget"))
        .collect();
    let spaces: Vec<_> = frames
        .iter()
        .enumerate()
        .map(|(id, frame)| memory::UserSpace::payload(frame, id, image))
        .collect();
    START.store(false, Ordering::Release);
    ARRIVED.store(0, Ordering::Release);
    NATIVE_ROOT.store(memory::table_root(), Ordering::Release);
    for (owner, local) in LOCALS.iter().enumerate() {
        assert!(matches!(
            local.phase.load(Ordering::Acquire),
            PHASE_IDLE | PHASE_DONE
        ));
        local.phase.store(PHASE_IDLE, Ordering::Release);
        // SAFETY: INV-RUNQUEUE: acquired terminal phase, no admitted reader/writer; CAS
        // admission prevents a stale secondary poll from entering an idle reset batch.
        let state = unsafe { &mut *local.state.get() };
        *state = State::ZERO;
        state.deadline = time::deadline_after(RUN_TIMEOUT);
        for (index, task) in state.tasks.iter_mut().enumerate() {
            let id = owner * TASKS + index;
            let mut context = Context::ZERO;
            context.pc = config::USER_PAYLOAD_BASE as u64;
            context.sp = memory::USER_STACK_TOP as u64;
            context.tpidr = id as u64;
            context.gpr[0] = id as u64;
            *task = Task {
                context,
                root: spaces[id].root(),
                data: spaces[id].data_address(),
                id,
                slice_budget: PAYLOAD_MAX_SLICES,
                ..Task::ZERO
            };
        }
        local.preempt.store(false, Ordering::Release);
    }
    memory::USER_EXECUTION_ACTIVE.store(config::ACTIVE_CPUS, Ordering::Release);
    for local in &LOCALS {
        local.phase.store(PHASE_ADMITTED, Ordering::Release);
    }
    crate::smp::ping(percpu::SECONDARY_CPU);
    run_local();
    crate::smp::wait(
        || LOCALS[percpu::SECONDARY_CPU].phase.load(Ordering::Acquire) == PHASE_DONE,
        "payload secondary completion timeout",
    );
    #[cfg(feature = "machine-events")]
    for local in &LOCALS {
        assert_eq!(local.phase.load(Ordering::Acquire), PHASE_DONE);
        // SAFETY: INV-RUNQUEUE: acquired quiescence, immutable reports, no next admission.
        let state = unsafe { &*local.state.get() };
        for task in &state.tasks {
            const REPORT_CHUNK_WORDS: usize = 64;
            for (chunk, words) in task.report[..task.report_len]
                .chunks(REPORT_CHUNK_WORDS)
                .enumerate()
            {
                event!(
                    "{{\"event\":\"user-report\",\"id\":{},\"offset\":{},\"words\":{:?}}}",
                    task.id,
                    chunk * REPORT_CHUNK_WORDS,
                    words
                );
            }
            event!(
                "{{\"event\":\"user-result\",\"id\":{},\"state\":{},\"exit\":{},\"fault\":{},\"slices\":{},\"length\":{},\"native_attempts\":{}}}",
                task.id,
                task.state,
                task.context.gpr[0],
                task.fault_class,
                task.slices,
                task.report_len,
                task.observations.attempts()
            );
        }
    }
    drop(spaces);
    for frame in frames {
        p.release(frame);
    }
    assert_eq!(p.available(), before, "payload frame leak");
    assert!(
        RUNNING_OWNER
            .iter()
            .all(|owner| owner.load(Ordering::Acquire) == NO_CPU)
    );
    SESSION.store(false, Ordering::Release);
}
