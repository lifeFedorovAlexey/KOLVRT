#![cfg(windows)]
use repository_checks::parse_json;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn launch(args: &[&str]) -> Output {
    Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(root().join("scripts/run-arena.ps1"))
        .args(args)
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap()
}
#[test]
fn launcher_help_and_invalid_inputs_do_not_start_measurements() {
    let help = launch(&["-Help"]);
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("Offline contract validation only")
    );
    for args in [
        &["assess"][..],
        &["compare", "missing.json"][..],
        &["unsupported"][..],
    ] {
        let rejected = launch(args);
        assert!(!rejected.status.success());
        assert!(rejected.stdout.is_empty());
    }
}
#[test]
fn launcher_default_runs_current_project_check_from_another_directory() {
    let result = launch(&[]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let json = parse_json(std::str::from_utf8(&result.stdout).unwrap()).unwrap();
    assert_eq!(json["state"], "PASS");
}
#[test]
fn launcher_profile_default_and_space_containing_paths_are_preserved() {
    let default = launch(&["profile"]);
    assert!(
        default.status.success(),
        "{}",
        String::from_utf8_lossy(&default.stderr)
    );
    let expected = parse_json(std::str::from_utf8(&default.stdout).unwrap()).unwrap();
    let directory = root().join("target/arena launcher paths");
    fs::create_dir_all(&directory).unwrap();
    let copied = directory.join("profile with spaces.json");
    fs::copy(
        root().join("research/arena/profiles/user-copy-range.json"),
        &copied,
    )
    .unwrap();
    let explicit = launch(&["profile", copied.to_str().unwrap()]);
    assert!(
        explicit.status.success(),
        "{}",
        String::from_utf8_lossy(&explicit.stderr)
    );
    assert_eq!(
        expected,
        parse_json(std::str::from_utf8(&explicit.stdout).unwrap()).unwrap()
    );
}
