//! Scope guard for the architectural split; compiler privacy and live kernel
//! rejection controls enforce ownership. This lexical check is not a safety proof.
use std::{fs, path::Path};
fn isolated(source: &str) -> bool {
    ![
        "boot_workload",
        "expected_class",
        "expected_far",
        "MARKER_BASE",
        "user_image_start",
        "write_volatile",
    ]
    .iter()
    .any(|name| source.contains(name))
}
#[test]
fn runtime_contains_no_fixture_expectations_or_user_data_writes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../kernel/src/scheduler");
    for file in fs::read_dir(root).unwrap() {
        let file = file.unwrap().path();
        let source = fs::read_to_string(&file).unwrap();
        assert!(isolated(&source), "fixture policy in {}", file.display());
        if file.file_name().unwrap() != "local.rs" {
            assert!(
                !source.contains("UnsafeCell") && !source.contains(".state.get()"),
                "unencapsulated storage in {}",
                file.display()
            );
        }
    }
    for fragment in [
        "crate::boot_workload::check(task)",
        "expected_class: u64",
        "MARKER_BASE + id",
        "write_volatile(user_data)",
    ] {
        assert!(!isolated(fragment));
    }
}
