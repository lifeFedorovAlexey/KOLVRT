//! One task inventory for both serial acceptance and partitioned CI execution.
use super::*;

#[derive(Clone)]
struct Task {
    id: String,
    prod: bool,
    tests: bool,
    flag: Option<&'static str>,
    feature: Option<&'static str>,
    marker: Option<String>,
    ipc_test: Option<&'static str>,
}

fn tasks() -> Vec<Task> {
    let mut tasks = Vec::new();
    for prod in [false, true] {
        for tests in [true, false] {
            tasks.push(Task {
                id: format!(
                    "{}-{}",
                    if prod { "prod" } else { "dev" },
                    if tests { "tests" } else { "boot" }
                ),
                prod,
                tests,
                flag: None,
                feature: None,
                marker: None,
                ipc_test: None,
            });
        }
    }
    for &(flag, marker) in NEGATIVE_CONTROLS {
        if observable_checks(flag).is_some() {
            tasks.push(observable_task(flag, false));
            continue;
        }
        let feature = match flag {
            "--negative-control" => "negative-test",
            "--panic-control" => "panic-test",
            "--ownership-control" => "ownership-test",
            "--retained-mapping-control" => "retained-mapping-test",
            "--secondary-panic-control" => "secondary-panic-test",
            "--retirement-control" => "retirement-negative",
            _ => unreachable!("unknown registered negative control"),
        };
        tasks.push(control(false, flag, feature, Some(marker.to_owned()), None));
    }
    for prod in [false, true] {
        for &(flag, feature, error) in SCHEDULER_CONTROLS {
            tasks.push(control(
                prod,
                flag,
                feature,
                Some(if feature.ends_with("-input") {
                    error.to_owned()
                } else {
                    format!("\"error\":\"{error}\"")
                }),
                None,
            ));
        }
        for &(flag, feature, marker) in FOUNDATION_CONTROLS
            .iter()
            .chain(USER_COPY_CONTROLS)
            .chain(HANDLE_CONTROLS)
            .chain(SECURITY_CONTROLS)
        {
            tasks.push(control(prod, flag, feature, Some(marker.to_owned()), None));
        }
        for &(flag, feature, test) in IPC_CONTROLS {
            tasks.push(control(prod, flag, feature, None, Some(test)));
        }
    }
    tasks
}

