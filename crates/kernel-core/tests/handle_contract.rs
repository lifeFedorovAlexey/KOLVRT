//! Process-local handle contract / LAW-009, LAW-013: admission and retention.
use kernel_core::{
    handles::{CreationError, Error, Handle, Kind, Namespace, Rights},
    process::{Completion, ProcessId, Reason, Table},
    wait::SharedEvent,
};

fn owners() -> (ProcessId, ProcessId) {
    let mut processes = Table::<2>::new();
    (
        processes.reserve(0..1).unwrap(),
        processes.reserve(1..2).unwrap(),
    )
}

fn create<const N: usize, const L: u64>(
    namespace: &mut Namespace<N, L>,
    owner: ProcessId,
    rights: Rights,
) -> Handle {
    namespace
        .create_completion_with_rights(
            owner,
            Completion {
                id: owner,
                reason: Reason::Exited(73),
            },
            rights,
            |_| Ok::<_, ()>(()),
        )
        .unwrap()
}

type NamespaceState = (Option<ProcessId>, usize, Vec<(u64, Option<Kind>)>);

fn state<const N: usize, const L: u64>(namespace: &Namespace<N, L>) -> NamespaceState {
    (
        namespace.owner(),
        namespace.live(),
        (0..N)
            .map(|slot| namespace.slot_state(slot).unwrap())
            .collect(),
    )
}

#[test]
fn all_rights_combinations_gate_lookup_and_transfer_without_escalation() {
    // ADR-0022 extends the provisional rights set; pin its wire bits while
    // enumerating all 64 held/requested combinations, including REVOKE.
    const KNOWN_BITS: u8 = 7;
    assert_eq!(Rights::SEND.bits(), 1);
    assert_eq!(Rights::TRANSFER.bits(), 2);
    assert_eq!(Rights::REVOKE.bits(), 4);
    assert_eq!(Rights::KNOWN.bits(), KNOWN_BITS);
    let (sender_owner, receiver_owner) = owners();
    for (kind, grant_bits) in [(Kind::Event, KNOWN_BITS), (Kind::Completion, 3)] {
        for held_bits in 0u8..=grant_bits {
            let held = Rights::from_bits(held_bits).unwrap();
            for requested_bits in 0u8..=KNOWN_BITS {
                let requested = Rights::from_bits(requested_bits).unwrap();
                let mut sender = Namespace::<1>::new();
                let mut receiver = Namespace::<1>::new();
                sender.bind(sender_owner).unwrap();
                receiver.bind(receiver_owner).unwrap();
                let handle = if kind == Kind::Event {
                    sender
                        .create_event_with_rights(
                            sender_owner,
                            SharedEvent::try_new().unwrap(),
                            held,
                            |_| Ok::<_, ()>(()),
                        )
                        .unwrap()
                } else {
                    create(&mut sender, sender_owner, held)
                };
                let target = sender.lookup(sender_owner, handle, kind).unwrap().target();
                let subset = requested_bits & !held_bits == 0;
                let lookup = sender.lookup_with_rights(sender_owner, handle, kind, requested);
                if subset {
                    assert_eq!(lookup.unwrap().rights(), held);
                } else {
                    assert_eq!(lookup.err(), Some(Error::Rights));
                }
                let before = (state(&sender), state(&receiver));
                let result = sender.transfer(
                    sender_owner,
                    handle,
                    &mut receiver,
                    receiver_owner,
                    requested,
                );
                if held_bits & 2 != 0 && subset {
                    let delegated = result.unwrap();
                    let received = receiver.lookup(receiver_owner, delegated, kind).unwrap();
                    assert_eq!(received.rights(), requested);
                    assert_eq!(received.target(), target);
                    if kind == Kind::Completion {
                        assert_eq!(
                            received.completion(),
                            Some(Completion {
                                id: sender_owner,
                                reason: Reason::Exited(73)
                            })
                        );
                    } else {
                        assert!(received.event().is_some());
                        assert_eq!(
                            received.signal(),
                            if requested_bits & 1 != 0 {
                                Ok(true)
                            } else {
                                Err(Error::Rights)
                            }
                        );
                    }
                    assert_eq!(state(&sender), before.0);
                    assert_eq!(receiver.live(), 1);
                    let accepted = received.retain().unwrap();
                    sender.retire(sender_owner).unwrap();
                    receiver.retire(receiver_owner).unwrap();
                    assert_eq!(accepted.target(), target);
                    assert_eq!(accepted.rights(), requested);
                    if kind == Kind::Completion {
                        assert_eq!(accepted.completion().unwrap().reason, Reason::Exited(73));
                    } else {
                        assert_eq!(
                            accepted.event().unwrap().register(),
                            requested_bits & 1 == 0
                        );
                        assert!(!accepted.event().unwrap().consume());
                    }
                } else {
                    assert_eq!(
                        result,
                        Err(Error::Rights),
                        "held={held_bits} requested={requested_bits}"
                    );
                    assert_eq!((state(&sender), state(&receiver)), before);
                }
            }
        }
    }
    for unknown in (KNOWN_BITS + 1)..=255 {
        assert_eq!(Rights::from_bits(unknown), None);
    }
}

