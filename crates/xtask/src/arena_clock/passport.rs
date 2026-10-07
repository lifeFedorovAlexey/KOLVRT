//! Immutable schema-v1 bundles; producer assertions never imply reviewed admission.
use super::*;
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn value_digest(value: &Value) -> Result<String> {
    Ok(digest(serde_json::to_string(value)?.as_bytes()))
}
pub(super) fn snapshot() -> Result<Value> {
    let mut files = serde_json::Map::new();
    for path in [
        "research/arena/standards.json",
        "research/arena/profiles/clock-query-dev.json",
        "research/arena/profiles/clock-query-prod.json",
        "research/arena/clock-query/dev-environment.json",
        "research/arena/clock-query/prod-environment.json",
        "research/arena/clock-query/resources.json",
        "research/arena/clock-query/input.json",
        "research/arena/clock-query/protocol.json",
    ] {
        let bytes = fs::read(path)?;
        files.insert(
            path.into(),
            json!({"sha256":digest(&bytes),"value":serde_json::from_slice::<Value>(&bytes)?}),
        );
    }
    Ok(Value::Object(files))
}
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
fn review(path: &str, negative: bool) -> Result<Value> {
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
    let mut required = vec![
        "crates/xtask/src/arena_clock.rs",
        "crates/xtask/src/arena_clock/passport.rs",
    ];
    if negative {
        required.extend([
            "crates/repository-checks/src/arena.rs",
            "schemas/arena-run.schema.json",
        ]);
    } else {
        required.extend([
            "apps/native-runtime/src/lib.rs",
            "tests/native-apps/src/clock-client.rs",
        ]);
    }
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
fn retain_review(bundle: &Path, negative: bool, artifacts: &mut Vec<Value>) -> Result<Vec<String>> {
    let name = if negative {
        "negative-review"
    } else {
        "source-review"
    };
    let path = format!("research/arena/clock-query/{name}.json");
    let value = match review(&path, negative) {
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
fn artifact(
    bundle: &Path,
    name: &str,
    source: &Path,
    artifacts: &mut Vec<Value>,
) -> Result<String> {
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
        fs::write(bundle.join(name), bytes)?;
        name.to_owned()
    };
    if !artifacts.iter().any(|a| a["path"] == path) {
        artifacts.push(json!({"path":path,"sha256":hash}));
    }
    Ok(hash)
}
fn manifest_matches(attempt: &Value, manifest: &Value) -> bool {
    let Some(args) = attempt["run"]["arguments"].as_array() else {
        return false;
    };
    let expected = manifest["qemu_arguments_without_kernel"].as_array();
    let arguments_match = expected.is_some_and(|expected| {
        args.len() == expected.len() + 2
            && &args[..expected.len()] == expected.as_slice()
            && args[expected.len()] == "-kernel"
            && args.last() == Some(&attempt["kernel"]["artifact"])
    });
    arguments_match
        && attempt["result"]["frequency"] == manifest["expected_counter_frequency"]
        && attempt["run"]["configured_cpus"] == manifest["configured_cpus"]
        && attempt["run"]["active_cpus"] == manifest["active_cpus"]
        && attempt["kernel_build"]["compiler"] == manifest["compiler_version"]
        && attempt["kernel_build"]["target"] == manifest["target"]
        && attempt["kernel_build"]["features"] == manifest["kernel_features"]
        && attempt["run"]["qemu_version"]
            .as_str()
            .is_some_and(|version| {
                version.lines().next().is_some_and(|line| {
                    line.split_whitespace().nth(3) == manifest["qemu_version"].as_str()
                })
            })
}
pub(super) fn write(root: &Path, campaign: &Value) -> Result<()> {
    let frozen = &campaign["definition_snapshot"];
    if *frozen != snapshot()? {
        write_json(
            root.join("passport-summary.json"),
            &json!({"admission_state":"INELIGIBLE","record_eligible":false,"reason":"measurement definitions changed during campaign"}),
        )?;
        return Err("clock measurement definitions changed".into());
    }
    let registry = &frozen["research/arena/standards.json"]["value"];
    let attempts = campaign["attempts"]
        .as_array()
        .ok_or("clock attempts absent")?;
    let mut summaries = Vec::new();
    for profile_name in ["dev", "prod"] {
        let profile_path = format!("research/arena/profiles/clock-query-{profile_name}.json");
        let profile = &frozen[&profile_path]["value"];
        let manifest_path = format!("research/arena/clock-query/{profile_name}-environment.json");
        let manifest = &frozen[&manifest_path]["value"];
        for pair in 0..3 {
            let pair_attempts: Vec<_> = attempts
                .iter()
                .filter(|a| a["profile"] == profile_name && a["pair"] == pair)
                .collect();
            let off = pair_attempts.iter().find(|a| a["mode"] == 1).copied();
            let on = pair_attempts.iter().find(|a| a["mode"] == 2).copied();
            let bundle = root.join(format!("passports/{profile_name}-{pair}"));
            fs::create_dir_all(&bundle)?;
            write_json(
                bundle.join("pair.json"),
                &json!({"profile":profile_name,"pair":pair,"attempts":pair_attempts}),
            )?;
            let Some((off, on)) = off
                .zip(on)
                .filter(|(off, on)| off["status"] == "passed" && on["status"] == "passed")
            else {
                let summary = json!({"profile":profile_name,"pair":pair,"admission_state":"INELIGIBLE","record_eligible":false,"reason":"missing or failed fixed pair; raw attempts and diagnostics retained"});
                write_json(bundle.join("assessment.json"), &summary)?;
                summaries.push(summary);
                continue;
            };
            let valid = campaign["status"] == "passed"
                && campaign["compiler_version"]
                    .as_str()
                    .and_then(|s| s.split_whitespace().nth(1))
                    == manifest["compiler_version"].as_str()
                && manifest_matches(off, manifest)
                && manifest_matches(on, manifest);
            let mut artifacts = Vec::new();
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
            let environment_hash = artifact(
                &bundle,
                "environment.json",
                Path::new(&manifest_path),
                &mut artifacts,
            )?;
            artifact(
                &bundle,
                "resources.json",
                Path::new("research/arena/clock-query/resources.json"),
                &mut artifacts,
            )?;
            artifact(
                &bundle,
                "input.json",
                Path::new("research/arena/clock-query/input.json"),
                &mut artifacts,
            )?;
            let mut image_hash = String::new();
            for (mode, attempt) in [("off", off), ("on", on)] {
                let kernel = Path::new(
                    attempt["kernel"]["artifact"]
                        .as_str()
                        .ok_or("kernel artifact missing")?,
                );
                let hash = artifact(
                    &bundle,
                    &format!("{mode}-kernel.elf"),
                    kernel,
                    &mut artifacts,
                )?;
                if hash != attempt["kernel"]["sha256"] {
                    return Err("kernel image changed before bundle retention".into());
                }
                if mode == "off" {
                    image_hash = hash;
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
                        &bundle,
                        &format!("{mode}-{name}"),
                        &directory.join(name),
                        &mut artifacts,
                    )?;
                }
                for role in ["root", "service", "client"] {
                    let app = &attempt["applications"][role];
                    let hash = artifact(
                        &bundle,
                        &format!("{mode}-{role}.elf"),
                        Path::new(
                            app["artifact"]
                                .as_str()
                                .ok_or("application artifact absent")?,
                        ),
                        &mut artifacts,
                    )?;
                    if hash != app["sha256"] {
                        return Err("application image changed before bundle retention".into());
                    }
                }
            }
            let status = if valid { "PASS" } else { "FAIL" };
            let reason = if valid {
                "Fresh pair passed actor/host oracle and declared environment; no execution attestation"
            } else {
                "Campaign or exact declared environment mismatch; raw observations retained, not admitted"
            };
            let source_review = retain_review(&bundle, false, &mut artifacts)?;
            let negative_review = retain_review(&bundle, true, &mut artifacts)?;
            let security:Vec<_>=profile["security"]["sfr"].as_array().ok_or("SFR missing")?.iter().map(|sfr| {
                let sars:Vec<_>=sfr["required_sar"].as_array().unwrap().iter().map(|id| {
                    let evidence = match id.as_str() {Some("arena.sar.clock-execution") => vec!["pair.json".to_owned()], Some("arena.sar.clock-negative") => negative_review.clone(), Some("arena.sar.clock-source-review") => source_review.clone(), _ => vec![]};
                    let state = if evidence.is_empty() {"NOT_RUN"} else if valid {"PASS"} else {"FAIL"};
                    json!({"sar_id":id,"status":state,"reason":if evidence.is_empty() {"Required independent evidence absent or source/log validation failed"} else {"Source-bound evidence retained and validated; not certification"},"evidence":evidence})
                }).collect();
                let all=sars.iter().all(|s|s["status"]=="PASS");
                json!({"sfr_id":sfr["id"],"status":if all {"PASS"} else {"NOT_RUN"},"reason":"Bounded evidence only; missing SAR is not inferred from functional success","sar_results":sars})
            }).collect();
            let observation = json!({"metric_id":profile["metrics"][0]["id"],"state":"AVAILABLE","samples":off["result"]["sample_wall_ticks"],"warmup_samples":off["result"]["warmup_wall_ticks"],"failures":0,"dropped":0,"unfinished":0});
            let mut run = json!({"schema_version":1,"run_id":format!("clock-{profile_name}-{pair}-{}",root.file_name().unwrap().to_string_lossy()),"scope":"KERNEL_QEMU","profile_snapshot":profile,"registry_snapshot":registry,"profile_sha256":value_digest(profile)?,"registry_sha256":value_digest(registry)?,"provenance":{"exact_commit":campaign["exact_commit"],"source_sha256":source_hash,"image_sha256":image_hash,"submission":"CLOCK pilot; raw source inventory represents compiled working tree; not independent attestation","contribution_state":"UNATTRIBUTED","contributors":[],"contribution_evidence":[]},"environment_manifest":{"path":"environment.json","sha256":environment_hash},"artifacts":artifacts,"correctness":{"status":status,"reason":reason,"evidence":["pair.json"]},"security_results":security,"observations":[observation],"overhead_result":{"protocol_id":profile["overhead"]["id"],"protocol_version":profile["overhead"]["version"],"matched_workload_sha256":profile["overhead"]["matched_workload_sha256"],"state":"AVAILABLE","reason":"Incremental intermediate recorder stores; not total timestamp-probe cost","off_samples":off["result"]["sample_wall_ticks"],"on_samples":on["result"]["sample_wall_ticks"],"evidence":["pair.json"]},"warmup":{"status":status,"reason":"Four fixed retained conditioning samples completed; does not assert statistical stabilization","evidence":["pair.json"]}});
            let prefix = format!("passports/{profile_name}-{pair}/");
            let paths: Vec<String> = run["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|a| a["path"].as_str())
                .filter(|p| !p.starts_with("images/"))
                .map(str::to_owned)
                .collect();
            qualify_paths(&mut run, &paths, &prefix);
            let run_path = root.join(format!("{profile_name}-{pair}-run.json"));
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
            let assessment=serde_json::from_slice::<Value>(&output.stdout).unwrap_or_else(|_|json!({"admission_state":"INELIGIBLE","record_eligible":false,"reason":"existing assessor rejected malformed or incomplete evidence","exit_code":output.status.code()}));
            write_json(bundle.join("assessment.json"), &assessment)?;
            summaries.push(
                json!({"profile":profile_name,"pair":pair,"run":run_path,"assessment":assessment}),
            );
        }
    }
    write_json(
        root.join("passport-summary.json"),
        &json!({"schema_version":1,"record_eligible":false,"pairs":summaries}),
    )?;
    Ok(())
}
fn qualify_paths(value: &mut Value, paths: &[String], prefix: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_manifest_rejects_incomplete_and_wrong_frequency() {
        let manifest = json!({"qemu_arguments_without_kernel":[],"expected_counter_frequency":62500000,"configured_cpus":2,"active_cpus":2,"compiler_version":"1.99.0","target":"aarch64-unknown-none","kernel_features":[],"qemu_version":"10.1.0"});
        let mut a = json!({"kernel":{"artifact":"kernel.elf"},"result":{"frequency":62500000},"run":{"arguments":["-kernel","kernel.elf"],"configured_cpus":2,"active_cpus":2,"qemu_version":"QEMU emulator version 10.1.0\n"},"kernel_build":{"compiler":"1.99.0","target":"aarch64-unknown-none","features":[]}});
        assert!(manifest_matches(&a, &manifest));
        a["result"]["frequency"] = json!(1);
        assert!(!manifest_matches(&a, &manifest));
        assert!(!manifest_matches(&json!({}), &manifest));
    }
}
