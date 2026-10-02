use repository_checks::{cost_l, parse_json, read_json, write_json};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}
fn validator() -> &'static jsonschema::Validator {
    static SCHEMA: OnceLock<jsonschema::Validator> = OnceLock::new();
    SCHEMA.get_or_init(|| cost_l::schema(&root()).unwrap())
}
fn candidate() -> Value {
    read_json(&root().join("research/cost-l/COST-L-0001.json")).unwrap()
}
fn errors(value: &Value) -> Vec<String> {
    cost_l::record_errors(validator(), &root(), value, "COST-L-0001.json")
}
fn active() -> Value {
    let mut value = candidate();
    value["status"] = json!("ACTIVE");
    value["native"]["status"] = json!("ACCEPTED");
    value["support"]["status"] = json!("SUPPORTED");
    value["support"]["owner"] = json!("synthetic test owner");
    value["support"]["owner_unknown_reason"] = Value::Null;
    value["support"]["start"] = json!("2026-10-02");
    value["support"]["until"] = json!("2027-10-02");
    value["compat_modules"] = json!([{
        "id":"fixture-adapter", "semantic_version":"1.0.0",
        "artifact_sha256":"a".repeat(64), "scope":"synthetic tests only"
    }]);
    value["affected_consumers"] = json!([{
        "id":"offline-test-package", "kind":"OFFLINE_RECOVERY", "dependency":"DIRECT",
        "module_refs":[{"id":"fixture-adapter","semantic_version":"1.0.0","scope":"synthetic tests only"}], "scope":"synthetic tests only",
        "support_until":"2027-10-02"
    }]);
    for status in ["CONFIRMED", "ACTIVE"] {
        value["lifecycle"].as_array_mut().unwrap().push(json!({
            "status":status,"date":"2026-10-02","decision":"Synthetic transition fixture; not production evidence."
        }));
    }
    value
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "kolvrt-cost-l-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        write_json(
            &path.join("registry.json"),
            &json!({"schema_version":1,"allocated_ids":["COST-L-0001"]}),
        )
        .unwrap();
        Self(path)
    }
    fn put(&self, name: &str, value: &Value) {
        write_json(&self.0.join(name), value).unwrap();
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn actual_registry_is_valid_and_remains_research_only() {
    let records = cost_l::validate(&root(), &root().join("research/cost-l")).unwrap();
    assert!(!records.is_empty());
    for record in records {
        assert_eq!(record["status"], "CANDIDATE");
        assert!(record["compat_modules"].as_array().unwrap().is_empty());
        assert!(record["affected_consumers"].as_array().unwrap().is_empty());
        assert!(
            record["runtime_cost"]
                .as_object()
                .unwrap()
                .values()
                .all(|m| m["value"].is_null())
        );
    }
}
#[test]
fn closed_schema_required_fields_formats_and_bounds_are_enforced() {
    let schema = read_json(&root().join("schemas/cost-l.schema.json")).unwrap();
    for field in schema["required"].as_array().unwrap() {
        let mut value = candidate();
        value
            .as_object_mut()
            .unwrap()
            .remove(field.as_str().unwrap());
        assert!(!errors(&value).is_empty(), "missing {field} accepted");
    }
    for (pointer, bad) in [
        ("/title", json!(" ")),
        ("/title", json!("x".repeat(257))),
        ("/schema_version", json!(2)),
        ("/introduced_at", json!("2026-02-30")),
        ("/history/classification", json!("EVERYTHING_IS_A_BUG")),
        ("/sources/0/commit", json!("invented")),
        ("/sources/0/url", json!("file:///private")),
        ("/authority_implications", json!("EXTRA_DMA_AUTHORITY")),
        ("/confidence", json!(0.99)),
        ("/open_questions", json!(vec!["x"; 129])),
    ] {
        let mut value = candidate();
        *value.pointer_mut(pointer).unwrap() = bad;
        assert!(!errors(&value).is_empty(), "{pointer} accepted");
    }
    let mut unknown = candidate();
    unknown["history"]["made_up"] = json!(true);
    assert!(!errors(&unknown).is_empty());
    assert!(parse_json(r#"{"history":{"id":1,"id":2}}"#).is_err());
}
#[test]
fn identity_evidence_and_unknown_origin_cannot_be_fabricated() {
    for (pointer, bad) in [
        ("/id", json!("COST-L-0000")),
        ("/evidence/root_cause", json!(["S999"])),
        ("/history/unknown_origin_reason", Value::Null),
        ("/native/references", json!(["../outside.md"])),
        ("/source_cases", json!(["KOL-PATH-9999"])),
        ("/sources/0/kind", json!("SECONDARY_ANALYSIS")),
        (
            "/sources/0/url",
            json!("https://private:credential@example.com/file"),
        ),
        ("/sources/0/date", json!("2027-01-01")),
        ("/related_cost_l", json!(["COST-L-0001"])),
    ] {
        let mut value = candidate();
        *value.pointer_mut(pointer).unwrap() = bad;
        assert!(!errors(&value).is_empty(), "{pointer} accepted");
    }
    let mut duplicate = candidate();
    let source = duplicate["sources"][0].clone();
    duplicate["sources"].as_array_mut().unwrap().push(source);
    assert!(!errors(&duplicate).is_empty());
    assert!(
        !cost_l::record_errors(validator(), &root(), &candidate(), "COST-L-0002.json").is_empty()
    );
}
#[test]
fn support_is_not_runtime_observation_or_an_authority_waiver() {
    let valid = active();
    assert!(errors(&valid).is_empty(), "{:?}", errors(&valid));
    for (pointer, bad) in [
        ("/support/owner", Value::Null),
        ("/support/until", Value::Null),
        ("/support/until", json!("2026-01-01")),
        ("/compat_modules", json!([])),
        ("/affected_consumers", json!([])),
        ("/affected_consumers/0/module_refs/0/id", json!("missing")),
        ("/affected_consumers/0/support_until", json!("2028-01-01")),
        ("/native/status", json!("PROPOSED")),
        ("/status", json!("RETIRED")),
        ("/lifecycle/1/status", json!("ACTIVE")),
    ] {
        let mut value = valid.clone();
        *value.pointer_mut(pointer).unwrap() = bad;
        assert!(!errors(&value).is_empty(), "{pointer} accepted");
    }
    let mut research = valid.clone();
    research["status"] = json!("CANDIDATE");
    research["support"]["status"] = json!("RESEARCH_ONLY");
    research["lifecycle"] = candidate()["lifecycle"].clone();
    assert!(!errors(&research).is_empty());
}
#[test]
fn unknown_is_not_zero_and_measurements_need_provenance() {
    for (pointer, bad) in [
        ("/runtime_cost/cpu/value", json!(0)),
        ("/runtime_cost/cpu/reason", Value::Null),
        ("/runtime_cost/cpu/unit", json!("percent")),
        ("/runtime_cost/cpu/coverage", json!("COMPLETE")),
        ("/runtime_cost/cpu/state", json!("MEASURED")),
    ] {
        let mut value = candidate();
        *value.pointer_mut(pointer).unwrap() = bad;
        assert!(!errors(&value).is_empty(), "{pointer} accepted");
    }
    let mut measured = candidate();
    measured["last_observed_at"] = json!("2026-10-02");
    measured["runtime_cost/cpu"] = Value::Null; // Unknown property cannot sneak into a record.
    assert!(!errors(&measured).is_empty());
    measured.as_object_mut().unwrap().remove("runtime_cost/cpu");
    measured["runtime_cost"]["cpu"] = json!({
        "state":"MEASURED","value":0,"unit":"ns","scope":"synthetic zero observation",
        "denominator":"one fixture operation","coverage":"COMPLETE","reason":null,
        "provenance":{"artifact":"missing.json","sha256":"b".repeat(64),"source_type":"host_model"}
    });
    assert!(!errors(&measured).is_empty());
}
#[test]
fn registry_rejects_empty_oversized_duplicate_and_dangling_records() {
    let temp = Temp::new();
    assert!(cost_l::validate(&root(), &temp.0).is_err());
    let mut value = candidate();
    value["related_cost_l"] = json!(["COST-L-9999"]);
    temp.put("COST-L-0001.json", &value);
    assert!(
        cost_l::validate(&root(), &temp.0)
            .unwrap_err()
            .contains("dangling related")
    );
    value = candidate();
    value["exceptions"] = json!([format!("EXC-{:04}", 9999)]);
    temp.put("COST-L-0001.json", &value);
    assert!(
        cost_l::validate(&root(), &temp.0)
            .unwrap_err()
            .contains("dangling EXC")
    );
    temp.put("COST-L-0001.json", &candidate());
    temp.put("COST-L-0002.json", &candidate());
    assert!(cost_l::validate(&root(), &temp.0).is_err());
    fs::remove_file(temp.0.join("COST-L-0002.json")).unwrap();
    fs::write(
        temp.0.join("COST-L-0001.json"),
        " ".repeat(cost_l::MAX_RECORD_BYTES as usize + 1),
    )
    .unwrap();
    assert!(
        cost_l::validate(&root(), &temp.0)
            .unwrap_err()
            .contains("byte limit")
    );
    fs::write(
        temp.0.join("COST-L-0001.json"),
        r#"{"id":"COST-L-0001","id":"COST-L-0002"}"#,
    )
    .unwrap();
    assert!(
        cost_l::validate(&root(), &temp.0)
            .unwrap_err()
            .contains("duplicate JSON key")
    );
}

#[test]
fn allocation_ledger_prevents_accidental_tombstone_loss() {
    let temp = Temp::new();
    temp.put("COST-L-0001.json", &candidate());
    assert!(cost_l::validate(&root(), &temp.0).is_ok());
    temp.put(
        "registry.json",
        &json!({"schema_version":1,"allocated_ids":["COST-L-0001","COST-L-0002"]}),
    );
    assert!(
        cost_l::validate(&root(), &temp.0)
            .unwrap_err()
            .contains("allocation ledger differs")
    );
}

#[test]
fn measured_zero_is_accepted_only_with_an_exact_bounded_receipt() {
    use sha2::{Digest, Sha256};
    let temp = Temp::new();
    let receipt = b"{\"fixture_only\":true,\"cpu_ns\":0}";
    fs::write(temp.0.join("receipt.json"), receipt).unwrap();
    fs::write(temp.0.join("contract.md"), "synthetic native contract").unwrap();
    let mut value = candidate();
    value["native"]["references"] = json!(["contract.md"]);
    value["last_observed_at"] = json!("2026-10-02");
    value["runtime_cost"]["cpu"] = json!({
        "state":"MEASURED","value":0,"unit":"ns","scope":"synthetic host fixture only",
        "denominator":"one fixture operation","coverage":"COMPLETE","reason":null,
        "provenance":{"artifact":"receipt.json","sha256":format!("{:x}",Sha256::digest(receipt)),"source_type":"host_model"}
    });
    assert!(cost_l::record_errors(validator(), &temp.0, &value, "COST-L-0001.json").is_empty());
    fs::write(temp.0.join("receipt.json"), b"changed receipt").unwrap();
    assert!(
        cost_l::record_errors(validator(), &temp.0, &value, "COST-L-0001.json")
            .iter()
            .any(|e| e.contains("digest mismatch"))
    );
    value["runtime_cost"]["cpu"]["provenance"]["artifact"] = json!("../receipt.json");
    assert!(!cost_l::record_errors(validator(), &temp.0, &value, "COST-L-0001.json").is_empty());
}

#[test]
fn retired_history_cannot_erase_unexpired_offline_obligations() {
    let mut value = active();
    value["status"] = json!("RETIRED");
    value["support"]["status"] = json!("RETIRED");
    for status in ["DEPRECATED", "RETIRING", "RETIRED"] {
        value["lifecycle"].as_array_mut().unwrap().push(
            json!({"status":status,"date":"2026-10-02","decision":"synthetic retirement fixture"}),
        );
    }
    assert!(
        errors(&value)
            .iter()
            .any(|e| e.contains("completed finite obligations"))
    );
    value["research_date"] = json!("2027-10-03");
    value["lifecycle"][3]["date"] = json!("2027-10-02");
    value["lifecycle"][4]["date"] = json!("2027-10-03");
    value["lifecycle"][5]["date"] = json!("2027-10-03");
    assert!(errors(&value).is_empty(), "{:?}", errors(&value));
    // The record remains; a rejected research candidate can retire without ever supporting code.
    let mut rejected = candidate();
    rejected["status"] = json!("RETIRED");
    rejected["support"]["status"] = json!("RETIRED");
    rejected["lifecycle"].as_array_mut().unwrap().push(json!({"status":"RETIRED","date":"2026-10-02","decision":"No demonstrated incompatibility; preserve historical candidate."}));
    assert!(errors(&rejected).is_empty());
}
#[test]
fn actual_cli_reports_success_and_nonzero_for_invalid_registry() {
    let exe = env!("CARGO_BIN_EXE_repository-checks");
    let good = Command::new(exe)
        .current_dir(root())
        .arg("check-cost-l")
        .output()
        .unwrap();
    assert!(good.status.success(), "{:?}", good);
    assert!(
        String::from_utf8(good.stdout)
            .unwrap()
            .contains("no runtime support or authority")
    );
    let temp = Temp::new();
    temp.put("COST-L-0001.json", &json!({"id":"COST-L-0001"}));
    let bad = Command::new(exe)
        .current_dir(root())
        .args(["check-cost-l", "--directory"])
        .arg(&temp.0)
        .output()
        .unwrap();
    assert!(!bad.status.success());
    let unexpected = Command::new(exe)
        .current_dir(root())
        .args(["check-cost-l", "--unknown"])
        .output()
        .unwrap();
    assert!(!unexpected.status.success());
}

#[test]
fn module_versions_are_pinned_and_shared_artifacts_cannot_conflict() {
    let mut value = active();
    let mut newer = value["compat_modules"][0].clone();
    newer["semantic_version"] = json!("2.0.0");
    newer["artifact_sha256"] = json!("b".repeat(64));
    value["compat_modules"].as_array_mut().unwrap().push(newer);
    assert!(errors(&value).is_empty(), "{:?}", errors(&value));
    value["affected_consumers"][0]["module_refs"][0]["semantic_version"] = json!("9.0.0");
    assert!(errors(&value).iter().any(|e| e.contains("absent module")));
    value = active();
    let temp = Temp::new();
    temp.put(
        "registry.json",
        &json!({"schema_version":1,"allocated_ids":["COST-L-0001","COST-L-0002"]}),
    );
    temp.put("COST-L-0001.json", &value);
    value["id"] = json!("COST-L-0002");
    temp.put("COST-L-0002.json", &value);
    assert!(cost_l::validate(&root(), &temp.0).is_ok());
    value["compat_modules"][0]["artifact_sha256"] = json!("b".repeat(64));
    temp.put("COST-L-0002.json", &value);
    assert!(
        cost_l::validate(&root(), &temp.0)
            .unwrap_err()
            .contains("conflicting module artifact")
    );
}

#[test]
fn support_deadline_is_the_first_unsupported_calendar_day() {
    let mut value = active();
    value["research_date"] = value["support"]["until"].clone();
    assert!(errors(&value).iter().any(|e| e.contains("already expired")));
    value["status"] = json!("RETIRING");
    value["support"]["status"] = json!("EXPIRED");
    for status in ["DEPRECATED", "RETIRING"] {
        value["lifecycle"].as_array_mut().unwrap().push(
            json!({"status":status,"date":"2027-10-02","decision":"synthetic support expiry"}),
        );
    }
    assert!(errors(&value).is_empty(), "{:?}", errors(&value));
    value["support"]["until"] = json!("2028-01-01");
    assert!(
        errors(&value)
            .iter()
            .any(|e| e.contains("reached its declared deadline"))
    );
}
