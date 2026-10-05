use migration_workbench::{ITERATIONS, Receipt, execute, expected_checksum};
use std::{fs, path::Path, process::Command};

#[test]
fn two_paths_preserve_oracle_with_distinct_measured_compatibility_debt() {
    let native = execute(true, true).unwrap();
    let compat = execute(false, true).unwrap();
    assert_eq!(native.checksum, expected_checksum());
    assert_eq!(compat.checksum, native.checksum);
    assert_eq!(native.native_admissions, ITERATIONS);
    assert_eq!(native.compat_admissions, 0);
    assert_eq!(compat.compat_admissions, ITERATIONS);
    assert_eq!(compat.copied_bytes, ITERATIONS * 4);
    assert_eq!(native.copied_bytes, 0);
    assert!(!native.warmup_batches_ns.is_empty());
    assert!(!compat.warmup_batches_ns.is_empty());
    assert!(native.elapsed_ns > 0 && compat.elapsed_ns > 0);
}

#[test]
fn executable_contract_checks_and_failures_propagate() {
    for binary in [
        env!("CARGO_BIN_EXE_native-fixture"),
        env!("CARGO_BIN_EXE_compat-fixture"),
    ] {
        let output = std::process::Command::new(binary)
            .arg("verify")
            .output()
            .unwrap();
        assert!(output.status.success());
        let receipt: Receipt = serde_json::from_slice(&output.stdout).unwrap();
        assert!(receipt.passed && receipt.checks >= 12);
        let failed = std::process::Command::new(binary)
            .arg("invalid-mode")
            .output()
            .unwrap();
        assert!(!failed.status.success());
        assert!(failed.stdout.is_empty());
    }
}

#[test]
fn lifecycle_worker_migrates_persistent_state_and_restores_snapshot() {
    let worker = env!("CARGO_BIN_EXE_lifecycle-fixture");
    let root = std::env::temp_dir().join(format!(
        "kolvrt-lifecycle-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state = root.join("state");
    fs::create_dir_all(&state).unwrap();
    let call = |mode: &str, path: &Path| Command::new(worker).arg(mode).arg(path).output().unwrap();
    assert!(call("seed", &state).status.success());
    assert!(call("verify-v1", &state).status.success());
    let snapshot = fs::read(state.join("state-v1.json")).unwrap();
    assert!(call("migrate", &state).status.success());
    assert_eq!(fs::read(state.join("state-v1.json")).unwrap(), snapshot);
    assert!(call("verify-v2", &state).status.success());

    assert!(call("verify-v1", &state).status.success());
    assert_eq!(fs::read(state.join("state-v1.json")).unwrap(), snapshot);

    let invalid = root.join("invalid");
    fs::create_dir_all(&invalid).unwrap();
    fs::write(
        invalid.join("state-v1.json"),
        b"{\"schema\":1,\"records\":[999]}",
    )
    .unwrap();
    assert!(!call("migrate", &invalid).status.success());
    assert!(!invalid.join("state-v2.json").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn contract_only_receipts_explicitly_mark_memory_as_not_evaluated() {
    use host_process_metrics::{PeakMemoryMeasurement, PeakMemoryUnavailable};
    let receipt = execute(true, false).unwrap();
    assert_eq!(
        receipt.peak_memory_bytes,
        PeakMemoryMeasurement::Unavailable {
            reason: PeakMemoryUnavailable::NotEvaluated
        }
    );
    let json = serde_json::to_value(&receipt).unwrap();
    assert_eq!(json["peak_memory_bytes"]["state"], "unavailable");
    assert_eq!(json["peak_memory_bytes"]["reason"], "not_evaluated");
    assert!(serde_json::from_value::<Receipt>(serde_json::json!({"passed":true})).is_err());
    let mut missing = json;
    missing.as_object_mut().unwrap().remove("peak_memory_bytes");
    assert!(serde_json::from_value::<Receipt>(missing).is_err());
}
