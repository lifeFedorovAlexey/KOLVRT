use native_apps::supervision::{fresh_binding, needs_replacement};
#[test]
fn live_service_cannot_be_replaced() {
    assert_eq!(needs_replacement([0, 0, 0, 0, 0]), Ok(false));
}
#[test]
fn actual_terminal_outcomes_permit_replacement() {
    for kind in 1..=4 {
        assert_eq!(needs_replacement([0, kind, 14, 0, 7]), Ok(true));
    }
}
#[test]
fn stale_denied_and_unknown_responses_do_not_authorize_replacement() {
    for status in [2, 11, 12] {
        assert_eq!(needs_replacement([status, 2, 0, 0, 7]), Err(status));
    }
    assert_eq!(needs_replacement([0, 99, 0, 0, 7]), Err(1));
}
#[test]
fn fresh_binding_requires_new_identity_success_and_zero_initial_state() {
    assert_eq!(fresh_binding(1, [258, 3, 0, 0]), Ok(()));
    for reply in [
        [0, 3, 0, 0],
        [258, 1, 0, 0],
        [258, 0, 0, 0],
        [258, 3, 12, 0],
        [258, 3, 0, 11],
    ] {
        assert!(fresh_binding(1, reply).is_err());
    }
}

#[test]
fn session_policy_rejects_unknown_configuration() {
    use native_apps::supervision::SessionPolicy;
    assert_eq!(SessionPolicy::decode(1), Ok(SessionPolicy::Persistent));
    assert_eq!(
        SessionPolicy::decode(2),
        Ok(SessionPolicy::FinishAfterClient)
    );
    for argument in [0, 3, u64::MAX] {
        assert_eq!(SessionPolicy::decode(argument), Err(1));
    }
}
