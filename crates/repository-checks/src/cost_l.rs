//! Bounded offline COST-L declarations. Validation does not mint authority or unload code.
use crate::{CheckResult, finish, parse_json, read_json};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Component, Path},
};

pub const MAX_RECORD_BYTES: u64 = 64 * 1024;
pub const MAX_RECORDS: usize = 4096;

pub fn schema(root: &Path) -> CheckResult<jsonschema::Validator> {
    schema_file(root, "schemas/cost-l.schema.json")
}

pub(crate) fn schema_file(root: &Path, name: &str) -> CheckResult<jsonschema::Validator> {
    let value = read_json(&root.join(name))?;
    jsonschema::meta::validate(&value).map_err(|e| e.to_string())?;
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&value)
        .map_err(|e| e.to_string())
}

pub(crate) fn bounded_json(path: &Path) -> CheckResult<Value> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_RECORD_BYTES {
        return Err(format!(
            "{}: non-file or record exceeds byte limit",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_RECORD_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err("record grew beyond byte limit".into());
    }
    let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    parse_json(text).map_err(|e| format!("{}: {e}", path.display()))
}

pub(crate) fn local_file(root: &Path, name: &str) -> CheckResult<std::path::PathBuf> {
    let relative = Path::new(name);
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
        || name.contains('\\')
        || name.contains(':')
    {
        return Err(format!(
            "reference must be a relative repository file: {name}"
        ));
    }
    let target = root.join(relative);
    let resolved = target.canonicalize().map_err(|e| format!("{name}: {e}"))?;
    let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
    if !resolved.starts_with(canonical_root) || !resolved.is_file() {
        return Err(format!(
            "reference escapes repository or is not a file: {name}"
        ));
    }
    Ok(resolved)
}

fn transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("CANDIDATE", "CONFIRMED" | "RETIRED")
            | ("CONFIRMED", "ACTIVE" | "RETIRED")
            | ("ACTIVE", "DEPRECATED")
            | ("DEPRECATED", "ACTIVE" | "RETIRING")
            | ("RETIRING", "DEPRECATED" | "RETIRED")
    )
}

fn module_key(module: &Value) -> (&str, &str, &str) {
    (
        module["id"].as_str().unwrap(),
        module["semantic_version"].as_str().unwrap(),
        module["scope"].as_str().unwrap(),
    )
}

