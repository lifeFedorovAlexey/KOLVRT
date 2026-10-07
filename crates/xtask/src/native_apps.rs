//! Standalone ELF build/boot acceptance. Never extracts ELF into raw user code.
use super::*;
const MAGIC: u64 = 0x4b4f_4c56_5254_3701;

pub fn validate(events: &[Value]) -> Result<Value> {
    if events
        .iter()
        .any(|e| matches!(e["event"].as_str(), Some("panic" | "fatal")) || e["status"] == "fail")
    {
        return Err("native kernel failure".into());
    }
    let reports: Vec<_> = events
        .iter()
        .filter(|e| e["event"] == "native-user-report")
        .collect();
    if reports.len() != 1 {
        return Err("missing unique userspace result; ordinary boot is insufficient".into());
    }
    let words = reports[0]["words"]
        .as_array()
        .ok_or("invalid userspace result words")?;
    if words.first() == Some(&json!(MAGIC)) && words.get(1) == Some(&json!(3)) {
        return validate_lifecycle(events, words);
    }
    if words.first() == Some(&json!(MAGIC)) && words.get(1) == Some(&json!(4)) {
        return validate_session(events, words);
    }
    Err("unknown completed-native scenario schema".into())
}

pub fn smoke(args: &[String]) -> Result<()> {
    run_suite(args, "counter-client", 2)
}
pub fn runtime(args: &[String]) -> Result<()> {
    run_suite(args, "counter-client", 1)
}
pub fn selftest(args: &[String]) -> Result<()> {
    let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args([
            "test",
            "--locked",
            "-p",
            "native-userspace",
            "-p",
            "kernel-core",
            "-p",
            "native-protocol-model",
            "-p",
            "native-state-models",
        ])
        .status()?;
    if !status.success() {
        return Err("L1 component tests failed".into());
    }
    for binary in ["counter-client", "selftest-abi-client"] {
        run_suite(args, binary, 2)?;
        if binary == "counter-client" {
            run_suite(args, binary, 1)?;
            crash_current_runtime()?;
        }
    }
    controls(args)?;
    Ok(())
}
pub fn crash_recovery(args: &[String]) -> Result<()> {
    run_suite(args, "counter-client", 2)?;
    crash_current_runtime()
}
fn crash_current_runtime() -> Result<()> {
    let sources = source_inventory()?;
    let status = Command::new("node")
        .env("QEMU_AARCH64", qemu()?)
        .args([
            "tests/system/native-crash-recovery.cjs",
            "target/kernel/native-counter-client-2-positive.json",
        ])
        .status()?;
    if !status.success() {
        return Err("production supervisor crash-recovery SYSTEM scenario failed".into());
    }
    if source_inventory()? != sources {
        return Err("sources changed during production supervisor crash scenario".into());
    }
    Ok(())
}
pub fn lifecycle(args: &[String]) -> Result<()> {
    run_suite(args, "selftest-lifecycle-peer", 6)
}
pub fn controls(args: &[String]) -> Result<()> {
    for options in [
        vec![
            "test",
            "--locked",
            "-p",
            "kernel-core",
            "--test",
            "elf_contract",
        ],
        vec![
            "test",
            "--locked",
            "-p",
            "native-userspace",
            "--test",
            "counter",
        ],
        vec![
            "test",
            "--locked",
            "-p",
            "native-apps",
            "--test",
            "supervision",
        ],
        vec![
            "test",
            "--locked",
            "-p",
            "xtask",
            "--bin",
            "xtask",
            "lifecycle_oracle",
        ],
    ] {
        let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .args(&options)
            .status()?;
        if !status.success() {
            return Err(
                format!("production-method negative input tests failed: {options:?}").into(),
            );
        }
    }
    lifecycle(args)?;
    fs::write(
        "target/kernel/native-negative-inputs.json",
        serde_json::to_string_pretty(&json!({
            "scope":"invalid inputs to actual production methods/public ABI and host oracle; no source mutation or corrupted application implementation",
            "coverage":[
                {"name":"elf","level":"UNIT","evidence":"kernel_core::elf::parse malformed image inputs"},
                {"name":"authorization","level":"SYSTEM","evidence":"actual denied child lifecycle/SEND/reply and root binding calls"},
                {"name":"stale-instance","level":"SYSTEM","evidence":"old lifecycle token and SEND handle return exact stale errors"},
                {"name":"readiness-binding","level":"SYSTEM","evidence":"wrong target token denied; correct binding reaches actual service"},
                {"name":"reply","level":"UNIT/SYSTEM","evidence":"production reply parser malformed inputs; forged public reply rejected"},
                {"name":"ipc","level":"SYSTEM","evidence":"wrong endpoint kind rejected; actual submit/wait/collect/RPC round trips"},
                {"name":"resource-release","level":"SYSTEM/UNIT","evidence":"live replacement denied, actual kernel zero charges/frames restored, host oracle rejects retained-resource observations"}
            ],
            "source_files":source_inventory()?,"hardware":"UNKNOWN",
            "limitations":["Not source-mutant detection", "Not production supervisor crash-recovery acceptance", "Fixture mutations are UNIT inputs, not kernel execution"]
        }))?,
    )?;
    Ok(())
}
fn run_suite(args: &[String], client_binary: &str, mode: u64) -> Result<()> {
    run_mode(args, client_binary, mode)
}
fn run_mode(args: &[String], client_binary: &str, mode: u64) -> Result<()> {
    if args
        .iter()
        .any(|arg| arg != "--prod" && !(mode == 1 && arg == "--live"))
    {
        return Err(
            "native commands accept --prod; service-run additionally accepts --live".into(),
        );
    }
    let live = args.iter().any(|arg| arg == "--live");
    let profiles: Vec<_> = if live {
        vec![args.iter().any(|arg| arg == "--prod")]
    } else if args.is_empty() {
        vec![false, true]
    } else {
        vec![true]
    };
    let sources = source_inventory()?;
    let mut results = serde_json::Map::new();
    for prod in profiles {
        let profile = if prod { "prod" } else { "dev" };
        fs::create_dir_all("target/kernel/apps")?;
        let mut guest = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        guest
            .args([
                "build",
                "--locked",
                "-p",
                "native-apps",
                "--features",
                "guest",
                "--target",
                "aarch64-unknown-none",
            ])
            .env_remove("CARGO_TARGET_DIR");
        if client_binary.starts_with("selftest-") {
            guest.args(["-p", "native-selftests"]);
        }
        if prod {
            guest.arg("--release");
        }
        if !guest.status()?.success() {
            return Err("standalone native applications failed to build".into());
        }
        let mut apps = serde_json::Map::new();
        let mut images = Vec::new();
        for (role, binary) in [
            (
                "root",
                if mode == 6 {
                    "selftest-lifecycle-client"
                } else {
                    "native-supervisor"
                },
            ),
            ("service", "counter-service"),
            ("client", client_binary),
        ] {
            let original = PathBuf::from(format!(
                "target/aarch64-unknown-none/{}/{binary}",
                if prod { "release" } else { "debug" }
            ));
            let bytes = fs::read(&original)?;
            let digest = Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            let destination =
                PathBuf::from(format!("target/kernel/apps/{profile}-{role}-{digest}.elf"));
            if destination.exists() {
                if fs::read(&destination)? != bytes {
                    return Err("immutable ELF digest path content mismatch".into());
                }
            } else {
                fs::write(&destination, &bytes)?;
            }
            images.push(destination.clone());
            apps.insert(role.into(),json!({"artifact":destination,"original_elf_bytes":bytes.len(),"sha256":Sha256::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),"host_extracted_raw":false}));
        }
        let elf = build_mode(
            prod,
            false,
            Some("native-apps"),
            true,
            if mode == 6 { 1 } else { mode },
            Some(
                &images
                    .try_into()
                    .map_err(|_| "three original ELF images required")?,
            ),
        )?;
        let suite_elf = PathBuf::from(format!(
            "target/kernel/{profile}-{client_binary}-{mode}-positive.elf"
        ));
        fs::copy(&elf, &suite_elf)?;
        if mode == 1 {
            if live {
                println!(
                    "Persistent native runtime: {}; stop the host process to end QEMU",
                    suite_elf.display()
                );
                let status = Command::new(qemu()?).args(qemu_args(&suite_elf)).status()?;
                return Err(format!(
                    "Persistent runtime stopped: {status}; no clean-shutdown acceptance claimed"
                )
                .into());
            }
            let result = observe_runtime(&suite_elf)?;
            results.insert(profile.into(), json!({"applications":apps,"result":result}));
            continue;
        }
        execute_native(&suite_elf)?;
        let events = read_json(suite_elf.with_extension("results.json"))?;
        let observed = events.as_array().ok_or("native events absent")?;
        let result = validate(observed)?;
        results.insert(profile.into(),json!({"applications":apps,"result":result,"kernel_build":read_json(elf.with_file_name(format!("{profile}-native-apps-build.json")))?,"run":read_json(suite_elf.with_extension("run.json"))?,"events":events}));
        println!("Native ELF {client_binary} passed ({profile}): {result}");
    }
    if sources != source_inventory()? {
        return Err("native application sources changed during smoke".into());
    }
    fs::write(
        format!("target/kernel/native-{client_binary}-{mode}-positive.json"),
        serde_json::to_string_pretty(
            &json!({"schema_version":1,"source_files":sources,"profiles":results,"scope":"standalone ELF native IPC smoke, not complete Phase 3.7 or hardware acceptance"}),
        )?,
    )?;
    Ok(())
}

