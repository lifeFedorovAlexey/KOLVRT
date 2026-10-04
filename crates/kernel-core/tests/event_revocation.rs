//! ADR-0022: synchronous admission versus target-wide revocation; host evidence.
use kernel_core::{
    handles::{CreationError, Error, Kind, Namespace, Rights},
    process::{Completion, Reason, Table},
    wait::{Event, SharedEvent, SignalError},
};
use std::sync::{Arc, Barrier};

#[test]
fn every_completion_revoke_grant_rejects_before_generation_or_publication() {
    let mut processes = Table::<1>::new();
    let owner = processes.reserve(0..1).unwrap();
    let mut namespace = Namespace::<1, 1>::new();
    namespace.bind(owner).unwrap();
    for bits in 4..=7 {
        assert_eq!(
            namespace.create_completion_with_rights(
                owner,
                Completion {
                    id: owner,
                    reason: Reason::Exited(1)
                },
                Rights::from_bits(bits).unwrap(),
                |_| -> Result<(), ()> { panic!("invalid grant must reject before publication") }
            ),
            Err(CreationError::Handle(Error::Rights))
        );
        assert_eq!(namespace.slot_state(0), Some((0, None)));
        assert_eq!(namespace.live(), 0);
        assert_eq!(namespace.owner(), Some(owner));
    }
    let handle = namespace
        .create_completion(
            owner,
            Completion {
                id: owner,
                reason: Reason::Exited(1),
            },
            |_| Ok::<_, ()>(()),
        )
        .unwrap();
    assert_eq!(handle.encode(), 256);
    assert_eq!(
        namespace
            .lookup(owner, handle, Kind::Completion)
            .unwrap()
            .rights(),
        Rights::NONE
    );
}

#[test]
fn default_grants_are_minimal_and_denied_effects_leave_the_latch_unchanged() {
    let mut processes = Table::<1>::new();
    let owner = processes.reserve(0..1).unwrap();
    let mut namespace = Namespace::<3>::new();
    namespace.bind(owner).unwrap();
    let event = namespace
        .create_event(owner, SharedEvent::try_new().unwrap(), |_| Ok::<_, ()>(()))
        .unwrap();
    let completion = namespace
        .create_completion(
            owner,
            Completion {
                id: owner,
                reason: Reason::Exited(1),
            },
            |_| Ok::<_, ()>(()),
        )
        .unwrap();
    let no_send = namespace
        .create_event_with_rights(
            owner,
            SharedEvent::try_new().unwrap(),
            Rights::REVOKE,
            |_| Ok::<_, ()>(()),
        )
        .unwrap();
    assert_eq!(
        namespace
            .lookup(owner, event, Kind::Event)
            .unwrap()
            .rights(),
        Rights::SEND
    );
    assert_eq!(
        namespace
            .lookup(owner, completion, Kind::Completion)
            .unwrap()
            .rights(),
        Rights::NONE
    );
    assert_eq!(namespace.revoke(owner, event), Err(Error::Rights));
    let reference = namespace.lookup(owner, event, Kind::Event).unwrap();
    assert_eq!(reference.signal(), Ok(true));
    assert!(!reference.event().unwrap().register());
    let reference = namespace.lookup(owner, no_send, Kind::Event).unwrap();
    assert_eq!(reference.signal(), Err(Error::Rights));
    assert!(reference.event().unwrap().register());
    assert!(!reference.event().unwrap().consume());
    assert_eq!(reference.revoke(), Ok(true));
    assert_eq!(reference.signal(), Err(Error::Rights));
}

#[test]
fn delegated_revoke_preserves_pending_work_retention_and_permanent_revocation() {
    let mut processes = Table::<2>::new();
    let a = processes.reserve(0..1).unwrap();
    let b = processes.reserve(1..2).unwrap();
    let mut source = Namespace::<1>::new();
    let mut receiver = Namespace::<1>::new();
    source.bind(a).unwrap();
    receiver.bind(b).unwrap();
    let handle = source
        .create_event_with_rights(a, SharedEvent::try_new().unwrap(), Rights::ALL, |_| {
            Ok::<_, ()>(())
        })
        .unwrap();
    let delegated = source
        .transfer(a, handle, &mut receiver, b, Rights::REVOKE)
        .unwrap();
    let accepted = source
        .lookup(a, handle, Kind::Event)
        .unwrap()
        .retain()
        .unwrap();
    let target = accepted.target();
    assert_eq!(
        source.lookup(a, handle, Kind::Event).unwrap().signal(),
        Ok(true)
    );
    assert_eq!(receiver.revoke(b, delegated), Ok(true));
    assert_eq!(
        source.lookup(a, handle, Kind::Event).unwrap().signal(),
        Err(Error::Revoked)
    );
    source.close(a, handle).unwrap();
    receiver.retire(b).unwrap();
    assert_eq!(accepted.target(), target);
    assert!(!accepted.event().unwrap().register()); // committed pending effect survives
    assert!(!accepted.event().unwrap().consume()); // consumed exactly once
    assert_eq!(
        accepted.event().unwrap().admit_signal(),
        Err(SignalError::Revoked)
    );
    let fresh = source
        .create_event(a, SharedEvent::try_new().unwrap(), |_| Ok::<_, ()>(()))
        .unwrap();
    let fresh = source.lookup(a, fresh, Kind::Event).unwrap();
    assert_ne!(fresh.target(), target);
    assert_eq!(fresh.signal(), Ok(true));
    assert!(!fresh.event().unwrap().register());
    assert_eq!(
        accepted.event().unwrap().admit_signal(),
        Err(SignalError::Revoked)
    );
}

#[test]
fn signal_racing_revoke_has_one_ordered_admission_and_never_loses_an_accepted_effect() {
    for _ in 0..128 {
        let event = Arc::new(Event::new());
        let barrier = Arc::new(Barrier::new(2));
        let publisher = event.clone();
        let ready = barrier.clone();
        let signal = std::thread::spawn(move || {
            ready.wait();
            publisher.admit_signal()
        });
        barrier.wait();
        assert!(event.revoke());
        let result = signal.join().unwrap();
        assert!(matches!(result, Ok(true) | Err(SignalError::Revoked)));
        assert_eq!(event.register(), result.is_err());
        assert!(!event.consume());
        assert_eq!(event.admit_signal(), Err(SignalError::Revoked));
        assert!(!event.revoke());

        let owner = SharedEvent::try_new().unwrap();
        let publisher = owner.try_clone().unwrap();
        let ready = barrier.clone();
        let signal = std::thread::spawn(move || {
            ready.wait();
            publisher.admit_signal()
        });
        barrier.wait();
        assert!(owner.revoke());
        let result = signal.join().unwrap();
        assert!(matches!(result, Ok(true) | Err(SignalError::Revoked)));
        assert_eq!(owner.register(), result.is_err());
        assert!(!owner.consume());
        assert_eq!(owner.admit_signal(), Err(SignalError::Revoked));
        assert!(!owner.revoke());
    }
}
