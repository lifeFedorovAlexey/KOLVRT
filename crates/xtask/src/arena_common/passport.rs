//! Shared immutable artifact custody, source-bound reviews and schema-v1 admission.
use super::*;
fn repository_path(path: &str) -> Result<PathBuf> {
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err("review evidence path must stay within repository".into());
    }
    let resolved = fs::canonicalize(path)?;
    if !resolved.starts_with(fs::canonicalize(".")?) {
        return Err("review evidence escapes repository".into());
    }
    Ok(resolved)
}
fn review(path: &str, negative: bool, required: &[&str]) -> Result<Value> {
    let value = read_json(path)?;
    if value["schema_version"] != 1
        || value["status"] != if negative { "passed" } else { "accepted" }
        || value["scope"].as_array().is_none_or(|a| {
            a.is_empty()
                || a.iter()
                    .any(|v| v.as_str().is_none_or(|s| s.trim().is_empty()))
        })
    {
        return Err("missing accepted review scope".into());
    }
    let files = value["source_files"]
        .as_object()
        .ok_or("review source map absent")?;
    if required.iter().any(|p| !files.contains_key(*p)) {
        return Err("review missing required implementation source".into());
    }
    for (path, hash) in files {
        if hash != &digest(&fs::read(repository_path(path)?)?) {
            return Err(format!("review source digest mismatch: {path}").into());
        }
    }
    if negative {
        let commands = value["commands"]
            .as_array()
            .filter(|a| !a.is_empty())
            .ok_or("review commands absent")?;
        for command in commands {
            if command["exit_code"] != 0
                || command["command"]
                    .as_str()
                    .is_none_or(|s| s.trim().is_empty())
            {
                return Err("review command did not pass".into());
            }
            let log = command["log_path"]
                .as_str()
                .ok_or("review command log absent")?;
            if command["log_sha256"] != digest(&fs::read(repository_path(log)?)?) {
                return Err("review command log digest mismatch".into());
            }
        }
    } else if value["reviewers"].as_array().is_none_or(|a| a.is_empty()) {
        return Err("independent reviewers absent".into());
    }
    Ok(value)
}
pub(crate) fn retain_review(
    bundle: &Path,
    directory: &str,
    negative: bool,
    required: &[&str],
    artifacts: &mut Vec<Value>,
) -> Result<Vec<String>> {
    let name = if negative {
        "negative-review"
    } else {
        "source-review"
    };
    let path = format!("{directory}/{name}.json");
    let value = match review(&path, negative, required) {
        Ok(value) => value,
        Err(error) => {
            write_json(
                bundle.join(format!("{name}-unavailable.json")),
                &json!({"status":"UNAVAILABLE","reason":error.to_string()}),
            )?;
            return Ok(vec![]);
        }
    };
    let local = format!("{name}.json");
    artifact(bundle, &local, Path::new(&path), artifacts)?;
    let mut refs = vec![local];
    if negative {
        for (index, command) in value["commands"].as_array().unwrap().iter().enumerate() {
            let local = format!("negative-command-{index}.log");
            artifact(
                bundle,
                &local,
                &repository_path(command["log_path"].as_str().unwrap())?,
                artifacts,
            )?;
            refs.push(local);
        }
    }
    Ok(refs)
}
pub(crate) fn artifact(
    bundle: &Path,
    name: &str,
    source: &Path,
    artifacts: &mut Vec<Value>,
) -> Result<String> {
    if Path::new(name)
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
        || Path::new(name).components().count() != 1
    {
        return Err("unsafe artifact name".into());
    }
    let bytes = fs::read(source)?;
    let hash = digest(&bytes);
    let path = if name.ends_with(".elf") {
        let root = bundle
            .parent()
            .and_then(Path::parent)
            .ok_or("bundle root absent")?;
        fs::create_dir_all(root.join("images"))?;
        let relative = format!("images/{hash}.elf");
        let destination = root.join(&relative);
        if destination.exists() {
            if fs::read(&destination)? != bytes {
                return Err("immutable shared image mismatch".into());
            }
        } else {
            fs::write(destination, bytes)?;
        }
        relative
    } else {
        let root = bundle
            .parent()
            .and_then(Path::parent)
            .ok_or("bundle root absent")?;
        fs::create_dir_all(root.join("objects"))?;
        let object = root.join("objects").join(&hash);
        if object.exists() {
            if fs::read(&object)? != bytes {
                return Err("immutable artifact object mismatch".into());
            }
        } else {
            fs::write(&object, &bytes)?;
        }
        let destination = bundle.join(name);
        if destination.exists() {
            if fs::read(&destination)? != bytes {
                return Err("immutable artifact path changed".into());
            }
        } else {
            fs::hard_link(object, &destination)?;
        }
        name.to_owned()
    };
    if !artifacts.iter().any(|a| a["path"] == path) {
        artifacts.push(json!({"path":path,"sha256":hash}));
    }
    Ok(hash)
}
pub(crate) fn qualify_paths(value: &mut Value, paths: &[String], prefix: &str) {
    match value {
        Value::String(s) if paths.contains(s) => *s = format!("{prefix}{s}"),
        Value::Array(a) => {
            for v in a {
                qualify_paths(v, paths, prefix)
            }
        }
        Value::Object(o) => {
            for v in o.values_mut() {
                qualify_paths(v, paths, prefix)
            }
        }
        _ => {}
    }
}

