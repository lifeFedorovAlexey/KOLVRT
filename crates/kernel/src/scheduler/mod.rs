//! Persistent fixed-affinity runtime. Boot policy and expected evidence live elsewhere.
mod config;
mod local;
pub(crate) mod task;
use crate::{
    cpu,
    cpu::context::{Context, ESR_SVC64, PSTATE_MODE_MASK, Trap, USER_EL0T},
    memory, percpu,
    platform::config as platform_config,
    time,
};
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use kernel_core::{execution as abi, scheduling::ownership::Phase};
use local::Local;
use task::*;
pub(crate) const TASKS: usize = config::TASKS_PER_CPU;
pub(crate) const CAPACITY: usize = config::TOTAL_TASKS;
const NO_TASK: usize = TASKS;
const NO_CPU: usize = usize::MAX;
const QUANTUM: time::Duration = time::Duration::from_millis(1);
pub(crate) struct Completed {
    pub switches: usize,
    pub tasks: [TaskResult; config::TOTAL_TASKS],
    pub owners_released: bool,
    #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
    generation: u64,
}
#[cfg(all(feature = "machine-events", feature = "boot-payload"))]
pub(crate) const REPORT_CHUNK_WORDS: usize = 64;
#[cfg(all(feature = "machine-events", feature = "boot-payload"))]
impl Completed {
    pub fn report_chunk(&self, id: usize, offset: usize) -> ([u64; REPORT_CHUNK_WORDS], usize) {
        assert!(id < config::TOTAL_TASKS);
        LOCALS[id / TASKS].inspect(self.generation, |state| {
            let task = &state.tasks[id % TASKS];
            assert_eq!(task.id(), id);
            assert_eq!(task.generation(), self.generation, "stale report task");
            let mut words = [0; REPORT_CHUNK_WORDS];
            assert!(offset <= task.report_len);
            let length = (task.report_len - offset).min(REPORT_CHUNK_WORDS);
            words[..length].copy_from_slice(&task.report[offset..offset + length]);
            (words, length)
        })
    }
}
// Indexed construction fixes immutable owner identity; CPU count is currently pinned at two.
static LOCALS: [Local; platform_config::ACTIVE_CPUS] = [
    Local::new(percpu::BOOT_CPU),
    Local::new(percpu::SECONDARY_CPU),
];
static NATIVE_ROOT: AtomicU64 = AtomicU64::new(0);
static START: AtomicBool = AtomicBool::new(false);
static ARRIVED: AtomicUsize = AtomicUsize::new(0);
static SESSION: AtomicBool = AtomicBool::new(false);
static STEP: AtomicBool = AtomicBool::new(false);
static CURSOR: [AtomicUsize; platform_config::ACTIVE_CPUS] =
    [const { AtomicUsize::new(NO_TASK) }; platform_config::ACTIVE_CPUS];
static RUNNING_OWNER: [AtomicUsize; config::TOTAL_TASKS] =
    [const { AtomicUsize::new(NO_CPU) }; config::TOTAL_TASKS];
