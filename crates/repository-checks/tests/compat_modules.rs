use repository_checks::{compat_modules, cost_l, read_json};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}
fn metadata() -> Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| compat_modules::cargo_metadata(&root()).unwrap())
        .clone()
}
fn inventory() -> Value {
    read_json(&root().join("policy/compatibility-modules.json")).unwrap()
}
fn records() -> Vec<Value> {
    cost_l::validate(&root(), &root().join("research/cost-l")).unwrap()
}
fn check(inventory: &Value, records: &[Value], metadata: &Value) -> Result<(), String> {
    compat_modules::validate(&root(), inventory, records, metadata)
}
fn reject(inventory: &Value, records: &[Value], metadata: &Value, expected: &str) {
    let error = check(inventory, records, metadata).unwrap_err();
    assert!(error.contains(expected), "Expected {expected}; got {error}");
}
fn production() -> (Value, Vec<Value>, Value) {
    let mut inv = inventory();
    let mut records = records();
    let module = &mut inv["modules"][0];
    module["classification"] = json!("PRODUCTION");
    module["debt_ids"] = json!(["COST-L-0001"]);
    module["debt_unassigned_reason"] = Value::Null;
    // Deliberately non-executable retained bytes: this fixture tests identity, not module execution.
    let path = "policy/exceptions.json";
    let digest = Sha256::digest(fs::read(root().join(path)).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    module["artifact"] = json!({"path":path,"sha256":digest});
    let debt = &mut records[0];
    debt["status"] = json!("ACTIVE");
    debt["native"]["status"] = json!("ACCEPTED");
    debt["support"]["status"] = json!("SUPPORTED");
    debt["support"]["owner"] = module["owner"].clone();
    debt["support"]["owner_unknown_reason"] = Value::Null;
    debt["support"]["start"] = json!("2026-10-02");
    debt["support"]["until"] = module["support_until"].clone();
    debt["compat_modules"] = json!([{"id":module["id"],"semantic_version":module["semantic_version"],"scope":module["scope"],"artifact_sha256":digest}]);
    let mut consumer = module["consumers"][0].clone();
    consumer["module_refs"] = json!([{"id":module["id"],"semantic_version":module["semantic_version"],"scope":module["scope"]}]);
    debt["affected_consumers"] = json!([consumer]);
    for status in ["CONFIRMED", "ACTIVE"] {
        debt["lifecycle"].as_array_mut().unwrap().push(json!({"status":status,"date":"2026-10-02","decision":"Synthetic validator fixture only; no production acceptance."}));
    }
    let schema = cost_l::schema(&root()).unwrap();
    assert!(cost_l::record_errors(&schema, &root(), debt, "COST-L-0001.json").is_empty());
    (inv, records, metadata())
}

#[test]
fn current_inventory_is_synthetic_and_actual_cli_passes() {
    let inv = inventory();
    assert_eq!(inv["modules"].as_array().unwrap().len(), 3);
    for module in inv["modules"].as_array().unwrap() {
        assert_eq!(module["classification"], "SYNTHETIC");
        assert_eq!(module["debt_ids"], json!([]));
        assert!(module["artifact"].is_null());
    }
    check(&inv, &records(), &metadata()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_repository-checks"))
        .arg("check-compatibility")
        .current_dir(root())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(env!("CARGO_BIN_EXE_repository-checks"))
        .args(["check-compatibility", "--skip-native"])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(!output.status.success());
}
#[test]
fn valid_production_relations_are_explicitly_test_fixtures() {
    let (mut i, mut r, m) = production();
    check(&i, &r, &m).unwrap();
    // Conversely, one debt can pin distinct contracts from the same package/artifact.
    i["modules"][1]["classification"] = json!("PRODUCTION");
    i["modules"][1]["artifact"] = i["modules"][0]["artifact"].clone();
    i["modules"][1]["debt_ids"] = json!(["COST-L-0001"]);
    i["modules"][1]["debt_unassigned_reason"] = Value::Null;
    let second = &i["modules"][1];
    r[0]["compat_modules"].as_array_mut().unwrap().push(json!({"id":second["id"],"semantic_version":second["semantic_version"],"scope":second["scope"],"artifact_sha256":second["artifact"]["sha256"]}));
    r[0]["affected_consumers"][0]["module_refs"].as_array_mut().unwrap().push(json!({"id":second["id"],"semantic_version":second["semantic_version"],"scope":second["scope"]}));
    check(&i, &r, &m).unwrap();
}
#[test]
fn missing_justification_and_synthetic_promotion_reject() {
    let mut i = inventory();
    i["modules"][0]["debt_unassigned_reason"] = Value::Null;
    reject(&i, &records(), &metadata(), "explain synthetic");
    let mut i = inventory();
    i["modules"][0]["debt_ids"] = json!(["COST-L-0001"]);
    reject(&i, &records(), &metadata(), "synthetic module");
    let (mut i, r, m) = production();
    i["modules"][0]["debt_ids"] = json!([]);
    reject(&i, &r, &m, "production module needs");
}
#[test]
fn closed_schema_bounds_versions_dates_and_identity_duplicates_reject() {
    for (field, value) in [
        ("semantic_version", json!("latest")),
        ("owner", json!("")),
        ("support_until", json!("2027-02-30")),
        ("extra", json!(true)),
    ] {
        let mut i = inventory();
        i["modules"][0][field] = value;
        assert!(check(&i, &records(), &metadata()).is_err());
    }
    let mut i = inventory();
    let mut duplicate = i["modules"][0].clone();
    duplicate["owner"] = json!("different fixture owner");
    i["modules"].as_array_mut().unwrap().push(duplicate);
    reject(&i, &records(), &metadata(), "duplicate module");
    let mut i = inventory();
    i["modules"] = json!(vec![i["modules"][0].clone(); 129]);
    assert!(check(&i, &records(), &metadata()).is_err());
}
#[test]
fn dangling_debts_exceptions_and_missing_reverse_manifests_reject() {
    let (mut i, r, m) = production();
    i["modules"][0]["debt_ids"] = json!(["COST-L-9999"]);
    reject(&i, &r, &m, "dangling debt");
    let mut i = inventory();
    i["modules"][0]["exception_ids"] = json!([format!("EXC-{:04}", 9999)]);
    reject(&i, &records(), &metadata(), "dangling exception");
    let (mut i, r, m) = production();
    i["modules"].as_array_mut().unwrap().remove(0);
    reject(&i, &r, &m, "no manifest for debt module");
}
#[test]
fn artifact_source_scope_and_versions_must_match() {
    let (mut i, r, m) = production();
    i["modules"][0]["artifact"]["sha256"] = json!("0".repeat(64));
    reject(&i, &r, &m, "artifact digest mismatch");
    let (mut i, r, m) = production();
    i["modules"][0]["scope"] = json!("other scope");
    reject(&i, &r, &m, "mismatched debt-to-module");
    let mut i = inventory();
    i["modules"][0]["package_version"] = json!("9.0.0");
    reject(&i, &records(), &metadata(), "package version mismatch");
    let mut i = inventory();
    i["modules"][0]["sources"][0]["sha256_lf"] = json!("0".repeat(64));
    reject(&i, &records(), &metadata(), "stale source");
    let mut i = inventory();
    i["modules"][0]["sources"].as_array_mut().unwrap().remove(1);
    reject(&i, &records(), &metadata(), "source binding omits");
    let mut i = inventory();
    i["modules"][0]["sources"][0]["path"] = json!("../outside");
    reject(&i, &records(), &metadata(), "relative repository file");
}
#[test]
fn consumers_are_bidirectional_and_deadlines_are_not_erased() {
    let (mut i, r, m) = production();
    i["modules"][0]["consumers"][0]["dependency"] = json!("TRANSITIVE");
    reject(&i, &r, &m, "missing reciprocal consumer");
    let (i, mut r, m) = production();
    let mut c = r[0]["affected_consumers"][0].clone();
    c["id"] = json!("offline-fixture");
    c["kind"] = json!("OFFLINE_RECOVERY");
    r[0]["affected_consumers"].as_array_mut().unwrap().push(c);
    reject(&i, &r, &m, "consumer absent from manifest");
    let mut i = inventory();
    i["modules"][0]["consumers"][0]["support_until"] = json!("2030-01-01");
    reject(&i, &records(), &metadata(), "consumer outlives support");
    let (mut i, r, m) = production();
    i["modules"][0]["support_until"] = json!("2030-01-01");
    reject(&i, &r, &m, "support scope mismatch");
}
#[test]
fn many_module_and_debt_edges_do_not_require_matching_owners() {
    let (mut i, mut r, m) = production();
    let mut second = r[0].clone();
    second["id"] = json!("COST-L-0002");
    second["support"]["owner"] = json!("separate fixture debt maintainer");
    r[1] = second;
    i["modules"][0]["debt_ids"] = json!(["COST-L-0001", "COST-L-0002"]);
    check(&i, &r, &m).unwrap();
}
#[test]
fn retired_and_research_only_debt_cannot_back_current_production() {
    for state in ["CANDIDATE", "CONFIRMED", "RETIRING", "RETIRED"] {
        let (i, mut r, m) = production();
        r[0]["status"] = json!(state);
        reject(&i, &r, &m, "debt lifecycle");
    }
    let (mut i, mut r, mut m) = production();
    i["modules"][0]["status"] = json!("RETIRED");
    r[0]["status"] = json!("RETIRED");
    r[0]["research_date"] = json!("2027-04-01");
    r[0]["support"]["status"] = json!("RETIRED");
    for status in ["DEPRECATED", "RETIRING", "RETIRED"] {
        r[0]["lifecycle"].as_array_mut().unwrap().push(json!({"status":status,"date":"2027-04-01","decision":"Synthetic retirement fixture; no real unloading."}));
    }
    let schema = cost_l::schema(&root()).unwrap();
    let errors = cost_l::record_errors(&schema, &root(), &r[0], "COST-L-0001.json");
    assert!(errors.is_empty(), "{errors:?}");
    m["packages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["name"] == "window-compat")
        .unwrap()["metadata"]["kolvrt"]["compatibility_modules"]
        .as_array_mut()
        .unwrap()
        .retain(|id| id != "window.inclusive/1.0.0");
    check(&i, &r, &m).unwrap();
}
#[test]
fn cargo_marker_and_feature_cannot_be_omitted_or_reused() {
    let mut m = metadata();
    let p = m["packages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["name"] == "window-compat")
        .unwrap();
    p["metadata"]["kolvrt"]["compatibility_modules"] = json!([]);
    reject(&inventory(), &records(), &m, "missing Cargo module marker");
    let mut i = inventory();
    i["modules"][0]["feature"] = json!("missing");
    reject(&i, &records(), &metadata(), "missing Cargo feature");
    let mut m = metadata();
    let p = m["packages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["name"] == "window-compat")
        .unwrap();
    p["metadata"]["kolvrt"]["compatibility_modules"]
        .as_array_mut()
        .unwrap()
        .push(json!("unlisted/1.0.0"));
    reject(
        &inventory(),
        &records(),
        &m,
        "lacks current matching manifest",
    );
}
#[test]
fn native_closure_rejects_optional_renamed_target_build_and_dev_edges() {
    let data = metadata();
    let dep = data["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "routing")
        .unwrap()["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "window-compat")
        .unwrap()
        .clone();
    for kind in [Value::Null, json!("build"), json!("dev")] {
        let mut m = data.clone();
        let mut d = dep.clone();
        d["kind"] = kind;
        d["optional"] = json!(true);
        d["rename"] = json!("harmless-name");
        d["target"] = json!("cfg(target_os = \"none\")");
        m["packages"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|p| p["name"] == "kernel-core")
            .unwrap()["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(d);
        reject(
            &inventory(),
            &records(),
            &m,
            "native closure reaches compatibility",
        );
    }
    let mut m = data.clone();
    m["packages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["name"] == "kernel-core")
        .unwrap()["dependencies"] = json!([{"name":"window-compat","path":null,"optional":true}]);
    reject(&inventory(), &records(), &m, "external/unresolved");
}

#[test]
fn real_cargo_parses_disabled_renamed_target_dependencies_before_rejection() {
    let path = std::env::temp_dir().join(format!("kolvrt-compat-cargo-{}", std::process::id()));
    fs::create_dir(&path).unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    let _cleanup = Cleanup(path.clone());
    fs::write(
        path.join("Cargo.toml"),
        "[workspace]\nmembers=['kernel','kernel-core','window-compat']\nresolver='3'\n",
    )
    .unwrap();
    for (folder, name) in [
        ("kernel", "kolvrt-kernel"),
        ("kernel-core", "kernel-core"),
        ("window-compat", "window-compat"),
    ] {
        fs::create_dir_all(path.join(folder).join("src")).unwrap();
        fs::write(
            path.join(folder).join("src/lib.rs"),
            "// Cargo declaration fixture; never compiled.\n",
        )
        .unwrap();
        let base = format!("[package]\nname='{name}'\nversion='0.1.0'\nedition='2024'\n");
        let extra = if folder == "kernel" {
            "[dependencies]\nkernel-core={path='../kernel-core'}\n[target.'cfg(target_os = \"none\")'.dependencies]\ncompat_alias={package='window-compat',path='../window-compat',optional=true}\n"
        } else {
            ""
        };
        fs::write(path.join(folder).join("Cargo.toml"), base + extra).unwrap();
    }
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["generate-lockfile", "--offline"])
        .current_dir(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let m = compat_modules::cargo_metadata(&path).unwrap();
    let dep = m["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "kolvrt-kernel")
        .unwrap()["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "window-compat")
        .unwrap();
    assert_eq!(dep["rename"], "compat_alias");
    assert_eq!(dep["optional"], true);
    assert!(!dep["target"].is_null());
    // Binding errors from this deliberately different workspace are expected too.
    reject(
        &inventory(),
        &records(),
        &m,
        "native closure reaches compatibility",
    );
}

#[test]
fn fixture_exception_cannot_extend_software_support() {
    let mut i = inventory();
    i["modules"][2]["support_until"] = json!("2028-01-01");
    reject(
        &i,
        &records(),
        &metadata(),
        "module outlives software exception",
    );
}
