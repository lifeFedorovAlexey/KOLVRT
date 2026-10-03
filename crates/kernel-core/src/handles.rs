//! Process-local references to two existing native primitives. No authority.
use crate::{
    process::{Completion, ProcessId},
    wait::Event,
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
pub enum Error {
    Invalid,
    Stale,
    WrongType,
    ForeignProcess,
    Inactive,
    Capacity,
    GenerationExhausted,
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
    Event(Event),
    Completion(Completion),
}
struct Resource {
    identity: TargetId,
    value: Primitive,
}
impl Resource {
    fn kind(&self) -> Kind {
        match self.value {
            Primitive::Event(_) => Kind::Event,
            Primitive::Completion(_) => Kind::Completion,
        }
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
impl Retained<'_> {
    pub fn target(&self) -> TargetId {
        self.resource.identity
    }
    pub fn event(&self) -> Option<&Event> {
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
        let index = self.index(caller, handle)?;
        let resource = self.slots[index].resource.as_ref().ok_or(Error::Stale)?;
        if !cfg!(feature = "handle-type-negative") && resource.kind() != expected {
            return Err(Error::WrongType);
        }
        Ok(Retained { resource })
    }
    pub fn close(&mut self, caller: ProcessId, handle: Handle) -> Result<(), Error> {
        let index = self.index(caller, handle)?;
        // Retained borrows have ended before this exclusive operation.
        self.slots[index].resource = None;
        Ok(())
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
        event: Event,
        publish: impl FnOnce(Handle) -> Result<(), E>,
    ) -> Result<Handle, CreationError<E>> {
        self.create(caller, Primitive::Event(event), publish)
    }
    pub fn create_completion<E>(
        &mut self,
        caller: ProcessId,
        completion: Completion,
        publish: impl FnOnce(Handle) -> Result<(), E>,
    ) -> Result<Handle, CreationError<E>> {
        self.create(caller, Primitive::Completion(completion), publish)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ids() -> (ProcessId, ProcessId) {
        let mut p = crate::process::Table::<2>::new();
        (p.reserve(0..1).unwrap(), p.reserve(1..2).unwrap())
    }
    fn event<const N: usize, const L: u64>(n: &mut Namespace<N, L>, id: ProcessId) -> Handle {
        n.create_event(id, Event::new(), |_| Ok::<_, ()>(()))
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
            n.create_event(a, Event::new(), |_| Ok::<_, ()>(())).err(),
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
            n.create_event(a, Event::new(), |h| {
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
            n.create_event(a, Event::new(), |_| Ok::<_, ()>(())).err(),
            Some(CreationError::Handle(Error::GenerationExhausted))
        );
        assert_eq!(n.slot_state(0), Some((2, None)));
        assert_eq!(n.retire(a), Ok(0));
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
