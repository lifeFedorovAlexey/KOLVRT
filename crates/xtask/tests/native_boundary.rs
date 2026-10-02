use std::{collections::BTreeSet, path::Path, process::Command};

fn only_native(tree: &str) -> bool {
    tree.lines()
        .map(|line| line.split_whitespace().next().unwrap_or(""))
        .collect::<BTreeSet<_>>()
        == BTreeSet::from(["kernel-core", "kolvrt-kernel"])
}
#[test]
fn native_dependency_direction_and_negative_control() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tree = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(root)
        .args([
            "tree",
            "--locked",
            "-p",
            "kolvrt-kernel",
            "--target",
            "aarch64-unknown-none",
            "--no-default-features",
            "--all-features",
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ])
        .output()
        .unwrap();
    assert!(tree.status.success());
    let tree = String::from_utf8(tree.stdout).unwrap();
    assert!(only_native(&tree), "native dependency reversal: {tree}");
    assert!(!only_native(&format!("{tree}\nwindow-compat v0.1.0")));
    assert!(!only_native(&format!("{tree}\nrouting v0.1.0")));
}
