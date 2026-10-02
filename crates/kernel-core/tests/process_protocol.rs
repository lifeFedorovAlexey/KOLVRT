use kernel_core::process::{Completion, CreationStep, Error, ProcessId, Reason, State, Table};

#[test]
fn reservation_validates_cpu_range_and_uses_only_its_slots() {
    let mut table = Table::<3>::default();

    assert_eq!(table.reserve(1..1), Err(Error::ForeignCpu));
    assert_eq!(table.reserve(2..1), Err(Error::ForeignCpu));
    assert_eq!(table.reserve(0..4), Err(Error::ForeignCpu));
    assert_eq!(
        table.reserve(usize::MAX..usize::MAX),
        Err(Error::ForeignCpu)
    );
    assert_eq!(table.live(), 0);

    let first = table.reserve(1..3).unwrap();
    let second = table.reserve(1..3).unwrap();
    assert_eq!((first.slot(), second.slot()), (1, 2));
    assert_eq!(table.state(first), Ok(State::Creating));
    assert_eq!(table.state(second), Ok(State::Creating));
    assert_eq!(table.live(), 2);
    assert_eq!(table.reserve(1..3), Err(Error::Capacity));

    let outside_range = table.reserve(0..1).unwrap();
    assert_eq!(outside_range.slot(), 0);
    assert_eq!(table.live(), 3);
}

#[test]
fn rollback_is_creation_only_and_never_publishes_a_completion() {
    let mut table = Table::<1>::new();
    let id = table.reserve(0..1).unwrap();
    let rollback = table
        .rollback(id, CreationStep::Commit)
        .expect("creation can be rolled back");

    assert_eq!(
        rollback,
        Completion {
            id,
            reason: Reason::CreationFailed(CreationStep::Commit)
        }
    );
    assert_eq!(table.state(id), Err(Error::Stale));
    assert_eq!(table.completion(id), Err(Error::Stale));
    assert_eq!(table.live(), 0);
    assert_eq!(table.rollback(id, CreationStep::Slot), Err(Error::Stale));

    let next = table.reserve(0..1).unwrap();
    assert!(next.generation() > id.generation());
    assert_eq!(table.prepared(next), Ok(()));
    assert_eq!(
        table.rollback(next, CreationStep::Space),
        Err(Error::Transition)
    );
    assert_eq!(table.state(next), Ok(State::Prepared));
}

#[test]
fn completion_reason_validation_and_reclamation_follow_terminal_state() {
    let reasons = [
        Reason::Exited(u64::MAX),
        Reason::Faulted {
            class: u64::MAX,
            address: usize::MAX,
        },
        Reason::BudgetExpired,
    ];

    for reason in reasons {
        let mut table = Table::<1>::new();
        let id = table.reserve(0..1).unwrap();
        assert_eq!(table.completion(id), Err(Error::Transition));
        assert_eq!(table.complete(id, reason, true), Err(Error::Transition));
        assert_eq!(table.reclaim(id, true), Err(Error::Transition));
        table.prepared(id).unwrap();
        table.start(id).unwrap();
        assert_eq!(table.complete(id, reason, false), Err(Error::NotQuiescent));
        assert_eq!(table.state(id), Ok(State::Admitted));
        table.complete(id, reason, true).unwrap();

        let expected = Completion { id, reason };
        assert_eq!(table.completion(id), Ok(expected));
        assert_eq!(table.validate_completion(expected), Ok(()));
        assert_eq!(
            table.validate_completion(Completion {
                id,
                reason: Reason::Exited(0),
            }),
            Err(Error::Stale)
        );
        assert_eq!(
            table.complete(id, Reason::CreationFailed(CreationStep::Frames), true),
            Err(Error::Transition)
        );
        assert_eq!(table.reclaim(id, false), Err(Error::NotQuiescent));
        table.reclaim(id, true).unwrap();
        assert_eq!(table.state(id), Ok(State::Reclaiming));
        assert_eq!(table.completion(id), Err(Error::Transition));
        table.released(id).unwrap();
        assert_eq!(table.state(id), Err(Error::Stale));
        assert_eq!(table.live(), 0);
    }
}

#[test]
fn process_id_exposes_identity_without_weakening_generation_checks() {
    let mut table = Table::<2>::new();
    let id = table.reserve(1..2).unwrap();
    assert_eq!(id.slot(), 1);
    assert_eq!(id.generation(), 1);

    let id_type_is_copy = |value: ProcessId| value;
    assert_eq!(id_type_is_copy(id), id);
    assert_eq!(table.state(id), Ok(State::Creating));
}
