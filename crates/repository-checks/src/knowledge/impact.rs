//! Per-change review declarations; classification remains a human judgment.
use super::*;

fn implementation_path(path: &str) -> bool {
    // Deliberately conservative: new files in executable/build roots need review
    // even if no existing feature owns them. Additional declared sources count too.
    path.starts_with("crates/")
        || path.starts_with("scripts/")
        || path.starts_with(".github/workflows/")
        || path.starts_with(".cargo/")
        || matches!(
            path,
            "Cargo.toml"
                | "Cargo.lock"
                | "build.rs"
                | "package.json"
                | "package-lock.json"
                | "rust-toolchain.toml"
        )
}

pub(super) fn check(
    root: &Path,
    base: &str,
    changed: &BTreeSet<&str>,
    knowledge: &Knowledge,
    old_features: &BTreeMap<String, Value>,
    run: impl Fn(&[&str]) -> CheckResult<String>,
) -> CheckResult<()> {
    let nodes = knowledge.graph["nodes"].as_object().unwrap();
    let mut files: BTreeSet<String> = changed
        .iter()
        .filter(|p| implementation_path(p))
        .map(|p| (*p).to_owned())
        .collect();
    let mut affected = BTreeSet::new();
    for (id, node) in nodes {
        if let Some(feature) = node.get("feature") {
            for source in strings(feature, "sources").into_iter().chain(
                old_features
                    .get(id)
                    .into_iter()
                    .flat_map(|f| strings(f, "sources")),
            ) {
                if changed.contains(source.as_str()) {
                    files.insert(source);
                    affected.insert(id.clone());
                }
            }
        }
    }
    if files.is_empty() {
        return Ok(());
    }
    let declaration = read_json(&root.join("docs/implementation-impact.json")).map_err(|e| {
        format!("implementation changes require docs/implementation-impact.json: {e}")
    })?;
    let validator = jsonschema::validator_for(&read_json(
        &root.join("schemas/implementation-impact.schema.json"),
    )?)
    .map_err(|e| e.to_string())?;
    validator
        .validate(&declaration)
        .map_err(|e| format!("impact declaration: {e}"))?;
    let base_commit = run(&["rev-parse", "--verify", &format!("{base}^{{commit}}")])?;
    if declaration["base_commit"].as_str() != Some(base_commit.trim()) {
        return Err("impact declaration base_commit differs from reviewed base; regenerate and review dispositions".into());
    }
    let mut covered = BTreeSet::new();
    for entry in declaration["files"].as_array().unwrap() {
        let path = entry["path"].as_str().unwrap();
        if !files.contains(path) || !covered.insert(path.to_owned()) {
            return Err(format!("duplicate or unrelated impact file {path}"));
        }
        let before = run(&["show", &format!("{base}:{path}")])
            .ok()
            .map(|s| hash(&s));
        let after = if root.join(path).exists() {
            safe_path(root, path)?;
            Some(hash(&read(&root.join(path))?))
        } else {
            None
        };
        if entry["before_sha256_lf"] != json!(before) || entry["after_sha256_lf"] != json!(after) {
            return Err(format!("impact declaration source digest mismatch: {path}"));
        }
        let owners: Vec<_> = nodes
            .iter()
            .filter(|(_, n)| strings(&n["feature"], "sources").iter().any(|s| s == path))
            .map(|(id, _)| id)
            .collect();
        match entry["disposition"].as_str().unwrap() {
            "new-feature" if !owners.iter().any(|id| !old_features.contains_key(*id)) => {
                return Err(format!(
                    "{path}: new-feature requires a newly enrolled canonical feature"
                ));
            }
            "existing-feature"
                if owners.is_empty()
                    && !old_features
                        .values()
                        .any(|f| strings(f, "sources").iter().any(|s| s == path)) =>
            {
                return Err(format!("{path}: existing-feature has no declared owner"));
            }
            "no-feature-impact" if !owners.is_empty() => {
                return Err(format!(
                    "{path}: owned source requires feature dispositions"
                ));
            }
            _ => {}
        }
    }
    if covered != files {
        return Err("implementation impact declaration does not cover every changed implementation/build file".into());
    }
    let mut reviewed = BTreeSet::new();
    for entry in declaration["features"].as_array().unwrap() {
        let id = entry["id"].as_str().unwrap();
        if !affected.contains(id) || !reviewed.insert(id.to_owned()) {
            return Err(format!("duplicate or unrelated feature impact {id}"));
        }
        let node = &nodes[id];
        let feature = &node["feature"];
        let path = node["path"].as_str().unwrap();
        match entry["disposition"].as_str().unwrap() {
            "semantic-change" => {
                let before = run(&["show", &format!("{base}:{path}")]).unwrap_or_default();
                let after = read(&root.join(path))?;
                let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
                if !changed.contains(path) || normalize(&before) == normalize(&after) {
                    return Err(format!(
                        "{id}: semantic-change requires a substantive canonical update"
                    ));
                }
            }
            "evidence-change" => {
                if old_features.get(id).is_some_and(|old| {
                    old["verification"] == feature["verification"]
                        && old["acceptance"] == feature["acceptance"]
                        && old["readiness_acceptance"] == feature["readiness_acceptance"]
                }) {
                    return Err(format!(
                        "{id}: evidence-change requires changed evidence metadata"
                    ));
                }
            }
            "no-impact" => {} // Visible explanation is reviewed; never proves semantic equivalence.
            _ => unreachable!(),
        }
    }
    if reviewed != affected {
        return Err("missing explicit semantic-change/evidence-change/no-impact disposition for affected feature".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&declaration).map_err(|e| e.to_string())?
    );
    Ok(())
}