pub(super) fn observable_checks(flag: &str) -> Option<&'static [&'static str]> {
    match flag {
        "--scheduler-aarch32-control" => Some(&["process_aarch32_context_rejected"]),
        "--scheduler-user-irq-control" => Some(&["process_masked_user_irq_rejected"]),
        "--scheduler-context-control" => Some(&["process_privileged_context_rejected"]),
        "--scheduler-start-control" => Some(&["scheduler_duplicate_start_rejected"]),
        "--scheduler-foreign-control" => Some(&[
            "scheduler_foreign_cpu_rejected",
            "process_registry_and_cpu_ownership",
        ]),
        "--scheduler-reentry-control" => Some(&["scheduler_reentry_rejected"]),
        "--scheduler-stale-control" => Some(&["scheduler_stale_generation_rejected"]),
        "--scheduler-reset-control" => Some(&["scheduler_live_reset_rejected"]),
        "--scheduler-inspect-control" => Some(&["scheduler_early_inspection_rejected"]),
        "--scheduler-complete-control" => Some(&["completion_publication_state_inputs"]),
        "--scheduler-irq-control" => Some(&[
            "scheduler_unmasked_access_rejected",
            "process_unmasked_access_rejected",
        ]),
        "--irq-simd-restore-control" => Some(&["irq_simd_context"]),
        "--process-unlink-control" => Some(&[
            "process_unlinked_reclaim_rejected",
            "el0_quiescent_reclamation",
        ]),
        "--process-start-control" => Some(&[
            "process_duplicate_start_rejected",
            "process_terminal_rejections",
        ]),
        "--process-stale-control" => Some(&[
            "process_slot_generation_reuse",
            "process_terminal_rejections",
        ]),
        "--process-reclaim-control" => {
            Some(&["process_live_reclaim_rejected", "el0_quiescent_reclamation"])
        }
        "--checkpoint-publication-control" => Some(&[
            "completion_publication_state_inputs",
            "el0_smp_ownership",
            "el0_quiescent_reclamation",
        ]),
        "--process-exit-control" => Some(&[
            "process_duplicate_completion_rejected",
            "process_terminal_rejections",
            "process_normal_exit_and_fault",
        ]),
        "--process-rollback-control" => Some(&[
            "process_creation_rollback",
            "elf_creation_failure_is_transactional",
        ]),
        "--asid-reuse-control" => Some(&[
            "asid_reuse_requires_invalidation",
            "asid_reuse_same_va_both_cpus",
            "asid_pool_exhaustion_both_cpus",
        ]),
        "--user-retirement-control" => Some(&[
            "process_live_reclaim_rejected",
            "el0_quiescent_reclamation",
            "process_terminal_rejections",
        ]),
        "--user-context-control" => Some(&[
            "user_context_invalid_states_rejected",
            "el0_context_preservation",
            "el0_timer_switches",
        ]),
        "--user-root-control" => Some(&[
            "el0_memory_isolation",
            "el0_kernel_memory_rejected",
            "el0_foreign_memory_rejected",
            "el0_quiescent_reclamation",
        ]),
        "--shootdown-control" => Some(&[
            "smp_retirement_pending",
            "smp_no_premature_reuse",
            "smp_remote_ack",
            "smp_safe_reuse",
        ]),
        "--remote-tlbi-control" => Some(&[
            "smp_future_ack_rejected",
            "smp_remote_ack",
            "smp_remote_tlb_invalidation",
        ]),
        _ => None,
    }
}
fn observable_task(flag: &'static str, prod: bool) -> Task {
    let label = match flag {
        "--shootdown-control" => "shootdown-invariants",
        "--remote-tlbi-control" => "remote-tlbi-invariants",
        "--asid-reuse-control" => "asid-reuse-invariants",
        "--irq-simd-restore-control" => "irq-simd-roundtrip-invariant",
        "--checkpoint-publication-control" => "completion-publication-state-inputs",
        _ => flag,
    };
    Task {
        id: format!("{}-{label}", if prod { "prod" } else { "dev" }),
        prod,
        tests: true,
        flag: Some(flag),
        feature: None,
        marker: None,
        ipc_test: None,
    }
}
pub(super) fn run_observable(flag: &str, prod: bool) -> Result<()> {
    let task = tasks()
        .into_iter()
        .find(|task| task.prod == prod && task.flag == Some(flag))
        .ok_or("unregistered observable control")?;
    execute_task(&task).map(|_| ())
}
fn required_observations(events: &Value, names: &[&str]) -> Result<bool> {
    let events = events.as_array().ok_or("control observations absent")?;
    Ok(names.iter().all(|name| {
        let found = events
            .iter()
            .filter(|e| e["event"] == "test" && e["name"] == *name)
            .collect::<Vec<_>>();
        found.len() == 1 && found[0]["status"] == "pass"
    }))
}

fn control(
    prod: bool,
    flag: &'static str,
    feature: &'static str,
    marker: Option<String>,
    ipc_test: Option<&'static str>,
) -> Task {
    if observable_checks(flag).is_some() {
        return observable_task(flag, prod);
    }
    Task {
        id: format!("{}-{flag}", if prod { "prod" } else { "dev" }),
        prod,
        tests: true,
        flag: Some(flag),
        feature: Some(feature),
        marker,
        ipc_test,
    }
}

fn coverage_kind(task: &Task) -> &'static str {
    match task.flag {
        None => "ordinary",
        Some(
            "--shootdown-control"
            | "--remote-tlbi-control"
            | "--asid-reuse-control"
            | "--irq-simd-restore-control",
        ) => "invariant",
        Some(flag) if observable_checks(flag).is_some() => "negative-input",
        _ if task.feature.is_some_and(|f| f.ends_with("-input")) => "negative-input",
        _ => "legacy-failure",
    }
}