fn observe_runtime(elf: &Path) -> Result<Value> {
    let log = elf.with_extension("runtime.log");
    let emulator = qemu()?;
    let version = String::from_utf8(Command::new(&emulator).arg("--version").output()?.stdout)?;
    let run = json!({"qemu_version":version,"arguments":qemu_args(elf),"accelerator":"TCG","kernel_sha256":Sha256::digest(fs::read(elf)?).iter().map(|b|format!("{b:02x}")).collect::<String>()});
    let mut child = Command::new(emulator)
        .args(qemu_args(elf))
        .stdin(Stdio::null())
        .stdout(fs::File::create(&log)?)
        .stderr(fs::File::create(elf.with_extension("stderr"))?)
        .spawn()?;
    let start = Instant::now();
    let observed = (|| -> Result<Vec<Value>> {
        loop {
            if child.try_wait()?.is_some() {
                return Err("persistent runtime exited unexpectedly".into());
            }
            let text = fs::read_to_string(&log)?;
            let complete = text.rfind('\n').map_or("", |end| &text[..=end]);
            let events = output::parse(complete)?;
            if events.iter().any(|e| {
                e["event"] == "native-report-progress"
                    && e["words"].as_array().is_some_and(|w| {
                        w.len() == 6
                            && w[0] == MAGIC
                            && w[1] == 2
                            && w[2] == 12
                            && w[3].as_u64().is_some_and(|n| n > 0)
                            && w[4] == 0
                            && w[5] == 1
                    })
            }) {
                return Ok(events);
            }
            if start.elapsed() > Duration::from_secs(10) {
                return Err("persistent EL0 runtime observation missing".into());
            }
            thread::sleep(QEMU_POLL_INTERVAL);
        }
    })();
    let _ = child.kill();
    child.wait()?;
    let observed = observed?;
    if observed
        .iter()
        .any(|e| matches!(e["event"].as_str(), Some("panic" | "fatal" | "native-boot")))
    {
        return Err("persistent runtime failed or terminated".into());
    }
    if !observed.iter().any(|e| {
        e["event"] == "native-completion"
            && e["selector"] == 1
            && e["kind"] == "exit"
            && e["code"] == 0
    }) || observed
        .iter()
        .filter(|e| e["event"] == "native-image" && e["selector"] == 0)
        .count()
        != 1
        || observed
            .iter()
            .any(|e| e["event"] == "native-process-terminal" && e["owner"] == 1)
    {
        return Err(
            "ordinary runtime clean client/single nonfaulting service evidence missing".into(),
        );
    }
    fs::write(
        elf.with_extension("runtime-results.json"),
        serde_json::to_string_pretty(&observed)?,
    )?;
    Ok(
        json!({"service_outlives_client":true,"counter_after_client_exit":12,"kernel_watchdog_ms":0,"guest_shutdown":false,"host_observation_stop":true,"resource_leaks":"NOT_MEASURED_ON_FORCED_VM_STOP","kernel_sha256":Sha256::digest(fs::read(elf)?).iter().map(|b|format!("{b:02x}")).collect::<String>(),"run":run,"events":observed}),
    )
}

