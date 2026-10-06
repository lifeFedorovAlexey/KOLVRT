//! Bootstrap-only native process owner. No EL0 create endpoint or authority model.
use crate::{cpu, memory, percpu, platform::config, scheduler, time};
use core::sync::atomic::{AtomicBool, Ordering};
static REGISTRY_CREATED: AtomicBool = AtomicBool::new(false);
use cpu::context::Context;
use kernel_core::process::Table;
pub(crate) use kernel_core::process::{Completion, CreationStep, Error, ProcessId, Reason, State};
use scheduler::task::{
    Admission, CONTEXT_EXITED, CONTEXT_FAULTED, CONTEXT_TERMINATED, CONTEXT_TIMED_OUT,
};
pub(crate) const CAPACITY: usize = scheduler::CAPACITY;
const OWNER_CAPACITY: usize = scheduler::TASKS;
#[derive(Clone, Copy)]
pub(crate) enum Origin {
    Bootstrap,
    El0,
}
#[derive(Clone, Copy)]
pub(crate) struct Spec<'a> {
    pub image: &'a [u8],
    pub image_format: ImageFormat,
    pub limits: kernel_core::domain::Limits,
    pub context: Context,
    pub owner: usize,
    pub entry: usize,
    /// Explicit workload limit; None means normal exit/fault determines lifetime.
    pub slice_limit: Option<usize>,
}
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ImageFormat {
    RawFixture,
    Elf64Aarch64,
}
#[derive(Debug)]
pub(crate) struct CreationFailure {
    pub error: Error,
    pub completion: Option<Completion>,
}
struct Object {
    id: ProcessId,
    space: memory::OwnedUserSpace,
    context: Context,
    slice_limit: Option<usize>,
    slices: usize,
    el0_residency_ticks: u64,
    native_window_service_ticks: u64,
    observations: crate::execution::Observations,
    blocked: bool,
    domain: kernel_core::domain::Owner,
    _memory_charge: kernel_core::domain::Memory,
}
/// Mutable CPU0-owned value, not global shared storage. Owned frames make it
/// non-Send/non-Sync. Callers retain it independently of dispatch/workload lifetime.
pub(crate) struct Registry {
    table: Table<CAPACITY>,
    objects: [Option<Object>; CAPACITY],
    handles: [crate::handles::Namespace; CAPACITY],
}
pub(crate) fn context_contract() -> Result<(), Error> {
    if percpu::id() != percpu::BOOT_CPU {
        return Err(Error::ForeignCpu);
    }
    if !cpu::irq_masked() {
        return Err(Error::IrqEnabled);
    }
    crate::sync::assert_scheduler_unlocked();
    Ok(())
}
fn trace(id: ProcessId, state: State, step: Option<CreationStep>) {
    #[cfg(feature = "diagnostics")]
    crate::diagnostics::status(
        "DEBUG",
        "process",
        format_args!(
            "slot={} generation={} owner={} state={state:?} rollback={step:?}",
            id.slot(),
            id.generation(),
            id.slot() / OWNER_CAPACITY
        ),
    );
    #[cfg(not(feature = "diagnostics"))]
    let _ = (id, state, step);
}
impl Registry {
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(|error| reject(error))
    }
    pub fn try_new() -> Result<Self, Error> {
        context_contract()?;
        REGISTRY_CREATED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| Error::AlreadyOwned)?;
        Ok(Self {
            table: Table::new(),
            objects: core::array::from_fn(|_| None),
            handles: core::array::from_fn(|_| crate::handles::Namespace::new()),
        })
    }

    pub fn create(
        &mut self,
        physical: &mut memory::Physical,
        origin: Origin,
        spec: Spec<'_>,
        fail_at: Option<CreationStep>,
    ) -> Result<ProcessId, CreationFailure> {
        self.create_bounded(physical, origin, spec, fail_at, spec.limits)
    }
    pub fn create_bounded(
        &mut self,
        physical: &mut memory::Physical,
        origin: Origin,
        spec: Spec<'_>,
        fail_at: Option<CreationStep>,
        limits: kernel_core::domain::Limits,
    ) -> Result<ProcessId, CreationFailure> {
        self.create_inner(physical, origin, spec, fail_at, limits)
            .map_err(|(error, completion)| CreationFailure { error, completion })
    }
    fn create_inner(
        &mut self,
        physical: &mut memory::Physical,
        origin: Origin,
        spec: Spec<'_>,
        fail_at: Option<CreationStep>,
        limits: kernel_core::domain::Limits,
    ) -> Result<ProcessId, (Error, Option<Completion>)> {
        context_contract().map_err(|e| (e, None))?;
        if !matches!(origin, Origin::Bootstrap) {
            return Err((Error::Unauthorized, None));
        }
        if memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire) != 0 {
            return Err((Error::NotQuiescent, None));
        }
        let payload = spec.entry == config::USER_PAYLOAD_BASE;
        let (image, image_memory_size, image_pages) = match spec.image_format {
            ImageFormat::RawFixture => (
                spec.image,
                spec.image.len(),
                spec.image.len().div_ceil(config::PAGE_BYTES),
            ),
            ImageFormat::Elf64Aarch64 => {
                let plan = kernel_core::elf::parse(
                    spec.image,
                    config::USER_PAYLOAD_BASE,
                    config::USER_PAYLOAD_BYTES,
                    config::PAGE_BYTES,
                )
                .map_err(|_| (Error::InvalidImage, None))?;
                if plan.segment_count != 1
                    || plan.entry != spec.entry
                    || plan.segments[0].virtual_address != config::USER_PAYLOAD_BASE
                    || plan.segments[0].flags & kernel_core::elf::PF_X == 0
                    || plan.segments[0].flags & kernel_core::elf::PF_W != 0
                {
                    return Err((Error::InvalidImage, None));
                }
                let segment = plan.segments[0];
                let end = segment
                    .file_offset
                    .checked_add(segment.file_size)
                    .ok_or((Error::InvalidImage, None))?;
                (
                    &spec.image[segment.file_offset..end],
                    segment.memory_size,
                    plan.page_count,
                )
            }
        };
        if spec.owner >= config::ACTIVE_CPUS
            || spec.image.is_empty()
            || !(spec.entry == memory::USER_CODE || payload)
            || image.is_empty()
            || image_memory_size == 0
            || image_memory_size > config::USER_PAYLOAD_BYTES
            || spec.image_format == ImageFormat::Elf64Aarch64 && !payload
            || image.len()
                > if payload {
                    config::USER_PAYLOAD_BYTES
                } else {
                    config::PAGE_BYTES
                }
            || spec.context.pc != spec.entry as u64
            || spec.context.sp != memory::USER_STACK_TOP as u64
            || !cpu::context::valid_user_context(&spec.context)
            || spec.slice_limit == Some(0)
        {
            return Err((Error::InvalidImage, None));
        }
        let range = spec.owner * OWNER_CAPACITY..(spec.owner + 1) * OWNER_CAPACITY;
        let id = self.table.reserve(range).map_err(|e| (e, None))?;
        trace(id, State::Creating, None);
        let mut frame = None;
        let mut space = None;
        let mut domain = None;
        let mut memory_charge = None;
        let transaction = (|| {
            if fail_at == Some(CreationStep::Slot) {
                return Err((Error::Allocation, CreationStep::Slot));
            }
            let (owner, charge) = kernel_core::domain::Owner::new(
                id,
                limits,
                memory::USER_SPACE_PAGES + if payload { image_pages } else { 0 },
            )
            .map_err(|_| (Error::Allocation, CreationStep::Frames))?;
            domain = Some(owner);
            memory_charge = Some(charge);
            frame = Some(
                physical
                    .allocate(
                        memory::USER_SPACE_PAGES + if payload { image_pages } else { 0 },
                        1,
                    )
                    .ok_or((Error::Allocation, CreationStep::Frames))?,
            );
            if fail_at == Some(CreationStep::Frames) {
                return Err((Error::Allocation, CreationStep::Frames));
            }
            space = Some(memory::OwnedUserSpace::new(
                frame.take().unwrap(),
                id,
                image,
                spec.entry,
                image_memory_size,
            ));
            if fail_at == Some(CreationStep::Space) {
                return Err((Error::Allocation, CreationStep::Space));
            }
            let context = spec.context;
            if fail_at == Some(CreationStep::Context) {
                return Err((Error::Allocation, CreationStep::Context));
            }
            if fail_at == Some(CreationStep::Commit) {
                return Err((Error::Allocation, CreationStep::Commit));
            }
            self.objects[id.slot()] = Some(Object {
                id,
                space: space.take().unwrap(),
                context,
                slice_limit: spec.slice_limit,
                slices: 0,
                el0_residency_ticks: 0,
                native_window_service_ticks: 0,
                observations: crate::execution::Observations::ZERO,
                blocked: false,
                domain: domain.take().unwrap(),
                _memory_charge: memory_charge.take().unwrap(),
            });
            self.handles[id.slot()]
                .bind_domain(
                    id,
                    self.objects[id.slot()].as_ref().unwrap().domain.reference(),
                )
                .expect("fresh process namespace");
            scheduler::reset_event(id);
            self.table.prepared(id).expect("transaction state");
            Ok(())
        })();
        if let Err((error, step)) = transaction {
            if let Some(space) = space {
                #[cfg(not(feature = "process-rollback-negative"))]
                space.rollback(physical);
                #[cfg(feature = "process-rollback-negative")]
                core::mem::forget(space);
            }
            if let Some(frame) = frame {
                physical.release(frame);
            }
            let completion = self
                .table
                .rollback(id, step)
                .expect("creation rollback state");
            trace(id, State::Free, Some(step));
            return Err((error, Some(completion)));
        }
        trace(id, State::Prepared, None);
        Ok(id)
    }
    pub fn bootstrap_grant(
        &mut self,
        id: ProcessId,
        service: ProcessId,
        event: kernel_core::wait::SharedEvent,
        rights: kernel_core::handles::Rights,
    ) -> Result<kernel_core::handles::Handle, kernel_core::handles::Error> {
        context_contract().map_err(|_| kernel_core::handles::Error::Denied)?;
        if self.table.state(id).ok() != Some(State::Prepared)
            || self.table.state(service).ok() != Some(State::Prepared)
        {
            return Err(kernel_core::handles::Error::Denied);
        }
        self.handles[id.slot()]
            .create_granted_event(id, service, event, rights, |_| Ok::<_, ()>(()))
            .map_err(|e| match e {
                kernel_core::handles::CreationError::Handle(e) => e,
                kernel_core::handles::CreationError::Publication(()) => unreachable!(),
            })
    }
    pub fn domain_reference(&self, id: ProcessId) -> kernel_core::domain::Reference {
        context_contract().expect("domain reference bootstrap owner");
        self.table
            .state(id)
            .expect("exact live domain process identity");
        let object = self.objects[id.slot()]
            .as_ref()
            .expect("retained domain object");
        assert_eq!(object.id, id);
        object.domain.reference()
    }
    #[cfg(feature = "kernel-tests")]
    pub fn security_input(&mut self, id: ProcessId, values: [u64; 7]) {
        context_contract().unwrap();
        assert_eq!(self.table.state(id), Ok(State::Prepared));
        self.objects[id.slot()].as_mut().unwrap().context.gpr[20..27].copy_from_slice(&values);
    }
    pub fn start(&mut self, id: ProcessId) -> Result<(), Error> {
        context_contract()?;
        self.table.start(id)?;
        trace(id, State::Admitted, None);
        Ok(())
    }
    /// Trusted bootstrap binds one exact prepared service; the nondelegable
    /// receiver handle is separate from subsequently installed scoped SEND grants.
    pub fn bootstrap_endpoint(
        &mut self,
        service: ProcessId,
        capacity: usize,
    ) -> Result<
        (kernel_core::ipc::Reference, kernel_core::handles::Handle),
        kernel_core::handles::Error,
    > {
        use kernel_core::handles::{CreationError, Error as H};
        context_contract().map_err(|_| H::Denied)?;
        if self.table.state(service).ok() != Some(State::Prepared)
            || memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire) != 0
        {
            return Err(H::Denied);
        }
        let domain = self.objects[service.slot()]
            .as_ref()
            .ok_or(H::Denied)?
            .domain
            .reference();
        let reference = crate::ipc::create(domain, service.slot() / OWNER_CAPACITY, capacity)
            .map_err(|error| {
                if error == kernel_core::ipc::Error::Exhausted {
                    H::Budget
                } else {
                    H::Denied
                }
            })?;
        let handle = self.handles[service.slot()].create_endpoint_receiver(
            service,
            reference
                .try_clone()
                .expect("fresh endpoint reference bound"),
            |_| Ok::<_, ()>(()),
        );
        match handle {
            Ok(handle) => Ok((reference, handle)),
            Err(error) => {
                crate::ipc::discard(&reference);
                drop(reference);
                crate::ipc::reap();
                Err(match error {
                    CreationError::Handle(error) => error,
                    CreationError::Publication(()) => unreachable!(),
                })
            }
        }
    }
    pub fn bootstrap_sender(
        &mut self,
        client: ProcessId,
        endpoint: &kernel_core::ipc::Reference,
        rights: kernel_core::handles::Rights,
    ) -> Result<kernel_core::handles::Handle, kernel_core::handles::Error> {
        use kernel_core::handles::{CreationError, Error as H};
        context_contract().map_err(|_| H::Denied)?;
        if self.table.state(client).ok() != Some(State::Prepared)
            || memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire) != 0
        {
            return Err(H::Denied);
        }
        self.handles[client.slot()]
            .create_endpoint_sender(
                client,
                endpoint.try_clone().map_err(|_| H::ReferenceExhausted)?,
                rights,
                |_| Ok::<_, ()>(()),
            )
            .map_err(|error| match error {
                CreationError::Handle(error) => error,
                CreationError::Publication(()) => unreachable!(),
            })
    }
    pub fn ipc_input(&mut self, id: ProcessId, role: u64, handle: kernel_core::handles::Handle) {
        context_contract().expect("IPC bootstrap fixture setup");
        assert_eq!(self.table.state(id), Ok(State::Prepared));
        let context = &mut self.objects[id.slot()]
            .as_mut()
            .expect("prepared IPC process")
            .context;
        context.gpr[20] = role;
        context.gpr[21] = handle.encode();
    }
    pub fn ipc_control_input(&mut self, id: ProcessId, handle: kernel_core::handles::Handle) {
        context_contract().expect("IPC control bootstrap setup");
        assert_eq!(self.table.state(id), Ok(State::Prepared));
        self.objects[id.slot()]
            .as_mut()
            .expect("prepared control process")
            .context
            .gpr[22] = handle.encode();
    }
    pub fn ipc_survivor_input(&mut self, id: ProcessId, handle: kernel_core::handles::Handle) {
        context_contract().expect("IPC survivor bootstrap setup");
        assert_eq!(self.table.state(id), Ok(State::Prepared));
        self.objects[id.slot()]
            .as_mut()
            .expect("prepared survivor process")
            .context
            .gpr[23] = handle.encode();
    }
    pub fn ipc_previous_token_input(&mut self, id: ProcessId, token: u64) {
        context_contract().expect("IPC previous token fixture setup");
        assert_eq!(self.table.state(id), Ok(State::Prepared));
        self.objects[id.slot()]
            .as_mut()
            .expect("prepared token fixture")
            .context
            .gpr[23] = token;
    }
    pub fn ipc_queue_input(&mut self, id: ProcessId, capacity: usize) {
        context_contract().expect("IPC queue bootstrap setup");
        assert_eq!(self.table.state(id), Ok(State::Prepared));
        assert!([1, 4].contains(&capacity));
        self.objects[id.slot()]
            .as_mut()
            .expect("prepared queue process")
            .context
            .gpr[23] = capacity as u64;
    }
    #[cfg(feature = "ipc-benchmark")]
    pub fn ipc_bench_input(&mut self, id: ProcessId, payload: usize, kind: u64) {
        context_contract().expect("IPC benchmark bootstrap setup");
        assert_eq!(self.table.state(id), Ok(State::Prepared));
        assert!([0, 8, 64, 256].contains(&payload) && (1..=9).contains(&kind));
        let context = &mut self.objects[id.slot()]
            .as_mut()
            .expect("benchmark object")
            .context;
        context.gpr[23] = payload as u64;
        context.gpr[24] = kind;
    }
    pub fn ipc_authority_input(
        &mut self,
        id: ProcessId,
        missing: kernel_core::handles::Handle,
        event: kernel_core::handles::Handle,
        control: kernel_core::handles::Handle,
    ) {
        context_contract().expect("IPC authority bootstrap setup");
        assert_eq!(self.table.state(id), Ok(State::Prepared));
        self.objects[id.slot()]
            .as_mut()
            .expect("prepared authority process")
            .context
            .gpr[22..25]
            .copy_from_slice(&[missing.encode(), event.encode(), control.encode()]);
    }
    pub fn state(&self, id: ProcessId) -> Result<State, Error> {
        context_contract()?;
        self.table.state(id)
    }
    #[cfg(feature = "kernel-tests")]
    pub fn seed_handle(&mut self, id: ProcessId) -> kernel_core::handles::Handle {
        context_contract().expect("handle bootstrap owner");
        assert_eq!(self.table.state(id), Ok(State::Prepared));
        for _ in 0..CAPACITY - 1 {
            self.handles[id.slot()]
                .create_event_with_rights(
                    id,
                    kernel_core::wait::SharedEvent::try_new().expect("shared handle event quota"),
                    kernel_core::handles::Rights::ALL,
                    |_| Ok::<_, ()>(()),
                )
                .unwrap();
        }
        self.handles[id.slot()]
            .create_event_with_rights(
                id,
                kernel_core::wait::SharedEvent::try_new().expect("shared handle event quota"),
                kernel_core::handles::Rights::ALL,
                |_| Ok::<_, ()>(()),
            )
            .unwrap()
    }
    #[cfg(feature = "kernel-tests")]
    pub fn transfer_handle(
        &mut self,
        sender: ProcessId,
        handle: kernel_core::handles::Handle,
        receiver: ProcessId,
        rights: kernel_core::handles::Rights,
    ) -> Result<kernel_core::handles::Handle, kernel_core::handles::Error> {
        use kernel_core::handles::Error as HandleError;
        context_contract().map_err(|_| HandleError::ForeignProcess)?;
        if sender == receiver {
            return Err(HandleError::Invalid);
        }
        if self.table.state(sender).ok() != Some(State::Prepared)
            || self.table.state(receiver).ok() != Some(State::Prepared)
        {
            return Err(HandleError::Inactive);
        }
        if sender.slot() < receiver.slot() {
            let (before, after) = self.handles.split_at_mut(receiver.slot());
            before[sender.slot()].transfer(sender, handle, &mut after[0], receiver, rights)
        } else {
            let (before, after) = self.handles.split_at_mut(sender.slot());
            after[0].transfer(
                sender,
                handle,
                &mut before[receiver.slot()],
                receiver,
                rights,
            )
        }
    }
    #[cfg(feature = "kernel-tests")]
    pub fn handle_input(&mut self, id: ProcessId, raw: u64, expected: u64) {
        context_contract().expect("handle fixture setup");
        assert_eq!(self.table.state(id), Ok(State::Prepared));
        let object = self.objects[id.slot()].as_mut().unwrap();
        object.context.gpr[20] = raw;
        object.context.gpr[21] = expected;
    }
    #[cfg(feature = "kernel-tests")]
    pub fn delegated_handle_input(
        &mut self,
        id: ProcessId,
        raw: u64,
        expected: u64,
        delegated: u64,
    ) {
        self.handle_input(id, raw, expected);
        self.objects[id.slot()].as_mut().unwrap().context.gpr[22] = delegated;
    }
    #[cfg(feature = "kernel-tests")]
    pub fn transfer_target_input(&mut self, sender: ProcessId, receiver: ProcessId) {
        context_contract().expect("handle transfer fixture setup");
        assert_eq!(self.table.state(sender), Ok(State::Prepared));
        assert_eq!(self.table.state(receiver), Ok(State::Prepared));
        let context = &mut self.objects[sender.slot()].as_mut().unwrap().context;
        context.gpr[23] = receiver.slot() as u64;
        context.gpr[24] = receiver.generation();
    }
    #[cfg(feature = "kernel-tests")]
    pub fn wait_before_handle_input(&mut self, id: ProcessId) {
        context_contract().expect("handle wait fixture setup");
        assert_eq!(self.table.state(id), Ok(State::Prepared));
        self.objects[id.slot()].as_mut().unwrap().context.gpr[25] = 1;
    }
    #[cfg(feature = "kernel-tests")]
    pub fn handle_state(&self, id: ProcessId) -> (usize, bool) {
        context_contract().expect("handle inspection owner");
        (
            self.handles[id.slot()].live(),
            self.handles[id.slot()].owner().is_some(),
        )
    }
    pub fn live(&self) -> usize {
        context_contract().expect("process inspection owner");
        self.table.live()
    }
    pub fn data_address(&self, id: ProcessId) -> Result<usize, Error> {
        context_contract()?;
        self.table.state(id)?;
        Ok(self.objects[id.slot()]
            .as_ref()
            .ok_or(Error::Stale)?
            .space
            .data_address())
    }
    pub fn asid(&self, id: ProcessId) -> Result<u16, Error> {
        context_contract()?;
        self.table.state(id)?;
        Ok(self.objects[id.slot()]
            .as_ref()
            .ok_or(Error::Stale)?
            .space
            .lease()
            .asid)
    }
    #[cfg(feature = "kernel-tests")]
    pub fn image_address(&self, id: ProcessId) -> Result<usize, Error> {
        context_contract()?;
        self.table.state(id)?;
        Ok(self.objects[id.slot()]
            .as_ref()
            .ok_or(Error::Stale)?
            .space
            .image_address())
    }
    /// Kernel-owned resident frame charge for an exact live process identity.
    /// This remains valid after terminal completion and before explicit reclaim.
    #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
    pub fn resident_pages(&self, id: ProcessId) -> Result<usize, Error> {
        context_contract()?;
        self.table.state(id)?;
        let object = self.objects[id.slot()].as_ref().ok_or(Error::Stale)?;
        if object.id != id {
            return Err(Error::Stale);
        }
        Ok(object.space.resident_pages())
    }
    /// Internal synchronous completion driver, not a public wait ABI. No default
    /// workload deadline. The future service loop may schedule another dispatch.
    pub fn dispatch(&mut self, timeout: Option<time::Duration>) -> scheduler::Completed {
        self.dispatch_inner(timeout, false, false)
    }
    /// Preemptible scheduling step: surviving processes stay Admitted and retain
    /// their full context/accounting. No deadline is used to make dispatch return.
    pub fn step(&mut self) -> scheduler::Completed {
        self.dispatch_inner(None, true, false)
    }
    /// Privileged mechanism called only by the exact authorized lifecycle owner
    /// after the two-CPU native-root barrier. No manifest policy is interpreted here.
    pub fn lifecycle_bind(
        &mut self,
        supervisor: ProcessId,
        child: ProcessId,
    ) -> Result<
        (
            kernel_core::ipc::Reference,
            kernel_core::handles::Handle,
            kernel_core::ipc::Reference,
            kernel_core::handles::Handle,
        ),
        kernel_core::handles::Error,
    > {
        use kernel_core::handles::{CreationError, Error as H, Rights};
        context_contract().map_err(|_| H::Denied)?;
        if memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire) != 0
            || self.table.state(supervisor).ok() != Some(State::Admitted)
            || self.table.state(child).ok() != Some(State::Prepared)
        {
            return Err(H::Denied);
        }
        let (endpoint, receiver) = self.bootstrap_endpoint(child, 1)?;
        let feedback = match crate::ipc::create(
            self.domain_reference(supervisor),
            supervisor.slot() / OWNER_CAPACITY,
            1,
        ) {
            Ok(value) => value,
            Err(_) => {
                crate::ipc::discard(&endpoint);
                self.handles[child.slot()]
                    .close(child, receiver)
                    .expect("rollback receiver");
                return Err(H::Budget);
            }
        };
        let mut installed = [None; 3];
        let transaction = (|| {
            let send = self.handles[supervisor.slot()]
                .create_endpoint_sender(
                    supervisor,
                    endpoint.try_clone().map_err(|_| H::ReferenceExhausted)?,
                    Rights::SEND,
                    |_| Ok::<_, ()>(()),
                )
                .map_err(|e| match e {
                    CreationError::Handle(e) => e,
                    CreationError::Publication(()) => unreachable!(),
                })?;
            installed[0] = Some(send);
            let receive = self.handles[supervisor.slot()]
                .create_endpoint_receiver(
                    supervisor,
                    feedback.try_clone().map_err(|_| H::ReferenceExhausted)?,
                    |_| Ok::<_, ()>(()),
                )
                .map_err(|e| match e {
                    CreationError::Handle(e) => e,
                    CreationError::Publication(()) => unreachable!(),
                })?;
            installed[1] = Some(receive);
            let notify = self.handles[child.slot()]
                .create_endpoint_sender(
                    child,
                    feedback.try_clone().map_err(|_| H::ReferenceExhausted)?,
                    Rights::SEND,
                    |_| Ok::<_, ()>(()),
                )
                .map_err(|e| match e {
                    CreationError::Handle(e) => e,
                    CreationError::Publication(()) => unreachable!(),
                })?;
            installed[2] = Some(notify);
            self.ipc_input(child, 0, receiver);
            self.ipc_control_input(child, notify);
            Ok((send, receive))
        })();
        match transaction {
            Ok((send, receive)) => Ok((endpoint, send, feedback, receive)),
            Err(error) => {
                for handle in installed[..2].iter().flatten() {
                    self.handles[supervisor.slot()]
                        .close(supervisor, *handle)
                        .expect("rollback supervisor handle");
                }
                if let Some(handle) = installed[2] {
                    self.handles[child.slot()]
                        .close(child, handle)
                        .expect("rollback feedback sender");
                }
                self.handles[child.slot()]
                    .close(child, receiver)
                    .expect("rollback receiver");
                crate::ipc::discard(&endpoint);
                crate::ipc::discard(&feedback);
                Err(error)
            }
        }
    }
    pub fn lifecycle_close(
        &mut self,
        owner: ProcessId,
        handle: kernel_core::handles::Handle,
    ) -> Result<(), kernel_core::handles::Error> {
        context_contract().map_err(|_| kernel_core::handles::Error::Denied)?;
        if memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire) != 0 {
            return Err(kernel_core::handles::Error::Denied);
        }
        self.handles[owner.slot()].close(owner, handle)
    }
    pub fn lifecycle_stop(&mut self, id: ProcessId) -> Result<(), Error> {
        context_contract()?;
        if memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire) != 0
            || self.table.state(id)? != State::Admitted
            || !scheduler::detached(id)
        {
            return Err(Error::NotQuiescent);
        }
        // Termination becomes an owner-local terminal transition at the next
        // checkpoint. Its exact saved root remains owned until ASID retirement.
        self.objects[id.slot()].as_mut().unwrap().blocked = false;
        scheduler::lifecycle_stop(id);
        Ok(())
    }
    pub fn discard_prepared(
        &mut self,
        physical: &mut memory::Physical,
        id: ProcessId,
    ) -> Result<(), Error> {
        context_contract()?;
        if self.table.state(id)? != State::Prepared || !scheduler::detached(id) {
            return Err(Error::NotQuiescent);
        }
        self.handles[id.slot()]
            .retire(id)
            .map_err(|_| Error::Transition)?;
        self.table.discard_prepared(id, true)?;
        let object = self.objects[id.slot()].take().ok_or(Error::Stale)?;
        #[cfg(not(feature = "process-rollback-negative"))]
        object.space.rollback(physical);
        #[cfg(feature = "process-rollback-negative")]
        {
            let _ = physical;
            core::mem::forget(object);
        }
        self.table.released(id)
    }
    pub fn checkpoint(&mut self) -> scheduler::Completed {
        self.dispatch_inner(None, true, true)
    }
    pub fn management_reply(&mut self, id: ProcessId, values: [u64; 5]) -> Result<(), Error> {
        context_contract()?;
        if self.table.state(id)? != State::Admitted {
            return Err(Error::Transition);
        }
        self.objects[id.slot()]
            .as_mut()
            .ok_or(Error::Stale)?
            .context
            .gpr[..5]
            .copy_from_slice(&values);
        Ok(())
    }
    pub fn dispatch_ipc(&mut self, timeout: Option<time::Duration>) -> scheduler::Completed {
        self.dispatch_inner(timeout, false, true)
    }
    /// Trusted bootstrap notification, not an EL0 send API. Identity lookup and
    /// coordinator ownership precede touching the retained event of this generation.
    pub fn signal(&mut self, id: ProcessId) -> Result<bool, Error> {
        context_contract()?;
        if self.table.state(id)? != State::Admitted {
            return Err(Error::Transition);
        }
        let object = self.objects[id.slot()].as_mut().ok_or(Error::Stale)?;
        let fresh = scheduler::event(id.slot()).signal();
        if object.blocked && scheduler::event(id.slot()).consume() {
            object.blocked = false;
        }
        Ok(fresh)
    }
    pub fn blocked(&self, id: ProcessId) -> Result<bool, Error> {
        context_contract()?;
        if self.table.state(id)? != State::Admitted {
            return Err(Error::Transition);
        }
        Ok(self.objects[id.slot()]
            .as_ref()
            .ok_or(Error::Stale)?
            .blocked)
    }
    fn dispatch_inner(
        &mut self,
        timeout: Option<time::Duration>,
        step: bool,
        continuous: bool,
    ) -> scheduler::Completed {
        context_contract().expect("process dispatch owner");
        let mut tasks: [Option<Admission<'_>>; CAPACITY] = core::array::from_fn(|_| None);
        for ((slot, object), handles) in
            self.objects.iter().enumerate().zip(self.handles.iter_mut())
        {
            let Some(object) = object.as_ref() else {
                continue;
            };
            if self.table.state(object.id).ok() != Some(State::Admitted) {
                continue;
            }
            tasks[slot] = Some(Admission {
                identity: object.id,
                handles,
                space: &object.space,
                context: object.context,
                slice_budget: object.slice_limit,
                slices: object.slices,
                el0_residency_ticks: object.el0_residency_ticks,
                native_window_service_ticks: object.native_window_service_ticks,
                observations: object.observations,
                blocked: object.blocked,
            });
        }
        let completed = if step && continuous {
            scheduler::checkpoint(&mut tasks)
        } else if step {
            scheduler::step(&mut tasks)
        } else if continuous {
            scheduler::dispatch_ipc(&mut tasks, timeout)
        } else {
            scheduler::dispatch(&mut tasks, timeout)
        };
        for result in completed.tasks.iter().filter(|r| r.process_generation != 0) {
            let object = self.objects[result.id]
                .as_mut()
                .expect("admitted process retained");
            assert_eq!(
                object.id.generation(),
                result.process_generation,
                "stale scheduler process completion"
            );
            object.context = result.context;
            object.slices = result.slices;
            object.el0_residency_ticks = result.el0_residency_ticks;
            object.native_window_service_ticks = result.native_window_service_ticks;
            object.observations = result.observations;
            object.blocked = result.state == scheduler::task::CONTEXT_BLOCKED;
            if !continuous && object.blocked && scheduler::event(object.id.slot()).consume() {
                object.blocked = false;
            }
            if step && result.state == scheduler::task::CONTEXT_BLOCKED {
                assert!(scheduler::detached(object.id));
                continue;
            }
            if step && result.state == scheduler::task::CONTEXT_READY {
                assert!(scheduler::detached(object.id));
                continue;
            }
            #[cfg(feature = "diagnostics")]
            if self.handles[result.id].live() != 0 {
                crate::diagnostics::status(
                    "DEBUG",
                    "handles",
                    format_args!(
                        "process_slot={} process_generation={} capacity={} active={} lifecycle=retiring",
                        object.id.slot(),
                        object.id.generation(),
                        self.handles[result.id].capacity(),
                        self.handles[result.id].live()
                    ),
                );
                for slot in 0..self.handles[result.id].capacity() {
                    if let Some((generation, Some(kind))) = self.handles[result.id].slot_state(slot)
                    {
                        crate::diagnostics::status(
                            "DEBUG",
                            "handles",
                            format_args!("slot={} generation={} kind={kind:?}", slot, generation),
                        );
                    }
                }
            }
            object.domain.close();
            if !cfg!(feature = "handle-retirement-negative") || self.handles[result.id].live() == 0
            {
                self.handles[result.id]
                    .retire(object.id)
                    .expect("terminal namespace owner");
            }
            let reason = match result.state {
                CONTEXT_EXITED => Reason::Exited(result.context.gpr[0]),
                CONTEXT_FAULTED => Reason::Faulted {
                    class: result.fault_class,
                    address: result.fault_far,
                },
                CONTEXT_TIMED_OUT => Reason::BudgetExpired,
                CONTEXT_TERMINATED => Reason::Terminated,
                _ => panic!("nonterminal process completion"),
            };
            let detached = scheduler::detached(object.id);
            self.table
                .complete(object.id, reason, detached)
                .unwrap_or_else(|error| reject(error));
            object.context = result.context;
            trace(object.id, State::Completed, None);
        }
        completed
    }
    #[cfg(feature = "kernel-tests")]
    pub fn repeat_completion(&mut self, id: ProcessId) -> Result<(), Error> {
        context_contract()?;
        let completion = self.table.completion(id)?;
        self.table
            .complete(id, completion.reason, scheduler::detached(id))
    }
    pub fn completion(&self, id: ProcessId) -> Result<Completion, Error> {
        context_contract()?;
        self.table.completion(id)
    }
    pub fn validate_completion(&self, completion: Completion) -> Result<(), Error> {
        context_contract()?;
        self.table.validate_completion(completion)
    }
    pub fn reclaim(&mut self, physical: &mut memory::Physical, id: ProcessId) -> Result<(), Error> {
        context_contract()?;
        self.table.state(id)?;
        let quiescent = scheduler::detached(id);
        self.table.reclaim(id, quiescent)?;
        trace(id, State::Reclaiming, None);
        let object = self.objects[id.slot()].take().ok_or(Error::Stale)?;
        assert_eq!(object.id, id);
        if self.handles[id.slot()].live() != 0 || self.handles[id.slot()].owner().is_some() {
            crate::event!("{{\"event\":\"handle-retirement-reject\",\"status\":\"fail\"}}");
            panic!("handle namespace must retire before reclamation");
        }
        object.space.reclaim(physical);
        self.table.released(id)?;
        trace(id, State::Free, None);
        Ok(())
    }
}
pub(crate) fn reject(error: Error) -> ! {
    #[cfg(feature = "process-contract-negative")]
    crate::event!(
        "{{\"event\":\"process-reject\",\"status\":\"fail\",\"error\":\"{:?}\"}}",
        error
    );
    panic!("process contract rejection: {error:?}")
}

impl Drop for Registry {
    fn drop(&mut self) {
        assert_eq!(self.table.live(), 0, "live process registry abandonment");
    }
}
