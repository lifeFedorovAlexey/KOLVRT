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
            tasks.push(control(
                prod,
                flag,
                feature,
                None,
                (!test.starts_with("assertion:")).then_some(test),
            ));
        }
    }
    tasks
}

pub(super) fn observable_checks(flag: &str) -> Option<&'static [&'static str]> {
    match flag {
        "--ipc-wake-publication-control" => {
            Some(&["ipc_mailbox_publication_and_generation_guards"])
        }
        "--ipc-wake-generation-control" => Some(&["ipc_mailbox_publication_and_generation_guards"]),
        "--ipc-wait-recheck-control" => Some(&[
            "ipc_el0_cpu0_to_cpu1",
            "ipc_el0_cpu1_to_cpu0",
            "ipc_queue_full_fifo_and_reclamation",
        ]),
        "--ipc-request-generation-control" => Some(&["ipc_payload_snapshot_result_id_and_stress"]),
        "--ipc-double-charge-release-control" => {
            Some(&["ipc_duplicate_terminal_retains_outcome_and_charges"])
        }
        "--ipc-charge-release-control" => Some(&[
            "ipc_deadline_before_effect",
            "ipc_request_quota_failure_has_no_phantom_work",
        ]),
        "--ipc-cancel-control" => Some(&[
            "ipc_cancel_before_commit",
            "ipc_queued_head_nonhead_cancel_fifo",
        ]),
        "--ipc-service-death-control" => Some(&[
            "ipc_service_death_queued",
            "ipc_both_peers_die_with_accepted_work",
            "ipc_empty_service_death_reclamation",
        ]),
        "--ipc-double-terminal-control" => Some(&[
            "ipc_duplicate_terminal_retains_outcome_and_charges",
            "ipc_payload_snapshot_result_id_and_stress",
        ]),
        "--ipc-capacity-control" => Some(&[
            "ipc_queue_full_fifo_and_reclamation",
            "ipc_request_quota_failure_has_no_phantom_work",
        ]),
        "--ipc-fifo-control" => Some(&[
            "ipc_queue_full_fifo_and_reclamation",
            "ipc_concurrent_producers_fifo_and_reclamation",
        ]),
        "--ipc-id-reuse-control" => Some(&["ipc_payload_snapshot_result_id_and_stress"]),
        "--ipc-receive-copy-control" => Some(&["ipc_receive_copy_failure_retains_queue"]),
        "--ipc-collect-copy-control" => Some(&["ipc_collect_copy_failure_retains_result"]),
        "--ipc-send-rights-control" => Some(&["ipc_authority_denial_revoke_and_retention"]),
        "--ipc-revoke-control" => Some(&[
            "ipc_authority_denial_revoke_and_retention",
            "ipc_concurrent_revoke_admission_retains_accepted",
        ]),
        "--ipc-deadline-control" => {
            Some(&["ipc_deadline_before_effect", "ipc_deadline_after_commit"])
        }
        "--ipc-service-token-control" => Some(&[
            "ipc_service_token_invalid_caller_rejected",
            "ipc_el0_cpu0_to_cpu1",
        ]),
        "--user-copy-snapshot-control" => Some(&["user_copy_el0_boundary_and_snapshot"]),
        "--user-copy-recovery-control" => Some(&[
            "user_copy_el0_boundary_and_snapshot",
            "user_copy_lifetime_and_reclamation",
        ]),
        "--handle-generation-control" => {
            Some(&["handle_el0_identity_type_generation_and_lifetime"])
        }
        "--handle-owner-control" => Some(&[
            "handle_cross_process_reference_isolation",
            "handle_el0_identity_type_generation_and_lifetime",
        ]),
        "--handle-type-control" => Some(&["handle_el0_identity_type_generation_and_lifetime"]),
        "--handle-reuse-control" => Some(&[
            "handle_el0_identity_type_generation_and_lifetime",
            "handle_exit_fault_cleanup_and_process_reuse",
        ]),
        "--handle-transfer-rights-control" => Some(&[
            "handle_el0_transfer_transaction_attenuation",
            "handle_el0_identity_type_generation_and_lifetime",
        ]),
        "--handle-retirement-control" => Some(&[
            "handle_retirement_invalid_owner_rejected",
            "handle_exit_fault_cleanup_and_process_reuse",
        ]),
        "--domain-budget-control" => Some(&[
            "domain_memory_budget_enforced",
            "domain_el0_request_and_queue_budgets",
        ]),
        "--domain-identity-control" => Some(&[
            "capability_el0_scope_attenuation_and_denial",
            "domain_rebind_does_not_restore_grants",
        ]),
        "--domain-teardown-control" => Some(&[
            "domain_teardown_retains_accepted_notification",
            "domain_reclaimed_sender_retains_request_charge",
            "domain_fault_peer_progress_and_reclamation",
        ]),
        "--capability-revoke-control" => Some(&[
            "capability_el0_scope_attenuation_and_denial",
            "capability_revocation_retains_admitted_effect",
            "capability_cross_cpu_revoke_admission",
        ]),
        "--capability-scope-control" => Some(&[
            "capability_el0_scope_attenuation_and_denial",
            "capability_service_rebind_preserves_scope",
        ]),
        "--supervision-authority-control" => Some(&["supervision_el0_lifecycle"]),
        "--supervision-stale-control" => Some(&["supervision_el0_lifecycle"]),
        "--supervision-wait-control" => Some(&["supervision_el0_lifecycle"]),
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
        "--ipc-fifo-control" => "ipc-fifo-invariants",
        "--ipc-wait-recheck-control" => "ipc-wait-progress-invariants",
        "--ipc-wake-publication-control" => "ipc-wake-publication-invariants",
        "--ipc-service-death-control" => "ipc-service-death-invariants",
        "--supervision-wait-control" => "supervision-wait-invariants",
        "--domain-teardown-control" => "domain-teardown-invariants",
        "--user-copy-snapshot-control" => "user-copy-snapshot-invariant",
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
        Some(
            "--supervision-wait-control"
            | "--domain-teardown-control"
            | "--user-copy-snapshot-control",
        ) => "invariant",
        Some(
            "--capability-revoke-control" | "--capability-scope-control" | "--handle-reuse-control",
        ) => "mixed-input-and-invariant",
        Some(
            "--ipc-fifo-control"
            | "--ipc-wait-recheck-control"
            | "--ipc-wake-publication-control"
            | "--ipc-service-death-control"
            | "--ipc-charge-release-control",
        ) => "invariant",
        Some(
            "--ipc-cancel-control"
            | "--ipc-revoke-control"
            | "--ipc-deadline-control"
            | "--ipc-request-generation-control"
            | "--ipc-id-reuse-control",
        ) => "mixed-input-and-invariant",
        Some(flag) if observable_checks(flag).is_some() => "negative-input",
        _ if task.feature.is_some_and(|f| f.ends_with("-input")) => "negative-input",
        _ => "legacy-failure",
    }
}