fn validate_session(events: &[Value], words: &[Value]) -> Result<Value> {
    if words.len() != 7
        || words[2].as_u64().is_none_or(|n| n < 3)
        || words[3] != 12
        || words[4].as_u64().is_none_or(|n| n == 0)
        || words[5].as_u64().is_none_or(|n| n == 0)
        || words[6] != 3
    {
        return Err("ordinary supervisor session result invalid".into());
    }
    let one = |event: &str| -> Result<&Value> {
        let found: Vec<_> = events.iter().filter(|e| e["event"] == event).collect();
        if found.len() != 1 {
            return Err(format!("missing unique {event}").into());
        }
        Ok(found[0])
    };
    if one("native-root-image")?["format"] != "elf64" {
        return Err("root ELF missing".into());
    }
    let ended = one("native-boot")?;
    if ended["status"] != "complete"
        || ended["root_exit"] != 0
        || ended["owners_released"] != true
        || ended["frames_restored"] != true
        || ended["live_processes"] != 0
        || ended["live_domains"] != 0
    {
        return Err("ordinary session resources retained".into());
    }
    for (selector, kind) in [(0, "terminated"), (1, "exit")] {
        let images: Vec<_> = events
            .iter()
            .filter(|e| e["event"] == "native-image" && e["selector"] == selector)
            .collect();
        let completions: Vec<_> = events
            .iter()
            .filter(|e| e["event"] == "native-completion" && e["selector"] == selector)
            .collect();
        if images.len() != 1
            || images[0]["format"] != "elf64"
            || completions.len() != 1
            || completions[0]["kind"] != kind
            || completions[0]["code"] != 0
            || completions[0]["generation"] != images[0]["generation"]
        {
            return Err("ordinary session actual child completion missing".into());
        }
    }
    Ok(
        json!({"supervisor_ready":true,"production_supervisor_tested":true,"client_loaded_as_elf":true,
        "requests_completed":words[2],"counter_final":12,"service_instance":words[4],"client_exit":"success",
        "guest_shutdown":true,"resource_leaks":0,"frames_restored":true,"owners_released":true,
        "service_restarted":false,"scope":"ordinary production session finishes after its client; separate persistent and crash scenarios remain required","hardware":"UNKNOWN"}),
    )
}

