use core::num::NonZeroU64;
use kernel_core::{
    device::{Console, Descriptor, Error, Kind, Reservation, Trigger},
    platform,
};
const DTB: &[u8] = include_bytes!("../../../research/fixtures/virt-10.1.dtb");
fn descriptor() -> Descriptor {
    platform::discover(DTB).unwrap().boot_console().unwrap()
}
fn owner(scope: u64) -> Console {
    Console::new(NonZeroU64::new(scope).unwrap())
}

#[test]
fn parser_snapshot_is_immutable_and_reserved_without_grant() {
    let mut parsed = platform::discover(DTB).unwrap();
    let descriptor = parsed.boot_console().unwrap();
    assert_eq!(descriptor.mmio_base(), parsed.uart.base);
    assert_eq!(descriptor.mmio_size(), parsed.uart.size);
    assert_eq!(descriptor.irq(), parsed.uart_irq);
    assert_eq!(
        descriptor.interrupt_controller(),
        parsed.interrupt_controller
    );
    assert_eq!(descriptor.kind(), Kind::Pl011);
    assert_eq!(descriptor.reservation(), Reservation::BootConsole);
    assert_eq!(descriptor.trigger(), Trigger::LevelHigh);
    parsed.uart.base = 0;
    parsed.uart.size = u64::MAX;
    parsed.uart_irq = 0;
    parsed.interrupt_controller = 0;
    assert_eq!(parsed.boot_console().unwrap(), descriptor);
}
#[test]
fn forged_observations_are_rejected_without_mutating_current_identity() {
    let mut console = owner(1);
    let id = console.observe(descriptor()).unwrap();
    let claim = console.claim(id).unwrap();
    for forged in [
        kernel_core::device::Claim {
            mmio_base: claim.mmio_base + 4096,
            ..claim
        },
        kernel_core::device::Claim {
            mmio_size: claim.mmio_size + 4096,
            ..claim
        },
        kernel_core::device::Claim {
            irq: claim.irq + 1,
            ..claim
        },
        kernel_core::device::Claim {
            interrupt_controller: claim.interrupt_controller + 1,
            ..claim
        },
    ] {
        assert_eq!(console.verify(forged), Err(Error::DescriptorMismatch));
        assert_eq!(console.claim(id).unwrap(), claim);
    }
    for forged in [
        kernel_core::device::Claim {
            generation: 0,
            ..claim
        },
        kernel_core::device::Claim {
            generation: claim.generation + 1,
            ..claim
        },
        kernel_core::device::Claim { scope: 0, ..claim },
        kernel_core::device::Claim { scope: 2, ..claim },
    ] {
        assert_eq!(console.verify(forged), Err(Error::StaleIdentity));
    }
    assert_eq!(console.observe(descriptor()), Err(Error::AlreadyObserved));
    assert_eq!(console.claim(id).unwrap(), claim);
    assert_eq!(*console.verify(claim).unwrap(), descriptor());
}
#[test]
fn identical_descriptor_cannot_resurrect_invalidated_identity() {
    let mut console = owner(1);
    let first = console.observe(descriptor()).unwrap();
    let stale = console.claim(first).unwrap();
    console.invalidate(first).unwrap();
    assert_eq!(console.verify(stale), Err(Error::StaleIdentity));
    let replacement = console.observe(descriptor()).unwrap();
    assert_ne!(first, replacement);
    assert_eq!(replacement.generation().get(), first.generation().get() + 1);
    assert_eq!(console.resolve(first), Err(Error::StaleIdentity));
    assert_eq!(console.verify(stale), Err(Error::StaleIdentity));
    assert_eq!(console.invalidate(first), Err(Error::StaleIdentity));
    assert_eq!(*console.resolve(replacement).unwrap(), descriptor());
}
#[test]
fn different_trusted_scopes_reject_cross_owner_identity_and_claim() {
    let mut first = owner(1);
    let mut second = owner(2);
    let a = first.observe(descriptor()).unwrap();
    let b = second.observe(descriptor()).unwrap();
    assert_eq!(a.generation(), b.generation());
    assert_ne!(a, b);
    assert_eq!(second.resolve(a), Err(Error::StaleIdentity));
    assert_eq!(
        second.verify(first.claim(a).unwrap()),
        Err(Error::StaleIdentity)
    );
    assert_eq!(second.invalidate(a), Err(Error::StaleIdentity));
    assert!(second.resolve(b).is_ok());
}