fn describe(task: &Task) -> Value {
    json!({"id":task.id,"profile":if task.prod {"prod"} else {"dev"},"tests":task.tests,"flag":task.flag,"feature":task.feature,"expected_marker":task.marker,"expected_ipc_test":task.ipc_test,"coverage_kind":coverage_kind(task),"observable_checks":task.flag.and_then(observable_checks),"control_scope":if matches!(task.flag, Some("--shootdown-control"|"--remote-tlbi-control"|"--asid-reuse-control")){"positive invariant coverage; not equivalent to missing ACK or skipped invalidation controls"}else if task.flag == Some("--checkpoint-publication-control"){"production Ownership UNIT invalid inputs; not a withheld remote publication SYSTEM test"}else if task.flag == Some("--irq-simd-restore-control"){"actual IRQ/SIMD round-trip invariant; no skipped-restore detection claim"}else if task.feature.is_some_and(|f| f.ends_with("-input")){"production adapter assertions invoked with forbidden test inputs"}else if task.flag.is_some_and(|f| f.starts_with("--scheduler-")){"production Ownership UNIT guards plus mapped adapter checks; not corrupted runtime storage"}else if task.flag.and_then(observable_checks).is_some(){"real invalid inputs; no implementation mutation"}else{"legacy"}})
}

pub fn plan_document() -> Result<Value> {
    Ok(
        json!({"schema_version":1,"source_files":source_inventory()?,"qemu_arguments":qemu_args(Path::new("<ELF>")),"tasks":tasks().iter().map(describe).collect::<Vec<_>>()}),
    )
}

fn selected(position: usize, shard: Option<(usize, usize)>) -> bool {
    shard.is_none_or(|(index, count)| position % count == index)
}

fn ipc_failure(events: &Value, test: &str) -> Result<bool> {
    Ok(events
        .as_array()
        .ok_or("IPC control events absent")?
        .iter()
        .any(|event| {
            ((event["event"] == "test" && event["name"] == test)
                || test
                    .strip_prefix("reject:")
                    .is_some_and(|error| event["event"] == "ipc-reject" && event["error"] == error)
                || test.strip_prefix("supervision:").is_some_and(|error| {
                    event["event"] == "supervision-reject" && event["error"] == error
                }))
                && event["status"] == "fail"
        }))
}

fn control_diagnostic(id: &str, reason: &str, output: &str) -> String {
    let lines = output.lines().collect::<Vec<_>>();
    let tail = lines[lines.len().saturating_sub(24)..].join("\n");
    format!("{reason}: {id}\nSubprocess output (last 24 lines):\n{tail}")
}

fn execute_task(task: &Task) -> Result<Value> {
    let profile = if task.prod { "prod" } else { "dev" };
    let artifact = format!(
        "target/kernel/{profile}-{}",
        task.feature
            .unwrap_or(if task.tests { "tests" } else { "boot" })
    );
    // Retained metadata must have been produced by this actual task invocation.
    for suffix in [".run.json", ".results.json", "-build.json"] {
        match fs::remove_file(format!("{artifact}{suffix}")) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    if let Some(checks) = task.flag.and_then(observable_checks) {
        let elf = build(task.prod, true, None, true)?;
        execute(&elf, true, true)?;
        if !required_observations(&read_json(format!("{artifact}.results.json"))?, checks)? {
            return Err(format!("required invariant observations absent: {}", task.id).into());
        }
    } else if let Some(flag) = task.flag {
        let mut command = Command::new(env::current_exe()?);
        command.args(["test", flag]);
        if task.prod {
            command.arg("--prod");
        }
        let output = command.output()?;
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        fs::write(format!("target/kernel/{}.host.log", task.id), &text)?;
        // These sidecars were removed before invocation. A compiler failure, even
        // one printing the expected marker, cannot witness a guest control.
        let missing = ["-build.json", ".run.json", ".results.json"]
            .into_iter()
            .filter(|suffix| !Path::new(&format!("{artifact}{suffix}")).is_file())
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(control_diagnostic(
                &task.id,
                &format!(
                    "negative control did not produce runtime evidence; missing {}",
                    missing.join(", ")
                ),
                &text,
            )
            .into());
        }
        let witnessed = if task.feature.is_some_and(|f| f.ends_with("-input")) {
            assertion_witness(&text, &read_json(format!("{artifact}.results.json"))?, flag)?
        } else if let Some(test) = task.ipc_test {
            ipc_failure(&read_json(format!("{artifact}.results.json"))?, test)?
        } else {
            task.marker
                .as_ref()
                .is_some_and(|marker| text.contains(marker))
        };
        if output.status.success() || !witnessed {
            return Err(control_diagnostic(
                &task.id,
                "required negative control not witnessed",
                &text,
            )
            .into());
        }
    } else {
        let elf = build(task.prod, task.tests, None, true)?;
        execute(&elf, task.tests, true)?;
    }
    println!("Matrix task passed: {} (executed)", task.id);
    Ok(
        json!({"id":task.id,"outcome":"passed","observation":"executed","build":read_json(format!("{artifact}-build.json"))?,"run":read_json(format!("{artifact}.run.json"))?}),
    )
}

