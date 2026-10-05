//! Declared module/debt consistency. No loader, authority or retirement automation.
use crate::{CheckResult, cost_l, finish, hash, parse_json, read, read_json};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::Path,
    process::Command,
};

fn key(value: &Value) -> String {
    format!(
        "{}/{}",
        value["id"].as_str().unwrap(),
        value["semantic_version"].as_str().unwrap()
    )
}
fn same_module(a: &Value, b: &Value) -> bool {
    ["id", "semantic_version", "scope"]
        .iter()
        .all(|field| a[field] == b[field])
}
fn same_consumer(a: &Value, b: &Value) -> bool {
    ["id", "kind", "dependency", "scope", "support_until"]
        .iter()
        .all(|field| a[field] == b[field])
}
fn strings(value: &Value) -> impl Iterator<Item = &str> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
}

/// Cargo parses declarations, including disabled optional, target, renamed and build/dev edges.
/// No builds or network resolution are performed. External native edges are denied, not guessed.
pub fn cargo_metadata(root: &Path) -> CheckResult<Value> {
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .current_dir(root)
        .args([
            "metadata",
            "--locked",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Cargo declarations unavailable: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    if output.stdout.len() > 16 * 1024 * 1024 {
        return Err("Cargo metadata exceeds 16 MiB".into());
    }
    parse_json(std::str::from_utf8(&output.stdout).map_err(|e| e.to_string())?)
}

/// Validate manifests against already validated COST-L records and Cargo-parsed declarations.
/// This public split permits synthetic graph mutations without executing fixture manifests.
pub fn validate(
    root: &Path,
    inventory: &Value,
    debts: &[Value],
    metadata: &Value,
) -> CheckResult<()> {
    let schema = cost_l::schema_file(root, "schemas/compatibility-modules.schema.json")?;
    finish(
        schema
            .iter_errors(inventory)
            .map(|e| e.to_string())
            .collect(),
    )?;
    let modules = inventory["modules"].as_array().unwrap();
    let packages = metadata["packages"]
        .as_array()
        .ok_or("Cargo packages unavailable")?;
    let workspace: BTreeSet<_> = metadata["workspace_members"]
        .as_array()
        .ok_or("Cargo workspace members unavailable")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let packages: Vec<_> = packages
        .iter()
        .filter(|p| p["id"].as_str().is_some_and(|id| workspace.contains(id)))
        .collect();
    let mut names = BTreeMap::new();
    for p in &packages {
        let name = p["name"].as_str().ok_or("Cargo package name unavailable")?;
        if names.insert(name, *p).is_some() {
            return Err("ambiguous workspace package name".into());
        }
    }
    let debt_map: BTreeMap<_, _> = debts
        .iter()
        .map(|d| (d["id"].as_str().unwrap(), d))
        .collect();
    let exceptions = read_json(&root.join("policy/exceptions.json"))?;
    let exc: BTreeSet<_> = exceptions["records"]
        .as_array()
        .ok_or("exceptions unavailable")?
        .iter()
        .map(|v| v["id"].as_str().unwrap())
        .collect();
    let mut identities = BTreeMap::new();
    let mut errors = Vec::new();
    for module in modules {
        let identity = key(module);
        if identities.insert(identity.clone(), module).is_some() {
            errors.push(format!("duplicate module {identity}"));
        }
        let package_name = module["package"].as_str().unwrap();
        let current = module["status"] == "DECLARED";
        let synthetic = module["classification"] == "SYNTHETIC";
        if synthetic
            && (!module["debt_ids"].as_array().unwrap().is_empty() || !module["artifact"].is_null())
        {
            errors.push(format!(
                "{identity}: synthetic module must not claim production debt/artifact"
            ));
        }
        if !synthetic
            && (module["debt_ids"].as_array().unwrap().is_empty()
                || module["artifact"].is_null()
                || !module["debt_unassigned_reason"].is_null())
        {
            errors.push(format!(
                "{identity}: production module needs debt and artifact justification"
            ));
        }
        if synthetic && module["debt_unassigned_reason"].is_null() {
            errors.push(format!("{identity}: explain synthetic debt exclusion"));
        }
        if current {
            match names.get(package_name) {
                Some(package) => {
                    let sources: BTreeSet<_> = module["sources"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|s| cost_l::local_file(root, s["path"].as_str().unwrap()))
                        .collect::<CheckResult<_>>()?;
                    let required = std::iter::once(&package["manifest_path"]).chain(
                        package["targets"]
                            .as_array()
                            .ok_or("Cargo targets unavailable")?
                            .iter()
                            .map(|t| &t["src_path"]),
                    );
                    for path in required {
                        let path = Path::new(path.as_str().ok_or("Cargo source path unavailable")?)
                            .canonicalize()
                            .map_err(|e| e.to_string())?;
                        if !sources.contains(&path) {
                            errors.push(format!(
                                "{identity}: source binding omits Cargo manifest/target"
                            ));
                        }
                    }
                    if package["version"] != module["package_version"] {
                        errors.push(format!("{identity}: package version mismatch"));
                    }
                    if package["features"]
                        .get(module["feature"].as_str().unwrap())
                        .is_none()
                    {
                        errors.push(format!("{identity}: missing Cargo feature"));
                    }
                    let marked = package["metadata"]["kolvrt"]["compatibility_modules"]
                        .as_array()
                        .is_some_and(|ids| ids.iter().any(|id| id == &identity));
                    if !marked {
                        errors.push(format!("{identity}: missing Cargo module marker"));
                    }
                }
                None => errors.push(format!("{identity}: package is not in workspace")),
            }
        }
        for source in module["sources"].as_array().unwrap() {
            let name = source["path"].as_str().unwrap();
            let path = cost_l::local_file(root, name)?;
            if hash(&read(&path)?) != source["sha256_lf"] {
                errors.push(format!("{identity}: stale source {name}"));
            }
        }
        if !module["artifact"].is_null() {
            let artifact = &module["artifact"];
            let path = cost_l::local_file(root, artifact["path"].as_str().unwrap())?;
            if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
                return Err("artifact exceeds 64 MiB".into());
            }
            let mut bytes = Vec::new();
            fs::File::open(path)
                .map_err(|e| e.to_string())?
                .take(64 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 64 * 1024 * 1024 {
                return Err("artifact grew beyond 64 MiB".into());
            }
            let digest = format!("{:x}", Sha256::digest(bytes));
            if digest != artifact["sha256"] {
                errors.push(format!("{identity}: artifact digest mismatch"));
            }
        }
        for id in strings(&module["exception_ids"]) {
            if !exc.contains(id) {
                errors.push(format!("{identity}: dangling exception {id}"));
            } else {
                let exception = exceptions["records"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|e| e["id"] == id)
                    .unwrap();
                if !exception["boundaries"]
                    .as_array()
                    .ok_or("exception boundaries unavailable")?
                    .iter()
                    .any(|boundary| {
                        module["sources"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|s| s["path"] == *boundary)
                    })
                {
                    errors.push(format!(
                        "{identity}: exception has no matching source boundary"
                    ));
                }
                if exception["kind"] == "software"
                    && exception["support_until"]
                        .as_str()
                        .is_none_or(|until| module["support_until"].as_str().unwrap() > until)
                {
                    errors.push(format!("{identity}: module outlives software exception"));
                }
            }
        }
        let consumers = module["consumers"].as_array().unwrap();
        let mut consumer_ids = BTreeSet::new();
        for consumer in consumers {
            if !consumer_ids.insert(consumer["id"].as_str().unwrap()) {
                errors.push(format!("{identity}: duplicate consumer"));
            }
            if consumer["support_until"].as_str().unwrap()
                > module["support_until"].as_str().unwrap()
            {
                errors.push(format!("{identity}: consumer outlives support"));
            }
        }
        for id in strings(&module["debt_ids"]) {
            let Some(debt) = debt_map.get(id) else {
                errors.push(format!("{identity}: dangling debt {id}"));
                continue;
            };
            let state = debt["status"].as_str().unwrap();
            if (current && !matches!(state, "ACTIVE" | "DEPRECATED"))
                || (!current && !matches!(state, "RETIRING" | "RETIRED"))
            {
                errors.push(format!(
                    "{identity}: debt lifecycle cannot support declared distribution"
                ));
            }
            let reciprocal = debt["compat_modules"].as_array().unwrap().iter().any(|m| {
                same_module(m, module) && m["artifact_sha256"] == module["artifact"]["sha256"]
            });
            if !reciprocal {
                errors.push(format!(
                    "{identity}: missing or mismatched debt-to-module relation"
                ));
            }
            if debt["support"]["until"]
                .as_str()
                .is_none_or(|until| module["support_until"].as_str().unwrap() > until)
            {
                errors.push(format!("{identity}: owner/support scope mismatch"));
            }
            for consumer in consumers {
                let reciprocal = debt["affected_consumers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|c| {
                        same_consumer(c, consumer)
                            && c["module_refs"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|m| same_module(m, module))
                    });
                if !reciprocal {
                    errors.push(format!(
                        "{identity}: missing reciprocal consumer {}",
                        consumer["id"]
                    ));
                }
            }
        }
    }
    for debt in debts {
        for referenced in debt["compat_modules"].as_array().unwrap() {
            let Some(module) = identities.get(&key(referenced)) else {
                errors.push(format!("{}: no manifest for debt module", debt["id"]));
                continue;
            };
            if !same_module(module, referenced)
                || !strings(&module["debt_ids"]).any(|id| id == debt["id"].as_str().unwrap())
            {
                errors.push(format!(
                    "{}: manifest does not reciprocate debt",
                    debt["id"]
                ));
            }
            for consumer in debt["affected_consumers"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| {
                    c["module_refs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|m| same_module(m, referenced))
                })
            {
                if !module["consumers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|c| same_consumer(c, consumer))
                {
                    errors.push(format!("{}: consumer absent from manifest", debt["id"]));
                }
            }
        }
    }
    for package in &packages {
        let marker = &package["metadata"]["kolvrt"]["compatibility_modules"];
        if marker.is_null() {
            continue;
        }
        let ids = marker
            .as_array()
            .ok_or("Cargo compatibility marker must be an array")?;
        if ids.is_empty() || ids.len() > 128 {
            errors.push("Cargo compatibility marker must contain 1..128 identities".into());
        }
        let mut seen = BTreeSet::new();
        for id in ids {
            let id = id
                .as_str()
                .ok_or("Cargo module marker identity must be text")?;
            if !seen.insert(id) {
                errors.push("duplicate Cargo compatibility marker".into());
            }
            if identities
                .get(id)
                .is_none_or(|m| m["package"] != package["name"] || m["status"] != "DECLARED")
            {
                errors.push(format!(
                    "{id}: Cargo marker lacks current matching manifest"
                ));
            }
        }
    }
    // Conservative union of declarations: optional/renamed/cfg/build/dev edges cannot hide in a native root.
    let mut pending = vec!["kolvrt-kernel", "kernel-core"];
    let mut visited = BTreeSet::new();
    while let Some(name) = pending.pop() {
        if !visited.insert(name) {
            continue;
        }
        let package = names.get(name).ok_or("required native root missing")?;
        if !package["metadata"]["kolvrt"]["compatibility_modules"].is_null()
            || modules.iter().any(|m| m["package"] == name)
        {
            errors.push(format!(
                "native closure reaches compatibility package {name}"
            ));
        }
        for dep in package["dependencies"]
            .as_array()
            .ok_or("Cargo dependencies unavailable")?
        {
            let target = dep["name"]
                .as_str()
                .ok_or("Cargo dependency name unavailable")?;
            let workspace_target = names.get(target).filter(|p| {
                let Some(path) = dep["path"].as_str() else {
                    return false;
                };
                Path::new(path).canonicalize().ok()
                    == Path::new(p["manifest_path"].as_str().unwrap())
                        .parent()
                        .and_then(|p| p.canonicalize().ok())
            });
            if workspace_target.is_none() {
                errors.push(format!(
                    "native closure has external/unresolved dependency {name} -> {target}"
                ));
            } else {
                pending.push(target);
            }
        }
    }
    finish(errors)
}

pub fn check(root: &Path) -> CheckResult<()> {
    let debts = cost_l::validate(root, &root.join("research/cost-l"))?;
    let inventory = cost_l::bounded_json(&root.join("policy/compatibility-modules.json"))?;
    validate(root, &inventory, &debts, &cargo_metadata(root)?)
}
