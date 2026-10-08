use repository_checks::{arena, parse_json, read_json};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn registry() -> Value {
    read_json(&root().join("research/arena/standards.json")).unwrap()
}
fn profile() -> Value {
    read_json(&root().join("research/arena/profiles/user-copy-range.json")).unwrap()
}
fn seal(run: &mut Value) {
    run["profile_sha256"] = json!(arena::digest(&run["profile_snapshot"]));
    run["registry_sha256"] = json!(arena::digest(&run["registry_snapshot"]));
}
fn fixture() -> (PathBuf, Value) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let bundle = root().join(format!(
        "target/arena-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&bundle).unwrap();
    fs::write(bundle.join("environment.json"), b"synthetic environment\n").unwrap();
    fs::write(
        bundle.join("evidence.txt"),
        b"synthetic oracle, source, image and SAR assertions\n",
    )
    .unwrap();
    let env_digest = arena::bytes_digest(b"synthetic environment\n");
    let evidence_digest =
        arena::bytes_digest(b"synthetic oracle, source, image and SAR assertions\n");
    let mut p = profile();
    p["review_state"] = json!("REVIEWED");
    p["environment"]["configuration_sha256"] = json!(env_digest);
    p["environment"]["platform_kind"] = json!("QEMU");
    p["environment"]["architecture"] = json!("ARM64");
    let mut run = json!({"schema_version":1,"run_id":"synthetic-import-control","scope":"KERNEL_QEMU","profile_snapshot":p,"registry_snapshot":registry(),"profile_sha256":"0".repeat(64),"registry_sha256":"0".repeat(64),"provenance":{"exact_commit":"1".repeat(40),"source_sha256":evidence_digest,"image_sha256":evidence_digest,"submission":"synthetic acceptance fixture, not an executed kernel run","contribution_state":"UNATTRIBUTED","contributors":[],"contribution_evidence":[]},"environment_manifest":{"path":"environment.json","sha256":env_digest},"artifacts":[{"path":"environment.json","sha256":env_digest},{"path":"evidence.txt","sha256":evidence_digest}],"correctness":{"status":"PASS","reason":"Synthetic asserted oracle result","evidence":["evidence.txt"]},"security_results":[{"sfr_id":"arena.sfr.range-bounds","status":"PASS","reason":"Synthetic SFR assertion","sar_results":[{"sar_id":"arena.sar.range-deterministic","status":"PASS","reason":"Synthetic deterministic assertion","evidence":["evidence.txt"]},{"sar_id":"arena.sar.range-negative","status":"PASS","reason":"Synthetic deliberate negative assertion","evidence":["evidence.txt"]}]}],"observations":[{"metric_id":"arena.metric.range-median","state":"AVAILABLE","samples":(0..100).map(|i|i as f64).collect::<Vec<_>>(),"warmup_samples":[7,8],"failures":0,"dropped":0,"unfinished":0}],"overhead_result":{"protocol_id":"arena.overhead.host-range","protocol_version":"1","matched_workload_sha256":p["overhead"]["matched_workload_sha256"],"state":"AVAILABLE","reason":"Synthetic matched observer assertion","off_samples":[1,2],"on_samples":[3,4],"evidence":["evidence.txt"]}});
    run["warmup"] =
        json!({"status":"PASS","reason":"Synthetic warmup assertion","evidence":["evidence.txt"]});
    seal(&mut run);
    (bundle, run)
}
fn report(bundle: &Path, run: &Value) -> Value {
    arena::assess(&root(), bundle, run).unwrap()
}
#[test]
fn checked_registry_and_example_resolve_canonical_targets() {
    arena::check(&root()).unwrap();
}
#[test]
fn malformed_registry_and_fake_standard_classification_are_rejected() {
    let valid = registry();
    arena::validate_registry(&root(), &valid).unwrap();
    for (field, bad) in [
        ("source_kind", json!("DE_FACTO")),
        ("last_checked", json!("2026-02-30")),
        (
            "authoritative_source",
            json!("https://example.invalid/standard"),
        ),
    ] {
        let mut r = valid.clone();
        r["entries"][0][field] = bad;
        assert!(arena::validate_registry(&root(), &r).is_err(), "{field}");
    }
    let mut duplicate = valid.clone();
    duplicate["entries"]
        .as_array_mut()
        .unwrap()
        .push(valid["entries"][0].clone());
    assert!(arena::validate_registry(&root(), &duplicate).is_err());
    assert!(parse_json(r#"{"schema_version":1,"schema_version":2}"#).is_err());
}
#[test]
fn published_standard_cannot_be_superseded_by_draft_or_cycles() {
    let mut r = registry();
    r["entries"][0]["superseded_by"] =
        json!({"id":r["entries"][1]["id"],"version":r["entries"][1]["version"]});
    r["entries"][1]["publication_state"] = json!("DRAFT");
    assert!(arena::validate_registry(&root(), &r).is_err());
    r["entries"][1]["publication_state"] = json!("PUBLISHED");
    r["entries"][1]["superseded_by"] =
        json!({"id":r["entries"][0]["id"],"version":r["entries"][0]["version"]});
    assert!(arena::validate_registry(&root(), &r).is_err());
}
#[test]
fn metrics_need_sources_units_and_explicit_custom_origin() {
    for field in ["methodologies", "units", "denominator"] {
        let mut p = profile();
        p["metrics"][0].as_object_mut().unwrap().remove(field);
        assert!(
            arena::validate_profile(&root(), &registry(), &p).is_err(),
            "{field}"
        );
    }
    let mut p = profile();
    p["metrics"][0]["methodologies"][0]["version"] = json!("unregistered");
    assert!(arena::validate_profile(&root(), &registry(), &p).is_err());
    p = profile();
    p["metrics"][0]["origin"] = json!("REFERENCED");
    assert!(arena::validate_profile(&root(), &registry(), &p).is_err());
    p = profile();
    p["arena_score"] = json!(99);
    assert!(arena::validate_profile(&root(), &registry(), &p).is_err());
}
#[test]
fn prod_forbids_internal_attribution_even_for_shared_metric() {
    let mut p = profile();
    p["environment"]["execution_profile"] = json!("PROD");
    arena::validate_profile(&root(), &registry(), &p).unwrap();
    p["environment"]["measurement"]["instrumentation"] = json!("DEV_INTERNAL");
    assert!(arena::validate_profile(&root(), &registry(), &p).is_err());
    p["environment"]["measurement"]["instrumentation"] = json!("EXTERNAL");
    p["metrics"][0]["visibility"] = json!("DEV_INTERNAL");
    assert!(arena::validate_profile(&root(), &registry(), &p).is_err());
}
#[test]
fn semantic_environment_and_protocol_changes_never_share_class() {
    let original = profile();
    let r = registry();
    let mutations: Vec<(&str, &str, Value)> = vec![
        ("target", "functional_version", json!("changed")),
        ("workload", "version", json!("changed")),
        ("environment", "architecture", json!("ARM64")),
        ("environment", "platform_kind", json!("PHYSICAL_HARDWARE")),
        ("environment", "execution_profile", json!("PROD")),
        ("sampling", "analysis_version", json!("2")),
        ("security", "version", json!("2")),
        ("overhead", "version", json!("2")),
    ];
    for (parent, field, value) in mutations {
        let mut changed = original.clone();
        changed[parent][field] = value;
        assert!(
            arena::compatible(&root(), &r, &original, &r, &changed).is_err(),
            "{parent}/{field}"
        );
    }
    let mut changed = original.clone();
    changed["metrics"][0]["methodologies"][0]["version"] = json!("2");
    assert!(arena::compatible(&root(), &r, &original, &r, &changed).is_err());
}
#[test]
fn implementation_changes_do_not_freeze_class_but_require_fresh_artifacts() {
    let (bundle, mut run) = fixture();
    let first = report(&bundle, &run);
    assert_eq!(first["admission_state"], "STRUCTURALLY_ADMISSIBLE");
    assert_eq!(first["record_eligible"], false);
    run["provenance"]["exact_commit"] = json!("2".repeat(40));
    assert_eq!(
        first["comparison_class"],
        report(&bundle, &run)["comparison_class"]
    );
    run["provenance"]["image_sha256"] = json!("3".repeat(64));
    assert!(arena::assess(&root(), &bundle, &run).is_err());
}
#[test]
fn changed_current_registry_does_not_rewrite_old_snapshots() {
    let (bundle, run) = fixture();
    let before = report(&bundle, &run);
    let mut new = registry();
    new["registry_version"] = json!("new");
    new["entries"][0]["version"] = json!("future");
    arena::validate_registry(&root(), &new).unwrap();
    assert_eq!(before, report(&bundle, &run));
    let mut forged = run.clone();
    forged["registry_snapshot"]["entries"][0]["scope"] = json!(["edited historical scope"]);
    assert!(arena::assess(&root(), &bundle, &forged).is_err());
}
#[test]
fn same_version_labels_cannot_hide_methodology_change() {
    let (bundle, run) = fixture();
    let mut changed = run["registry_snapshot"].clone();
    changed["entries"][3]["scope"] = json!(["different methodology"]);
    assert!(
        arena::compatible(
            &root(),
            &run["registry_snapshot"],
            &run["profile_snapshot"],
            &changed,
            &run["profile_snapshot"]
        )
        .is_err()
    );
    assert_eq!(report(&bundle, &run)["record_eligible"], false);
}
#[test]
fn broken_oracle_or_missing_sar_cannot_admit_fastest_result() {
    let (bundle, valid) = fixture();
    for mutation in 0..4 {
        let mut run = valid.clone();
        run["observations"][0]["samples"] = json!(vec![0; 100]);
        match mutation {
            0 => run["correctness"]["status"] = json!("FAIL"),
            1 => run["correctness"]["evidence"] = json!([]),
            2 => run["security_results"][0]["sar_results"] = json!([]),
            _ => run["security_results"][0]["status"] = json!("FAIL"),
        }
        let result = report(&bundle, &run);
        assert_eq!(result["admission_state"], "INELIGIBLE");
        assert_eq!(result["record_eligible"], false);
    }
}
#[test]
fn dropped_unfinished_and_bad_sample_plan_remain_ineligible() {
    let (bundle, valid) = fixture();
    for field in ["failures", "dropped", "unfinished"] {
        let mut run = valid.clone();
        run["observations"][0][field] = json!(1);
        assert_eq!(report(&bundle, &run)["admission_state"], "INELIGIBLE");
    }
    let mut run = valid.clone();
    run["observations"][0]["samples"] = json!([1]);
    assert_eq!(report(&bundle, &run)["admission_state"], "INELIGIBLE");
    run = valid;
    run["observations"][0]["warmup_samples"] = json!([]);
    assert_eq!(report(&bundle, &run)["admission_state"], "INELIGIBLE");
}
#[test]
fn missing_observation_keeps_reason_and_never_becomes_zero() {
    let (bundle, mut run) = fixture();
    run["observations"][0] = json!({"metric_id":"arena.metric.range-median","state":"UNAVAILABLE","reason":"counter_not_supported","missing_observations":100,"evidence":[]});
    let r = report(&bundle, &run);
    assert_eq!(r["admission_state"], "INELIGIBLE");
    assert_eq!(
        r["measurements"]["arena.metric.range-median"]["reason"],
        "counter_not_supported"
    );
    assert!(
        r["measurements"]["arena.metric.range-median"]
            .get("value")
            .is_none()
    );
}
#[test]
fn overhead_is_matched_versioned_and_not_subtracted() {
    let (bundle, mut run) = fixture();
    let r = report(&bundle, &run);
    assert_eq!(
        r["measurements"]["arena.metric.range-median"]["value"],
        49.0
    );
    run["overhead_result"]["protocol_version"] = json!("different");
    assert!(arena::assess(&root(), &bundle, &run).is_err());
    run["overhead_result"]["protocol_version"] = json!("1");
    run["overhead_result"]["on_samples"] = json!([]);
    assert_eq!(report(&bundle, &run)["admission_state"], "INELIGIBLE");
}
#[test]
fn evidence_corruption_escape_and_unlinked_assertions_are_rejected() {
    let (bundle, valid) = fixture();
    let mut run = valid.clone();
    run["correctness"]["evidence"] = json!(["missing.txt"]);
    assert!(arena::assess(&root(), &bundle, &run).is_err());
    run = valid.clone();
    run["artifacts"][1]["path"] = json!("../evidence.txt");
    assert!(arena::assess(&root(), &bundle, &run).is_err());
    fs::write(bundle.join("evidence.txt"), "tampered").unwrap();
    assert!(arena::assess(&root(), &bundle, &valid).is_err());
}
#[test]
fn accepted_authorship_requires_links_and_fixture_never_becomes_kernel_evidence() {
    let (bundle, mut run) = fixture();
    run["provenance"]["contribution_state"] = json!("ACCEPTED");
    assert!(arena::assess(&root(), &bundle, &run).is_err());
    run["provenance"]["contribution_state"] = json!("UNATTRIBUTED");
    run["scope"] = json!("SYNTHETIC_FIXTURE");
    assert!(arena::assess(&root(), &bundle, &run).is_err());
    run["profile_snapshot"]["environment"]["platform_kind"] = json!("HOST_FIXTURE");
    seal(&mut run);
    let r = report(&bundle, &run);
    assert_eq!(r["admission_state"], "INELIGIBLE");
    assert_eq!(r["record_eligible"], false);
}

#[test]
fn warmup_failure_and_whitespace_units_cannot_admit() {
    let (bundle, mut run) = fixture();
    run["warmup"]["status"] = json!("FAIL");
    assert_eq!(report(&bundle, &run)["admission_state"], "INELIGIBLE");
    let mut p = profile();
    p["metrics"][0]["units"] = json!("   ");
    assert!(arena::validate_profile(&root(), &registry(), &p).is_err());
}
#[test]
fn cli_enforces_nonzero_rejections_without_publishing_records() {
    let (bundle, mut run) = fixture();
    let path = bundle.join("run.json");
    run["correctness"]["status"] = json!("FAIL");
    fs::write(&path, serde_json::to_string(&run).unwrap()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_repository-checks"))
        .current_dir(&bundle)
        .args(["arena", "assess"])
        .arg("run.json")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let assessment = parse_json(std::str::from_utf8(&output.stdout).unwrap()).unwrap();
    assert_eq!(assessment["admission_state"], "INELIGIBLE");
    assert_eq!(assessment["record_eligible"], false);
    let other = bundle.join("other.json");
    run["correctness"]["status"] = json!("PASS");
    fs::write(&path, serde_json::to_string(&run).unwrap()).unwrap();
    run["profile_snapshot"]["environment"]["execution_profile"] = json!("PROD");
    seal(&mut run);
    fs::write(&other, serde_json::to_string(&run).unwrap()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_repository-checks"))
        .current_dir(root())
        .args(["arena", "compare"])
        .arg(&path)
        .arg(&other)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn readable_report_and_diff_use_actual_admission_and_compatibility() {
    let (bundle, mut run) = fixture();
    let left = bundle.join("a-run.json");
    let right = bundle.join("b-run.json");
    fs::write(&left, serde_json::to_string(&run).unwrap()).unwrap();
    run["run_id"] = json!("synthetic-second-repeat");
    run["observations"][0]["samples"] = json!((0..100).map(|i| i as f64 + 2.0).collect::<Vec<_>>());
    fs::write(&right, serde_json::to_string(&run).unwrap()).unwrap();
    let rendered = arena::report::render(&root(), Some(&bundle)).unwrap();
    assert!(rendered.admissible);
    assert!(rendered.text.contains("MEDIAN"));
    assert!(rendered.text.contains("N=100"));
    assert!(rendered.text.contains("No baseline selected"));
    assert!(rendered.text.find("a-run.json").unwrap() < rendered.text.find("b-run.json").unwrap());
    let diff = arena::report::diff(&root(), &left, &right).unwrap();
    assert!(diff.contains("+2.000000"));
    assert!(diff.contains("Same retained source snapshot"));
    assert!(diff.contains("No statistical significance"));
    run["observations"][0]["failures"] = json!(1);
    fs::write(&right, serde_json::to_string(&run).unwrap()).unwrap();
    let rejected = arena::report::render(&root(), Some(&right)).unwrap();
    assert!(!rejected.admissible);
    assert!(rejected.text.contains("failures=1"));
    assert!(rejected.text.contains("Admission reason:"));
    assert!(arena::report::diff(&root(), &left, &right).is_err());
    let mut unavailable = run.clone();
    unavailable["observations"][0] = json!({"metric_id":"arena.metric.range-median","state":"INCONCLUSIVE","reason":"actual samples missing","missing_observations":100,"evidence":["evidence.txt"]});
    unavailable["overhead_result"]["state"] = json!("UNAVAILABLE");
    unavailable["overhead_result"]["off_samples"] = json!([]);
    unavailable["overhead_result"]["on_samples"] = json!([]);
    fs::write(&right, serde_json::to_string(&unavailable).unwrap()).unwrap();
    let absent = arena::report::render(&root(), Some(&right)).unwrap();
    assert!(!absent.admissible);
    assert!(absent.text.contains("INCONCLUSIVE: actual samples missing"));
    assert!(absent.text.contains("Recorder reason:"));
    assert!(arena::report::diff(&root(), &left, &right).is_err());
    run["observations"][0]["failures"] = json!(0);
    run["profile_snapshot"]["workload"]["seed"] = json!("different-class");
    seal(&mut run);
    fs::write(&right, serde_json::to_string(&run).unwrap()).unwrap();
    assert!(arena::report::diff(&root(), &left, &right).is_err());
    let classes = arena::report::render(&root(), Some(&bundle)).unwrap();
    assert_eq!(
        classes.text.matches("Workload:").count(),
        2,
        "same profile ID with different comparison classes needs distinct headers"
    );
    fs::write(&right, "broken").unwrap();
    let corrupt = arena::report::render(&root(), Some(&bundle)).unwrap();
    assert!(!corrupt.admissible);
    assert!(corrupt.text.contains("INVALID"));
    assert!(corrupt.text.contains("a-run.json"));
    fs::remove_dir_all(bundle).unwrap();
}
