//! Coverage-domain note: the host-library `cargo llvm-cov` aggregate excludes
//! the AArch64 kernel package and `xtask`'s binary entrypoint/router files.
//! These process tests exercise selected host CLI paths separately; kernel
//! coverage belongs to QEMU-backed kernel tests and must be reported separately.

use std::{
    path::Path,
    process::{Command, Output},
};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .expect("xtask executable starts")
}

#[test]
fn command_line_rejects_invalid_color_and_usage_without_running_kernel() {
    let color = run(&["--color=purple"]);
    assert!(!color.status.success());
    assert!(String::from_utf8_lossy(&color.stderr).contains("use --color=auto|always|never"));

    let unknown = run(&["unknown-command"]);
    assert!(!unknown.status.success());
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("usage:"));

    let bad_run_option = run(&["run", "--unexpected"]);
    assert!(!bad_run_option.status.success());
    assert!(String::from_utf8_lossy(&bad_run_option.stderr).contains("usage: cargo xtask run"));
}

#[test]
fn audit_subcommand_writes_a_parseable_inventory() {
    let output = run(&["audit"]);
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("unsafe inventory:"));

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/kernel/unsafe-audit.json");
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(
        value["dependency_boundary"],
        "compiler runtime and generated instructions require compiler trust; no compatibility dependency"
    );
    assert!(value["locations"].is_array());
    assert!(value["assembly"].is_array());
}

#[test]
fn compare_subcommand_revalidates_retained_measurements_without_claiming_a_winner() {
    let evidence = "research/results/kernel-phase2.json";
    let output = run(&["compare", evidence, evidence]);
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.matches("observations only").count(), 2);
    assert!(stdout.contains("\"baseline_ticks\""));
    assert!(stdout.contains("\"candidate_ticks\""));
}
