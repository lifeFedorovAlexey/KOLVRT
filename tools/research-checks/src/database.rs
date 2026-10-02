use crate::{CheckResult, finish, read_json};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const FACTS: [&str; 6] = [
    "historical_context",
    "original_reason",
    "root_cause",
    "observable_behavior",
    "compatibility_dependency",
    "linux_current_solution",
];
const PRIMARY: [&str; 6] = [
    "SOURCE_CODE",
    "COMMIT",
    "OFFICIAL_DOCUMENTATION",
    "BUG_REPORT",
    "REPRODUCER",
    "MAINTAINER_DISCUSSION",
];

pub fn json_files(directory: &Path) -> CheckResult<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory).map_err(|e| format!("{}: {e}", directory.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|x| x == "json") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn schema(root: &Path) -> CheckResult<jsonschema::Validator> {
    let schema = read_json(&root.join("schemas/pathology/case.schema.json"))?;
    jsonschema::meta::validate(&schema).map_err(|e| format!("invalid schema: {e}"))?;
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|e| format!("schema: {e}"))
}

pub fn case_errors(validator: &jsonschema::Validator, case: &Value, filename: &str) -> Vec<String> {
    let mut errors: Vec<String> = validator
        .iter_errors(case)
        .map(|e| format!("{filename}: {e}"))
        .collect();
    if !errors.is_empty() {
        return errors;
    }
    let id = case["id"].as_str().unwrap(); // The closed schema established these types.
    let mut fail = |message: &str| errors.push(format!("{id}: {message}"));
    if filename != format!("{id}.json") {
        fail("filename must match ID");
    }
    let sources = case["sources"].as_array().unwrap();
    let ids: BTreeSet<_> = sources.iter().map(|s| s["id"].as_str().unwrap()).collect();
    if ids.len() != sources.len() {
        fail("duplicate source ID");
    }
    for source in sources {
        let url = source["url"].as_str().unwrap();
        let host = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"));
        if host.is_none_or(|h| h.split('/').next().is_none_or(str::is_empty)) {
            fail("invalid HTTP source URL");
        }
        if source["research_date"] != case["research_date"] {
            fail("source research date differs from case");
        }
        if let Some(date) = source["date"].as_str()
            && date > source["research_date"].as_str().unwrap()
        {
            fail("source date is after research date");
        }
        if ["commit", "tag_version", "date"]
            .iter()
            .any(|k| source[k].is_null())
            && source["provenance_limitations"]
                .as_str()
                .unwrap()
                .trim()
                .is_empty()
        {
            fail("unknown provenance needs an explanation");
        }
    }
    for field in FACTS {
        let refs = case["evidence"][field].as_array().unwrap();
        if refs.is_empty() || refs.iter().any(|r| !ids.contains(r.as_str().unwrap())) {
            fail(&format!("invalid or missing evidence for {field}"));
        }
    }
    let no_questions = case["open_questions"].as_array().unwrap().is_empty();
    if case["first_known_version"].is_null() && no_questions {
        fail("unknown first version needs open questions");
    }
    let decision = case["kolvrt_decision"].as_str().unwrap();
    let compat = case["compatibility_required"].as_str().unwrap();
    let no_scope = case["compatibility_scope"] == "NONE";
    if decision == "COMPAT_ONLY" && compat != "YES" {
        fail("COMPAT_ONLY requires YES");
    }
    if compat == "NO" && !no_scope {
        fail("NO compatibility requires NONE scope");
    }
    if matches!(compat, "YES" | "CONDITIONAL") && no_scope {
        fail("compatibility requires a concrete scope");
    }
    if decision == "RESEARCH_REQUIRED" && no_questions {
        fail("research decision requires open questions");
    }
    if case["status"] == "UNRESOLVED" && no_questions {
        fail("unresolved case requires open questions");
    }
    if !sources
        .iter()
        .any(|s| PRIMARY.contains(&s["kind"].as_str().unwrap()))
    {
        fail("at least one primary source required");
    }
    errors
}

pub fn validate(root: &Path, directory: &Path, minimum: usize) -> CheckResult<Vec<Value>> {
    if minimum == 0 {
        return Err("minimum case count must be positive".into());
    }
    let validator = schema(root)?;
    let paths = json_files(directory)?;
    let mut errors = Vec::new();
    if paths.len() < minimum {
        errors.push(format!(
            "expected at least {minimum} cases, found {}",
            paths.len()
        ));
    }
    let mut ids = BTreeSet::new();
    let mut cases = Vec::new();
    for path in paths {
        match read_json(&path) {
            Err(e) => errors.push(e),
            Ok(case) => {
                errors.extend(case_errors(
                    &validator,
                    &case,
                    &path.file_name().unwrap().to_string_lossy(),
                ));
                if let Some(id) = case["id"].as_str()
                    && !ids.insert(id.to_owned())
                {
                    errors.push(format!("duplicate ID: {id}"));
                }
                cases.push(case);
            }
        }
    }
    finish(errors)?;
    Ok(cases)
}

pub fn check_ledger(root: &Path) -> CheckResult<()> {
    let ledger = read_json(&root.join("research/sources/case-sources.json"))?;
    let entries = ledger.as_array().ok_or("source ledger must be an array")?;
    let mut ids = BTreeSet::new();
    let mut errors = Vec::new();
    for entry in entries {
        let id = entry["id"].as_str().ok_or("missing source ID")?;
        if !ids.insert(id) {
            errors.push(format!("duplicate source ID in ledger: {id}"));
        }
    }
    for path in json_files(&root.join("research/pathology"))? {
        let case = read_json(&path)?;
        for source in case["sources"]
            .as_array()
            .ok_or("case sources must be an array")?
        {
            let mut canonical = source.clone();
            canonical
                .as_object_mut()
                .ok_or("source must be an object")?
                .remove("subsystem");
            if !entries.contains(&canonical) {
                errors.push(format!(
                    "{}: source differs from ledger: {}",
                    path.display(),
                    source["id"]
                ));
            }
        }
    }
    finish(errors)
}
