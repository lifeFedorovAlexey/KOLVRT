//! Immutable schema-v1 bundles; producer assertions never imply reviewed admission.
use super::*;
use crate::arena_common::passport::{artifact, assess, retain_attempt, retain_review};
use crate::arena_common::{digest, manifest_matches, value_digest};
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
    for path in ["docs/kernel/clock.md", "crates/xtask/src/arena_clock.rs"] {
        let text = fs::read_to_string(path)?.replace("\r\n", "\n");
        files.insert(path.into(), json!({"sha256_lf":digest(text.as_bytes())}));
    }
    Ok(Value::Object(files))
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
            let pins_valid = profile["target"]["contract_sha256"]
                == frozen["docs/kernel/clock.md"]["sha256_lf"]
                && profile["workload"]["oracle"]["sha256"]
                    == frozen["crates/xtask/src/arena_clock.rs"]["sha256_lf"]
                && profile["workload"]["semantics_sha256"]
                    == frozen["research/arena/clock-query/protocol.json"]["sha256"]
                && profile["overhead"]["matched_workload_sha256"]
                    == frozen["research/arena/clock-query/protocol.json"]["sha256"]
                && profile["workload"]["input_sha256"]
                    == frozen["research/arena/clock-query/input.json"]["sha256"]
                && profile["environment"]["configuration_sha256"]
                    == frozen[&manifest_path]["sha256"]
                && profile["environment"]["resource_policy_sha256"]
                    == frozen["research/arena/clock-query/resources.json"]["sha256"];
            let valid = pins_valid
                && campaign["status"] == "passed"
                && campaign["compiler_version"]
                    .as_str()
                    .and_then(|s| s.split_whitespace().nth(1))
                    == manifest["compiler_version"].as_str()
                && manifest_matches(off, manifest)
                && manifest_matches(on, manifest);
            let mut artifacts = Vec::new();
            artifact(
                &bundle,
                "protocol.json",
                Path::new("research/arena/clock-query/protocol.json"),
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
            let image_hash = retain_attempt(&bundle, "off", off, &mut artifacts)?;
            retain_attempt(&bundle, "on", on, &mut artifacts)?;
            let status = if valid { "PASS" } else { "FAIL" };
            let reason = if valid {
                "Fresh pair passed actor/host oracle and declared environment; no execution attestation"
            } else {
                "Campaign or exact declared environment mismatch; raw observations retained, not admitted"
            };
            let source_review = retain_review(
                &bundle,
                "research/arena/clock-query",
                false,
                &[
                    "crates/xtask/src/arena_clock.rs",
                    "crates/xtask/src/arena_clock/passport.rs",
                    "crates/xtask/src/arena_common.rs",
                    "crates/xtask/src/arena_common/passport.rs",
                    "apps/native-runtime/src/lib.rs",
                    "tests/native-apps/src/clock-client.rs",
                ],
                &mut artifacts,
            )?;
            let negative_review = retain_review(
                &bundle,
                "research/arena/clock-query",
                true,
                &[
                    "crates/xtask/src/arena_clock.rs",
                    "crates/xtask/src/arena_clock/passport.rs",
                    "crates/xtask/src/arena_common.rs",
                    "crates/xtask/src/arena_common/passport.rs",
                    "crates/repository-checks/src/arena.rs",
                    "schemas/arena-run.schema.json",
                ],
                &mut artifacts,
            )?;
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
            let run = json!({"schema_version":1,"run_id":format!("clock-{profile_name}-{pair}-{}",root.file_name().unwrap().to_string_lossy()),"scope":"KERNEL_QEMU","profile_snapshot":profile,"registry_snapshot":registry,"profile_sha256":value_digest(profile)?,"registry_sha256":value_digest(registry)?,"provenance":{"exact_commit":campaign["exact_commit"],"source_sha256":source_hash,"image_sha256":image_hash,"submission":"CLOCK pilot; raw source inventory represents compiled working tree; not independent attestation","contribution_state":"UNATTRIBUTED","contributors":[],"contribution_evidence":[]},"environment_manifest":{"path":"environment.json","sha256":environment_hash},"artifacts":artifacts,"correctness":{"status":status,"reason":reason,"evidence":["pair.json"]},"security_results":security,"observations":[observation],"overhead_result":{"protocol_id":profile["overhead"]["id"],"protocol_version":profile["overhead"]["version"],"matched_workload_sha256":profile["overhead"]["matched_workload_sha256"],"state":"AVAILABLE","reason":"Incremental intermediate recorder stores; not total timestamp-probe cost","off_samples":off["result"]["sample_wall_ticks"],"on_samples":on["result"]["sample_wall_ticks"],"evidence":["pair.json"]},"warmup":{"status":status,"reason":"Four fixed retained conditioning samples completed; does not assert statistical stabilization","evidence":["pair.json"]}});
            let entry = assess(root, &bundle, &format!("{profile_name}-{pair}"), run)?;
            summaries.push(json!({"profile":profile_name,"pair":pair,"run":entry["run"],"assessment":entry["assessment"]}));
        }
    }
    let rejected: Vec<_> = summaries
        .iter()
        .filter(|entry| entry["assessment"]["admission_state"] != "STRUCTURALLY_ADMISSIBLE")
        .cloned()
        .collect();
    write_json(
        root.join("passport-summary.json"),
        &json!({"schema_version":1,"record_eligible":false,"admission_state":if rejected.is_empty(){"STRUCTURALLY_ADMISSIBLE"}else{"INELIGIBLE"},"pairs":summaries}),
    )?;
    if !rejected.is_empty() {
        return Err(format!(
            "clock admission INELIGIBLE; all pairs retained at {}: {}",
            root.display(),
            serde_json::to_string(&rejected)?
        )
        .into());
    }
    Ok(())
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
