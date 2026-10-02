use migration_workbench::{ITERATIONS, Receipt, execute, expected_checksum};

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
