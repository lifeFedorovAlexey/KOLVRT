//! Bounded PL011 boot-console observations, not device access authority.
//!
//! The boot adapter publishes once. The pure invalidation/re-observation contract
//! does not implement hardware hotplug, driver binding, MMIO grants or reset.
use core::num::NonZeroU64;

/// Immutable native observations copied only after complete firmware validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    mmio_base: u64,
    mmio_size: u64,
    irq: u32,
    interrupt_controller: u32,
}
impl Descriptor {
    pub(crate) const fn validated(
        mmio_base: u64,
        mmio_size: u64,
        irq: u32,
        interrupt_controller: u32,
    ) -> Self {
        Self {
            mmio_base,
            mmio_size,
            irq,
            interrupt_controller,
        }
    }
    pub const fn mmio_base(self) -> u64 {
        self.mmio_base
    }
    pub const fn mmio_size(self) -> u64 {
        self.mmio_size
    }
    /// GIC INTID, already translated from the validated SPI specifier.
    pub const fn irq(self) -> u32 {
        self.irq
    }
    /// Validated firmware controller identity; not an interrupt routing grant.
    pub const fn interrupt_controller(self) -> u32 {
        self.interrupt_controller
    }
    pub const fn kind(self) -> Kind {
        Kind::Pl011
    }
    pub const fn reservation(self) -> Reservation {
        Reservation::BootConsole
    }
    pub const fn trigger(self) -> Trigger {
        Trigger::LevelHigh
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Pl011,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reservation {
    BootConsole,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    LevelHigh,
}

impl crate::platform::Description {
    /// The validated snapshot is independent of this description's mutable fields.
    pub fn boot_console(&self) -> Result<Descriptor, Error> {
        self.boot_console.ok_or(Error::UnvalidatedDescription)
    }
}

/// Identity is scoped to a trusted observation owner within one boot session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    scope: NonZeroU64,
    generation: NonZeroU64,
}
impl Identity {
    pub const fn scope(self) -> NonZeroU64 {
        self.scope
    }
    pub const fn generation(self) -> NonZeroU64 {
        self.generation
    }
}

/// Untrusted statement about an observation. A matching claim conveys no rights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Claim {
    pub scope: u64,
    pub generation: u64,
    pub mmio_base: u64,
    pub mmio_size: u64,
    pub irq: u32,
    pub interrupt_controller: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnvalidatedDescription,
    AlreadyObserved,
    StaleIdentity,
    DescriptorMismatch,
    GenerationExhausted,
}

/// One reserved console observation, with nonwrapping owner-local generations.
///
/// The trusted owner must allocate a scope unique among all owners whose claims
/// can be compared in this boot session, and must not reset/reuse it while claims
/// survive. This type does not allocate machine-global or persistent identities.
/// It deliberately provides neither Clone nor a generation-reset operation.
pub struct Console {
    scope: NonZeroU64,
    last_generation: u64,
    current: Option<(Identity, Descriptor)>,
}
impl Console {
    pub const fn new(scope: NonZeroU64) -> Self {
        Self {
            scope,
            last_generation: 0,
            current: None,
        }
    }
    pub fn observe(&mut self, descriptor: Descriptor) -> Result<Identity, Error> {
        if self.current.is_some() {
            return Err(Error::AlreadyObserved);
        }
        let generation = self
            .last_generation
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .ok_or(Error::GenerationExhausted)?;
        let identity = Identity {
            scope: self.scope,
            generation,
        };
        self.last_generation = generation.get();
        self.current = Some((identity, descriptor));
        Ok(identity)
    }
    pub fn resolve(&self, identity: Identity) -> Result<&Descriptor, Error> {
        self.current
            .as_ref()
            .filter(|(current, _)| *current == identity)
            .map(|(_, descriptor)| descriptor)
            .ok_or(Error::StaleIdentity)
    }
    pub fn invalidate(&mut self, identity: Identity) -> Result<(), Error> {
        self.resolve(identity)?;
        self.current = None;
        Ok(())
    }
    pub fn claim(&self, identity: Identity) -> Result<Claim, Error> {
        let descriptor = self.resolve(identity)?;
        Ok(Claim {
            scope: identity.scope.get(),
            generation: identity.generation.get(),
            mmio_base: descriptor.mmio_base,
            mmio_size: descriptor.mmio_size,
            irq: descriptor.irq,
            interrupt_controller: descriptor.interrupt_controller,
        })
    }
    pub fn verify(&self, claim: Claim) -> Result<&Descriptor, Error> {
        let identity = Identity {
            scope: NonZeroU64::new(claim.scope).ok_or(Error::StaleIdentity)?,
            generation: NonZeroU64::new(claim.generation).ok_or(Error::StaleIdentity)?,
        };
        let descriptor = self.resolve(identity)?;
        if claim.mmio_base != descriptor.mmio_base
            || claim.mmio_size != descriptor.mmio_size
            || claim.irq != descriptor.irq
            || claim.interrupt_controller != descriptor.interrupt_controller
        {
            return Err(Error::DescriptorMismatch);
        }
        Ok(descriptor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhausted_sequence_never_wraps_or_republishes() {
        let descriptor =
            crate::platform::discover(include_bytes!("../../../research/fixtures/virt-10.1.dtb"))
                .unwrap()
                .boot_console()
                .unwrap();
        let mut owner = Console::new(NonZeroU64::new(1).unwrap());
        owner.last_generation = u64::MAX - 1;
        let last = owner.observe(descriptor).unwrap();
        assert_eq!(last.generation().get(), u64::MAX);
        owner.invalidate(last).unwrap();
        for _ in 0..2 {
            assert_eq!(owner.observe(descriptor), Err(Error::GenerationExhausted));
            assert_eq!(owner.resolve(last), Err(Error::StaleIdentity));
            assert!(owner.current.is_none());
            assert_eq!(owner.last_generation, u64::MAX);
        }
    }
}