pub fn run(shard: Option<(usize, usize)>) -> Result<()> {
    if shard.is_some_and(|(index, count)| count == 0 || count > 16 || index >= count) {
        return Err("invalid matrix shard; require 0 <= index < count <= 16".into());
    }
    let destination = shard.map_or_else(
        || "target/kernel/matrix-execution.json".to_owned(),
        |(index, _)| format!("target/kernel/shard-{index}.json"),
    );
    if Path::new(&destination).exists() {
        fs::remove_file(&destination)?;
    }
    let plan = plan_document()?;
    let mut completed = Vec::new();
    for (position, task) in tasks().iter().enumerate() {
        if selected(position, shard) {
            completed.push(execute_task(task)?);
        }
    }
    if plan["source_files"] != source_inventory()? {
        return Err("matrix sources changed during execution".into());
    }
    fs::write(
        destination,
        serde_json::to_string_pretty(
            &json!({"schema_version":1,"plan":plan,"shard_index":shard.map(|v|v.0),"shard_count":shard.map(|v|v.1),"completed":completed,"scope":if shard.is_some() {"partial; requires exact-source aggregate"} else {"complete serial matrix"}}),
        )?,
    )?;
    Ok(())
}

pub fn run_one(flag: &str, prod: bool) -> Result<()> {
    let task = tasks()
        .into_iter()
        .find(|task| task.prod == prod && task.flag == Some(flag))
        .ok_or("unregistered matrix task")?;
    let sources = source_inventory()?;
    execute_task(&task)?;
    if sources != source_inventory()? {
        return Err("matrix task sources changed during execution".into());
    }
    Ok(())
}

pub fn run_ipc() -> Result<()> {
    for task in tasks().iter().filter(|task| task.ipc_test.is_some()) {
        execute_task(task)?;
    }
    Ok(())
}

