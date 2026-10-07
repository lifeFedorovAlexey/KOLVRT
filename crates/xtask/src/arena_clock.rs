//! Fixed, retained CLOCK-query experiments through the ordinary native ELF executor.
use super::*;
mod passport;
const MAGIC: u64 = 0x434c_4b01;
const ORDERS: [[u64; 2]; 3] = [[1, 2], [2, 1], [1, 2]];

pub(super) fn validate(events: &[Value]) -> Result<Value> {
    if events
        .iter()
        .any(|e| matches!(e["event"].as_str(), Some("panic" | "fatal")) || e["status"] == "fail")
    {
        return Err("clock scenario kernel failure".into());
    }
    let unique = |name: &str| -> Result<&Value> {
        let found: Vec<_> = events.iter().filter(|e| e["event"] == name).collect();
        if found.len() != 1 {
            return Err(format!("clock scenario requires unique {name}").into());
        }
        Ok(found[0])
    };
    let loader = unique("native-root-image")?;
    if loader["format"] != "elf64" || loader["generation"].as_u64().is_none_or(|g| g == 0) {
        return Err("clock scenario missing original ELF loader witness".into());
    }
    let complete = unique("native-boot")?;
    if complete["status"] != "complete"
        || complete["root_exit"] != 0
        || complete["owners_released"] != true
        || complete["frames_restored"] != true
        || complete["live_processes"] != 0
        || complete["live_domains"] != 0
    {
        return Err("clock scenario did not exit and reclaim all resources".into());
    }
    let words: Vec<u64> = unique("native-user-report")?["words"]
        .as_array()
        .ok_or("clock report words absent")?
        .iter()
        .map(|w| w.as_u64().ok_or("clock report requires exact u64 words"))
        .collect::<std::result::Result<_, _>>()?;
    if words.len() != 64
        || words[0] != MAGIC
        || words[1] != 1
        || !matches!(words[2], 1 | 2)
        || words[3] == 0
        || words[4..8] != [4, 24, 28, 0]
    {
        return Err("clock report header/schema/count/read-window mismatch".into());
    }
    let wall = &words[8..36];
    let window = &words[36..64];
    let residual: Vec<u64> = wall
        .iter()
        .zip(window)
        .map(|(wall, window)| {
            wall.checked_sub(*window)
                .ok_or("execution window exceeds wall interval")
        })
        .collect::<std::result::Result<_, _>>()?;
    Ok(
        json!({"mode":words[2],"frequency":words[3],"warmup_wall_ticks":&wall[..4],"sample_wall_ticks":&wall[4..],"warmup_execution_window_ticks":&window[..4],"sample_execution_window_ticks":&window[4..],"warmup_unattributed_ticks":&residual[..4],"sample_unattributed_ticks":&residual[4..],"read_window_total_ticks":0,"completed_queries":28,"raw_words":words,"owners_released":true,"frames_restored":true}),
    )
}
fn validate_mode(events: &[Value], mode: u64) -> Result<Value> {
    let result = validate(events)?;
    if result["mode"] != mode {
        return Err("clock observer mode does not match scheduled invocation".into());
    }
    Ok(result)
}
fn write_json(path: impl AsRef<Path>, value: &Value) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}
fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        return Err("clock source provenance git query failed".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
pub(super) fn run(args: &[String]) -> Result<()> {
    if args != ["clock-query"] {
        return Err("usage: cargo xtask arena run clock-query".into());
    }
    for (name, _) in env::vars_os() {
        let name = name.to_string_lossy();
        if matches!(
            name.as_ref(),
            "RUSTFLAGS"
                | "CARGO_ENCODED_RUSTFLAGS"
                | "CARGO_BUILD_RUSTFLAGS"
                | "CARGO_TARGET_AARCH64_UNKNOWN_NONE_RUSTFLAGS"
                | "RUSTC"
                | "RUSTC_WRAPPER"
                | "RUSTC_WORKSPACE_WRAPPER"
        ) || name.starts_with("CARGO_PROFILE_")
        {
            return Err(format!("fixed CLOCK class rejects build override {name}").into());
        }
    }
    let compiler = Command::new("rustc").arg("--version").output()?;
    if !compiler.status.success() {
        return Err("cannot observe actual rustc version".into());
    }
    let compiler_version = String::from_utf8(compiler.stdout)?;
    let root = PathBuf::from(format!(
        "target/kernel/arena-clock/{}-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        std::process::id()
    ));
    fs::create_dir_all(root.parent().ok_or("campaign parent absent")?)?;
    fs::create_dir(&root)?;
    let sources = source_inventory()?;
    let definition_snapshot = passport::snapshot()?;
    write_json(root.join("source-files.json"), &sources)?;
    let exact_commit = git(&["rev-parse", "HEAD"])?;
    let dirty = !git(&["status", "--porcelain"])?.is_empty();
    let mut attempts = Vec::new();
    let mut source_unchanged = true;
    'campaign: for prod in [false, true] {
        let profile = if prod { "prod" } else { "dev" };
        for (pair, order) in ORDERS.iter().enumerate() {
            for (position, &mode) in order.iter().enumerate() {
                if source_inventory()? != sources {
                    source_unchanged = false;
                    break 'campaign;
                }
                let directory = root.join(format!(
                    "{profile}/pair-{pair}/{}",
                    if mode == 1 { "off" } else { "on" }
                ));
                fs::create_dir_all(&directory)?;
                let mut attempt = json!({"profile":profile,"pair":pair,"order":position,"mode":mode,"directory":directory,"status":"failed"});
                let outcome = (|| -> Result<()> {
                    let (built, applications) = native_apps::build_images(
                        prod,
                        ["selftest-clock-client", "counter-service", "counter-client"],
                        mode,
                    )?;
                    attempt["applications"] = applications;
                    let elf = directory.join("kernel.elf");
                    fs::copy(&built, &elf)?;
                    let digest = Sha256::digest(fs::read(&elf)?)
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>();
                    attempt["kernel"] = json!({"artifact":elf,"sha256":digest});
                    let build = read_json(
                        built.with_file_name(format!("{profile}-native-apps-build.json")),
                    )?;
                    write_json(directory.join("kernel-build.json"), &build)?;
                    attempt["kernel_build"] = build;
                    execute_validated(&elf, false, true, false, true, Some(validate))?;
                    let run = read_json(elf.with_extension("run.json"))?;
                    if run["elf_sha256"] != digest {
                        return Err("clock kernel identity changed during execution".into());
                    }
                    let events = read_json(elf.with_extension("results.json"))?;
                    attempt["result"] =
                        validate_mode(events.as_array().ok_or("clock events absent")?, mode)?;
                    attempt["run"] = run;
                    attempt["events"] = events;
                    Ok(())
                })();
                if let Err(error) = outcome {
                    attempt["error"] = json!(error.to_string());
                } else {
                    attempt["status"] = json!("passed");
                }
                // Preserve available diagnostics even when the shared executor rejected the run.
                for (key, suffix) in [("run", "run.json"), ("events", "results.json")] {
                    let path = directory.join("kernel.elf").with_extension(suffix);
                    if path.exists() {
                        match read_json(path) {
                            Ok(value) => attempt[key] = value,
                            Err(error) => {
                                attempt["status"] = json!("failed");
                                attempt[format!("{key}_error")] = json!(error.to_string());
                            }
                        }
                    }
                }
                write_json(directory.join("attempt.json"), &attempt)?;
                attempts.push(attempt);
            }
        }
    }
    source_unchanged &= source_inventory()? == sources;
    let passed = attempts.iter().filter(|a| a["status"] == "passed").count();
    let frequencies_consistent = ["dev", "prod"].iter().all(|profile| {
        let frequencies: BTreeSet<_> = attempts
            .iter()
            .filter(|a| a["profile"] == *profile && a["status"] == "passed")
            .filter_map(|a| a["result"]["frequency"].as_u64())
            .collect();
        frequencies.len() == 1
    });
    let success =
        source_unchanged && frequencies_consistent && passed == 12 && attempts.len() == 12;
    let campaign = json!({"schema_version":1,"compiler_version":compiler_version,"definition_snapshot":definition_snapshot,"exact_commit":exact_commit,"worktree_dirty":dirty,"source_files":sources,"source_unchanged":source_unchanged,"frequencies_consistent":frequencies_consistent,"planned_attempts":12,"attempted":attempts.len(),"passed":passed,"status":if success {"passed"} else {"failed"},"attempts":attempts,"record_eligible":false,"limitations":["QEMU TCG timer ticks, not physical throughput","Execution window includes transition accounting and host stalls; not exclusive CPU","READ_WINDOW body is zero for this CLOCK-only scenario; residual is unattributed","Matched observer comparison measures incremental recorder writes, not full CLOCK measurement overhead","Four fixed warmups do not prove stabilization; 24 samples do not establish tail latency"]});
    write_json(root.join("campaign.json"), &campaign)?;
    passport::write(&root, &campaign)?;
    if !success {
        return Err(format!(
            "clock campaign failed; all attempted runs retained at {}",
            root.display()
        )
        .into());
    }
    println!("Clock campaign retained at {}", root.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<Value> {
        let mut words = vec![MAGIC, 1, 1, 62_500_000, 4, 24, 28, 0];
        words.extend([100; 28]);
        words.extend([40; 28]);
        vec![
            json!({"event":"native-root-image","format":"elf64","generation":1}),
            json!({"event":"native-user-report","words":words}),
            json!({"event":"native-boot","status":"complete","root_exit":0,"owners_released":true,"frames_restored":true,"live_processes":0,"live_domains":0}),
        ]
    }
    #[test]
    fn clock_oracle_accepts_exact_input_and_preserves_samples() {
        let result = validate_mode(&fixture(), 1).unwrap();
        assert_eq!(result["sample_wall_ticks"].as_array().unwrap().len(), 24);
        assert_eq!(result["sample_unattributed_ticks"][0], 60);
        assert!(validate_mode(&fixture(), 2).is_err());
        assert_eq!(ORDERS, [[1, 2], [2, 1], [1, 2]]);
    }
    #[test]
    fn clock_oracle_rejects_missing_duplicate_and_failure_events() {
        for index in 0..3 {
            let mut f = fixture();
            f.remove(index);
            assert!(validate(&f).is_err());
            let mut f = fixture();
            f.push(f[index].clone());
            assert!(validate(&f).is_err());
        }
        for e in [
            json!({"event":"panic"}),
            json!({"event":"fatal"}),
            json!({"event":"scheduler-reject","status":"fail"}),
        ] {
            let mut f = fixture();
            f.push(e);
            assert!(validate(&f).is_err());
        }
    }
    #[test]
    fn clock_oracle_rejects_forged_headers_counts_and_arithmetic() {
        for (index, value) in [
            (0, json!(0)),
            (1, json!(0)),
            (2, json!(3)),
            (3, json!(0)),
            (4, json!(0)),
            (5, json!(23)),
            (6, json!(27)),
            (7, json!(1)),
            (8, json!(1.5)),
            (36, json!(101)),
            (8, json!(-1)),
        ] {
            let mut f = fixture();
            f[1]["words"][index] = value;
            assert!(validate(&f).is_err(), "{index}");
        }
        let mut f = fixture();
        f[1]["words"].as_array_mut().unwrap().pop();
        assert!(validate(&f).is_err());
    }
    #[test]
    fn clock_oracle_rejects_unreclaimed_or_failed_root() {
        for (field, value) in [
            ("status", json!("root-failed")),
            ("root_exit", json!(1)),
            ("owners_released", json!(false)),
            ("frames_restored", json!(false)),
            ("live_processes", json!(1)),
            ("live_domains", json!(1)),
        ] {
            let mut f = fixture();
            f[2][field] = value;
            assert!(validate(&f).is_err());
        }
    }
}
