//! Privileged lifecycle mechanism. Images, placement and maximum authority are
//! immutable bootstrap grants; ordering/readiness/restart/shutdown policy is EL0.
use crate::{
    cpu::context::Context,
    memory,
    process::{ImageFormat, Origin, ProcessId, Reason, Registry, Spec, State},
    scheduler::task::Task,
};
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use kernel_core::{domain::Limits, handles::Handle, ipc::Reference};
pub(crate) const CONTROL: u16 = 0xb0;
static INSTALLED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static OWNER_SLOT: AtomicUsize = AtomicUsize::new(usize::MAX);
static OWNER_GENERATION: AtomicU64 = AtomicU64::new(0);
static PENDING: AtomicBool = AtomicBool::new(false);
static WORDS: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
/// Capturing registers never creates authority. The executing Task is provenance.
pub(crate) fn capture(task: &Task, frame: &mut Context) {
    if !cfg!(feature = "supervision-authority-negative")
        && (!ACTIVE.load(Ordering::Acquire)
            || task.id() != OWNER_SLOT.load(Ordering::Acquire)
            || task.process_generation() != OWNER_GENERATION.load(Ordering::Acquire))
    {
        frame.gpr[..5].copy_from_slice(&[11, 0, 0, 0, 0]);
        return;
    }
    if frame.gpr[4] != 1 {
        frame.gpr[..5].copy_from_slice(&[1, 0, 0, 0, 0]);
        return;
    }
    if PENDING.load(Ordering::Acquire) {
        frame.gpr[..5].copy_from_slice(&[13, 0, 0, 0, 0]);
        return;
    }
    for (word, value) in WORDS.iter().zip(frame.gpr) {
        word.store(value, Ordering::Relaxed);
    }
    PENDING.store(true, Ordering::Release);
}
#[derive(Clone, Copy)]
pub(crate) struct Grant {
    pub image: &'static [u8],
    pub image_format: ImageFormat,
    /// Finite immutable SEND destination; selectors/tokens cannot widen it.
    pub send_to: Option<usize>,
    pub owner: usize,
    pub limits: Limits,
    /// Maximum total instantiations, not an automatic restart policy.
    pub instances: usize,
}
struct Instance {
    id: ProcessId,
    endpoint: Reference,
    sender: Handle,
    feedback: Reference,
    feedback_receiver: Handle,
}
struct Entry {
    grant: Grant,
    remaining: usize,
    token: u64,
    instance: Option<Instance>,
    outbound: Option<Handle>,
}
/// Linear CPU0 authority owner. No service selector, caller-supplied image,
/// manifest, quota or ProcessId can widen these grants through the native entry.
pub(crate) struct Scope {
    supervisor: ProcessId,
    entries: [Entry; 3],
    sealed: bool,
    next_token: u64,
    _local: core::marker::PhantomData<*mut ()>,
}
impl Scope {
    pub fn install(supervisor: ProcessId, grants: [Grant; 3]) -> Self {
        crate::process::context_contract().expect("bootstrap lifecycle owner");
        assert!(
            INSTALLED
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok(),
            "bootstrap is single use"
        );
        assert!(
            grants
                .iter()
                .all(|g| !g.image.is_empty() && g.instances > 0 && g.instances <= 8)
        );
        assert!(
            grants.iter().enumerate().all(|(index, grant)| grant
                .send_to
                .is_none_or(|target| target < grants.len() && target != index)),
            "invalid immutable SEND grant edge"
        );
        OWNER_SLOT.store(supervisor.slot(), Ordering::Relaxed);
        OWNER_GENERATION.store(supervisor.generation(), Ordering::Relaxed);
        ACTIVE.store(true, Ordering::Release);
        Self {
            supervisor,
            entries: grants.map(|grant| Entry {
                remaining: grant.instances,
                grant,
                token: 0,
                instance: None,
                outbound: None,
            }),
            sealed: false,
            next_token: 1,
            _local: core::marker::PhantomData,
        }
    }
    pub fn service(&mut self, registry: &mut Registry, physical: &mut memory::Physical) {
        crate::process::context_contract().expect("lifecycle barrier owner");
        assert_eq!(
            memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire),
            0,
            "lifecycle requires acquired owner barrier"
        );
        assert!(crate::scheduler::detached(self.supervisor));
        if !PENDING.swap(false, Ordering::Acquire) {
            return;
        }
        let command = core::array::from_fn::<_, 4, _>(|i| WORDS[i].load(Ordering::Relaxed));
        let reply = self.execute(registry, physical, command);
        registry
            .management_reply(self.supervisor, reply)
            .expect("exact supervisor reply");
    }
    fn execute(
        &mut self,
        registry: &mut Registry,
        physical: &mut memory::Physical,
        [op, selector, token, argument]: [u64; 4],
    ) -> [u64; 5] {
        if op == 5 {
            if selector != 0 || token != 0 || argument != 0 || self.sealed {
                return [11, 0, 0, 0, 0];
            }
            self.sealed = true;
            // Unused bootstrap launch grants are extinguished. Already issued
            // exact service capabilities retain only their accounted remainder.
            for entry in &mut self.entries {
                if entry.token == 0 {
                    entry.remaining = 0;
                }
            }
            return [0, 0, 0, 0, 0];
        }
        let Ok(index) = usize::try_from(selector) else {
            return [11, 0, 0, 0, 0];
        };
        if op == 6 {
            return self.bind_sender(registry, index, token, argument);
        }
        let Some(entry) = self.entries.get_mut(index) else {
            return [11, 0, 0, 0, 0];
        };
        if op == 1 {
            if self.sealed || token != 0 || entry.token != 0 {
                return [11, 0, 0, 0, 0];
            }
        } else if !cfg!(feature = "supervision-stale-negative")
            && (token == 0 || token != entry.token)
        {
            return [2, 0, 0, 0, 0];
        }
        match op {
            1 | 3 => {
                if entry.remaining == 0 {
                    return [12, 0, 0, 0, 0];
                }
                if let Some(old) = entry.instance.as_ref()
                    && registry.completion(old.id).is_err()
                {
                    return [13, 0, 0, 0, 0];
                }
                if let Some(old) = entry.instance.take() {
                    match registry.lifecycle_close(self.supervisor, old.sender) {
                        Ok(()) | Err(kernel_core::handles::Error::Stale) => {}
                        Err(error) => panic!("exact old sender cleanup: {error:?}"),
                    }
                    match registry.lifecycle_close(self.supervisor, old.feedback_receiver) {
                        Ok(()) | Err(kernel_core::handles::Error::Stale) => {}
                        Err(error) => panic!("exact old feedback cleanup: {error:?}"),
                    }
                    crate::ipc::discard(&old.feedback);
                    registry
                        .reclaim(physical, old.id)
                        .expect("detached old instance");
                    entry.outbound = None; // old namespace retirement consumed its grants
                    drop(old);
                    crate::ipc::deferred::poll(&[]);
                }
                let mut context = Context::ZERO;
                let image_entry = match entry.grant.image_format {
                    ImageFormat::RawFixture => memory::USER_CODE,
                    ImageFormat::Elf64Aarch64 => crate::platform::config::USER_PAYLOAD_BASE,
                };
                context.pc = image_entry as u64;
                context.sp = memory::USER_STACK_TOP as u64;
                context.gpr[23] = argument; // opaque initialized service input, never authority
                let before_creation = physical.available();
                let created = registry.create(
                    physical,
                    Origin::Bootstrap,
                    Spec {
                        image: entry.grant.image,
                        image_format: entry.grant.image_format,
                        limits: entry.grant.limits,
                        context,
                        owner: entry.grant.owner,
                        entry: image_entry,
                        slice_limit: None,
                    },
                    None,
                );
                let Ok(id) = created else {
                    return [12, 0, 0, 0, 0];
                };
                let bound = registry.lifecycle_bind(self.supervisor, id);
                let Ok((endpoint, sender, feedback, feedback_receiver)) = bound else {
                    registry
                        .discard_prepared(physical, id)
                        .expect("unpublished creation rollback");
                    crate::ipc::deferred::poll(&[]);
                    crate::ipc::reap();
                    assert_eq!(
                        physical.available(),
                        before_creation,
                        "unpublished frame rollback"
                    );
                    assert_eq!(registry.state(id), Err(kernel_core::process::Error::Stale));
                    return [12, 0, 0, 0, 0];
                };
                let fresh = self.next_token;
                self.next_token = self
                    .next_token
                    .checked_add(1)
                    .expect("lifecycle generation exhausted");
                entry.token = fresh;
                entry.remaining -= 1;
                registry.start(id).expect("prepared authorized instance");
                entry.instance = Some(Instance {
                    id,
                    endpoint,
                    sender,
                    feedback,
                    feedback_receiver,
                });
                [
                    0,
                    fresh,
                    sender.encode(),
                    feedback_receiver.encode(),
                    id.generation(),
                ]
            }
            2 => {
                if argument != 0 {
                    return [1, 0, 0, 0, 0];
                }
                let Some(instance) = entry.instance.as_ref() else {
                    return [2, 0, 0, 0, 0];
                };
                match registry.completion(instance.id) {
                    Err(crate::process::Error::Transition) => [0, 0, 0, 0, 0],
                    Err(_) => [2, 0, 0, 0, 0],
                    Ok(c) => match c.reason {
                        Reason::Exited(code) => [0, 1, code, 0, instance.id.generation()],
                        Reason::Faulted { class, address } => {
                            [0, 2, class, address as u64, instance.id.generation()]
                        }
                        Reason::Terminated => [0, 3, 0, 0, instance.id.generation()],
                        Reason::BudgetExpired => [0, 4, 0, 0, instance.id.generation()],
                        Reason::CreationFailed(_) => [2, 0, 0, 0, 0],
                    },
                }
            }
            4 => {
                if argument != 0 {
                    return [1, 0, 0, 0, 0];
                }
                let Some(instance) = entry.instance.as_ref() else {
                    return [2, 0, 0, 0, 0];
                };
                // Stop admission before asking the process owner to terminate.
                crate::ipc::discard(&instance.endpoint);
                if registry.completion(instance.id).is_ok() {
                    return [0, 0, 0, 0, 0];
                }
                if registry.lifecycle_stop(instance.id).is_err() {
                    return [13, 0, 0, 0, 0];
                }
                [0, 0, 0, 0, 0]
            }
            _ => [1, 0, 0, 0, 0],
        }
    }
    fn bind_sender(
        &mut self,
        registry: &mut Registry,
        client: usize,
        token: u64,
        target_token: u64,
    ) -> [u64; 5] {
        let Some(entry) = self.entries.get(client) else {
            return [11, 0, 0, 0, 0];
        };
        if token == 0 || token != entry.token {
            return [2, 0, 0, 0, 0];
        }
        let Some(target) = entry.grant.send_to else {
            return [11, 0, 0, 0, 0];
        };
        let Some(client_id) = entry.instance.as_ref().map(|instance| instance.id) else {
            return [2, 0, 0, 0, 0];
        };
        if registry.state(client_id) != Ok(State::Admitted) {
            return [2, 0, 0, 0, 0];
        }
        let destination = &self.entries[target];
        if target_token == 0 || target_token != destination.token {
            return [2, 0, 0, 0, 0];
        }
        let Some(instance) = destination.instance.as_ref() else {
            return [2, 0, 0, 0, 0];
        };
        if registry.state(instance.id) != Ok(State::Admitted) {
            return [2, 0, 0, 0, 0];
        }
        let Ok(endpoint) = instance.endpoint.try_clone() else {
            return [12, 0, 0, 0, 0];
        };
        // Only a sender previously minted by this exact grant may be closed.
        if let Some(old) = self.entries[client].outbound.take() {
            match registry.lifecycle_close(client_id, old) {
                Ok(()) | Err(kernel_core::handles::Error::Stale) => {}
                Err(_) => return [11, 0, 0, 0, 0],
            }
        }
        match registry.lifecycle_sender(client_id, &endpoint) {
            Ok(handle) => {
                self.entries[client].outbound = Some(handle);
                [0, handle.encode(), target_token, client_id.generation(), 0]
            }
            Err(kernel_core::handles::Error::Budget) => [12, 0, 0, 0, 0],
            Err(_) => [11, 0, 0, 0, 0],
        }
    }
    pub fn retire(mut self, registry: &mut Registry, physical: &mut memory::Physical) {
        ACTIVE.store(false, Ordering::Release);
        assert!(
            !PENDING.load(Ordering::Acquire),
            "unconsumed lifecycle command"
        );
        for entry in &mut self.entries {
            if let Some(instance) = entry.instance.take() {
                assert!(
                    registry.completion(instance.id).is_ok(),
                    "supervisor left a live service"
                );
                crate::ipc::discard(&instance.endpoint);
                crate::ipc::discard(&instance.feedback);
                registry
                    .reclaim(physical, instance.id)
                    .expect("final detached service");
            }
        }
        assert_eq!(registry.state(self.supervisor), Ok(State::Completed));
        crate::ipc::deferred::poll(&[]);
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Release);
        assert!(
            self.entries.iter().all(|entry| entry.instance.is_none()),
            "live lifecycle authority abandonment"
        );
        assert!(
            !PENDING.load(Ordering::Acquire),
            "pending lifecycle authority abandonment"
        );
    }
}
