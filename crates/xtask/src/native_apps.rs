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
    if words.len() != 11 || words[0] != MAGIC || words[1] != 1 || words[2] != 0 {
        return Err("native application reported failure or incompatible result".into());
    }
    let root_loaded = events
        .iter()
        .any(|e| e["event"] == "native-root-image" && e["format"] == "elf64");
    let client_loaded = events
        .iter()
        .any(|e| e["event"] == "native-image" && e["selector"] == 1 && e["format"] == "elf64");
    let services: Vec<_> = events
        .iter()
        .filter(|e| e["event"] == "native-image" && e["selector"] == 0 && e["format"] == "elf64")
        .collect();
    if !root_loaded
        || !client_loaded
        || services.len() != 2
        || services[0]["generation"] == services[1]["generation"]
    {
        return Err("standalone kernel ELF load/fresh process identity missing".into());
    }
    if !events.iter().any(|e| {
        e["event"] == "native-completion"
            && e["selector"] == 1
            && e["kind"] == "exit"
            && e["code"] == 0
    }) {
        return Err("client clean exit not observed by kernel".into());
    }
    let finished: Vec<_> = events
        .iter()
        .filter(|e| e["event"] == "native-boot")
        .collect();
    if finished.len() != 1
        || finished[0]["status"] != "complete"
        || finished[0]["root_exit"] != 0
        || finished[0]["owners_released"] != true
        || finished[0]["frames_restored"] != true
        || finished[0]["live_processes"] != 0
        || finished[0]["live_domains"] != 0
    {
        return Err("native resource release incomplete".into());
    }
    if words[3] != 1
        || words[4].as_u64().is_none_or(|n| n == 0)
        || words[5].as_u64().is_none_or(|n| n < 3)
        || words[6] != 12
        || words[7] != 1
        || words[8] != 1
        || words[9] != 1
        || words[10] != 0
    {
        return Err("native repeated operations/restart/binding/reset evidence missing".into());
    }
    Ok(
        json!({"supervisor_ready":true,"service_instance":words[4],"client_loaded_as_elf":true,"client_exit":"success","requests_completed":words[5],"counter_final":12,"service_restarted":true,"old_binding_rejected":true,"fresh_binding_completed":true,"restart_initial_value":0,"resource_leaks":0,"hardware":"UNKNOWN"}),
    )
}

