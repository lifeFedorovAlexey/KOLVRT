#[path = "../src/ipc_measure_protocol.rs"]
mod protocol;
use protocol::*;
fn arg(case: u64, mode: u64, warm: u64, count: u64) -> u64 {
    case | (mode << 8) | (count << 16) | (warm << 32)
}
#[test]
fn exact_pilot_topologies_and_record_budgets() {
    for case in 0..12 {
        let (warm, count) = if case == 9 { (3, 33) } else { (4, 32) };
        for mode in [1, 2] {
            let c = Config::parse(arg(case, mode, warm, count));
            assert_eq!(c.count() * if c.two() { 2 } else { 1 }, 36);
            assert!(128 + 36 * 8 <= 2048);
            assert_eq!(c.counter(), case >= 10);
            assert_eq!(c.same(), [0, 3, 6].contains(&case));
        }
    }
}
#[test]
fn rejects_undefined_modes_overflow_and_unbalanced_clients() {
    for input in [
        arg(12, 1, 4, 32),
        arg(0, 0, 4, 32),
        arg(0, 3, 4, 32),
        arg(0, 1, 8, 224),
        arg(2, 1, 3, 32),
        arg(9, 1, 4, 32),
        arg(0, 1, 4, 32) | (1 << 63),
    ] {
        assert!(std::panic::catch_unwind(|| Config::parse(input)).is_err());
    }
}
#[test]
fn payload_identity_and_ledger_are_sensitive_to_client_and_sequence() {
    let root = 1;
    let peer = (1u64 << 32) | 1;
    assert_ne!(
        (0..8).map(|i| pattern(root, i)).collect::<Vec<_>>(),
        (0..8).map(|i| pattern(peer, i)).collect::<Vec<_>>()
    );
    assert_ne!(hash(root, 1), hash(root, 2));
    assert_ne!(hash(root, 1), hash(peer, 1));
}