/// Validate one record against its schema and repository evidence references.
pub fn record_errors(
    validator: &jsonschema::Validator,
    root: &Path,
    record: &Value,
    filename: &str,
) -> Vec<String> {
    let mut errors: Vec<_> = validator
        .iter_errors(record)
        .map(|e| e.to_string())
        .collect();
    if !errors.is_empty() {
        return errors;
    }
    // Closed schema validation establishes all indexed types below.
    let id = record["id"].as_str().unwrap();
    let mut fail = |s: &str| errors.push(format!("{id}: {s}"));
    if id == "COST-L-0000" || filename != format!("{id}.json") {
        fail("reserved zero ID or filename differs from stable identity");
    }
    let date = record["research_date"].as_str().unwrap();
    let introduced = record["introduced_at"].as_str().unwrap();
    if introduced > date {
        fail("introduction is after research date");
    }
    if let Some(observed) = record["last_observed_at"].as_str()
        && observed > date
    {
        fail("last observation is after research date");
    }
    let sources = record["sources"].as_array().unwrap();
    let source_ids: BTreeSet<_> = sources.iter().map(|s| s["id"].as_str().unwrap()).collect();
    if source_ids.len() != sources.len() {
        fail("duplicate source identity");
    }
    if !sources.iter().any(|s| s["kind"] != "SECONDARY_ANALYSIS") {
        fail("at least one primary source is required");
    }
    for source in sources {
        if source["date"].as_str().is_some_and(|d| d > date) {
            fail("source date is after research date");
        }
        if source["commit"].is_null() && source["version"].is_null() {
            fail("source requires a pinned commit or version");
        }
        if matches!(source["kind"].as_str().unwrap(), "SOURCE_CODE" | "COMMIT")
            && source["commit"].is_null()
        {
            fail("source code and commits require an exact commit");
        }
        // A URL is a declaration, not fetched or authenticated by the validator.
        let authority = source["url"]
            .as_str()
            .unwrap()
            .trim_start_matches("https://")
            .split('/')
            .next()
            .unwrap();
        if authority.is_empty() || authority.contains('@') {
            fail("source URL has empty authority or embedded credentials");
        }
    }
    for field in ["linux_behavior", "historical_reason", "root_cause"] {
        if record["evidence"][field]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| !source_ids.contains(s.as_str().unwrap()))
        {
            fail("historical claim references an absent source");
        }
    }
    if record["history"]["first_known_version"].is_null()
        && (record["history"]["unknown_origin_reason"].is_null()
            || record["open_questions"].as_array().unwrap().is_empty())
    {
        fail("unknown origin requires reason and open question");
    }
    for reference in record["native"]["references"].as_array().unwrap() {
        if let Err(e) = local_file(root, reference.as_str().unwrap()) {
            fail(&e);
        }
    }
    for case in record["source_cases"].as_array().unwrap() {
        if !root
            .join(format!("research/cases/{}.json", case.as_str().unwrap()))
            .is_file()
        {
            fail("missing KOL-PATH source case");
        }
    }
    let status = record["status"].as_str().unwrap();
    let support = &record["support"];
    let wanted_support = match status {
        "CANDIDATE" | "CONFIRMED" => &["RESEARCH_ONLY"][..],
        "ACTIVE" => &["SUPPORTED"][..],
        "DEPRECATED" => &["DEPRECATED", "EXPIRED"][..],
        "RETIRING" => &["EXPIRED"][..],
        "RETIRED" => &["RETIRED"][..],
        _ => unreachable!(),
    };
    if !wanted_support.contains(&support["status"].as_str().unwrap()) {
        fail("debt lifecycle contradicts declared support");
    }
    let modules = record["compat_modules"].as_array().unwrap();
    let consumers = record["affected_consumers"].as_array().unwrap();
    let module_ids: BTreeSet<_> = modules.iter().map(module_key).collect();
    let consumer_ids: BTreeSet<_> = consumers
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    if module_ids.len() != modules.len() || consumer_ids.len() != consumers.len() {
        fail("duplicate module or consumer identity");
    }
    for consumer in consumers {
        if consumer["module_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| !module_ids.contains(&module_key(m)))
        {
            fail("consumer references an absent module");
        }
        if let Some(start) = support["start"].as_str()
            && consumer["support_until"].as_str().unwrap() <= start
        {
            fail("consumer support ends before declared support begins");
        }
        if let Some(until) = support["until"].as_str()
            && consumer["support_until"].as_str().unwrap() > until
        {
            fail("consumer support exceeds the module support window");
        }
    }
    if support["owner"].is_null() && support["owner_unknown_reason"].is_null() {
        fail("unknown owner needs an explanation");
    }
    if matches!(status, "ACTIVE" | "DEPRECATED" | "RETIRING") {
        if support["owner"].is_null() || support["start"].is_null() || support["until"].is_null() {
            fail("supported history requires owner and finite date window");
        }
        if modules.is_empty() {
            fail("supported history requires an explicit implemented module");
        }
        if status == "ACTIVE" && consumers.is_empty() {
            fail("ACTIVE requires named supported consumers");
        }
    }
    if matches!(status, "CONFIRMED" | "ACTIVE" | "DEPRECATED" | "RETIRING")
        && record["native"]["status"] != "ACCEPTED"
    {
        fail("confirmed incompatibility requires an accepted native difference");
    }
    if matches!(status, "CANDIDATE" | "CONFIRMED") && (!modules.is_empty() || !consumers.is_empty())
    {
        fail("research-only records cannot declare implemented support");
    }
    if let (Some(start), Some(until)) = (support["start"].as_str(), support["until"].as_str())
        && start >= until
    {
        fail("invalid support interval");
    }
    if support["owner"].is_string() && !support["owner_unknown_reason"].is_null() {
        fail("known owner cannot also claim unknown ownership");
    }
    if status == "ACTIVE" && support["until"].as_str().is_some_and(|until| until <= date) {
        fail("ACTIVE support has already expired at the record review date");
    }
    if status == "ACTIVE" && support["start"].as_str().is_some_and(|start| start > date) {
        fail("ACTIVE support has not yet begun");
    }
    if support["status"] == "EXPIRED" && support["until"].as_str().is_none_or(|until| until > date)
    {
        fail("EXPIRED support must have reached its declared deadline");
    }
    let events = record["lifecycle"].as_array().unwrap();
    if events[0]["status"] != "CANDIDATE" || events[0]["date"] != record["introduced_at"] {
        fail("lifecycle must start at the original candidate introduction");
    }
    if events.last().unwrap()["status"] != record["status"] {
        fail("lifecycle terminal state differs from current status");
    }
    if events
        .iter()
        .any(|event| event["date"].as_str().unwrap() > date)
    {
        fail("lifecycle event is after research date");
    }
    for pair in events.windows(2) {
        if !transition(
            pair[0]["status"].as_str().unwrap(),
            pair[1]["status"].as_str().unwrap(),
        ) || pair[0]["date"].as_str().unwrap() > pair[1]["date"].as_str().unwrap()
        {
            fail("illegal or out-of-order lifecycle transition");
        }
    }
    if matches!(status, "ACTIVE" | "DEPRECATED" | "RETIRING")
        && !events.iter().any(|e| e["status"] == "ACTIVE")
    {
        fail("supported lifecycle lacks activation history");
    }
    let was_active = events.iter().any(|e| e["status"] == "ACTIVE");
    if status == "RETIRED" && was_active {
        let retired_date = events.last().unwrap()["date"].as_str().unwrap();
        if support["owner"].is_null()
            || support["start"].is_null()
            || support["until"]
                .as_str()
                .is_none_or(|until| until > retired_date)
            || modules.is_empty()
            || consumers
                .iter()
                .any(|c| c["support_until"].as_str().unwrap() > retired_date)
        {
            fail(
                "retired supported history must retain owner/artifact and completed finite obligations",
            );
        }
    }
    let mut measured = false;
    for (dimension, metric) in record["runtime_cost"].as_object().unwrap() {
        let unit = match dimension.as_str() {
            "cpu" | "latency" => "ns",
            "memory" => "bytes",
            "throughput" => "operations/s",
            _ => "count",
        };
        if metric["unit"] != unit {
            fail("metric unit does not match its dimension");
        }
        if metric["state"] == "UNKNOWN" {
            if !metric["value"].is_null()
                || metric["reason"].is_null()
                || !metric["provenance"].is_null()
                || metric["coverage"] != "UNKNOWN"
            {
                fail("UNKNOWN requires null value/provenance, unknown coverage and a reason");
            }
        } else {
            measured = true;
            if metric["value"].is_null()
                || metric["denominator"].is_null()
                || metric["provenance"].is_null()
                || metric["coverage"] == "UNKNOWN"
            {
                fail("MEASURED requires value, denominator, provenance and known coverage");
                continue;
            }
            if metric["coverage"] == "PARTIAL" && metric["reason"].is_null() {
                fail("partial measured coverage requires its limitation");
            }
            let provenance = &metric["provenance"];
            match local_file(root, provenance["artifact"].as_str().unwrap()) {
                Err(e) => fail(&e),
                Ok(path) => match fs::metadata(&path) {
                    Ok(meta) if meta.len() <= MAX_RECORD_BYTES => match fs::File::open(&path)
                        .and_then(|file| {
                            let mut bytes = Vec::new();
                            file.take(MAX_RECORD_BYTES + 1).read_to_end(&mut bytes)?;
                            Ok(bytes)
                        }) {
                        Ok(bytes) => {
                            if bytes.len() as u64 > MAX_RECORD_BYTES {
                                fail("metric receipt grew beyond byte limit");
                                continue;
                            }
                            let digest = format!("{:x}", Sha256::digest(&bytes));
                            if provenance["sha256"] != digest {
                                fail("metric provenance artifact digest mismatch");
                            }
                        }
                        Err(e) => fail(&e.to_string()),
                    },
                    _ => fail("metric receipt exceeds bounded input size"),
                },
            }
        }
    }
    if measured && record["last_observed_at"].is_null() {
        fail("measured cost requires an observation date");
    }
    if record["related_cost_l"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r == id)
    {
        fail("related debt cannot refer to itself");
    }
    errors
}