fn describe(task: &Task) -> Value {
    json!({"id":task.id,"profile":if task.prod {"prod"} else {"dev"},"tests":task.tests,"flag":task.flag,"feature":task.feature,"expected_marker":task.marker,"expected_ipc_test":task.ipc_test,"coverage_kind":coverage_kind(task),"observable_checks":task.flag.and_then(observable_checks),"control_scope":if matches!(task.flag, Some("--shootdown-control"|"--remote-tlbi-control"|"--asid-reuse-control")){"positive invariant coverage; not equivalent to missing ACK or skipped invalidation controls"}else if task.flag == Some("--checkpoint-publication-control"){"production Ownership UNIT invalid inputs; not a withheld remote publication SYSTEM test"}else if matches!(task.flag, Some("--supervision-wait-control"|"--domain-teardown-control"|"--user-copy-snapshot-control")){"actual lifecycle/snapshot/retention invariant; no mutation-detection equivalence claimed"}else if task.flag == Some("--irq-simd-restore-control"){"actual IRQ/SIMD round-trip invariant; no skipped-restore detection claim"}else if task.feature.is_some_and(|f| f.ends_with("-input")){"production adapter assertions invoked with forbidden test inputs"}else if task.flag.is_some_and(|f| f.starts_with("--scheduler-")){"production Ownership UNIT guards plus mapped adapter checks; not corrupted runtime storage"}else if coverage_kind(task) == "invariant"{"actual production state/EL0/resource invariant; no omitted-implementation detection equivalence"}else if coverage_kind(task) == "mixed-input-and-invariant"{"real negative inputs plus production state/EL0/resource invariants"}else if task.flag.and_then(observable_checks).is_some(){"real invalid inputs; no implementation mutation"}else{"legacy"}})
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

// Invocation-local observations only: a donor is always an actual ordinary run.
fn reusable(task: &Task) -> bool {
    task.tests
        && task.feature.is_none()
        && task.marker.is_none()
        && task.ipc_test.is_none()
        && (task.flag.is_none() || task.flag.and_then(observable_checks).is_some())
}

fn ordinary_evidence(task: &Task, result: &Value) -> Result<()> {
    if !reusable(task) || result["observation"] != "executed" || result.get("reused_from").is_some()
    {
        return Err("matrix donor is not an executed ordinary suite".into());
    }
    let profile = if task.prod { "prod" } else { "dev" };
    let elf = format!("target/kernel/{profile}-tests.elf");
    let features = if task.prod {
        json!(["machine-events", "kernel-tests"])
    } else {
        json!(["machine-events", "diagnostics", "kernel-tests"])
    };
    if result["build"]["artifact"] != elf
        || result["build"]["features"] != features
        || result["run"]["arguments"] != json!(qemu_args(Path::new(&elf)))
        || !result["build"]["sha256"].is_string()
        || result["build"]["sha256"] != result["run"]["elf_sha256"]
    {
        return Err("matrix ordinary observation configuration mismatch".into());
    }
    let events = result["events"]
        .as_array()
        .ok_or("ordinary events absent")?;
    let suites = events
        .iter()
        .filter(|event| event["event"] == "suite")
        .collect::<Vec<_>>();
    if suites.len() != 1
        || suites[0]["status"] != "pass"
        || events.iter().any(|event| event["status"] == "fail")
    {
        return Err("matrix ordinary observation suite did not pass".into());
    }
    if let Some(checks) = task.flag.and_then(observable_checks)
        && !required_observations(&result["events"], checks)?
    {
        return Err(format!("required invariant observations absent: {}", task.id).into());
    }
    Ok(())
}

fn reuse_enabled(value: Option<&str>) -> Result<bool> {
    match value {
        None | Some("on") => Ok(true),
        Some("off") => Ok(false),
        _ => Err("KOLVRT_MATRIX_REUSE must be on or off".into()),
    }
}
fn reuse_policy() -> Result<bool> {
    match env::var("KOLVRT_MATRIX_REUSE") {
        Ok(value) => reuse_enabled(Some(&value)),
        Err(env::VarError::NotPresent) => reuse_enabled(None),
        Err(error) => Err(error.into()),
    }
}

#[derive(Default)]
struct Observations {
    // DEV and PROD cannot donate to one another. Never cache a reused result.
    donors: [Option<Value>; 2],
}
impl Observations {
    fn execute(&mut self, task: &Task) -> Result<Value> {
        if !reusable(task) {
            return execute_task(task);
        }
        let slot = usize::from(task.prod);
        if let Some(donor) = &self.donors[slot] {
            ordinary_evidence(task, donor)?;
            let elf = donor["build"]["artifact"]
                .as_str()
                .ok_or("donor ELF absent")?;
            let digest = Sha256::digest(fs::read(elf)?)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            if donor["build"]["sha256"] != digest {
                return Err("matrix donor ELF changed before observation reuse".into());
            }
            let mut result = donor.clone();
            result["id"] = json!(task.id);
            result["observation"] = json!("reused");
            result["reused_from"] = donor["id"].clone();
            println!(
                "Matrix task passed: {} (reused from {})",
                task.id, donor["id"]
            );
            return Ok(result);
        }
        let result = execute_task(task)?;
        ordinary_evidence(task, &result)?;
        self.donors[slot] = Some(result.clone());
        Ok(result)
    }
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
        let witnessed =
            if task.feature.is_some_and(|f| f.ends_with("-input")) && task.ipc_test.is_none() {
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
    let mut result = json!({"id":task.id,"outcome":"passed","observation":"executed","build":read_json(format!("{artifact}-build.json"))?,"run":read_json(format!("{artifact}.run.json"))?});
    if reusable(task) {
        result["events"] = read_json(format!("{artifact}.results.json"))?;
        ordinary_evidence(task, &result)?;
    }
    println!("Matrix task passed: {} (executed)", task.id);
    Ok(result)
}

pub fn run(shard: Option<(usize, usize)>) -> Result<()> {
    if shard.is_some_and(|(index, count)| count == 0 || count > 16 || index >= count) {
        return Err("invalid matrix shard; require 0 <= index < count <= 16".into());
    }
    let reuse = reuse_policy()?;
    let destination = shard.map_or_else(
        || "target/kernel/matrix-execution.json".to_owned(),
        |(index, _)| format!("target/kernel/shard-{index}.json"),
    );
    if Path::new(&destination).exists() {
        fs::remove_file(&destination)?;
    }
    let plan = plan_document()?;
    let mut completed = Vec::new();
    let mut observations = Observations::default();
    for (position, task) in tasks().iter().enumerate() {
        if selected(position, shard) {
            completed.push(if reuse {
                observations.execute(task)?
            } else {
                execute_task(task)?
            });
        }
    }
    if plan["source_files"] != source_inventory()? {
        return Err("matrix sources changed during execution".into());
    }
    fs::write(
        destination,
        serde_json::to_string_pretty(
            &json!({"schema_version":2,"reuse_policy":if reuse {"same-invocation"} else {"disabled"},"plan":plan,"shard_index":shard.map(|v|v.0),"shard_count":shard.map(|v|v.1),"completed":completed,"scope":if shard.is_some() {"partial; requires exact-source aggregate"} else {"complete serial matrix"}}),
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
    let reuse = reuse_policy()?;
    let sources = source_inventory()?;
    let mut observations = Observations::default();
    for task in tasks().iter().filter(|task| {
        task.flag
            .is_some_and(|flag| IPC_CONTROLS.iter().any(|entry| entry.0 == flag))
    }) {
        if reuse {
            observations.execute(task)?;
        } else {
            execute_task(task)?;
        }
    }
    if sources != source_inventory()? {
        return Err("IPC matrix sources changed during execution".into());
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
        "--ipc-teardown-control" => (
            "crates/kernel-core/src/ipc.rs",
            "fn drop(",
            "IPC endpoint release requires actual request/reference/wake quiescence",
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
    fn reuse_policy_accepts_only_explicit_on_off_or_absent() {
        assert!(reuse_enabled(None).unwrap());
        assert!(reuse_enabled(Some("on")).unwrap());
        assert!(!reuse_enabled(Some("off")).unwrap());
        for value in ["", "ON", "false", "0", " off", "off "] {
            assert!(reuse_enabled(Some(value)).is_err());
        }
    }
    fn ordinary_result(task: &Task, events: Value) -> Value {
        let profile = if task.prod { "prod" } else { "dev" };
        let elf = format!("target/kernel/{profile}-tests.elf");
        json!({"id":task.id,"observation":"executed",
            "build":{"artifact":elf,"sha256":"digest","features":if task.prod {
                vec!["machine-events", "kernel-tests"]
            } else {vec!["machine-events", "diagnostics", "kernel-tests"]}},
            "run":{"arguments":qemu_args(Path::new(&elf)),"elf_sha256":"digest"},
            "events":events})
    }
    #[test]
    fn reuse_requires_ordinary_profile_features_arguments_and_direct_execution() {
        let task = observable_task("--supervision-wait-control", false);
        let events = json!([
            {"event":"suite","status":"pass"},
            {"event":"test","name":"supervision_el0_lifecycle","status":"pass"}
        ]);
        let result = ordinary_result(&task, events);
        assert!(ordinary_evidence(&task, &result).is_ok());
        let mut prod = task.clone();
        prod.prod = true;
        assert!(ordinary_evidence(&prod, &result).is_err());
        for (field, value) in [
            ("observation", json!("reused")),
            ("reused_from", json!("other")),
            ("events", json!([])),
        ] {
            let mut invalid = result.clone();
            invalid[field] = value;
            assert!(ordinary_evidence(&task, &invalid).is_err());
        }
        for (section, field, value) in [
            (
                "build",
                "features",
                json!(["machine-events", "kernel-tests"]),
            ),
            ("run", "arguments", json!(["-smp", "1"])),
            ("run", "elf_sha256", json!("different")),
            ("build", "artifact", json!("target/kernel/prod-tests.elf")),
        ] {
            let mut invalid = result.clone();
            invalid[section][field] = value;
            assert!(ordinary_evidence(&task, &invalid).is_err());
        }
        let mut boot = task.clone();
        boot.tests = false;
        assert!(!reusable(&boot));
        let mut feature = task.clone();
        feature.feature = Some("panic-test");
        assert!(!reusable(&feature));
        let mut fatal = task;
        fatal.flag = Some("--panic-control");
        assert!(!reusable(&fatal));
    }
    #[test]
    fn reuse_rechecks_consumer_missing_duplicate_failed_checks_and_suite() {
        let task = observable_task("--supervision-wait-control", false);
        let suite = json!({"event":"suite","status":"pass"});
        let check = json!({"event":"test","name":"supervision_el0_lifecycle","status":"pass"});
        for events in [
            json!([suite.clone()]),
            json!([suite.clone(), check.clone(), check.clone()]),
            json!([suite.clone(), {"event":"test","name":"supervision_el0_lifecycle","status":"fail"}]),
            json!([suite.clone(), check.clone(), {"event":"panic","status":"fail"}]),
            json!([suite.clone(), suite, check.clone()]),
            json!([check]),
        ] {
            assert!(ordinary_evidence(&task, &ordinary_result(&task, events)).is_err());
        }
    }
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
