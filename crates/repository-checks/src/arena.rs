//! Offline Arena contracts and import admission. Artifact integrity is not execution attestation.
use crate::{CheckResult, knowledge::Knowledge, parse_json, read_json};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};

pub mod report;

const MAX_INPUT_BYTES: u64 = 32 * 1024 * 1024;

/// Canonical JSON digest (serde_json's ordered object maps), independent of formatting.
/// Raw evidence uses byte digests instead; never normalize binary or observation bytes.
pub fn digest(value: &Value) -> String {
    bytes_digest(
        serde_json::to_string(value)
            .expect("JSON Value serialization")
            .as_bytes(),
    )
}
pub fn bytes_digest(bytes: &[u8]) -> String {
    (Sha256::digest(bytes))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn input(path: &Path) -> CheckResult<Value> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_INPUT_BYTES {
        return Err("Arena input exceeds 32 MiB".into());
    }
    parse_json(&fs::read_to_string(path).map_err(|e| e.to_string())?)
}
fn schema(root: &Path, name: &str, value: &Value) -> CheckResult<()> {
    let schema = read_json(&root.join(format!("schemas/arena-{name}.schema.json")))?;
    jsonschema::meta::validate(&schema).map_err(|e| e.to_string())?;
    jsonschema::draft202012::options()
        .build(&schema)
        .map_err(|e| e.to_string())?
        .validate(value)
        .map_err(|e| format!("Arena {name} schema: {e}"))
}
fn s<'a>(v: &'a Value, field: &str) -> &'a str {
    v[field].as_str().expect("validated string")
}
fn a<'a>(v: &'a Value, field: &str) -> &'a [Value] {
    v[field].as_array().expect("validated array")
}
fn unique<'a>(items: &'a [Value], key: &str) -> CheckResult<BTreeMap<&'a str, &'a Value>> {
    let mut map = BTreeMap::new();
    for item in items {
        if map.insert(s(item, key), item).is_some() {
            return Err(format!("duplicate {key}: {}", s(item, key)));
        }
    }
    Ok(map)
}
fn valid_date(text: &str) -> bool {
    let parts: Vec<_> = text
        .split('-')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();
    if parts.len() != 3 {
        return false;
    }
    let (year, month, day) = (parts[0], parts[1], parts[2]);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => 0,
    };
    day > 0 && day <= days
}
pub fn validate_registry(root: &Path, registry: &Value) -> CheckResult<()> {
    schema(root, "standards", registry)?;
    let mut versions = BTreeMap::new();
    for entry in a(registry, "entries") {
        let key = (s(entry, "id"), s(entry, "version"));
        if versions.insert(key, entry).is_some() {
            return Err("duplicate registry ID/version".into());
        }
        if !valid_date(s(entry, "last_checked")) {
            return Err("invalid registry verification date".into());
        }
        let url = s(entry, "authoritative_source");
        let kind = s(entry, "source_kind");
        if s(entry, "title").starts_with("ISO/IEC") && kind != "INTERNATIONAL_STANDARD" {
            return Err(
                "ISO/IEC source cannot be represented as a practice or other source kind".into(),
            );
        }
        match kind {
            "INTERNATIONAL_STANDARD" if !url.starts_with("https://www.iso.org/") => {
                return Err("international standard requires ISO authoritative source".into());
            }
            "GOVERNMENT_GUIDANCE" if !url.starts_with("https://csrc.nist.gov/") => {
                return Err("government guidance requires NIST authoritative source in v1".into());
            }
            "DE_FACTO"
                if url.starts_with("https://www.iso.org/")
                    || url.starts_with("https://csrc.nist.gov/") =>
            {
                return Err("de-facto practice cannot masquerade as ISO/NIST".into());
            }
            "KOLVRT_DEFINED" if !url.starts_with("docs/") => {
                return Err(
                    "KOLVRT-defined methodology requires an explicit repository contract".into(),
                );
            }
            _ => {}
        }
        if kind != "KOLVRT_DEFINED" && !url.starts_with("https://") {
            return Err("external source must use HTTPS".into());
        }
        if url.contains("/latest/") || url.contains("/master/") || url.contains("/main/") {
            return Err("floating external source requires an immutable source locator".into());
        }
    }
    for (key, entry) in &versions {
        let mut visited = BTreeSet::new();
        let mut current = (*key, *entry);
        while !current.1["superseded_by"].is_null() {
            if !visited.insert(current.0) {
                return Err("cyclic standard supersession".into());
            }
            let next = &current.1["superseded_by"];
            let next_key = (s(next, "id"), s(next, "version"));
            let successor = *versions
                .get(&next_key)
                .ok_or("missing superseded-by registry version")?;
            if successor["publication_state"] != "PUBLISHED" {
                return Err("draft cannot supersede a published standard".into());
            }
            current = (next_key, successor);
        }
    }
    Ok(())
}
fn methodology<'a>(registry: &'a Value, reference: &Value) -> CheckResult<&'a Value> {
    a(registry, "entries")
        .iter()
        .find(|entry| entry["id"] == reference["id"] && entry["version"] == reference["version"])
        .filter(|entry| entry["publication_state"] == "PUBLISHED")
        .ok_or_else(|| {
            format!(
                "unregistered or draft methodology: {} / {}",
                reference["id"], reference["version"]
            )
        })
}
pub fn validate_profile(root: &Path, registry: &Value, profile: &Value) -> CheckResult<()> {
    validate_registry(root, registry)?;
    schema(root, "profile", profile)?;
    validate_profile_semantics(registry, profile)
}
fn validate_profile_semantics(registry: &Value, profile: &Value) -> CheckResult<()> {
    let metrics = unique(a(profile, "metrics"), "id")?;
    let security = &profile["security"];
    let sfr = unique(a(security, "sfr"), "id")?;
    let sar = unique(a(security, "sar"), "id")?;
    for requirement in sfr.values() {
        for needed in a(requirement, "required_sar") {
            if !sar.contains_key(needed.as_str().unwrap()) {
                return Err("SFR references undefined SAR".into());
            }
        }
    }
    for reference in a(security, "methodologies") {
        methodology(registry, reference)?;
    }
    for metric in metrics.values() {
        for reference in a(metric, "methodologies") {
            let source = methodology(registry, reference)?;
            if source["source_kind"] == "KOLVRT_DEFINED" && metric["origin"] != "KOLVRT_DEFINED" {
                return Err("own measurement must be marked KOLVRT_DEFINED".into());
            }
        }
        if metric["minimum_samples"].as_u64() > profile["sampling"]["samples"].as_u64() {
            return Err("profile sample plan cannot satisfy metric minimum".into());
        }
        if metric["visibility"] == "DEV_INTERNAL"
            && (profile["environment"]["execution_profile"] != "DEV"
                || profile["environment"]["measurement"]["instrumentation"] != "DEV_INTERNAL")
        {
            return Err("DEV_INTERNAL requires a DEV internal measurement mechanism".into());
        }
    }
    if profile["environment"]["execution_profile"] == "PROD"
        && profile["environment"]["measurement"]["instrumentation"] != "EXTERNAL"
    {
        return Err("PROD cannot contain Arena internal instrumentation".into());
    }
    Ok(())
}
fn canonical_targets(root: &Path, profile: &Value) -> CheckResult<()> {
    let knowledge = Knowledge::build(root)?;
    let nodes = knowledge.graph["nodes"]
        .as_object()
        .ok_or("missing canonical graph nodes")?;
    for key in ["block_id", "contract_id"] {
        let target = s(&profile["target"], key);
        let node = nodes
            .get(target)
            .ok_or_else(|| format!("unknown canonical Arena {key}: {target}"))?;
        let kinds = if key == "block_id" {
            &["feature", "subsystem-contract"][..]
        } else {
            &["contract-section", "api-contract", "subsystem-contract"][..]
        };
        if !kinds.contains(&s(node, "kind")) {
            return Err(format!("invalid canonical target kind: {target}"));
        }
    }
    let contract = &nodes[s(&profile["target"], "contract_id")];
    if profile["target"]["contract_sha256"] != contract["locations"]["en"]["sha256"] {
        return Err(
            "proposed profile contract digest differs from current canonical document".into(),
        );
    }
    Ok(())
}
/// Conservative v1 class: exact declared semantics and settings. Artifact/source
/// digests reside in run provenance and never silently redefine the contract.
pub fn comparison_class(profile: &Value) -> String {
    let mut contract = profile.clone();
    contract
        .as_object_mut()
        .expect("profile object")
        .remove("review_state");
    digest(&contract)
}
pub fn compatible(
    root: &Path,
    left_registry: &Value,
    left: &Value,
    right_registry: &Value,
    right: &Value,
) -> CheckResult<()> {
    validate_profile(root, left_registry, left)?;
    validate_profile(root, right_registry, right)?;
    if comparison_class(left) != comparison_class(right) {
        return Err("incompatible Arena comparison classes".into());
    }
    // Version labels cannot hide an edited historical source definition.
    for reference in a(&left["security"], "methodologies").iter().chain(
        a(left, "metrics")
            .iter()
            .flat_map(|m| a(m, "methodologies")),
    ) {
        if methodology(left_registry, reference)? != methodology(right_registry, reference)? {
            return Err("same methodology version has different frozen definitions".into());
        }
    }
    Ok(())
}
fn evidence_path(base: &Path, relative: &str) -> CheckResult<std::path::PathBuf> {
    if relative.is_empty()
        || relative.contains(['\\', ':'])
        || Path::new(relative)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("unsafe evidence path".into());
    }
    let base = base.canonicalize().map_err(|e| e.to_string())?;
    let path = base
        .join(relative)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !path.starts_with(&base) || !path.is_file() {
        return Err("evidence escapes bundle".into());
    }
    Ok(path)
}
fn evidence_refs(refs: &[Value], artifacts: &BTreeMap<&str, &Value>) -> CheckResult<()> {
    for reference in refs {
        if !artifacts.contains_key(reference.as_str().unwrap()) {
            return Err("evidence reference absent from digest-verified artifacts".into());
        }
    }
    Ok(())
}
/// Validate an immutable self-contained import using its own frozen definitions.
/// This never turns a producer assertion into independent attestation or records.
pub fn assess(root: &Path, bundle: &Path, run: &Value) -> CheckResult<Value> {
    schema(root, "run", run)?;
    let profile = &run["profile_snapshot"];
    let registry = &run["registry_snapshot"];
    validate_profile(root, registry, profile)?;
    if run["profile_sha256"] != digest(profile) || run["registry_sha256"] != digest(registry) {
        return Err("frozen profile/registry digest mismatch".into());
    }
    let artifacts = unique(a(run, "artifacts"), "path")?;
    for artifact in artifacts.values() {
        let path = evidence_path(bundle, s(artifact, "path"))?;
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > MAX_INPUT_BYTES {
            return Err("evidence artifact exceeds v1 limit".into());
        }
        if artifact["sha256"] != bytes_digest(&fs::read(path).map_err(|e| e.to_string())?) {
            return Err("evidence artifact byte digest mismatch".into());
        }
    }
    let environment = &run["environment_manifest"];
    if artifacts.get(s(environment, "path")).copied() != Some(environment)
        || environment["sha256"] != profile["environment"]["configuration_sha256"]
    {
        return Err("environment manifest digest/bundle mismatch".into());
    }
    for field in ["source_sha256", "image_sha256"] {
        if !artifacts
            .values()
            .any(|v| v["sha256"] == run["provenance"][field])
        {
            return Err(format!("missing exact {field} artifact"));
        }
    }
    evidence_refs(a(&run["correctness"], "evidence"), &artifacts)?;
    evidence_refs(a(&run["provenance"], "contribution_evidence"), &artifacts)?;
    if run["provenance"]["contribution_state"] == "ACCEPTED"
        && (a(&run["provenance"], "contributors").is_empty()
            || a(&run["provenance"], "contribution_evidence").is_empty())
    {
        return Err(
            "accepted contribution requires credited contributors and linked evidence".into(),
        );
    }
    evidence_refs(a(&run["warmup"], "evidence"), &artifacts)?;
    let required_scope = match s(&profile["environment"], "platform_kind") {
        "HOST_FIXTURE" => "SYNTHETIC_FIXTURE",
        "QEMU" => "KERNEL_QEMU",
        _ => "KERNEL_PHYSICAL_HARDWARE",
    };
    if s(run, "scope") != required_scope {
        return Err("run scope does not match environment platform".into());
    }
    let mut reasons = Vec::<String>::new();
    if run["correctness"]["status"] != "PASS" || a(&run["correctness"], "evidence").is_empty() {
        reasons.push("correctness_not_verified".into());
    }
    if profile["review_state"] != "REVIEWED" {
        reasons.push("profile_not_reviewed".into());
    }
    if run["scope"] == "SYNTHETIC_FIXTURE" {
        reasons.push("synthetic_fixture_not_kernel_evidence".into());
    }
    if run["warmup"]["status"] != "PASS" || a(&run["warmup"], "evidence").is_empty() {
        reasons.push("warmup_not_validated".into());
    }
    let security = &profile["security"];
    let requirements = unique(a(security, "sfr"), "id")?;
    let declared_sar = unique(a(security, "sar"), "id")?;
    let results = unique(a(run, "security_results"), "sfr_id")?;
    for (id, result) in &results {
        let requirement = requirements.get(id).ok_or("unknown SFR result")?;
        let sar_results = unique(a(result, "sar_results"), "sar_id")?;
        for (sar_id, evidence) in &sar_results {
            if !declared_sar.contains_key(sar_id)
                || !a(requirement, "required_sar").iter().any(|v| v == *sar_id)
            {
                return Err("unrelated SAR evidence cannot satisfy SFR".into());
            }
            evidence_refs(a(evidence, "evidence"), &artifacts)?;
        }
    }
    let mut verified = 0;
    let mut failed = 0;
    let mut not_evaluated = 0;
    for (id, requirement) in &requirements {
        let result = results.get(id);
        let pass = result.is_some_and(|r| {
            r["status"] == "PASS"
                && a(requirement, "required_sar").iter().all(|needed| {
                    a(r, "sar_results").iter().any(|e| {
                        e["sar_id"] == *needed
                            && e["status"] == "PASS"
                            && !a(e, "evidence").is_empty()
                    })
                })
        });
        if pass {
            verified += 1;
        } else if result.is_some_and(|r| {
            r["status"] == "FAIL" || a(r, "sar_results").iter().any(|e| e["status"] == "FAIL")
        }) {
            failed += 1;
        } else {
            not_evaluated += 1;
        }
        if requirement["mandatory"] == true && !pass {
            reasons.push(format!("mandatory_sfr_not_verified:{id}"));
        }
    }
    let metrics = unique(a(profile, "metrics"), "id")?;
    let observations = unique(a(run, "observations"), "metric_id")?;
    if metrics.keys().collect::<Vec<_>>() != observations.keys().collect::<Vec<_>>() {
        return Err("run must account for every metric exactly once".into());
    }
    let mut summaries = BTreeMap::new();
    for (id, metric) in &metrics {
        let observation = observations[id];
        if observation["state"] != "AVAILABLE" {
            evidence_refs(a(observation, "evidence"), &artifacts)?;
            reasons.push(format!(
                "metric_not_available:{id}:{}",
                s(observation, "state")
            ));
            summaries.insert(*id, observation.clone());
            continue;
        }
        let samples = a(observation, "samples");
        let warmup = a(observation, "warmup_samples");
        if samples.len() != profile["sampling"]["samples"].as_u64().unwrap() as usize
            || samples.len() < metric["minimum_samples"].as_u64().unwrap() as usize
            || warmup.len() != profile["sampling"]["warmup_samples"].as_u64().unwrap() as usize
        {
            reasons.push(format!("sample_plan_not_satisfied:{id}"));
        }
        for field in ["failures", "dropped", "unfinished"] {
            if observation[field] != 0 {
                reasons.push(format!("{field}:{id}"));
            }
        }
        let mut values: Vec<f64> = samples.iter().map(|x| x.as_f64().unwrap()).collect();
        values.sort_by(f64::total_cmp);
        let value = match s(metric, "statistic") {
            "COUNT" => values.len() as f64,
            "MEAN" => values.iter().map(|v| v / values.len() as f64).sum(),
            stat => {
                let p = match stat {
                    "MEDIAN" => 50,
                    "P95" => 95,
                    _ => 99,
                };
                values[(values.len() * p).div_ceil(100) - 1]
            }
        };
        if !value.is_finite() {
            return Err("non-finite metric summary".into());
        }
        summaries.insert(*id,json!({"state":"AVAILABLE","value":value,"units":metric["units"],"denominator":metric["denominator"],"sample_count":samples.len(),"methodologies":metric["methodologies"]}));
    }
    let overhead = &run["overhead_result"];
    let protocol = &profile["overhead"];
    if overhead["protocol_id"] != protocol["id"]
        || overhead["protocol_version"] != protocol["version"]
        || overhead["matched_workload_sha256"] != protocol["matched_workload_sha256"]
    {
        return Err("matched overhead protocol mismatch".into());
    }
    evidence_refs(a(overhead, "evidence"), &artifacts)?;
    if overhead["state"] != "AVAILABLE"
        || a(overhead, "on_samples").is_empty()
        || a(overhead, "off_samples").is_empty()
        || a(overhead, "on_samples").len() != a(overhead, "off_samples").len()
        || a(overhead, "evidence").is_empty()
    {
        reasons.push("matched_observer_cost_missing".into());
    }
    Ok(
        json!({"run_id":run["run_id"],"comparison_class":comparison_class(profile),"admission_state":if reasons.is_empty(){"STRUCTURALLY_ADMISSIBLE"}else{"INELIGIBLE"},"reasons":reasons,"security":{"sfr":requirements.len(),"verified":verified,"failed":failed,"not_evaluated":not_evaluated},"measurements":summaries,"record_eligible":false,"limitations":"Offline integrity and producer-asserted evidence only. Independent review, execution/source applicability and record publication are not implemented; no regression or certification claim."}),
    )
}
pub fn check(root: &Path) -> CheckResult<()> {
    let registry = input(&root.join("research/arena/standards.json"))?;
    validate_registry(root, &registry)?;
    let mut files = fs::read_dir(root.join("research/arena/profiles"))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    files.sort_by_key(|f| f.path());
    let mut profiles = BTreeSet::new();
    for file in files {
        if file.path().extension().is_some_and(|e| e == "json") {
            let profile = input(&file.path())?;
            validate_profile(root, &registry, &profile)?;
            canonical_targets(root, &profile)?;
            if !profiles.insert((
                s(&profile, "id").to_owned(),
                s(&profile, "version").to_owned(),
            )) {
                return Err("duplicate profile ID/version".into());
            }
        }
    }
    if profiles.is_empty() {
        return Err("Arena registry has no profile examples".into());
    }
    Ok(())
}
fn bundle_base(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}
pub fn cli(root: &Path, args: &[String]) -> CheckResult<()> {
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["report"] | ["report", _] => {
            let result = report::render(root, args.get(1).map(Path::new))?;
            print!("{}", result.text);
            return if result.admissible {
                Ok(())
            } else {
                Err(
                    "Arena report contains invalid or ineligible runs; reasons are shown above"
                        .into(),
                )
            };
        }
        ["diff", left, right] => {
            print!("{}", report::diff(root, Path::new(left), Path::new(right))?);
            return Ok(());
        }
        _ => {}
    }

    let registry = input(&root.join("research/arena/standards.json"))?;
    let output=match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["check"]=>{check(root)?;json!({"state":"PASS","scope":"Registry/profile validation only; no kernel run or certification"})},
        ["registry"]=>{
            check(root)?;
            let mut references=BTreeMap::<String,Vec<Value>>::new();
            for file in fs::read_dir(root.join("research/arena/profiles")).map_err(|e|e.to_string())? {
                let path=file.map_err(|e|e.to_string())?.path(); if path.extension().is_none_or(|e|e!="json") {continue;}
                let p=input(&path)?;
                for metric in a(&p,"metrics") {for r in a(metric,"methodologies") {references.entry(format!("{}@{}",s(r,"id"),s(r,"version"))).or_default().push(json!({"profile":p["id"],"measurement":metric["id"]}));}}
                for r in a(&p["security"],"methodologies") {references.entry(format!("{}@{}",s(r,"id"),s(r,"version"))).or_default().push(json!({"profile":p["id"],"security":p["security"]["id"]}));}
            }
            for refs in references.values_mut(){refs.sort_by_key(digest);refs.dedup();}
            json!({"registry":registry,"references":references})
        },
        ["profile",path]=>{let p=input(Path::new(path))?;validate_profile(root,&registry,&p)?;canonical_targets(root,&p)?;json!({"profile":p,"comparison_class":comparison_class(&p)})},
        ["compare",left,right]=>{
            let l=input(Path::new(left))?;let r=input(Path::new(right))?;
            let la=assess(root,bundle_base(Path::new(left)),&l)?;let ra=assess(root,bundle_base(Path::new(right)),&r)?;
            compatible(root,&l["registry_snapshot"],&l["profile_snapshot"],&r["registry_snapshot"],&r["profile_snapshot"])?;
            if la["admission_state"]!="STRUCTURALLY_ADMISSIBLE" || ra["admission_state"]!="STRUCTURALLY_ADMISSIBLE" {return Err("invalid/incomplete runs cannot be compared".into());}
            json!({"comparison_class":la["comparison_class"],"left":la,"right":ra,"scope":"Descriptive comparison only; no superiority, regression statistics or records"})
        },
        ["assess",path]=>{let run=input(Path::new(path))?;assess(root,bundle_base(Path::new(path)),&run)?},
        _=>return Err("usage: cargo xtask arena report [DIR|RUN] | diff BASE_RUN CANDIDATE_RUN | check | registry | profile PATH | assess RUN | compare LEFT_RUN RIGHT_RUN".into())
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?
    );
    if output["admission_state"] == "INELIGIBLE" {
        return Err("Arena admission rejected; retained assessment explains reasons".into());
    }
    Ok(())
}