/// Validate bounded records and registry-level relations; historical entries are retained.
pub fn validate(root: &Path, directory: &Path) -> CheckResult<Vec<Value>> {
    let validator = schema(root)?;
    if fs::symlink_metadata(directory)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("registry directory cannot be a symlink".into());
    }
    let allocation = bounded_json(&directory.join("registry.json"))?;
    schema_file(root, "schemas/cost-l-registry.schema.json")?
        .validate(&allocation)
        .map_err(|e| e.to_string())?;
    let allocated: BTreeSet<_> = allocation["allocated_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap().to_owned())
        .collect();
    if allocated.contains("COST-L-0000") {
        return Err("zero allocation identity is reserved".into());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|e| e == "json")
            && path.file_name().is_none_or(|name| name != "registry.json")
        {
            paths.push(path);
            if paths.len() > MAX_RECORDS {
                return Err("registry exceeds record input limit".into());
            }
        }
    }
    paths.sort();
    if paths.is_empty() {
        return Err("COST-L registry is empty".into());
    }
    let mut records = Vec::new();
    let mut errors = Vec::new();
    let mut ids = BTreeSet::new();
    for path in paths {
        match bounded_json(&path) {
            Err(e) => errors.push(e),
            Ok(value) => {
                errors.extend(record_errors(
                    &validator,
                    root,
                    &value,
                    &path.file_name().unwrap().to_string_lossy(),
                ));
                if let Some(id) = value["id"].as_str()
                    && !ids.insert(id.to_owned())
                {
                    errors.push(format!("duplicate COST-L identity: {id}"));
                }
                records.push(value);
            }
        }
    }
    if ids != allocated {
        errors.push("allocation ledger differs from records: retain tombstones and allocate each new stable ID".into());
    }
    finish(errors)?;
    let mut errors = Vec::new();
    let exception_registry = read_json(&root.join("policy/exceptions.json"))?;
    let exception_ids: BTreeSet<_> = exception_registry["records"]
        .as_array()
        .ok_or("exception registry has no records array")?
        .iter()
        .filter_map(|e| e["id"].as_str())
        .collect();
    let mut module_identities = BTreeMap::new();
    for record in &records {
        for target in record["related_cost_l"].as_array().unwrap() {
            if !ids.contains(target.as_str().unwrap()) {
                errors.push(format!(
                    "{}: dangling related COST-L identity",
                    record["id"]
                ));
            }
        }
        for exception in record["exceptions"].as_array().unwrap() {
            if !exception_ids.contains(exception.as_str().unwrap()) {
                errors.push(format!("{}: dangling EXC identity", record["id"]));
            }
        }
        for module in record["compat_modules"].as_array().unwrap() {
            let key = (
                module["id"].as_str().unwrap(),
                module["semantic_version"].as_str().unwrap(),
                module["scope"].as_str().unwrap(),
            );
            if let Some(prior) = module_identities.insert(key, module)
                && prior != module
            {
                errors.push(format!(
                    "{}: conflicting module artifact identity",
                    record["id"]
                ));
            }
        }
    }
    finish(errors)?;
    Ok(records)
}