pub(crate) fn assess(root: &Path, bundle: &Path, key: &str, mut run: Value) -> Result<Value> {
    let prefix = format!("passports/{key}/");
    let paths: Vec<String> = run["artifacts"]
        .as_array()
        .ok_or("artifacts absent")?
        .iter()
        .filter_map(|a| a["path"].as_str())
        .filter(|p| !p.starts_with("images/"))
        .map(str::to_owned)
        .collect();
    qualify_paths(&mut run, &paths, &prefix);
    let run_path = root.join(format!("{key}-run.json"));
    write_json(&run_path, &run)?;
    let output = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args([
            "run",
            "--locked",
            "-p",
            "repository-checks",
            "--",
            "arena",
            "assess",
        ])
        .arg(&run_path)
        .output()?;
    fs::write(bundle.join("assessment.stdout"), &output.stdout)?;
    fs::write(bundle.join("assessment.stderr"), &output.stderr)?;
    let assessment = serde_json::from_slice::<Value>(&output.stdout).unwrap_or_else(|_| json!({"admission_state":"INELIGIBLE","record_eligible":false,"reason":"existing assessor rejected malformed or incomplete evidence","exit_code":output.status.code()}));
    write_json(bundle.join("assessment.json"), &assessment)?;
    Ok(json!({"run":run_path,"assessment":assessment}))
}
/// Snapshot original bytes, not serializer replacements for raw-byte digest contracts.
pub(crate) fn snapshot(paths: &[String]) -> Result<Value> {
    let mut files = serde_json::Map::new();
    for path in paths {
        let raw = fs::read_to_string(path)?;
        let mut entry = json!({"sha256":digest(raw.as_bytes()),"sha256_lf":digest(raw.replace("\r\n","\n").as_bytes()),"raw":raw});
        if path.ends_with(".json") {
            entry["value"] = serde_json::from_str::<Value>(entry["raw"].as_str().unwrap())?;
        }
        files.insert(path.clone(), entry);
    }
    Ok(Value::Object(files))
}
pub(crate) fn retain_definitions(root: &Path, frozen: &Value) -> Result<()> {
    for (path, value) in frozen.as_object().ok_or("frozen definition map absent")? {
        let relative = Path::new(path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("unsafe definition path".into());
        }
        let raw = value["raw"]
            .as_str()
            .ok_or("frozen definition bytes absent")?;
        if value["sha256"] != digest(raw.as_bytes()) {
            return Err("frozen definition hash mismatch".into());
        }
        let destination = root.join("definitions").join(relative);
        fs::create_dir_all(destination.parent().ok_or("definition parent absent")?)?;
        if destination.exists() && fs::read(&destination)? != raw.as_bytes() {
            return Err("immutable definition already differs".into());
        }
        fs::write(destination, raw)?;
    }
    Ok(())
}
/// Both producers retain the actual execution inputs and outputs through this path.
pub(crate) fn retain_attempt(
    bundle: &Path,
    mode: &str,
    attempt: &Value,
    artifacts: &mut Vec<Value>,
) -> Result<String> {
    let kernel = Path::new(
        attempt["kernel"]["artifact"]
            .as_str()
            .ok_or("kernel artifact missing")?,
    );
    let hash = artifact(bundle, &format!("{mode}-kernel.elf"), kernel, artifacts)?;
    if hash != attempt["kernel"]["sha256"] {
        return Err("kernel image changed before bundle retention".into());
    }
    let directory = Path::new(
        attempt["directory"]
            .as_str()
            .ok_or("attempt directory missing")?,
    );
    for name in [
        "attempt.json",
        "kernel-build.json",
        "kernel.run.json",
        "kernel.results.json",
        "kernel.log",
        "kernel.stderr",
    ] {
        artifact(
            bundle,
            &format!("{mode}-{name}"),
            &directory.join(name),
            artifacts,
        )?;
    }
    for role in ["root", "service", "client"] {
        let app = &attempt["applications"][role];
        let got = artifact(
            bundle,
            &format!("{mode}-{role}.elf"),
            Path::new(
                app["artifact"]
                    .as_str()
                    .ok_or("application artifact absent")?,
            ),
            artifacts,
        )?;
        if got != app["sha256"] {
            return Err("application image changed before bundle retention".into());
        }
    }
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frozen_bytes_and_artifact_identity_are_preserved() {
        let root = env::temp_dir().join(format!(
            "kolvrt-arena-custody-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let bundle = root.join("passports/pair");
        fs::create_dir_all(&bundle).unwrap();
        let source = root.join("original.json");
        let raw = "{\r\n  \"value\": 1\r\n}\r\n";
        fs::write(&source, raw).unwrap();
        let mut artifacts = vec![];
        artifact(&bundle, "raw.json", &source, &mut artifacts).unwrap();
        assert_eq!(fs::read(bundle.join("raw.json")).unwrap(), raw.as_bytes());
        assert!(artifact(&bundle, "../escape", &source, &mut artifacts).is_err());
        fs::write(&source, "changed").unwrap();
        assert!(artifact(&bundle, "raw.json", &source, &mut artifacts).is_err());
        let frozen = json!({"research/protocol.json":{"raw":raw,"sha256":digest(raw.as_bytes())}});
        retain_definitions(&root, &frozen).unwrap();
        assert_eq!(
            fs::read(root.join("definitions/research/protocol.json")).unwrap(),
            raw.as_bytes()
        );
        let mut stale = frozen.clone();
        stale["research/protocol.json"]["raw"] = json!("other");
        assert!(retain_definitions(&root, &stale).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
