use super::{NO_TASK, TASKS};
use crate::cpu::context::Context;
pub(super) const CONTEXT_READY: usize = 0;
pub(super) const CONTEXT_RUNNING: usize = 1;
pub(crate) const CONTEXT_EXITED: usize = 2;
pub(crate) const CONTEXT_FAULTED: usize = 3;
pub(super) const CONTEXT_TIMED_OUT: usize = 4;
#[cfg(feature = "machine-events")]
use kernel_core::execution as abi;
#[derive(Clone, Copy)]
pub(crate) struct Task {
    pub context: Context,
    definition: Definition,
    pub state: usize,
    pub slices: usize,
    pub fault_class: u64,
    pub fault_far: usize,
    pub peer_faults_at_exit: usize,
    pub observations: crate::execution::Observations,
    #[cfg(feature = "machine-events")]
    pub report: [u64; abi::REPORT_WORDS],
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
    slice_budget: usize,
}
impl Task {
    pub const ZERO: Self = Self {
        context: Context::ZERO,
        definition: Definition {
            id: 0,
            generation: 0,
            root: 0,
            slice_budget: 0,
        },
        state: CONTEXT_READY,
        slices: 0,
        fault_class: 0,
        fault_far: 0,
        peer_faults_at_exit: 0,
        observations: crate::execution::Observations::ZERO,
        #[cfg(feature = "machine-events")]
        report: [0; abi::REPORT_WORDS],
        #[cfg(feature = "machine-events")]
        report_len: 0,
    };
    pub fn new(
        id: usize,
        generation: u64,
        root: u64,
        context: Context,
        slice_budget: usize,
    ) -> Self {
        Self {
            context,
            definition: Definition {
                id,
                generation,
                root,
                slice_budget,
            },
            ..Self::ZERO
        }
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
    pub fn slice_budget(&self) -> usize {
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
            fault_class: self.fault_class,
            id: self.id(),
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
    pub fault_class: u64,
    pub id: usize,
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
        state: CONTEXT_READY,
        slices: 0,
        fault_class: 0,
        id: 0,
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
    pub deadline: u64,
}
impl State {
    pub const ZERO: Self = Self {
        tasks: [Task::ZERO; TASKS],
        current: NO_TASK,
        switches: 0,
        deadline: 0,
    };
}