#[test]
fn rejected_transfer_never_changes_owner_generation_or_live_entries() {
    let (a, b) = owners();
    let mut sender = Namespace::<1>::new();
    let mut receiver = Namespace::<1, 1>::new();
    sender.bind(a).unwrap();
    let source = create(&mut sender, a, Rights::from_bits(3).unwrap());
    let before = (state(&sender), state(&receiver));
    assert_eq!(
        sender.transfer(a, source, &mut receiver, b, Rights::SEND),
        Err(Error::Inactive)
    );
    assert_eq!((state(&sender), state(&receiver)), before);
    receiver.bind(b).unwrap();
    let before = (state(&sender), state(&receiver));
    for (caller, handle, receiver_owner, error) in [
        (b, source, b, Error::ForeignProcess),
        (a, source, a, Error::ForeignProcess),
        (a, Handle::decode(0), b, Error::Invalid),
        (a, Handle::decode(source.encode() + 256), b, Error::Stale),
    ] {
        assert_eq!(
            sender.transfer(caller, handle, &mut receiver, receiver_owner, Rights::SEND),
            Err(error)
        );
        assert_eq!((state(&sender), state(&receiver)), before);
    }
    let full = create(&mut receiver, b, Rights::from_bits(3).unwrap());
    let before = (state(&sender), state(&receiver));
    assert_eq!(
        sender.transfer(a, source, &mut receiver, b, Rights::SEND),
        Err(Error::Capacity)
    );
    assert_eq!((state(&sender), state(&receiver)), before);
    receiver.close(b, full).unwrap();
    let before = (state(&sender), state(&receiver));
    assert_eq!(
        sender.transfer(a, source, &mut receiver, b, Rights::SEND),
        Err(Error::GenerationExhausted)
    );
    assert_eq!((state(&sender), state(&receiver)), before);
    assert_eq!(
        sender.lookup(a, source, Kind::Completion).unwrap().rights(),
        Rights::from_bits(3).unwrap()
    );
}

#[test]
fn exhausted_slot_does_not_hide_other_vacancies_or_reset_after_rebinding() {
    let (a, b) = owners();
    let mut namespace = Namespace::<2, 1>::new();
    namespace.bind(a).unwrap();
    let mut exposed = None;
    assert_eq!(
        namespace.create_completion(
            a,
            Completion {
                id: a,
                reason: Reason::Exited(0)
            },
            |h| {
                exposed = Some(h);
                Err("publication failed")
            }
        ),
        Err(CreationError::Publication("publication failed"))
    );
    assert_eq!(namespace.slot_state(0), Some((1, None)));
    assert_eq!(namespace.live(), 0);
    let next = create(&mut namespace, a, Rights::from_bits(3).unwrap());
    assert_eq!(next.encode(), 257); // second slot, generation one
    assert_eq!(
        namespace
            .lookup(a, exposed.unwrap(), Kind::Completion)
            .err(),
        Some(Error::Stale)
    );
    namespace.retire(a).unwrap();
    namespace.bind(b).unwrap();
    assert_eq!(
        namespace
            .create_completion(
                b,
                Completion {
                    id: b,
                    reason: Reason::Exited(0)
                },
                |_| { panic!("exhaustion must reject before publication") }
            )
            .err(),
        Some(CreationError::<()>::Handle(Error::GenerationExhausted))
    );
    assert_eq!(namespace.slot_state(0), Some((1, None)));
    assert_eq!(namespace.slot_state(1), Some((1, None)));
    assert_eq!(namespace.live(), 0);
}