/// Persistent native dispatch mechanism. Per-process identity is independent of
/// queue generation; lifetime and optional bounded limits belong to the caller.
pub(crate) fn dispatch(
    tasks: &[Option<Admission<'_>>; config::TOTAL_TASKS],
    timeout: Option<time::Duration>,
) -> Completed {
    dispatch_inner(tasks, timeout, false)
}
/// Return after at most one timer quantum/terminal event per CPU, without
/// requiring processes to terminate. This still retains the two-CPU barrier.
pub(crate) fn step(tasks: &[Option<Admission<'_>>; config::TOTAL_TASKS]) -> Completed {
    dispatch_inner(tasks, None, true)
}
fn dispatch_inner(
    tasks: &[Option<Admission<'_>>; config::TOTAL_TASKS],
    timeout: Option<time::Duration>,
    step: bool,
) -> Completed {
    percpu::primary_only();
    assert!(cpu::irq_masked(), "scheduler coordinator IRQ contract");
    assert_eq!(memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire), 0);
    assert_eq!(
        SESSION.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire),
        Ok(false),
        "overlapping scheduler admission"
    );
    let generation = LOCALS[percpu::BOOT_CPU]
        .generation()
        .checked_add(1)
        .expect("scheduler generation exhausted");
    START.store(false, Ordering::Release);
    STEP.store(step, Ordering::Release);
    ARRIVED.store(0, Ordering::Release);
    NATIVE_ROOT.store(memory::table_root(), Ordering::Release);
    for (owner, local) in LOCALS.iter().enumerate() {
        let mut state = State::ZERO;
        if step {
            state.current = CURSOR[owner].load(Ordering::Acquire);
        }
        state.deadline = timeout.map(time::deadline_after);
        for (index, task) in state.tasks.iter_mut().enumerate() {
            let id = owner * TASKS + index;
            if let Some(admitted) = tasks[id] {
                assert_eq!(admitted.identity.slot(), id);
                assert_eq!(admitted.space.slot(), id);
                *task = Task::new(
                    id,
                    admitted.identity.generation(),
                    admitted.space.root(),
                    admitted.context,
                    admitted.slice_budget,
                );
                task.bind_queue(generation);
                task.slices = admitted.slices;
                task.observations = admitted.observations;
                #[cfg(feature = "machine-events")]
                if local.completed() {
                    local.inspect(local.generation(), |previous| {
                        let old = &previous.tasks[index];
                        if old.process_generation() == admitted.identity.generation()
                            && old.id() == id
                        {
                            task.report.copy_from_slice(&old.report);
                            task.report_len = old.report_len;
                        }
                    });
                }
            }
        }
        local.prepare(generation, state);
        local.preempt.store(false, Ordering::Release);
    }
    memory::USER_EXECUTION_ACTIVE.store(platform_config::ACTIVE_CPUS, Ordering::Release);
    for local in &LOCALS {
        local.publish(generation);
    }
    crate::smp::ping(percpu::SECONDARY_CPU);
    run_local();
    crate::smp::wait_optional(
        || LOCALS.iter().all(Local::completed),
        timeout,
        "scheduler completion timeout",
    );
    assert_eq!(memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire), 0);
    let owners_released = RUNNING_OWNER
        .iter()
        .all(|owner| owner.load(Ordering::Acquire) == NO_CPU);
    assert!(owners_released, "scheduler executing owner retained");
    let mut completed = Completed {
        switches: 0,
        tasks: [TaskResult::ZERO; config::TOTAL_TASKS],
        owners_released,
        #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
        generation,
    };
    for (owner, local) in LOCALS.iter().enumerate() {
        completed.switches += local.inspect(generation, |state| state.switches);
        for index in 0..TASKS {
            // Copy bounded completion evidence; no borrowed state escapes inspection.
            let task = local.inspect(generation, |state| {
                if state.tasks[index].state != CONTEXT_VACANT {
                    assert_eq!(
                        state.tasks[index].generation(),
                        generation,
                        "stale scheduler task"
                    );
                }
                state.tasks[index].result()
            });
            completed.tasks[owner * TASKS + index] = task;
        }
    }
    #[cfg(not(feature = "process-unlink-negative"))]
    unlink(&completed);
    SESSION.store(false, Ordering::Release);
    completed
}
/// Unlink scheduler references before publishing process completion. Reports and
/// copied frame evidence remain diagnostic data; roots can no longer be dispatched.
pub(crate) fn unlink(completed: &Completed) {
    assert!(completed.owners_released);
    for local in &LOCALS {
        assert!(local.completed());
        local.quiescent(local.generation(), |state| {
            state.current = NO_TASK;
            for task in &mut state.tasks {
                task.unlink();
            }
        });
    }
}
pub(crate) fn detached(id: kernel_core::process::ProcessId) -> bool {
    let local = &LOCALS[id.slot() / TASKS];
    if !local.completed() || memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire) != 0 {
        return false;
    }
    RUNNING_OWNER[id.slot()].load(Ordering::Acquire) == NO_CPU
        && local.inspect(local.generation(), |state| {
            let task = &state.tasks[id.slot() % TASKS];
            !task.linked() || task.id() != id.slot() || task.process_generation() != id.generation()
        })
}
fn acquire_process(id: usize) {
    assert_eq!(id / TASKS, percpu::id(), "wrong process affinity");
    let result = RUNNING_OWNER[id].compare_exchange(
        NO_CPU,
        percpu::id(),
        Ordering::AcqRel,
        Ordering::Acquire,
    );
    #[cfg(feature = "scheduler-owner-negative")]
    if result.is_err() {
        crate::event!(
            "{{\"event\":\"scheduler-reject\",\"status\":\"fail\",\"error\":\"DuplicateOwner\"}}"
        );
    }
    assert_eq!(result, Ok(NO_CPU), "process already running");
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
pub fn on_timer() {
    let local = &LOCALS[percpu::id()];
    if local.phase() == Phase::Running {
        local.with(local.generation(), |_| {
            local.preempt.store(true, Ordering::Release)
        });
    }
}
pub fn poll_secondary() {
    if LOCALS[percpu::SECONDARY_CPU].phase() == Phase::Admitted {
        run_local();
        cpu::unmask();
    }
}
fn validate_task(task: &Task, index: usize, generation: u64) {
    #[cfg(feature = "scheduler-task-negative")]
    if task.generation() != generation {
        crate::event!(
            "{{\"event\":\"scheduler-reject\",\"status\":\"fail\",\"error\":\"StaleTask\"}}"
        );
    }
    assert_eq!(task.generation(), generation, "stale scheduler task");
    assert_eq!(
        task.id(),
        percpu::id() * TASKS + index,
        "foreign scheduler task"
    );
}
fn choose(state: &mut State, generation: u64) -> Option<usize> {
    let ready: [bool; TASKS] =
        core::array::from_fn(|index| state.tasks[index].state == CONTEXT_READY);
    let current = (state.current != NO_TASK).then_some(state.current);
    if let Some(index) = kernel_core::scheduling::next_ready(current, &ready) {
        validate_task(&state.tasks[index], index, generation);
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
    let generation = local.generation();
    if local.start(generation).is_err() {
        return;
    }
    #[cfg(feature = "scheduler-contract-negative")]
    if percpu::id() == percpu::BOOT_CPU {
        local.controls(generation);
    }
    let first = local.with(generation, |state| {
        let index = choose(state, generation)?;
        Some((
            state.tasks[index].context,
            state.tasks[index].root(),
            state.tasks[index].id(),
        ))
    });
    if let Some(first) = first {
        acquire_process(first.2);
    }
    #[cfg(feature = "scheduler-owner-negative")]
    if percpu::id() == percpu::BOOT_CPU {
        acquire_process(first.expect("control needs task").2);
    }
    // Both CPUs rendezvous before first EL0 entry, so the test covers concurrent queues.
    ARRIVED.fetch_or(1 << percpu::id(), Ordering::AcqRel);
    crate::smp::wait(
        || ARRIVED.load(Ordering::Acquire) == (1 << platform_config::ACTIVE_CPUS) - 1,
        "EL0 CPU rendezvous timeout",
    );
    if percpu::id() == percpu::BOOT_CPU {
        START.store(true, Ordering::Release);
    } else {
        crate::smp::wait(|| START.load(Ordering::Acquire), "EL0 start timeout");
    }
    if let Some(first) = first {
        // SAFETY: INV-USER-TTBR: caller retains live immutable roots through acquired
        // completion; no State reference spans ERET, native stack/code remain mapped.
        unsafe {
            activate(first.1);
        }
        cpu::timer(time::deadline_after(QUANTUM));
        // SAFETY: INV-USER-CONTEXT: checked frame ABI and permanent native stack.
        unsafe {
            cpu::context::enter(&first.0, local.resume_sp.as_ptr());
        }
    }
    assert_eq!(cpu::el(), cpu::CURRENT_EL1);
    memory::USER_EXECUTION_ACTIVE.fetch_sub(1, Ordering::AcqRel);
    let quiescent = cpu::active_root() == NATIVE_ROOT.load(Ordering::Acquire)
        && local.with(generation, |state| {
            state.tasks.iter().all(|task| {
                task.state != CONTEXT_RUNNING
                    && (STEP.load(Ordering::Acquire) || task.state != CONTEXT_READY)
            })
        })
        && RUNNING_OWNER[percpu::id() * TASKS..(percpu::id() + 1) * TASKS]
            .iter()
            .all(|owner| owner.load(Ordering::Acquire) == NO_CPU);
    local.complete(generation, quiescent);
}

pub(crate) fn trap(frame: &mut Context, kind: Trap) -> usize {
    let local = &LOCALS[percpu::id()];
    let generation = local.generation();
    local.with(generation, |state| {
        assert!(state.current < TASKS);
        let current = state.current;
        let peer_faults = state
            .tasks
            .iter()
            .filter(|t| t.state == CONTEXT_FAULTED)
            .count();
        let task = &mut state.tasks[current];
        validate_task(task, current, generation);
        assert_eq!(task.state, CONTEXT_RUNNING);
        assert_eq!(frame.pstate & PSTATE_MODE_MASK, USER_EL0T);
        task.context = *frame;
        if matches!(kind, Trap::Irq) {
            if !local.preempt.swap(false, Ordering::AcqRel) {
                return 0;
            }
            task.slices += 1;
            task.observations.service_timer(task.slices);
            task.state = if task
                .slice_budget()
                .is_some_and(|limit| task.slices >= limit)
            {
                CONTEXT_TIMED_OUT
            } else {
                CONTEXT_READY
            };
            #[cfg(feature = "user-context-negative")]
            {
                cpu::context::corrupt_saved(&mut task.context);
            }
        } else {
            let Trap::Sync {
                class,
                operation,
                far,
            } = kind
            else {
                unreachable!()
            };
            if class == ESR_SVC64 && native_call(task, frame, operation) {
                task.context = *frame;
                // Bounded synchronous native request: no locks, allocation or retained user
                // pointers; current task identity came from the owned runqueue, not registers.
                return 0;
            }
            if class == ESR_SVC64 && operation == abi::FINISH {
                task.state = CONTEXT_EXITED;
                task.peer_faults_at_exit = peer_faults;
            } else {
                task.state = CONTEXT_FAULTED;
                task.fault_class = class;
                task.fault_far = far;
            }
        }
        if state.deadline.is_some_and(|limit| cpu::ticks() >= limit) {
            for task in &mut state.tasks {
                if task.state == CONTEXT_READY || task.state == CONTEXT_RUNNING {
                    task.state = CONTEXT_TIMED_OUT;
                }
            }
        }
        release_process(state.tasks[current].id());
        if STEP.load(Ordering::Acquire) {
            CURSOR[percpu::id()].store(current, Ordering::Release);
            cpu::timer_stop();
            // SAFETY: INV-USER-RETIRE: saved READY contexts remain owned by the
            // Registry; native root/TLBI precede returning a nonterminal step.
            unsafe {
                cpu::activate_root(NATIVE_ROOT.load(Ordering::Acquire));
            }
            return local.resume_sp.load(Ordering::Acquire);
        }
        if let Some(next) = choose(state, generation) {
            acquire_process(state.tasks[next].id());
            *frame = state.tasks[next].context;
            // SAFETY: INV-USER-TTBR: next pinned live root; completed flush precedes ERET.
            unsafe {
                activate(state.tasks[next].root());
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
    })
}
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
