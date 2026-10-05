use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command};

#[test]
fn actual_survey_cli_matches_closed_export_schema_and_retained_synthetic_report() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: Value =
        serde_json::from_slice(&fs::read(root.join("schemas/host-survey.schema.json")).unwrap())
            .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let output = Command::new("node")
        .arg(root.join("scripts/host-survey.cjs"))
        .arg("sanitize")
        .arg(root.join("research/hardware/fixtures/synthetic-survey-input.json"))
        .output()
        .unwrap();
    assert!(output.status.success(), "synthetic survey CLI failed");
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(validator.is_valid(&report));
    let retained: Value = serde_json::from_slice(
        &fs::read(root.join("research/hardware/fixtures/synthetic-survey-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report, retained);
    for pointer in [
        "",
        "/system",
        "/system/cpus/0",
        "/devices/0",
        "/devices/0/ids",
        "/devices/0/driver",
        "/provenance",
        "/provenance/providers/0",
        "/coverage",
    ] {
        let mut invalid = report.clone();
        invalid
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(
                "private".into(),
                json!({"serial": "SYNTHETIC_DO_NOT_EXPORT"}),
            );
        assert!(!validator.is_valid(&invalid), "schema accepts {pointer}");
    }
    let mut unsafe_claim = report.clone();
    unsafe_claim["system"]["iommu"] = json!({"status":"OBSERVED","value":true});
    assert!(!validator.is_valid(&unsafe_claim));
    let mut unknown_scope = report;
    unknown_scope["scope"] = json!("boot_target");
    assert!(!validator.is_valid(&unknown_scope));
}
