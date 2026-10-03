use super::{NO_TASK, TASKS};
use crate::cpu::context::Context;
pub(crate) const CONTEXT_READY: usize = 0;
pub(super) const CONTEXT_RUNNING: usize = 1;
pub(crate) const CONTEXT_EXITED: usize = 2;
pub(crate) const CONTEXT_FAULTED: usize = 3;
pub(crate) const CONTEXT_TIMED_OUT: usize = 4;
pub(crate) const CONTEXT_BLOCKED: usize = 6;
pub(super) const CONTEXT_VACANT: usize = 5;
/// Small fully initialized admission descriptor; no diagnostic report storage is
/// copied through caller stacks. The retained root belongs to the process owner.
#[derive(Clone, Copy)]
pub(crate) struct Admission<'a> {
    pub identity: kernel_core::process::ProcessId,
    pub space: &'a crate::memory::OwnedUserSpace,
    pub context: Context,
    pub slice_budget: Option<usize>,
    pub slices: usize,
    /// Total generic-counter ticks observed while executing at EL0, accumulated
    /// at each synchronous exception/IRQ boundary.
    pub el0_residency_ticks: u64,
    /// EL1 execution time spent servicing the native routing backend.
    pub native_window_service_ticks: u64,
    pub observations: crate::execution::Observations,
    pub blocked: bool,
}
#[derive(Clone, Copy)]
pub(crate) struct Task {
    #[cfg(feature = "kernel-tests")]
    pub copy_snapshot: [u8; 8],
    pub context: Context,
    definition: Definition,
    pub state: usize,
    pub slices: usize,
    pub el0_residency_ticks: u64,
    pub native_window_service_ticks: u64,
    pub entered_at: u64,
    pub fault_class: u64,
    pub fault_far: usize,
    pub peer_faults_at_exit: usize,
    pub observations: crate::execution::Observations,
    #[cfg(feature = "machine-events")]
    pub report_len: usize,
}
/// Immutable after publication. Only construction replaces a definition; owner
/// mutation gets read-only accessors, not mutable identity/root/budget fields.
#[derive(Clone, Copy)]
struct Definition {
    id: usize,
    generation: u64,
    root: u64,
    slice_budget: Option<usize>,
    process_generation: u64,
    linked: bool,
}
impl Task {
    pub const ZERO: Self = Self {
        #[cfg(feature = "kernel-tests")]
        copy_snapshot: [0; 8],
        context: Context::ZERO,
        definition: Definition {
            id: 0,
            generation: 0,
            root: 0,
            slice_budget: None,
            process_generation: 0,
            linked: false,
        },
        state: CONTEXT_VACANT,
        slices: 0,
        el0_residency_ticks: 0,
        native_window_service_ticks: 0,
        entered_at: 0,
        fault_class: 0,
        fault_far: 0,
        peer_faults_at_exit: 0,
        observations: crate::execution::Observations::ZERO,
        #[cfg(feature = "machine-events")]
        report_len: 0,
    };
    pub fn new(
        id: usize,
        generation: u64,
        root: u64,
        context: Context,
        slice_budget: Option<usize>,
    ) -> Self {
        Self {
            context,
            definition: Definition {
                id,
                generation,
                root,
                slice_budget,
                process_generation: generation,
                linked: true,
            },
            state: CONTEXT_READY,
            ..Self::ZERO
        }
    }
    pub fn process_generation(&self) -> u64 {
        self.definition.process_generation
    }
    pub fn bind_queue(&mut self, generation: u64) {
        self.definition.generation = generation;
    }
    pub fn unlink(&mut self) {
        self.definition.root = 0;
        self.definition.linked = false;
    }
    pub fn linked(&self) -> bool {
        self.definition.linked
    }
    pub fn id(&self) -> usize {
        self.definition.id
    }
    pub fn generation(&self) -> u64 {
        self.definition.generation
    }
    pub fn root(&self) -> u64 {
        self.definition.root
    }
    pub fn slice_budget(&self) -> Option<usize> {
        self.definition.slice_budget
    }
    #[cfg(feature = "scheduler-task-negative")]
    pub fn corrupt_generation(&mut self) {
        self.definition.generation -= 1;
    }
    pub fn result(&self) -> TaskResult {
        TaskResult {
            context: self.context,
            state: self.state,
            slices: self.slices,
            el0_residency_ticks: self.el0_residency_ticks,
            native_window_service_ticks: self.native_window_service_ticks,
            observations: self.observations,
            fault_class: self.fault_class,
            id: self.id(),
            process_generation: self.process_generation(),
            fault_far: self.fault_far,
            peer_faults_at_exit: self.peer_faults_at_exit,
            #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
            report_len: self.report_len,
            #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
            native_attempts: self.observations.attempts(),
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct TaskResult {
    pub context: Context,
    pub state: usize,
    pub slices: usize,
    pub el0_residency_ticks: u64,
    pub native_window_service_ticks: u64,
    pub observations: crate::execution::Observations,
    pub fault_class: u64,
    pub id: usize,
    pub process_generation: u64,
    pub fault_far: usize,
    pub peer_faults_at_exit: usize,
    #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
    pub report_len: usize,
    #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
    pub native_attempts: u64,
}
impl TaskResult {
    pub const ZERO: Self = Self {
        context: Context::ZERO,
        state: CONTEXT_VACANT,
        slices: 0,
        el0_residency_ticks: 0,
        native_window_service_ticks: 0,
        observations: crate::execution::Observations::ZERO,
        fault_class: 0,
        id: 0,
        process_generation: 0,
        fault_far: 0,
        peer_faults_at_exit: 0,
        #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
        report_len: 0,
        #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
        native_attempts: 0,
    };
}
pub(super) struct State {
    pub tasks: [Task; TASKS],
    pub current: usize,
    pub switches: usize,
    pub deadline: Option<u64>,
}
impl State {
    pub const ZERO: Self = Self {
        tasks: [Task::ZERO; TASKS],
        current: NO_TASK,
        switches: 0,
        deadline: None,
    };
}