// PROD deliberately omits diagnostic panic messages. Match the actual assertion
// site and fresh guest panic event instead of enabling extra production logging.
fn assertion_witness(output: &str, events: &Value, flag: &str) -> Result<bool> {
    let (path, function, message) = match flag {
        "--scheduler-inner-lock-control" => (
            "crates/kernel/src/sync/mod.rs",
            "pub fn try_lock(",
            "lock inside scheduler ownership contract",
        ),
        "--scheduler-lock-control" => (
            "crates/kernel/src/sync/mod.rs",
            "pub fn assert_scheduler_unlocked(",
            "scheduler lock order contract",
        ),
        "--scheduler-task-control" => (
            "crates/kernel/src/scheduler/mod.rs",
            "fn validate_task(",
            "stale scheduler task",
        ),
        "--scheduler-owner-control" => (
            "crates/kernel/src/scheduler/mod.rs",
            "fn choose(",
            "duplicate running task",
        ),
        _ => return Err("unregistered assertion input".into()),
    };
    let source = fs::read_to_string(path)?;
    let lines = source.lines().collect::<Vec<_>>();
    let start = lines
        .iter()
        .position(|line| line.contains(function))
        .ok_or("assertion function absent")?;
    let end = (start + 1..lines.len())
        .find(|&i| lines[i].starts_with("fn ") || lines[i].trim_start().starts_with("pub fn "))
        .unwrap_or(lines.len());
    let sites = lines
        .iter()
        .enumerate()
        .skip(start)
        .take(end - start)
        .filter(|(_, line)| line.contains(&format!("\"{message}\"")))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if sites.len() != 1 {
        return Err("assertion site must be unique".into());
    }
    let line = (0..=sites[0])
        .rev()
        .find(|&i| lines[i].contains("assert!(") || lines[i].contains("assert_eq!("))
        .ok_or("assertion site absent")?
        + 1;
    let site = format!("kernel halted at {}:{line}", path.replace('/', "\\"));
    let slash_site = format!("kernel halted at {path}:{line}");
    panic_site_witness(output, events, &site, &slash_site, line)
}
fn panic_site_witness(
    output: &str,
    events: &Value,
    site: &str,
    slash_site: &str,
    line: usize,
) -> Result<bool> {
    Ok((output.contains(site) || output.contains(slash_site))
        && events
            .as_array()
            .ok_or("panic observations absent")?
            .iter()
            .any(|e| e["event"] == "panic" && e["status"] == "fail" && e["line"] == line))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assertion_oracle_rejects_unrelated_or_missing_guest_panic() {
        let site = "kernel halted at crates/kernel/src/sync/mod.rs:40";
        let panic = json!({"event":"panic","line":40,"status":"fail"});
        assert!(panic_site_witness(site, &json!([panic.clone()]), site, site, 40).unwrap());
        assert!(!panic_site_witness("unrelated panic", &json!([panic]), site, site, 40).unwrap());
        for events in [
            json!([]),
            json!([{"event":"panic","line":41,"status":"fail"}]),
            json!([{"event":"test","line":40,"status":"fail"}]),
            json!([{"event":"panic","line":40,"status":"pass"}]),
        ] {
            assert!(!panic_site_witness(site, &events, site, site, 40).unwrap());
        }
    }
    #[test]
    fn invariant_controls_reject_missing_failed_and_duplicate_observations() {
        let pass = json!({"event":"test","name":"guard","status":"pass"});
        assert!(required_observations(&json!([pass.clone()]), &["guard"]).unwrap());
        for events in [
            json!([]),
            json!([{"event":"test","name":"guard","status":"fail"}]),
            json!([pass.clone(), pass]),
            json!([{"event":"panic","status":"fail"}]),
        ] {
            assert!(!required_observations(&events, &["guard"]).unwrap());
        }
    }
    #[test]
    fn migrated_tasks_have_no_mutation_feature_or_failure_marker() {
        let plan = tasks();
        let migrated = plan
            .iter()
            .filter(|task| task.flag.and_then(observable_checks).is_some())
            .collect::<Vec<_>>();
        assert!(!migrated.is_empty());
        for task in migrated {
            assert!(task.feature.is_none());
            assert!(task.marker.is_none());
            assert!(task.ipc_test.is_none());
        }
    }
    #[test]
    fn shards_partition_every_required_task_once() {
        let plan = tasks();
        let expected = 4
            + NEGATIVE_CONTROLS.len()
            + 2 * (FOUNDATION_CONTROLS.len()
                + SCHEDULER_CONTROLS.len()
                + USER_COPY_CONTROLS.len()
                + HANDLE_CONTROLS.len()
                + SECURITY_CONTROLS.len()
                + IPC_CONTROLS.len());
        assert_eq!(plan.len(), expected);
        assert_eq!(
            plan.iter().map(|t| &t.id).collect::<BTreeSet<_>>().len(),
            expected
        );
        for count in [1, 2, 4, 8] {
            for position in 0..plan.len() {
                assert_eq!(
                    (0..count)
                        .filter(|index| selected(position, Some((*index, count))))
                        .count(),
                    1
                );
            }
        }
    }
    #[test]
    fn ipc_requires_the_registered_failure_not_an_unrelated_panic() {
        assert!(
            !ipc_failure(
                &json!([{"event":"test","name":"other","status":"fail"}]),
                "actual"
            )
            .unwrap()
        );
        assert!(
            !ipc_failure(
                &json!([{"event":"test","name":"actual","status":"pass"}]),
                "actual"
            )
            .unwrap()
        );
        assert!(
            ipc_failure(
                &json!([{"event":"test","name":"actual","status":"fail"}]),
                "actual"
            )
            .unwrap()
        );
        assert!(
            ipc_failure(
                &json!([{"event":"ipc-reject","error":"WrongProcessWake","status":"fail"}]),
                "reject:WrongProcessWake"
            )
            .unwrap()
        );
    }
    #[test]
    fn supervision_requires_its_exact_failure_kind_and_error() {
        let exact =
            json!([{"event":"supervision-reject","error":"WaitIdentityLost","status":"fail"}]);
        assert!(ipc_failure(&exact, "supervision:WaitIdentityLost").unwrap());
        for event in [
            json!({"event":"supervision-reject","error":"Other","status":"fail"}),
            json!({"event":"ipc-reject","error":"WaitIdentityLost","status":"fail"}),
            json!({"event":"supervision-reject","error":"WaitIdentityLost","status":"pass"}),
            json!({"event":"panic","status":"fail"}),
        ] {
            assert!(!ipc_failure(&json!([event]), "supervision:WaitIdentityLost").unwrap());
        }
        assert!(ipc_failure(&json!({}), "supervision:WaitIdentityLost").is_err());
    }
}