fn validate_lifecycle(events: &[Value], words: &[Value]) -> Result<Value> {
    if words.len() != 50 {
        return Err("lifecycle step ledger incomplete".into());
    }
    let mut steps = std::collections::BTreeMap::new();
    for row in words[2..].as_chunks::<4>().0 {
        let id = row[0].as_u64().ok_or("step id absent")?;
        let values = [
            row[1].as_u64().ok_or("step value absent")?,
            row[2].as_u64().ok_or("step value absent")?,
            row[3].as_u64().ok_or("step value absent")?,
        ];
        if steps.insert(id, values).is_some() {
            return Err("duplicate lifecycle step".into());
        }
    }
    let step = |id: u64| -> Result<[u64; 3]> {
        steps
            .get(&id)
            .copied()
            .ok_or_else(|| format!("missing step {id}").into())
    };
    let old = step(10)?;
    let fresh = step(15)?;
    let binding = step(13)?;
    let rebound = step(17)?;
    if old[0] == 0
        || old[1] == 0
        || step(11)?[0] != 13
        || step(12)? != [5, 12, 12]
        || binding[0] != 2
        || binding[1] != 0
        || binding[2] == 0
        || step(14)?[0] != 12
        || fresh[0] != 3
        || fresh[1] == 0
        || fresh[1] == old[0]
        || fresh[2] == old[1]
        || step(16)? != [2, 2, 0]
        || rebound[0] != 0
        || rebound[1] == 0
        || rebound[1] == binding[2]
        || step(18)?[0] != 1
        || step(18)?[1] != 0
        || step(19)?[0] != 11
        || step(19)?[1] != 2
        || step(20)?[0] != 12
        || step(21)?[0] != 3
    {
        return Err("lifecycle observed step contract violated".into());
    }
    if !events
        .iter()
        .any(|e| e["event"] == "native-root-image" && e["format"] == "elf64")
    {
        return Err("root ELF loader witness absent".into());
    }
    let service_images: Vec<_> = events
        .iter()
        .filter(|e| e["event"] == "native-image" && e["selector"] == 0 && e["format"] == "elf64")
        .collect();
    if service_images.len() != 2
        || service_images[0]["generation"] == service_images[1]["generation"]
    {
        return Err("real service ELF/fresh process generations absent".into());
    }
    for (selector, kind, code) in [
        (1, "exit", Some(0)),
        (2, "fault", None),
        (0, "terminated", None),
    ] {
        if !events.iter().any(|e| {
            e["event"] == "native-completion"
                && e["selector"] == selector
                && e["kind"] == kind
                && code.is_none_or(|c| e["code"] == c)
        }) {
            return Err(format!("kernel completion witness absent for selector {selector}").into());
        }
    }
    let ends: Vec<_> = events
        .iter()
        .filter(|e| e["event"] == "native-boot")
        .collect();
    if ends.len() != 1
        || ends[0]["status"] != "complete"
        || ends[0]["root_exit"] != 0
        || ends[0]["owners_released"] != true
        || ends[0]["frames_restored"] != true
        || ends[0]["live_processes"] != 0
        || ends[0]["live_domains"] != 0
    {
        return Err("actual lifecycle shutdown/reclamation incomplete".into());
    }
    Ok(
        json!({"scope":"production kernel/counter service through an external lifecycle ABI client; does not test production supervisor recovery policy","production_supervisor_tested":false,"production_recovery_method_tested":true,"service_stop_restarted":true,"service_crash_restarted":false,"fresh_binding_completed":true,"old_binding_rejected":true,"counter_final":step(20)?[0],"restart_initial_value":step(16)?[2],"external_actor_fault_contained":true,"client_exit":"success","resource_leaks":ends[0]["live_processes"],"frames_restored":ends[0]["frames_restored"],"observed_steps":steps,"hardware":"UNKNOWN"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observed_inputs() -> Vec<Value> {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/native-lifecycle-observations.json"
        ))
        .unwrap();
        fixture["events"].as_array().unwrap().clone()
    }
    #[test]
    fn normal_session_oracle_requires_actual_child_exit_and_resource_reclamation() {
        let sample: Vec<Value> = serde_json::from_str(include_str!(
            "../../../tests/fixtures/native-production-session-observations.json"
        ))
        .unwrap();
        assert_eq!(validate(&sample).unwrap()["resource_leaks"], 0);
        for key in [
            "owners_released",
            "frames_restored",
            "live_processes",
            "live_domains",
            "root_exit",
        ] {
            let mut bad = sample.clone();
            let end = bad
                .iter_mut()
                .find(|e| e["event"] == "native-boot")
                .unwrap();
            end[key] = if end[key].is_boolean() {
                json!(false)
            } else {
                json!(1)
            };
            assert!(validate(&bad).is_err());
        }
        for selector in [0, 1] {
            let mut bad = sample.clone();
            bad.retain(|e| !(e["event"] == "native-completion" && e["selector"] == selector));
            assert!(validate(&bad).is_err());
        }
    }

    #[test]
    fn lifecycle_oracle_accepts_observed_steps_and_actual_reclamation() {
        assert!(validate(&observed_inputs()).is_ok());
    }
    #[test]
    fn lifecycle_oracle_rejects_retained_resources_and_missing_owner_release() {
        for (key, value) in [
            ("frames_restored", json!(false)),
            ("owners_released", json!(false)),
            ("live_processes", json!(1)),
            ("live_domains", json!(1)),
        ] {
            let mut events = observed_inputs();
            events
                .iter_mut()
                .find(|e| e["event"] == "native-boot")
                .unwrap()[key] = value;
            assert!(
                validate(&events)
                    .unwrap_err()
                    .to_string()
                    .contains("shutdown/reclamation"),
                "{key}"
            );
        }
    }
    #[test]
    fn lifecycle_oracle_rejects_wrong_binding_stale_acceptance_and_fabricated_values() {
        for (step, column, value) in [
            (12, 1, 6),
            (13, 1, 0),
            (16, 1, 0),
            (16, 2, 0),
            (16, 3, 12),
            (19, 1, 0),
        ] {
            let mut events = observed_inputs();
            let words = events
                .iter_mut()
                .find(|e| e["event"] == "native-user-report")
                .unwrap()["words"]
                .as_array_mut()
                .unwrap();
            let row = words[2..]
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .find(|row| row[0] == step)
                .unwrap();
            row[column] = json!(value);
            assert!(
                validate(&events)
                    .unwrap_err()
                    .to_string()
                    .contains("step contract")
            );
        }
    }
    #[test]
    fn lifecycle_oracle_rejects_unrelated_panic_and_missing_loader_evidence() {
        let mut events = observed_inputs();
        events.push(json!({"event":"panic"}));
        assert!(validate(&events).is_err());
        let events: Vec<_> = observed_inputs()
            .into_iter()
            .filter(|e| e["event"] != "native-root-image")
            .collect();
        assert!(
            validate(&events)
                .unwrap_err()
                .to_string()
                .contains("loader witness")
        );
    }
}
