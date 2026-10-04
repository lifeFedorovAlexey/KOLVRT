//! Process-local references to two existing native primitives. No authority.
use crate::{
    process::{Completion, ProcessId},
    wait::{SharedEvent, SignalError},
};
pub const MAX_GENERATION: u64 = u64::MAX >> 8;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle(u64);
impl Handle {
    pub const fn decode(raw: u64) -> Self {
        Self(raw)
    }
    pub const fn encode(self) -> u64 {
        self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Event,
    Completion,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rights(u8);
impl Rights {
    pub const NONE: Self = Self(0);
    pub const SEND: Self = Self(1);
    pub const TRANSFER: Self = Self(2);
    pub const REVOKE: Self = Self(4);
    pub const KNOWN: Self = Self(Self::SEND.0 | Self::TRANSFER.0 | Self::REVOKE.0);
    pub const ALL: Self = Self::KNOWN;
    pub const fn from_bits(bits: u8) -> Option<Self> {
        if bits & !Self::KNOWN.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }
    pub const fn bits(self) -> u8 {
        self.0
    }
    const fn contains(self, required: Self) -> bool {
        self.0 & required.0 == required.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Stale,
    WrongType,
    ForeignProcess,
    Inactive,
    Capacity,
    GenerationExhausted,
    Rights,
    ReferenceExhausted,
    Revoked,
}
#[derive(Debug, PartialEq, Eq)]
pub enum CreationError<E> {
    Handle(Error),
    Publication(E),
}
/// Internal immutable target identity; not serialized to EL0 and not authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetId {
    creator: ProcessId,
    slot: usize,
    generation: u64,
}
// Narrow concrete primitive storage, not a generic invocation/object hierarchy.
enum Primitive {
    Event(SharedEvent),
    Completion(Completion),
}
struct Resource {
    identity: TargetId,
    rights: Rights,
    value: Primitive,
}
impl Resource {
    fn kind(&self) -> Kind {
        match self.value {
            Primitive::Event(_) => Kind::Event,
            Primitive::Completion(_) => Kind::Completion,
        }
    }
    fn delegated(&self, rights: Rights) -> Result<Self, Error> {
        let value = match &self.value {
            Primitive::Event(event) => {
                Primitive::Event(event.try_clone().ok_or(Error::ReferenceExhausted)?)
            }
            Primitive::Completion(completion) => Primitive::Completion(*completion),
        };
        Ok(Self {
            identity: self.identity,
            rights,
            value,
        })
    }
}
struct Slot {
    generation: u64,
    resource: Option<Resource>,
}
impl Slot {
    const EMPTY: Self = Self {
        generation: 0,
        resource: None,
    };
}
/// Linear owner; neither cloned nor copied. Generations persist across rebinding.
pub struct Namespace<const N: usize, const LIMIT: u64 = MAX_GENERATION> {
    owner: Option<ProcessId>,
    slots: [Slot; N],
}
/// Synchronous accepted borrow keeps the concrete resource alive. Close/retire
/// require an exclusive namespace borrow and cannot race this retained reference.
/// ```compile_fail
/// use kernel_core::{handles::{Namespace,Handle,Kind},process::ProcessId};
/// fn close_during_lookup(n: &mut Namespace<8>, caller: ProcessId, h: Handle) {
///     let reference = n.lookup(caller,h,Kind::Event).unwrap();
///     n.close(caller,h).unwrap();
///     reference.event().unwrap().signal();
/// }
/// ```
pub struct Retained<'a> {
    resource: &'a Resource,
}
/// Owned reference for accepted work that outlives the lookup call or source
/// handle. Dropping the final such reference releases its bounded resource slot.
pub struct OwnedRetained {
    resource: Resource,
}
impl Retained<'_> {
    pub fn target(&self) -> TargetId {
        self.resource.identity
    }
    pub fn event(&self) -> Option<&SharedEvent> {
        match &self.resource.value {
            Primitive::Event(e) => Some(e),
            _ => None,
        }
    }
    pub fn completion(&self) -> Option<Completion> {
        match &self.resource.value {
            Primitive::Completion(c) => Some(*c),
            _ => None,
        }
    }
    /// Revoke future admissions for a shared Event target. Existing accepted
    /// references and already-pending notification remain valid.
    pub fn revoke(&self) -> Result<bool, Error> {
        if !self.resource.rights.contains(Rights::REVOKE) {
            return Err(Error::Rights);
        }
        match &self.resource.value {
            Primitive::Event(event) => Ok(event.revoke()),
            Primitive::Completion(_) => Err(Error::WrongType),
        }
    }
    pub fn signal(&self) -> Result<bool, Error> {
        if !self.resource.rights.contains(Rights::SEND) {
            return Err(Error::Rights);
        }
        match &self.resource.value {
            Primitive::Event(event) => event
                .admit_signal()
                .map_err(|SignalError::Revoked| Error::Revoked),
            Primitive::Completion(_) => Err(Error::WrongType),
        }
    }
    pub fn rights(&self) -> Rights {
        self.resource.rights
    }
    pub fn retain(&self) -> Result<OwnedRetained, Error> {
        Ok(OwnedRetained {
            resource: self.resource.delegated(self.resource.rights)?,
        })
    }
}
impl OwnedRetained {
    pub fn target(&self) -> TargetId {
        self.resource.identity
    }
    pub fn rights(&self) -> Rights {
        self.resource.rights
    }
    pub fn event(&self) -> Option<&SharedEvent> {
        match &self.resource.value {
            Primitive::Event(event) => Some(event),
            _ => None,
        }
    }
    pub fn completion(&self) -> Option<Completion> {
        match &self.resource.value {
            Primitive::Completion(completion) => Some(*completion),
            _ => None,
        }
    }
}
impl<const N: usize, const LIMIT: u64> Default for Namespace<N, LIMIT> {
    fn default() -> Self {
        Self::new()
    }
}
impl<const N: usize, const LIMIT: u64> Namespace<N, LIMIT> {
    pub const fn new() -> Self {
        assert!(N > 0 && N <= 256 && LIMIT > 0 && LIMIT <= MAX_GENERATION);
        Self {
            owner: None,
            slots: [const { Slot::EMPTY }; N],
        }
    }
    pub fn bind(&mut self, owner: ProcessId) -> Result<(), Error> {
        if self.owner.is_some() || self.live() != 0 {
            return Err(Error::Inactive);
        }
        self.owner = Some(owner);
        Ok(())
    }
    pub fn owner(&self) -> Option<ProcessId> {
        self.owner
    }
    fn context(&self, caller: ProcessId) -> Result<(), Error> {
        match self.owner {
            None => Err(Error::Inactive),
            Some(owner) if owner != caller && !cfg!(feature = "handle-owner-negative") => {
                Err(Error::ForeignProcess)
            }
            _ => Ok(()),
        }
    }
    pub fn live(&self) -> usize {
        self.slots.iter().filter(|s| s.resource.is_some()).count()
    }
    pub fn capacity(&self) -> usize {
        N
    }
    pub fn slot_state(&self, slot: usize) -> Option<(u64, Option<Kind>)> {
        self.slots
            .get(slot)
            .map(|s| (s.generation, s.resource.as_ref().map(Resource::kind)))
    }
    fn index(&self, caller: ProcessId, handle: Handle) -> Result<usize, Error> {
        self.context(caller)?;
        let index = (handle.0 & 255) as usize;
        let generation = handle.0 >> 8;
        if generation == 0 {
            return Err(Error::Invalid);
        }
        let slot = self.slots.get(index).ok_or(Error::Invalid)?;
        if !cfg!(feature = "handle-generation-negative") && slot.generation != generation {
            return Err(Error::Stale);
        }
        if slot.resource.is_none() {
            return Err(Error::Stale);
        }
        Ok(index)
    }
    pub fn lookup(
        &self,
        caller: ProcessId,
        handle: Handle,
        expected: Kind,
    ) -> Result<Retained<'_>, Error> {
        self.lookup_with_rights(caller, handle, expected, Rights::NONE)
    }
    pub fn lookup_with_rights(
        &self,
        caller: ProcessId,
        handle: Handle,
        expected: Kind,
        required: Rights,
    ) -> Result<Retained<'_>, Error> {
        let index = self.index(caller, handle)?;
        let resource = self.slots[index].resource.as_ref().ok_or(Error::Stale)?;
        if !cfg!(feature = "handle-type-negative") && resource.kind() != expected {
            return Err(Error::WrongType);
        }
        if !resource.rights.contains(required) {
            return Err(Error::Rights);
        }
        Ok(Retained { resource })
    }
    /// Atomically validates sender authority, receiver ownership and capacity,
    /// then commits one new receiver-local generation. The source remains live;
    /// both slots retain the same immutable target identity and shared event.
    pub fn transfer<const M: usize, const OTHER_LIMIT: u64>(
        &self,
        caller: ProcessId,
        handle: Handle,
        receiver: &mut Namespace<M, OTHER_LIMIT>,
        receiver_owner: ProcessId,
        requested: Rights,
    ) -> Result<Handle, Error> {
        let source_index = self.index(caller, handle)?;
        let source = self.slots[source_index]
            .resource
            .as_ref()
            .ok_or(Error::Stale)?;
        let unauthorized = Rights::from_bits(requested.bits()).is_none()
            || !source.rights.contains(Rights::TRANSFER)
            || !source.rights.contains(requested);
        if unauthorized && !cfg!(feature = "handle-transfer-rights-negative") {
            return Err(Error::Rights);
        }
        receiver.context(receiver_owner)?;
        let target = receiver
            .slots
            .iter()
            .position(|slot| slot.resource.is_none() && slot.generation < OTHER_LIMIT)
            .ok_or_else(|| {
                if receiver.slots.iter().any(|slot| slot.resource.is_none()) {
                    Error::GenerationExhausted
                } else {
                    Error::Capacity
                }
            })?;
        let generation = receiver.slots[target]
            .generation
            .checked_add(1)
            .filter(|next| *next <= OTHER_LIMIT)
            .ok_or(Error::GenerationExhausted)?;
        let delegated = source.delegated(requested)?;
        receiver.slots[target].generation = generation;
        receiver.slots[target].resource = Some(delegated);
        Ok(Handle((generation << 8) | target as u64))
    }
    pub fn close(&mut self, caller: ProcessId, handle: Handle) -> Result<(), Error> {
        let index = self.index(caller, handle)?;
        // Retained borrows have ended before this exclusive operation.
        self.slots[index].resource = None;
        Ok(())
    }
    /// Revoke an Event for every delegated alias. Admission races with this
    /// operation are linearized in the target's atomic state; close stays local.
    pub fn revoke(&self, caller: ProcessId, handle: Handle) -> Result<bool, Error> {
        self.lookup_with_rights(caller, handle, Kind::Event, Rights::REVOKE)?
            .revoke()
    }
    pub fn retire(&mut self, caller: ProcessId) -> Result<usize, Error> {
        self.context(caller)?;
        let count = self.live();
        for slot in &mut self.slots {
            slot.resource = None;
        }
        self.owner = None;
        Ok(count)
    }
    fn create<E>(
        &mut self,
        caller: ProcessId,
        value: Primitive,
        rights: Rights,
        publish: impl FnOnce(Handle) -> Result<(), E>,
    ) -> Result<Handle, CreationError<E>> {
        self.context(caller).map_err(CreationError::Handle)?;
        let index = self
            .slots
            .iter()
            .position(|s| s.resource.is_none() && s.generation < LIMIT)
            .ok_or_else(|| {
                CreationError::Handle(if self.slots.iter().any(|s| s.resource.is_none()) {
                    Error::GenerationExhausted
                } else {
                    Error::Capacity
                })
            })?;
        let slot = &mut self.slots[index];
        // Reserve/burn identity before publication. Failure never resurrects it.
        if !cfg!(feature = "handle-reuse-negative") || slot.generation == 0 {
            slot.generation = slot
                .generation
                .checked_add(1)
                .filter(|g| *g <= LIMIT)
                .ok_or(CreationError::Handle(Error::GenerationExhausted))?;
        }
        let handle = Handle((slot.generation << 8) | index as u64);
        let resource = Resource {
            identity: TargetId {
                creator: caller,
                slot: index,
                generation: slot.generation,
            },
            rights,
            value,
        };
        // Resource remains owned by this stack value until infallible commit.
        // Error or unwinding drops it; the slot stays vacant with a burned generation.
        publish(handle).map_err(CreationError::Publication)?;
        slot.resource = Some(resource);
        Ok(handle)
    }
    pub fn create_event<E>(
        &mut self,
        caller: ProcessId,
        event: SharedEvent,
        publish: impl FnOnce(Handle) -> Result<(), E>,
    ) -> Result<Handle, CreationError<E>> {
        self.create_event_with_rights(caller, event, Rights::SEND, publish)
    }
    pub fn create_event_with_rights<E>(
        &mut self,
        caller: ProcessId,
        event: SharedEvent,
        rights: Rights,
        publish: impl FnOnce(Handle) -> Result<(), E>,
    ) -> Result<Handle, CreationError<E>> {
        self.create(caller, Primitive::Event(event), rights, publish)
    }
    pub fn create_completion<E>(
        &mut self,
        caller: ProcessId,
        completion: Completion,
        publish: impl FnOnce(Handle) -> Result<(), E>,
    ) -> Result<Handle, CreationError<E>> {
        self.create_completion_with_rights(caller, completion, Rights::NONE, publish)
    }
    pub fn create_completion_with_rights<E>(
        &mut self,
        caller: ProcessId,
        completion: Completion,
        rights: Rights,
        publish: impl FnOnce(Handle) -> Result<(), E>,
    ) -> Result<Handle, CreationError<E>> {
        self.create(caller, Primitive::Completion(completion), rights, publish)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wait::SharedEvent;
    fn ids() -> (ProcessId, ProcessId) {
        let mut p = crate::process::Table::<2>::new();
        (p.reserve(0..1).unwrap(), p.reserve(1..2).unwrap())
    }
    fn event<const N: usize, const L: u64>(n: &mut Namespace<N, L>, id: ProcessId) -> Handle {
        n.create_event(id, SharedEvent::try_new().unwrap(), |_| Ok::<_, ()>(()))
            .unwrap()
    }
    #[test]
    fn lookup_close_reuse_forged_type_and_capacity() {
        let (a, b) = ids();
        let mut n = Namespace::<2>::new();
        n.bind(a).unwrap();
        assert_eq!(
            n.lookup(a, Handle::decode(0), Kind::Event).err(),
            Some(Error::Invalid)
        );
        assert_eq!(
            n.lookup(a, Handle::decode(u64::MAX), Kind::Event).err(),
            Some(Error::Invalid)
        );
        let h = event(&mut n, a);
        assert_eq!(
            n.lookup(b, h, Kind::Event).err(),
            Some(Error::ForeignProcess)
        );
        assert_eq!(
            n.lookup(a, h, Kind::Completion).err(),
            Some(Error::WrongType)
        );
        {
            let r = n.lookup(a, h, Kind::Event).unwrap();
            assert!(r.event().unwrap().signal());
            assert!(!r.event().unwrap().register());
        }
        n.close(a, h).unwrap();
        assert_eq!(n.close(a, h), Err(Error::Stale));
        let new = event(&mut n, a);
        assert_ne!(h, new);
        assert_eq!(n.lookup(a, h, Kind::Event).err(), Some(Error::Stale));
        let completion = Completion {
            id: a,
            reason: crate::process::Reason::Exited(7),
        };
        let c = n
            .create_completion(a, completion, |_| Ok::<_, ()>(()))
            .unwrap();
        assert_eq!(
            n.lookup(a, c, Kind::Completion).unwrap().completion(),
            Some(completion)
        );
        assert_eq!(
            n.create_event(a, SharedEvent::try_new().unwrap(), |_| Ok::<_, ()>(()))
                .err(),
            Some(CreationError::Handle(Error::Capacity))
        );
        assert_eq!(n.retire(a), Ok(2));
        assert_eq!(n.live(), 0);
        assert_eq!(n.lookup(a, new, Kind::Event).err(), Some(Error::Inactive));
        n.bind(b).unwrap();
        let next = event(&mut n, b);
        assert_ne!(next, new);
        assert_eq!(n.lookup(b, new, Kind::Event).err(), Some(Error::Stale));
    }
    #[test]
    fn target_identity_survives_moves_and_reuse_changes_target() {
        let (a, _) = ids();
        let mut n = Namespace::<1>::new();
        n.bind(a).unwrap();
        let h = event(&mut n, a);
        let target = n.lookup(a, h, Kind::Event).unwrap().target();
        let mut moved = n;
        assert_eq!(moved.lookup(a, h, Kind::Event).unwrap().target(), target);
        moved.close(a, h).unwrap();
        let fresh = event(&mut moved, a);
        assert_ne!(
            moved.lookup(a, fresh, Kind::Event).unwrap().target(),
            target
        );
    }
    #[test]
    fn transactional_failure_burns_generation_and_exhaustion_never_wraps() {
        let (a, _) = ids();
        let mut n = Namespace::<1, 2>::new();
        n.bind(a).unwrap();
        let mut leaked = None;
        assert_eq!(
            n.create_event(a, SharedEvent::try_new().unwrap(), |h| {
                leaked = Some(h);
                Err(9)
            }),
            Err(CreationError::Publication(9))
        );
        assert_eq!(n.live(), 0);
        let h = event(&mut n, a);
        assert_eq!(
            n.lookup(a, leaked.unwrap(), Kind::Event).err(),
            Some(Error::Stale)
        );
        n.close(a, h).unwrap();
        assert_eq!(
            n.create_event(a, SharedEvent::try_new().unwrap(), |_| Ok::<_, ()>(()))
                .err(),
            Some(CreationError::Handle(Error::GenerationExhausted))
        );
        assert_eq!(n.slot_state(0), Some((2, None)));
        assert_eq!(n.retire(a), Ok(0));
    }
    #[test]
    fn transfer_is_receiver_local_transactional_and_attenuates_rights() {
        let (a, b) = ids();
        let mut sender = Namespace::<2>::new();
        let mut receiver = Namespace::<1>::new();
        sender.bind(a).unwrap();
        receiver.bind(b).unwrap();
        let source = sender
            .create_event_with_rights(a, SharedEvent::try_new().unwrap(), Rights::ALL, |_| {
                Ok::<_, ()>(())
            })
            .unwrap();
        let full = event(&mut receiver, b);
        let sender_before = sender.live();
        let receiver_before = receiver.live();
        assert_eq!(
            sender.transfer(a, source, &mut receiver, b, Rights::SEND),
            Err(Error::Capacity)
        );
        assert_eq!(sender.live(), sender_before);
        assert_eq!(receiver.live(), receiver_before);
        assert_eq!(
            sender
                .lookup_with_rights(a, source, Kind::Event, Rights::TRANSFER)
                .unwrap()
                .rights(),
            Rights::ALL
        );
        receiver.close(b, full).unwrap();
        let delegated = sender
            .transfer(a, source, &mut receiver, b, Rights::SEND)
            .unwrap();
        assert_ne!(source, delegated);
        let sender_target = sender.lookup(a, source, Kind::Event).unwrap().target();
        let receiver_ref = receiver
            .lookup_with_rights(b, delegated, Kind::Event, Rights::SEND)
            .unwrap();
        assert_eq!(receiver_ref.target(), sender_target);
        assert_eq!(receiver_ref.rights(), Rights::SEND);
        assert_eq!(
            receiver
                .lookup_with_rights(b, delegated, Kind::Event, Rights::TRANSFER)
                .err(),
            Some(Error::Rights)
        );
        sender
            .lookup(a, source, Kind::Event)
            .unwrap()
            .event()
            .unwrap()
            .signal();
        assert!(!receiver_ref.event().unwrap().register());
        sender.close(a, source).unwrap();
        let accepted = receiver
            .lookup(b, delegated, Kind::Event)
            .unwrap()
            .retain()
            .unwrap();
        receiver.close(b, delegated).unwrap();
        assert!(accepted.event().unwrap().signal());
        assert_eq!(sender.live(), 0);
        assert_eq!(receiver.live(), 0);
        drop(accepted);
        assert_eq!(
            receiver.transfer(b, delegated, &mut sender, a, Rights::ALL),
            Err(Error::Stale)
        );
    }

    #[test]
    fn event_revocation_serializes_with_admission_and_survives_aliases() {
        let (a, b) = ids();
        let mut source = Namespace::<2>::new();
        source.bind(a).unwrap();
        let event = SharedEvent::try_new().unwrap();
        let handle = source
            .create_event_with_rights(a, event, Rights::ALL, |_| Ok::<_, ()>(()))
            .unwrap();
        let mut receiver = Namespace::<1>::new();
        receiver.bind(b).unwrap();
        let delegated = source
            .transfer(a, handle, &mut receiver, b, Rights::SEND)
            .unwrap();
        assert_eq!(source.revoke(a, handle), Ok(true));
        assert_eq!(source.revoke(a, handle), Ok(false));
        assert_eq!(
            receiver
                .lookup(b, delegated, Kind::Event)
                .unwrap()
                .event()
                .unwrap()
                .admit_signal(),
            Err(crate::wait::SignalError::Revoked)
        );
        assert_eq!(source.close(a, handle), Ok(()));
        assert_eq!(
            receiver
                .lookup(b, delegated, Kind::Event)
                .unwrap()
                .event()
                .unwrap()
                .admit_signal(),
            Err(crate::wait::SignalError::Revoked)
        );
    }

    #[test]
    fn revoke_requires_explicit_right_and_does_not_alias_close() {
        let (a, _) = ids();
        let mut source = Namespace::<2>::new();
        source.bind(a).unwrap();
        let handle = source
            .create_event(a, SharedEvent::try_new().unwrap(), |_| Ok::<_, ()>(()))
            .unwrap();
        assert_eq!(source.revoke(a, handle), Err(Error::Rights));
        {
            let event = source.lookup(a, handle, Kind::Event).unwrap();
            assert_eq!(event.signal(), Ok(true));
        }
        source.close(a, handle).unwrap();
    }
    #[test]
    fn process_local_equal_values_and_stress() {
        let (a, b) = ids();
        let mut x = Namespace::<1>::new();
        let mut y = Namespace::<1>::new();
        x.bind(a).unwrap();
        y.bind(b).unwrap();
        let ha = event(&mut x, a);
        let hb = event(&mut y, b);
        assert_eq!(ha, hb);
        assert_ne!(
            x.lookup(a, ha, Kind::Event).unwrap().target(),
            y.lookup(b, hb, Kind::Event).unwrap().target()
        );
        x.lookup(a, ha, Kind::Event)
            .unwrap()
            .event()
            .unwrap()
            .signal();
        assert!(
            y.lookup(b, ha, Kind::Event)
                .unwrap()
                .event()
                .unwrap()
                .register()
        );
        x.close(a, ha).unwrap();
        for _ in 0..4096 {
            let h = event(&mut x, a);
            x.close(a, h).unwrap();
            assert_eq!(x.lookup(a, h, Kind::Event).err(), Some(Error::Stale));
        }
        assert_eq!(x.live(), 0);
    }
}