pub fn smoke(args: &[String]) -> Result<()> {
    run_suite(args, "smoke-client", 0)
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
    for (binary, mode) in [
        ("selftest-process", 2),
        ("selftest-ipc", 3),
        ("selftest-service", 4),
        ("selftest-elf", 5),
    ] {
        run_suite(args, binary, mode)?;
    }
    controls(args)?;
    Ok(())
}
pub fn controls(args: &[String]) -> Result<()> {
    for control in ["elf", "auth", "stale", "binding", "reply", "ipc", "reclaim"] {
        run_mode(
            args,
            if control == "auth" {
                "selftest-ipc"
            } else {
                "smoke-client"
            },
            if control == "auth" { 3 } else { 0 },
            Some(control),
        )?;
    }
    Ok(())
}
fn run_suite(args: &[String], client_binary: &str, mode: u64) -> Result<()> {
    run_mode(args, client_binary, mode, None)
}
fn run_mode(args: &[String], client_binary: &str, mode: u64, control: Option<&str>) -> Result<()> {
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
                if control == Some("reply") {
                    "guest,reply-negative"
                } else {
                    "guest"
                },
                "--target",
                "aarch64-unknown-none",
            ])
            .env_remove("CARGO_TARGET_DIR");
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
                if mode == 1 {
                    "native-supervisor"
                } else {
                    "selftest-supervisor"
                },
            ),
            (
                "service",
                if mode == 1 {
                    "counter-service"
                } else {
                    "selftest-counter-service"
                },
            ),
            ("client", client_binary),
        ] {
            let original = PathBuf::from(format!(
                "target/aarch64-unknown-none/{}/{binary}",
                if prod { "release" } else { "debug" }
            ));
            let mut bytes = fs::read(&original)?;
            if control == Some("elf") && role == "root" {
                bytes[0] = 0;
            }
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
        let feature = control
            .filter(|c| !matches!(*c, "elf" | "reply"))
            .map_or("native-apps".into(), |c| {
                format!("native-apps,native-{c}-negative")
            });
        let elf = build_mode(
            prod,
            false,
            Some(&feature),
            true,
            mode,
            Some(
                &images
                    .try_into()
                    .map_err(|_| "three original ELF images required")?,
            ),
        )?;
        let suite_elf = PathBuf::from(format!(
            "target/kernel/{profile}-{client_binary}-{mode}-{}.elf",
            control.unwrap_or("positive")
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
        if let Some(control) = control {
            let execution = execute_validated(&suite_elf, false, true, false, false, false);
            let events = read_json(suite_elf.with_extension("results.json"))?;
            let observed = events.as_array().ok_or("negative events absent")?;
            validate_control(observed, control)?;
            if validate(observed).is_ok() {
                return Err("negative control passed positive acceptance".into());
            }
            results.insert(profile.into(),json!({"applications":apps,"control":control,"precise_witness":true,"events":events,"run":read_json(suite_elf.with_extension("run.json"))?,"execution_rejected":execution.is_err()}));
            println!("Native {control} control detected ({profile})");
            continue;
        }
        execute_native(&suite_elf)?;
        let events = read_json(suite_elf.with_extension("results.json"))?;
        let observed = events.as_array().ok_or("native events absent")?;
        let result = validate(observed)?;
        validate_suite(observed, mode)?;
        results.insert(profile.into(),json!({"applications":apps,"result":result,"kernel_build":read_json(elf.with_file_name(format!("{profile}-native-apps-build.json")))?,"run":read_json(suite_elf.with_extension("run.json"))?,"events":events}));
        println!("Native ELF {client_binary} passed ({profile}): {result}");
    }
    if sources != source_inventory()? {
        return Err("native application sources changed during smoke".into());
    }
    fs::write(
        format!(
            "target/kernel/native-{client_binary}-{mode}-{}.json",
            control.unwrap_or("positive")
        ),
        serde_json::to_string_pretty(
            &json!({"schema_version":1,"source_files":sources,"profiles":results,"scope":"standalone ELF native IPC smoke, not complete Phase 3.7 or hardware acceptance"}),
        )?,
    )?;
    Ok(())
}

fn validate_suite(events: &[Value], mode: u64) -> Result<()> {
    let terminal = |owner: u64, state: u64| {
        events.iter().any(|e| {
            e["event"] == "native-process-terminal" && e["owner"] == owner && e["state"] == state
        })
    };
    if !terminal(0, 2) || !terminal(1, 3) {
        return Err("clean client exit or isolated service fault missing".into());
    }
    for owner in [0, 1] {
        if !events.iter().any(|e| {
            e["event"] == "native-process-terminal"
                && e["owner"] == owner
                && e["ipc_blocks"].as_u64().is_some_and(|n| n > 0)
                && e["ipc_wakes"].as_u64().is_some_and(|n| n > 0)
        }) {
            return Err("real IPC blocking/wakeup across owners missing".into());
        }
    }
    if mode == 2
        && !events.iter().any(|e| {
            e["event"] == "native-completion" && e["selector"] == 2 && e["kind"] == "fault"
        })
    {
        return Err("selftest-process auxiliary fault containment absent".into());
    }
    if mode == 3
        && !events.iter().any(|e| {
            e["event"] == "native-completion"
                && e["selector"] == 2
                && e["kind"] == "exit"
                && e["code"] == 0
        })
    {
        return Err("selftest-ipc denied authority auxiliary did not exit cleanly".into());
    }
    Ok(())
}

fn observe_runtime(elf: &Path) -> Result<Value> {
    let log = elf.with_extension("runtime.log");
    let emulator = qemu()?;
    let version = String::from_utf8(Command::new(&emulator).arg("--version").output()?.stdout)?;
    let run = json!({"qemu_version":version,"arguments":qemu_args(elf),"accelerator":"TCG","kernel_sha256":Sha256::digest(fs::read(elf)?).iter().map(|b|format!("{b:02x}")).collect::<String>()});
    let mut child = Command::new(emulator)
        .args(qemu_args(elf))
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

fn validate_control(events: &[Value], control: &str) -> Result<()> {
    let report = |code: u64| {
        events
            .iter()
            .any(|e| e["event"] == "native-user-report" && e["words"] == json!([MAGIC, 1, code]))
    };
    let client_exit = |code: u64| {
        events.iter().any(|e| {
            e["event"] == "native-process-terminal"
                && e["slot"] == 1
                && e["state"] == 2
                && e["exit_code"] == code
        })
    };
    let witness = match control {
        "elf" => events
            .iter()
            .any(|e| e["event"] == "native-image-reject" && e["error"] == "InvalidImage"),
        "auth" => report(121),
        "stale" => report(76),
        "binding" | "ipc" => client_exit(42),
        "reply" => client_exit(45),
        "reclaim" => events.iter().any(|e| {
            e["event"] == "native-boot"
                && e["status"] == "complete"
                && e["live_processes"] == 1
                && e["frames_restored"] == false
        }),
        _ => false,
    };
    if !witness {
        return Err(format!(
            "precise {control} negative witness absent; arbitrary panic is insufficient"
        )
        .into());
    }
    Ok(())
}
