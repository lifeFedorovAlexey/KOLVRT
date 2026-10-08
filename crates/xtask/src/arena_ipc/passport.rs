//! IPC population adapter for the shared schema-v1 custody and admission path.
use super::*;
use crate::arena_common::passport::{artifact, assess, retain_attempt, retain_review};
use crate::arena_common::{manifest_matches, value_digest};
const BASE: &str = "research/arena/ipc-query";
const CONTRACT: &str = "docs/research/ipc-measurement-contract.md";
const ORACLE: &str = "crates/xtask/src/arena_ipc.rs";
fn required(negative: bool) -> Vec<&'static str> {
    let mut paths = vec![
        ORACLE,
        "crates/xtask/src/arena_ipc/passport.rs",
        "crates/xtask/src/arena_ipc/analysis.rs",
        "crates/xtask/src/arena_common.rs",
        "crates/xtask/src/arena_common/passport.rs",
        "crates/xtask/src/native_apps.rs",
        "scripts/ci-evidence.cjs",
    ];
    if negative {
        paths.extend([
            "crates/repository-checks/src/arena.rs",
            "schemas/arena-run.schema.json",
            "scripts/tests/ci-infrastructure.test.cjs",
        ]);
    } else {
        paths.extend([
            "apps/native-runtime/src/lib.rs",
            "tests/native-apps/src/ipc-measure-root.rs",
            "tests/native-apps/src/ipc-measure-client.rs",
            "tests/native-apps/src/ipc-measure-peer.rs",
            "tests/native-apps/src/ipc_measure.rs",
            "tests/native-apps/src/ipc_measure_protocol.rs",
        ]);
    }
    paths
}
pub(super) fn paths() -> Vec<String> {
    let mut paths = vec![
        "research/arena/standards.json".into(),
        CONTRACT.into(),
        ORACLE.into(),
        "scripts/ci-evidence.cjs".into(),
    ];
    for name in [
        "protocol.json",
        "input.json",
        "resources.json",
        "dev-environment.json",
        "prod-environment.json",
    ] {
        paths.push(format!("{BASE}/{name}"));
    }
    for case in 0..12 {
        for profile in ["dev", "prod"] {
            for family in ["latency", "throughput"] {
                paths.push(format!(
                    "research/arena/profiles/ipc-query-{case}-{profile}-{family}.json"
                ));
            }
        }
    }
    paths
}
fn records(result: &Value) -> Result<Vec<&Value>> {
    Ok(result["records"]
        .as_array()
        .ok_or("IPC records absent")?
        .iter()
        .filter(|r| r["phase"] == 1)
        .collect())
}
pub(super) fn outcomes_match(off: &Value, on: &Value) -> Result<bool> {
    let left = records(off)?;
    let right = records(on)?;
    Ok(left.len() == right.len()
        && left.iter().zip(&right).all(|(a, b)| {
            a["request_id"] == b["request_id"]
                && a["stage"] == b["stage"]
                && a["status"] == b["status"]
        }))
}
fn overhead(profile: &Value, off: &Value, on: &Value) -> Result<Value> {
    let left = records(off)?;
    let right = records(on)?;
    let matched = outcomes_match(off, on)?;
    Ok(
        json!({"protocol_id":profile["overhead"]["id"],"protocol_version":profile["overhead"]["version"],"matched_workload_sha256":profile["overhead"]["matched_workload_sha256"],"state":if matched {"AVAILABLE"} else {"INCONCLUSIVE"},"reason":if matched {"All fixed offered request envelopes matched by request identity and observed outcome; recorder comparison includes failed offers, not pure syscall cost or an exact subtraction"}else{"Fixed offered workload retained, but outcome populations differ; comparison cannot isolate incremental recorder cost; no truncation or successful-only pairing"},"off_samples":left.iter().map(|r|r["wall_ticks"].clone()).collect::<Vec<_>>(),"on_samples":right.iter().map(|r|r["wall_ticks"].clone()).collect::<Vec<_>>(),"evidence":["pair.json"]}),
    )
}
fn observations(profile: &Value, result: &Value, family: &str) -> Result<Vec<Value>> {
    let losses = result["exhausted"].as_u64().ok_or("loss count absent")?
        + result["expired"].as_u64().ok_or("expiry count absent")?
        + result["other_errors"]
            .as_u64()
            .ok_or("error count absent")?;
    let (samples, warm) = if family == "latency" {
        (
            result["successful_wall_ticks"].clone(),
            result["warmup_successful_wall_ticks"].clone(),
        )
    } else {
        (
            json!([result["throughput"]["successful_operations_per_second"]]),
            json!([]),
        )
    };
    Ok(profile["metrics"].as_array().ok_or("metrics absent")?.iter().map(|metric| {
 if samples.as_array().is_none_or(|a|a.is_empty()) {json!({"metric_id":metric["id"],"state":"INCONCLUSIVE","reason":"No successful latency observations; all offered outcomes retained","missing_observations":profile["sampling"]["samples"],"evidence":["pair.json"]})}
 else {json!({"metric_id":metric["id"],"state":"AVAILABLE","samples":samples,"warmup_samples":warm,"failures":losses,"dropped":0,"unfinished":0})}
 }).collect())
}
fn checked_sources(value: &Value, required: &[&str]) -> Result<()> {
    let files = value
        .as_object()
        .filter(|f| !f.is_empty())
        .ok_or("source map absent")?;
    if required.iter().any(|p| !files.contains_key(*p)) {
        return Err("required security source absent".into());
    }
    for (path, hash) in files {
        let p = Path::new(path);
        if p.is_absolute()
            || p.components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("security source path unsafe".into());
        }
        if !fs::canonicalize(p)?.starts_with(fs::canonicalize(".")?)
            || hash != &digest(&fs::read(p)?)
        {
            return Err("security source mismatch".into());
        }
    }
    Ok(())
}
fn complete_matrix<'a>(data: &'a Value, expected: &Value) -> Result<&'a Value> {
    let matrix = if data.get("matrix_execution").is_some() {
        &data["matrix_execution"]
    } else {
        data
    };
    if matrix["schema_version"] != 2 {
        return Err("unsupported matrix receipt version".into());
    }
    if matrix["plan"] != *expected {
        return Err("matrix plan or current source inventory differs".into());
    }
    if matrix["shard_index"].is_number() || matrix["shard_count"].is_number() {
        return Err("partial shard is not full matrix evidence".into());
    }
    let tasks = expected["tasks"]
        .as_array()
        .ok_or("expected matrix tasks absent")?;
    let rows = matrix["completed"].as_array().ok_or("matrix rows absent")?;
    if tasks.is_empty() || tasks.len() != rows.len() {
        return Err("incomplete matrix inventory".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for row in rows {
        if row["outcome"] != "passed"
            || !ids.insert(row["id"].as_str().ok_or("matrix task id absent")?)
            || !tasks.iter().any(|t| t["id"] == row["id"])
        {
            return Err("failed, duplicate or unexpected matrix obligation".into());
        }
    }
    for profile in ["dev", "prod"] {
        if !tasks.iter().any(|t| t["profile"] == profile) {
            return Err("full matrix must cover DEV and PROD".into());
        }
    }
    Ok(matrix)
}
fn revalidate_matrix(path: &Path, expected: &Value, bundle: &Path) -> Result<()> {
    let plan = bundle.join("current-matrix-plan.json");
    write_json(&plan, expected)?;
    // Reuse the existing full profile/build/configuration/donor validator. The
    // aggregate retains task order and shard count, so the original partitions
    // can be reconstructed without treating separate invocations as one donor scope.
    let script = r#"const fs=require('node:fs');const {aggregate}=require('./scripts/ci-evidence.cjs');const data=JSON.parse(fs.readFileSync(process.argv[1],'utf8'));const m=data.matrix_execution||data;const plan=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));if(m.shard_index!=null||m.shard_count!=null)throw Error('partial matrix');const count=m.shards===undefined?1:m.shards;if(!Number.isInteger(count)||count<1||count>64)throw Error('invalid retained shard count');const shards=Array.from({length:count},(_,i)=>({schema_version:2,reuse_policy:m.reuse_policy||'same-invocation',plan:m.plan,shard_index:i,shard_count:count,completed:m.completed.filter(r=>plan.tasks.findIndex(t=>t.id===r.id)%count===i)}));aggregate(plan,shards,count);"#;
    let result = Command::new("node")
        .arg("-e")
        .arg(script)
        .arg(path)
        .arg(&plan)
        .output()?;
    if !result.status.success() {
        return Err(format!(
            "existing full matrix validator rejected evidence: {}",
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    Ok(())
}
fn executed_checks(data: &Value, expected: &Value) -> Result<std::collections::BTreeSet<String>> {
    let matrix = complete_matrix(data, expected)?;
    let mut actual = std::collections::BTreeSet::new();
    for row in matrix["completed"].as_array().unwrap() {
        if row["observation"] != "executed" {
            continue;
        }
        let task = expected["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == row["id"])
            .ok_or("matrix task absent")?;
        // The full matrix includes fatal/feature-specific tasks without ordinary
        // suite events. Their obligations were validated above; only the actual
        // unmodified ordinary DEV/PROD suites provide these named SFR witnesses.
        if task["tests"] != true || !task["flag"].is_null() || !task["feature"].is_null() {
            continue;
        }
        let profile = task["profile"].as_str().ok_or("matrix profile absent")?;
        let events = row["events"]
            .as_array()
            .ok_or("executed matrix events absent")?;
        let names: Vec<_> = events
            .iter()
            .filter(|e| e["event"] == "test")
            .filter_map(|e| e["name"].as_str())
            .collect();
        if output::validate(events, true, &names, 2).is_err() {
            continue;
        }
        for name in names {
            actual.insert(format!("{profile}:{name}"));
        }
    }
    Ok(actual)
}
pub(super) fn fixed_pair<'a>(rows: &[&'a Value], pair: usize) -> Result<(&'a Value, &'a Value)> {
    if rows.len() != 2 {
        return Err("fixed pair has missing or extra attempts".into());
    }
    for (position, row) in rows.iter().enumerate() {
        if row["order"] != position
            || row["mode"] != ORDERS[pair % 2][position]
            || row["status"] != "passed"
        {
            return Err("fixed pair order/mode/status mismatch".into());
        }
    }
    if rows[0]["mode"] == 1 {
        Ok((rows[0], rows[1]))
    } else {
        Ok((rows[1], rows[0]))
    }
}
pub(super) fn revalidated_attempt(attempt: &Value) -> bool {
    let checked = (|| -> Result<()> {
        let case = attempt["case"].as_u64().ok_or("attempt case absent")?;
        let mode = attempt["mode"].as_u64().ok_or("attempt mode absent")?;
        if attempt["argument"] != argument(case, mode)
            || attempt["run"]["elf_sha256"] != attempt["kernel"]["sha256"]
        {
            return Err("publication invocation identity mismatch".into());
        }
        let events = attempt["events"]
            .as_array()
            .ok_or("publication events absent")?;
        output::require_current_device_observation(events)?;
        let result = validate(events)?;
        crate::arena_common::validate_expected(&result, &[("case", case), ("mode", mode)])?;
        if result != attempt["result"] {
            return Err("publication result differs from raw events".into());
        }
        let directory = Path::new(
            attempt["directory"]
                .as_str()
                .ok_or("attempt directory absent")?,
        );
        if read_json(directory.join("attempt.json"))? != *attempt
            || read_json(directory.join("kernel.results.json"))? != attempt["events"]
            || read_json(directory.join("kernel.run.json"))? != attempt["run"]
            || read_json(directory.join("kernel-build.json"))? != attempt["kernel_build"]
        {
            return Err("campaign embedded data differs from retained raw attempt".into());
        }
        Ok(())
    })();
    checked.is_ok()
}
type WitnessCache =
    std::collections::BTreeMap<String, (String, std::collections::BTreeSet<String>)>;
fn kernel_evidence(
    bundle: &Path,
    sfr: &str,
    artifacts: &mut Vec<Value>,
    cache: &mut WitnessCache,
) -> Result<Vec<String>> {
    let path = format!("{BASE}/security-review.json");
    let v = read_json(&path)?;
    if v["schema_version"] != 1
        || v["status"] != "accepted"
        || v["reviewers"].as_array().is_none_or(|a| a.is_empty())
    {
        return Err("security review not accepted".into());
    }
    checked_sources(
        &v["source_files"],
        &[
            "crates/kernel-core/src/ipc.rs",
            "crates/kernel-core/src/handles.rs",
            "crates/kernel-core/src/domain.rs",
            "crates/kernel/src/ipc/native.rs",
        ],
    )?;
    let maps = v["mappings"].as_array().ok_or("security mappings absent")?;
    let selected: Vec<_> = maps
        .iter()
        .filter(|m| m["sfr_id"] == sfr && m["sar_id"] == "arena.sar.ipc.kernel-regression")
        .collect();
    if selected.len() != 1 {
        return Err("missing or duplicate security mapping".into());
    }
    let m = selected[0];
    if m["status"] != "PASS" || m["claim"].as_str().is_none_or(|s| s.trim().is_empty()) {
        return Err("security mapping not passed".into());
    }
    let checks = m["executed_checks"]
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or("executed checks absent")?;
    let paths = m["evidence_paths"]
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or("security evidence paths absent")?;
    let receipts = v["receipts"].as_array().ok_or("security receipts absent")?;
    let mut witnessed = std::collections::BTreeSet::new();
    let mut refs = Vec::new();
    for (i, p) in paths.iter().enumerate() {
        let p = p.as_str().ok_or("receipt path invalid")?;
        let matching: Vec<_> = receipts.iter().filter(|r| r["path"] == p).collect();
        if matching.len() != 1 {
            return Err("missing or duplicate receipt".into());
        }
        let receipt = matching[0];
        checked_sources(
            &receipt["source_files"],
            &[
                "crates/kernel-core/src/ipc.rs",
                "crates/kernel-core/src/handles.rs",
                "crates/kernel-core/src/domain.rs",
                "crates/kernel/src/ipc/native.rs",
            ],
        )?;
        let source = Path::new(p);
        if source.is_absolute()
            || source
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
            || !fs::canonicalize(source)?.starts_with(fs::canonicalize(".")?)
        {
            return Err("unsafe security receipt".into());
        }
        if !cache.contains_key(p) {
            let bytes = fs::read(source)?;
            let hash = digest(&bytes);
            let data: Value = serde_json::from_slice(&bytes)?;
            let expected = matrix::plan_document()?;
            complete_matrix(&data, &expected)?;
            revalidate_matrix(source, &expected, bundle)?;
            let actual = executed_checks(&data, &expected)?;
            cache.insert(p.to_owned(), (hash, actual));
        }
        let (hash, actual) = cache.get(p).unwrap();
        if receipt["sha256"] != *hash {
            return Err("security receipt digest mismatch".into());
        }
        for check in receipt["executed_checks"]
            .as_array()
            .ok_or("receipt executed checks absent")?
        {
            let name = check.as_str().ok_or("invalid executed check")?;
            if !actual.contains(&format!("dev:{name}")) || !actual.contains(&format!("prod:{name}"))
            {
                return Err(
                    "declared security check absent from actual executed matrix events".into(),
                );
            }
            witnessed.insert(name.to_owned());
        }
        let name = format!("security-{}-{i}.json", sfr.rsplit('.').next().unwrap());
        artifact(bundle, &name, source, artifacts)?;
        refs.push(name);
    }
    if checks
        .iter()
        .any(|c| c.as_str().is_none_or(|s| !witnessed.contains(s)))
    {
        return Err("security claim lacks executed witness".into());
    }
    artifact(bundle, "security-review.json", Path::new(&path), artifacts)?;
    refs.push("security-review.json".into());
    Ok(refs)
}
pub(super) fn write(root: &Path, campaign: &Value) -> Result<()> {
    if campaign["stage"] != "main" {
        return Err("pilot cannot be republished as preregistered main".into());
    }
    let frozen = &campaign["definition_snapshot"];
    if *frozen != crate::arena_common::passport::snapshot(&paths())?
        || campaign["source_files"] != source_inventory()?
    {
        return Err("publication definitions or current source differ from frozen campaign".into());
    }
    crate::arena_common::passport::retain_definitions(root, frozen)?;
    analysis::write(root, campaign)?;
    let attempts = campaign["attempts"].as_array().ok_or("attempts absent")?;
    let registry = &frozen["research/arena/standards.json"]["value"];
    let expected = campaign_plan(true);
    let schedule_valid = attempts.len() == expected.len()
        && attempts
            .iter()
            .zip(expected)
            .all(|(a, (case, prod, pair, position, mode))| {
                a["case"] == case
                    && a["profile"] == if prod { "prod" } else { "dev" }
                    && a["pair"] == pair
                    && a["order"] == position
                    && a["mode"] == mode
                    && a["argument"] == argument(case, mode)
            });
    let mut summaries = Vec::new();
    let mut witness_cache = WitnessCache::new();
    for case in 0..12 {
        for profile_name in ["dev", "prod"] {
            for pair in 0..12 {
                for family in ["latency", "throughput"] {
                    let key = format!("case-{case}-{profile_name}-{pair}-{family}");
                    let bundle = root.join("passports").join(&key);
                    fs::create_dir_all(&bundle)?;
                    let rows: Vec<_> = attempts
                        .iter()
                        .filter(|a| {
                            a["case"] == case && a["profile"] == profile_name && a["pair"] == pair
                        })
                        .collect();
                    write_json(
                        bundle.join("pair.json"),
                        &json!({"case":case,"profile":profile_name,"pair":pair,"attempts":rows}),
                    )?;
                    let (off, on) = match fixed_pair(&rows, pair) {
                        Ok(pair) => pair,
                        Err(error) => {
                            let assessment = json!({"admission_state":"INELIGIBLE","record_eligible":false,"reason":error.to_string()});
                            write_json(bundle.join("assessment.json"), &assessment)?;
                            summaries.push(json!({"key":key,"assessment":assessment}));
                            continue;
                        }
                    };
                    let profile = &frozen[format!(
                        "research/arena/profiles/ipc-query-{case}-{profile_name}-{family}.json"
                    )]["value"];
                    let manifest_path = format!("{BASE}/{profile_name}-environment.json");
                    let manifest = &frozen[&manifest_path]["value"];
                    let valid = schedule_valid
                        && campaign["functional_status"] == "passed"
                        && campaign["source_unchanged"] == true
                        && revalidated_attempt(off)
                        && revalidated_attempt(on)
                        && manifest_matches(off, manifest)
                        && manifest_matches(on, manifest)
                        && campaign["compiler_version"]
                            .as_str()
                            .and_then(|s| s.split_whitespace().nth(1))
                            == manifest["compiler_version"].as_str()
                        && profile["target"]["contract_sha256"] == frozen[CONTRACT]["sha256_lf"]
                        && profile["workload"]["oracle"]["sha256"] == frozen[ORACLE]["sha256_lf"]
                        && profile["workload"]["semantics_sha256"]
                            == frozen[format!("{BASE}/protocol.json")]["sha256"]
                        && profile["overhead"]["matched_workload_sha256"]
                            == profile["workload"]["semantics_sha256"]
                        && profile["workload"]["input_sha256"]
                            == frozen[format!("{BASE}/input.json")]["sha256"]
                        && profile["environment"]["configuration_sha256"]
                            == frozen[&manifest_path]["sha256"]
                        && profile["environment"]["resource_policy_sha256"]
                            == frozen[format!("{BASE}/resources.json")]["sha256"];
                    let mut artifacts = Vec::new();
                    artifact(
                        &bundle,
                        "paired-analysis.json",
                        &root.join("paired-analysis.json"),
                        &mut artifacts,
                    )?;
                    artifact(
                        &bundle,
                        "pair.json",
                        &bundle.join("pair.json"),
                        &mut artifacts,
                    )?;
                    let source_hash = artifact(
                        &bundle,
                        "source-files.json",
                        &root.join("source-files.json"),
                        &mut artifacts,
                    )?;
                    let env_hash = artifact(
                        &bundle,
                        "environment.json",
                        &root.join("definitions").join(&manifest_path),
                        &mut artifacts,
                    )?;
                    for (name, path) in [
                        ("protocol.json", format!("{BASE}/protocol.json")),
                        ("input.json", format!("{BASE}/input.json")),
                        ("resources.json", format!("{BASE}/resources.json")),
                        ("contract.md", CONTRACT.into()),
                        ("oracle.rs", ORACLE.into()),
                    ] {
                        artifact(
                            &bundle,
                            name,
                            &root.join("definitions").join(path),
                            &mut artifacts,
                        )?;
                    }
                    let image_hash = retain_attempt(&bundle, "off", off, &mut artifacts)?;
                    retain_attempt(&bundle, "on", on, &mut artifacts)?;
                    let source =
                        retain_review(&bundle, BASE, false, &required(false), &mut artifacts)?;
                    let negative =
                        retain_review(&bundle, BASE, true, &required(true), &mut artifacts)?;
                    let mut security = Vec::new();
                    for sfr in profile["security"]["sfr"].as_array().ok_or("SFR absent")? {
                        let mut sars = Vec::new();
                        for sar in sfr["required_sar"].as_array().ok_or("SAR absent")? {
                            let evidence = match sar.as_str() {
                                Some("arena.sar.ipc.source-review") => source.clone(),
                                Some("arena.sar.ipc.negative-inputs") => negative.clone(),
                                Some("arena.sar.ipc.workload-execution") => {
                                    vec!["pair.json".into()]
                                }
                                Some("arena.sar.ipc.kernel-regression") => match kernel_evidence(
                                    &bundle,
                                    sfr["id"].as_str().ok_or("SFR ID absent")?,
                                    &mut artifacts,
                                    &mut witness_cache,
                                ) {
                                    Ok(e) => e,
                                    Err(error) => {
                                        write_json(
                                            bundle.join(format!(
                                                "{}-unavailable.json",
                                                sfr["id"].as_str().unwrap()
                                            )),
                                            &json!({"reason":error.to_string()}),
                                        )?;
                                        vec![]
                                    }
                                },
                                _ => vec![],
                            };
                            sars.push(json!({"sar_id":sar,"status":if evidence.is_empty(){"NOT_RUN"}else if valid{"PASS"}else{"FAIL"},"reason":"Only source-bound reviewed evidence supports this scoped claim; absent evidence never inferred from ordinary workload success","evidence":evidence}));
                        }
                        security.push(json!({"sfr_id":sfr["id"],"status":if sars.iter().all(|v|v["status"]=="PASS"){"PASS"}else{"NOT_RUN"},"reason":"Bounded current-source evidence; not certification or omitted-operation fault injection","sar_results":sars}));
                    }
                    let status = if valid { "PASS" } else { "FAIL" };
                    let run = json!({"schema_version":1,"run_id":format!("ipc-{key}-{}",root.file_name().unwrap().to_string_lossy()),"scope":"KERNEL_QEMU","profile_snapshot":profile,"registry_snapshot":registry,"profile_sha256":value_digest(profile)?,"registry_sha256":value_digest(registry)?,"provenance":{"exact_commit":campaign["exact_commit"],"source_sha256":source_hash,"image_sha256":image_hash,"submission":"Fixed-offer real IPC main campaign; raw source inventory identifies compiled worktree; no independent attestation","contribution_state":"UNATTRIBUTED","contributors":[],"contribution_evidence":[]},"environment_manifest":{"path":"environment.json","sha256":env_hash},"artifacts":artifacts,"correctness":{"status":status,"reason":"Complete actual workload ledger and immutable environment checked; correct capacity refusal is not useful success","evidence":["pair.json"]},"security_results":security,"observations":observations(profile,&off["result"],family)?,"overhead_result":overhead(profile,&off["result"],&on["result"] )?,"warmup":{"status":status,"reason":"Actual request conditioning phase retained; throughput profile has no separate aggregate warmup epoch; stabilization not asserted","evidence":["pair.json"]}});
                    let entry = assess(root, &bundle, &key, run)?;
                    summaries.push(
                        json!({"key":key,"run":entry["run"],"assessment":entry["assessment"]}),
                    );
                }
            }
        }
    }
    for (path, (hash, _)) in &witness_cache {
        if digest(&fs::read(path)?) != *hash {
            return Err("security receipt changed during publication".into());
        }
    }
    let rejected = summaries
        .iter()
        .filter(|s| s["assessment"]["admission_state"] != "STRUCTURALLY_ADMISSIBLE")
        .count();
    write_json(
        root.join("passport-summary.json"),
        &json!({"schema_version":1,"record_eligible":false,"admission_state":if rejected==0{"STRUCTURALLY_ADMISSIBLE"}else{"INELIGIBLE"},"ineligible":rejected,"runs":summaries}),
    )?;
    if rejected != 0 {
        return Err(format!(
            "IPC admission INELIGIBLE for {rejected} run families; complete retained summary {}",
            root.join("passport-summary.json").display()
        )
        .into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_rejects_relabeling_raw_mismatch_and_missing_current_device() {
        let root = env::temp_dir().join(format!(
            "kolvrt-ipc-publication-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let words = crate::arena_ipc::tests::fixture();
        let d = kernel_core::platform::discover(include_bytes!(
            "../../../../research/fixtures/virt-10.1.dtb"
        ))
        .unwrap()
        .boot_console()
        .unwrap();
        let mut events = vec![
            json!({"event":"device-observation","status":"pass","kind":"pl011","reservation":"boot-console","scope":1,"generation":1,"mmio_base":d.mmio_base(),"mmio_size":d.mmio_size(),"irq":d.irq(),"interrupt_controller":d.interrupt_controller()}),
            json!({"event":"native-root-image","format":"elf64","slot":0,"generation":1}),
        ];
        for (index, chunk) in words.chunks(64).enumerate() {
            events.push(json!({"event":"native-user-report-chunk","version":2,"slot":0,"generation":1,"offset":index*64,"total":words.len(),"words":chunk}));
        }
        events.push(json!({"event":"native-user-report-end","version":2,"slot":0,"generation":1,"total":words.len()}));
        events.push(json!({"event":"native-boot","status":"complete","root_exit":0,"owners_released":true,"frames_restored":true,"live_processes":0,"live_domains":0}));
        let result = validate(&events).unwrap();
        let a = json!({"case":0,"mode":1,"argument":argument(0,1),"directory":root,"kernel":{"sha256":"hash"},"run":{"elf_sha256":"hash"},"kernel_build":{},"events":events,"result":result});
        write_json(root.join("attempt.json"), &a).unwrap();
        write_json(root.join("kernel.results.json"), &a["events"]).unwrap();
        write_json(root.join("kernel.run.json"), &a["run"]).unwrap();
        write_json(root.join("kernel-build.json"), &a["kernel_build"]).unwrap();
        assert!(revalidated_attempt(&a));
        for (key, value) in [("case", 1), ("mode", 2)] {
            let mut changed = a.clone();
            changed[key] = json!(value);
            changed["argument"] = json!(argument(
                changed["case"].as_u64().unwrap(),
                changed["mode"].as_u64().unwrap()
            ));
            assert!(!revalidated_attempt(&changed));
        }
        write_json(
            root.join("kernel.run.json"),
            &json!({"elf_sha256":"changed"}),
        )
        .unwrap();
        assert!(!revalidated_attempt(&a));
        write_json(root.join("kernel.run.json"), &a["run"]).unwrap();
        let mut missing = a.clone();
        missing["events"].as_array_mut().unwrap().remove(0);
        assert!(!revalidated_attempt(&missing));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn pair_order_missing_and_duplicates_cannot_be_published() {
        let a = json!({"order":0,"mode":1,"status":"passed"});
        let b = json!({"order":1,"mode":2,"status":"passed"});
        assert!(fixed_pair(&[&a, &b], 0).is_ok());
        assert!(fixed_pair(&[&a, &b], 1).is_err());
        assert!(fixed_pair(&[&a], 0).is_err());
        assert!(fixed_pair(&[&a, &a], 0).is_err());
    }
    #[test]
    fn security_witness_requires_complete_current_dev_and_prod_matrix() {
        let plan = json!({"source_files":[{"path":"actual.rs","sha256_lf":"hash"}],"tasks":[{"id":"dev-tests","profile":"dev","tests":true},{"id":"prod-tests","profile":"prod","tests":true},{"id":"dev-fatal","profile":"dev","tests":true,"feature":"panic","flag":"--panic"}]});
        let events = json!([{"event":"test","name":"real_check","status":"pass"},{"event":"suite","tests":1,"status":"pass"}]);
        let receipt = json!({"matrix_execution":{"schema_version":2,"plan":plan,"completed":[{"id":"dev-tests","outcome":"passed","observation":"executed","events":events},{"id":"prod-tests","outcome":"passed","observation":"executed","events":events},{"id":"dev-fatal","outcome":"passed","observation":"executed"}]}});
        let names = executed_checks(&receipt, &plan).unwrap();
        assert!(names.contains("dev:real_check"));
        assert!(names.contains("prod:real_check"));
        let mut version = receipt.clone();
        version["matrix_execution"]["schema_version"] = json!(99);
        assert!(executed_checks(&version, &plan).is_err());
        version["matrix_execution"]
            .as_object_mut()
            .unwrap()
            .remove("schema_version");
        assert!(executed_checks(&version, &plan).is_err());
        let mut missing = receipt.clone();
        missing["matrix_execution"]["completed"]
            .as_array_mut()
            .unwrap()
            .remove(1);
        assert!(executed_checks(&missing, &plan).is_err());
        let mut failed = receipt.clone();
        failed["matrix_execution"]["completed"][1]["outcome"] = json!("failed");
        assert!(executed_checks(&failed, &plan).is_err());
        let mut partial = receipt.clone();
        partial["matrix_execution"]["shard_index"] = json!(0);
        partial["matrix_execution"]["shard_count"] = json!(4);
        assert!(executed_checks(&partial, &plan).is_err());
        let mut stale = plan.clone();
        stale["source_files"] = json!([]);
        assert!(executed_checks(&receipt, &stale).is_err());
        let mut forged = receipt.clone();
        forged["matrix_execution"]["completed"][1]["observation"] = json!("reused");
        assert!(
            !executed_checks(&forged, &plan)
                .unwrap()
                .contains("prod:real_check")
        );
    }
    #[test]
    fn unequal_outcomes_do_not_truncate_observer_population() {
        let p = json!({"overhead":{"id":"a.b","version":"1","matched_workload_sha256":"hash"}});
        let a = json!({"records":[{"phase":1,"request_id":1,"stage":0,"status":0,"wall_ticks":42},{"phase":1,"request_id":2,"stage":1,"status":12,"wall_ticks":3}]});
        let mut b = a.clone();
        b["records"][1]["stage"] = json!(0);
        b["records"][1]["status"] = json!(0);
        let v = overhead(&p, &a, &b).unwrap();
        assert_eq!(v["state"], "INCONCLUSIVE");
        assert_eq!(v["off_samples"].as_array().unwrap().len(), 2);
        assert_eq!(overhead(&p, &a, &a).unwrap()["state"], "AVAILABLE");
    }
    #[test]
    fn throughput_is_one_actual_phase_and_losses_are_not_hidden() {
        let p = json!({"metrics":[{"id":"a.b"}]});
        let r = json!({"exhausted":12,"expired":0,"other_errors":0,"successful_wall_ticks":[5,6],"warmup_successful_wall_ticks":[7],"throughput":{"successful_operations_per_second":2.5}});
        let t = observations(&p, &r, "throughput").unwrap();
        assert_eq!(t[0]["samples"], json!([2.5]));
        assert_eq!(t[0]["warmup_samples"], json!([]));
        assert_eq!(t[0]["failures"], 12);
        let l = observations(&p, &r, "latency").unwrap();
        assert_eq!(l[0]["samples"], json!([5, 6]));
        assert_eq!(l[0]["failures"], 12);
    }
}
