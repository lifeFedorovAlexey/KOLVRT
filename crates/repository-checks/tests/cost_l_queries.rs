use repository_checks::{read_json, write_json};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
fn command(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_repository-checks"))
        .current_dir(root())
        .arg("cost-l")
        .args(args)
        .output()
        .unwrap()
}
fn result(args: &[&str]) -> Value {
    let output = command(args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
struct Registry(PathBuf);
impl Registry {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "kolvrt-cost-l-queries-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        write_json(
            &directory.join("registry.json"),
            &json!({"schema_version":1,"allocated_ids":["COST-L-0001","COST-L-0002"]}),
        )
        .unwrap();
        for id in ["COST-L-0001", "COST-L-0002"] {
            let mut record = read_json(&root().join("research/cost-l/COST-L-0001.json")).unwrap();
            record["id"] = json!(id);
            record["status"] = json!("ACTIVE");
            record["native"]["status"] = json!("ACCEPTED");
            record["support"] = json!({"status":"SUPPORTED","owner":"synthetic fixture owner","owner_unknown_reason":null,"start":"2026-10-02","until":"2027-10-02","migration_target":"synthetic native alternative","migration_strategy":"fixture migration only","removal_condition":"synthetic offline obligation retained"});
            record["compat_modules"] = json!([{"id":"shared-fixture","semantic_version":"1.0.0","artifact_sha256":"a".repeat(64),"scope":"synthetic tests only"}]);
            record["affected_consumers"] = json!([{"id":"fixture-consumer","kind":"OFFLINE_RECOVERY","dependency":if id=="COST-L-0001"{"DIRECT"}else{"TRANSITIVE"},"scope":"synthetic tests only","support_until":"2027-10-02","module_refs":[{"id":"shared-fixture","semantic_version":"1.0.0","scope":"synthetic tests only"}]}]);
            for state in ["CONFIRMED", "ACTIVE"] {
                record["lifecycle"].as_array_mut().unwrap().push(json!({"status":state,"date":"2026-10-02","decision":"Synthetic fixture, not real deployment."}));
            }
            write_json(&directory.join(format!("{id}.json")), &record).unwrap();
        }
        Self(directory)
    }
    fn query(&self, args: &[&str]) -> Value {
        let mut all = args.to_vec();
        all.extend(["--directory", self.0.to_str().unwrap(), "--json"]);
        result(&all)
    }
    fn edit(&self, id: &str, edit: impl FnOnce(&mut Value)) {
        let path = self.0.join(format!("{id}.json"));
        let mut record = read_json(&path).unwrap();
        edit(&mut record);
        write_json(&path, &record).unwrap();
    }
}
impl Drop for Registry {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn real_registry_reports_declared_zero_separately_from_global_unknown() {
    let list = result(&["list", "--json"]);
    assert_eq!(list["schema_version"], 1);
    assert_eq!(list["rows"].as_array().unwrap().len(), 3);
    for row in list["rows"].as_array().unwrap() {
        assert_eq!(row["declared_reach"], 0);
        assert_eq!(row["global_reach"], "UNKNOWN");
        assert_eq!(row["software_state"], "UNDETERMINED_NO_MODULE_DECLARATION");
        assert_eq!(row["runtime_cost"]["cpu"]["state"], "UNKNOWN");
        assert!(row["runtime_cost"]["cpu"]["value"].is_null());
    }
    assert_eq!(list["inventory"]["runtime_inspection"], "UNSUPPORTED");
    assert_eq!(
        list["inventory"]["production_manifest_integration"],
        "UNAVAILABLE_PENDING_ISSUE_47"
    );
    let consumers = result(&["consumers", "COST-L-0001", "--json"]);
    assert!(consumers["rows"].as_array().unwrap().is_empty());
    let human = command(&["show", "COST-L-0001"]);
    assert!(human.status.success());
    assert!(
        String::from_utf8(human.stdout)
            .unwrap()
            .contains("global coverage UNKNOWN")
    );
}

#[test]
fn both_directions_preserve_transitive_offline_and_shared_module_relations() {
    let registry = Registry::new();
    let deps = registry.query(&["deps", "fixture-consumer"]);
    assert_eq!(deps["rows"].as_array().unwrap().len(), 2);
    assert_eq!(deps["rows"][0]["consumer"]["dependency"], "DIRECT");
    assert_eq!(deps["rows"][1]["consumer"]["dependency"], "TRANSITIVE");
    for row in deps["rows"].as_array().unwrap() {
        let reverse = registry.query(&["consumers", row["debt_id"].as_str().unwrap()]);
        assert_eq!(reverse["rows"][0]["consumer"], row["consumer"]);
        assert_eq!(row["consumer"]["kind"], "OFFLINE_RECOVERY");
        assert_eq!(
            row["consumer"]["module_refs"][0]["semantic_version"],
            "1.0.0"
        );
        assert_eq!(row["transitive_path"], "NOT_RECORDED");
    }
    let first = registry.query(&["top", "--limit", "1"]);
    let second = registry.query(&["top", "--limit", "1", "--offset", "1"]);
    assert_eq!(first["rows"][0]["id"], "COST-L-0001");
    assert_eq!(first["pagination"]["next_offset"], 1);
    assert_eq!(second["rows"][0]["id"], "COST-L-0002");
    assert!(second["pagination"]["next_offset"].is_null());
    assert_eq!(
        registry.query(&["top", "--security", "quotas"])["pagination"]["matching_rows"],
        2
    );
    assert_eq!(
        registry.query(&["top", "--maintenance", "offline"])["pagination"]["matching_rows"],
        2
    );
    assert_eq!(
        registry.query(&["top", "--migration", "native alternative"])["pagination"]["matching_rows"],
        2
    );
}

#[test]
fn malformed_queries_and_inconsistent_graph_fail_without_stdout() {
    for args in [
        vec!["show", "COST-L-0000"],
        vec!["show", "COST-L-1"],
        vec!["show", "COST-L-9999"],
        vec!["deps", "not-declared"],
        vec!["runtime"],
        vec!["top", "--sort", "security"],
        vec!["top", "--sort", "cpu"],
        vec!["list", "--limit", "65"],
        vec!["list", "--json", "--json"],
        vec!["list", "--offset", "4"],
        vec!["list", "--unknown", "value"],
    ] {
        let output = command(&args);
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
    }
    for mutation in 0..3 {
        let registry = Registry::new();
        registry.edit("COST-L-0002", |r| match mutation {
            0 => r["compat_modules"][0]["artifact_sha256"] = json!("b".repeat(64)),
            1 => r["related_cost_l"] = json!(["COST-L-9999"]),
            _ => r["affected_consumers"][0]["module_refs"][0]["semantic_version"] = json!("9.0.0"),
        });
        let output = command(&[
            "show",
            "COST-L-0001",
            "--directory",
            registry.0.to_str().unwrap(),
            "--json",
        ]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn measured_zero_is_ranked_without_converting_unknown_or_foreign_scope() {
    let registry = Registry::new();
    let receipt = registry.0.join("receipt.txt");
    fs::write(&receipt, "{\"fixture\":true}\n").unwrap();
    // Provenance references must be repository-local, so retain the fixture there.
    let relative = format!("target/cost-l-query-receipt-{}.json", std::process::id());
    fs::copy(&receipt, root().join(&relative)).unwrap();
    use sha2::{Digest, Sha256};
    let digest = (Sha256::digest(fs::read(&receipt).unwrap()))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    registry.edit("COST-L-0002",|r| {
        r["last_observed_at"]=json!("2026-10-03");
        r["runtime_cost"]["cpu"]=json!({"state":"MEASURED","value":0,"unit":"ns","scope":"synthetic cost fixture","denominator":"one fixture operation","coverage":"PARTIAL","reason":"Synthetic partial fixture; not production","provenance":{"artifact":relative,"sha256":digest,"source_type":"host_model"}});
    });
    let ranked = registry.query(&[
        "top",
        "--sort",
        "cpu",
        "--measurement-scope",
        "synthetic cost fixture",
        "--denominator",
        "one fixture operation",
    ]);
    assert_eq!(ranked["rows"][0]["id"], "COST-L-0002");
    assert_eq!(ranked["rows"][0]["ranking_value"], 0);
    assert!(ranked["rows"][1]["ranking_value"].is_null());
    let unrelated = registry.query(&[
        "top",
        "--sort",
        "cpu",
        "--measurement-scope",
        "unrelated workload",
        "--denominator",
        "one fixture operation",
    ]);
    assert!(
        unrelated["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["ranking_value"].is_null())
    );
    fs::remove_file(root().join(relative)).unwrap();
}

#[test]
fn output_budget_rejects_the_whole_page_and_a_smaller_page_succeeds() {
    let registry = Registry::new();
    let ids: Vec<_> = (1..=40).map(|i| format!("COST-L-{i:04}")).collect();
    write_json(
        &registry.0.join("registry.json"),
        &json!({"schema_version":1,"allocated_ids":ids}),
    )
    .unwrap();
    for id in &ids {
        let mut record = read_json(&root().join("research/cost-l/COST-L-0001.json")).unwrap();
        record["id"] = json!(id);
        for metric in record["runtime_cost"].as_object_mut().unwrap().values_mut() {
            metric["scope"] = json!("s".repeat(4096));
        }
        record["security_impact"] = json!("s".repeat(4096));
        record["complexity_impact"] = json!("m".repeat(4096));
        for field in [
            "migration_target",
            "migration_strategy",
            "removal_condition",
        ] {
            record["support"][field] = json!("m".repeat(4096));
        }
        write_json(&registry.0.join(format!("{id}.json")), &record).unwrap();
    }
    let output = command(&[
        "list",
        "--directory",
        registry.0.to_str().unwrap(),
        "--json",
        "--limit",
        "40",
    ]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("output exceeds 1 MiB")
    );
    let one = registry.query(&["list", "--limit", "1"]);
    assert_eq!(one["rows"].as_array().unwrap().len(), 1);
    assert_eq!(one["pagination"]["next_offset"], 1);
}
