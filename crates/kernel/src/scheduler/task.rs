use super::{NO_TASK, TASKS};
use crate::cpu::context::Context;
pub(crate) const CONTEXT_READY: usize = 0;
pub(super) const CONTEXT_RUNNING: usize = 1;
pub(crate) const CONTEXT_EXITED: usize = 2;
pub(crate) const CONTEXT_FAULTED: usize = 3;
pub(crate) const CONTEXT_TIMED_OUT: usize = 4;
pub(crate) const CONTEXT_TERMINATED: usize = 7;
pub(crate) const CONTEXT_BLOCKED: usize = 6;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IpcWait {
    Wake {
        key: kernel_core::ipc::WaitKey,
        target: kernel_core::ipc::WaitTarget,
    },
    Finish,
    Retry,
}
pub(super) struct IpcPending {
    pub caller: kernel_core::process::ProcessId,
    pub action: crate::ipc::native::Action,
    pub copy: Result<usize, u64>,
}
pub(super) const CONTEXT_VACANT: usize = 5;
/// Small fully initialized admission descriptor; no diagnostic report storage is
/// copied through caller stacks. The retained root belongs to the process owner.
pub(crate) struct Admission<'a> {
    pub identity: kernel_core::process::ProcessId,
    pub handles: &'a mut crate::handles::Namespace,
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
    pub ipc_blocks: u64,
    pub ipc_terminal_blocks: u64,
    pub ipc_wakes: u64,
    pub(super) ipc_wait: Option<IpcWait>,
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
    lease: crate::asid::Lease,
    slice_budget: Option<usize>,
    process_generation: u64,
    linked: bool,
}
impl Task {
    pub const ZERO: Self = Self {
        ipc_blocks: 0,
        ipc_terminal_blocks: 0,
        ipc_wakes: 0,
        ipc_wait: None,
        #[cfg(feature = "kernel-tests")]
        copy_snapshot: [0; 8],
        context: Context::ZERO,
        definition: Definition {
            id: 0,
            generation: 0,
            root: 0,
            lease: crate::asid::Lease::NATIVE,
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
        lease: crate::asid::Lease,
        context: Context,
        slice_budget: Option<usize>,
    ) -> Self {
        Self {
            context,
            definition: Definition {
                id,
                generation,
                root,
                lease,
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
    /// The continuous IPC session cannot transfer a blocked context back to
    /// a coordinator. Its root stays linked until actual terminal quiescence.
    pub(super) fn unlink_ipc(&mut self) {
        if !matches!(
            self.state,
            CONTEXT_VACANT
                | CONTEXT_EXITED
                | CONTEXT_FAULTED
                | CONTEXT_TIMED_OUT
                | CONTEXT_TERMINATED
        ) || self.ipc_wait.is_some()
        {
            reject_ipc("BlockedTaskReclaim");
        }
        self.unlink();
    }
    /// Only the fixed owner may publish one runnable state for this exact wait.
    /// Replaying a publication cannot create a second runnable/running owner.
    pub(super) fn publish_ipc_ready(&mut self, key: kernel_core::ipc::WaitKey) {
        if key.process().slot() != self.id()
            || key.process().generation() != self.process_generation()
        {
            reject_ipc("WrongProcessWake");
        }
        if self.state != CONTEXT_BLOCKED
            || !matches!(self.ipc_wait, Some(IpcWait::Wake { key: saved, .. }) if saved == key)
        {
            reject_ipc("DuplicateReady");
        }
        self.ipc_wakes = self
            .ipc_wakes
            .checked_add(1)
            .expect("IPC wake counter exhausted");
        self.ipc_wait = None;
        self.state = CONTEXT_READY;
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
    pub fn lease(&self) -> crate::asid::Lease {
        self.definition.lease
    }
    pub fn slice_budget(&self) -> Option<usize> {
        self.definition.slice_budget
    }
    pub fn result(&self) -> TaskResult {
        TaskResult {
            ipc_blocks: self.ipc_blocks,
            ipc_terminal_blocks: self.ipc_terminal_blocks,
            ipc_wakes: self.ipc_wakes,
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
            #[cfg(all(
                feature = "machine-events",
                any(feature = "boot-payload", feature = "ipc-benchmark")
            ))]
            report_len: self.report_len,
            #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
            native_attempts: self.observations.attempts(),
        }
    }
}
fn reject_ipc(error: &'static str) -> ! {
    super::reject_ipc(error)
}
#[derive(Clone, Copy)]
pub(crate) struct TaskResult {
    pub ipc_blocks: u64,
    pub ipc_terminal_blocks: u64,
    pub ipc_wakes: u64,
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
    #[cfg(all(
        feature = "machine-events",
        any(feature = "boot-payload", feature = "ipc-benchmark")
    ))]
    pub report_len: usize,
    #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
    pub native_attempts: u64,
}
impl TaskResult {
    pub const ZERO: Self = Self {
        ipc_blocks: 0,
        ipc_terminal_blocks: 0,
        ipc_wakes: 0,
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
        #[cfg(all(
            feature = "machine-events",
            any(feature = "boot-payload", feature = "ipc-benchmark")
        ))]
        report_len: 0,
        #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
        native_attempts: 0,
    };
}
pub(super) struct State {
    pub ipc_pending: [Option<IpcPending>; TASKS],
    pub tasks: [Task; TASKS],
    pub handles: [crate::handles::Namespace; TASKS],
    pub requests: crate::security::Queue,
    pub current: usize,
    pub switches: usize,
    pub deadline: Option<u64>,
}
impl State {
    pub const ZERO: Self = Self {
        ipc_pending: [const { None }; TASKS],
        tasks: [Task::ZERO; TASKS],
        handles: [const { crate::handles::Namespace::new() }; TASKS],
        requests: crate::security::Queue::EMPTY,
        current: NO_TASK,
        switches: 0,
        deadline: None,
    };
}
