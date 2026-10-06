mod matrix;
mod native_apps;
mod output;
#[cfg(feature = "route-tools")]
mod routing_demo;
mod timing;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
// ELF64 little-endian header and PT_LOAD fields.
const ELF_CLASS_OFFSET: usize = 4;
const ELF_CLASS_64: u8 = 2;
const ELF_MACHINE_OFFSET: usize = 18;
const ELF_MACHINE_AARCH64: usize = 183;
const ELF_PROGRAM_TABLE_OFFSET: usize = 32;
const ELF_PROGRAM_ENTRY_SIZE_OFFSET: usize = 54;
const ELF_PROGRAM_COUNT_OFFSET: usize = 56;
const ELF_PT_LOAD: u32 = 1;
const ELF_FLAGS_OFFSET: usize = 4;
const ELF_FILE_SIZE_OFFSET: usize = 32;
const ELF_MEMORY_SIZE_OFFSET: usize = 40;
const ELF_EXECUTE: u32 = 1;
const ELF_WRITE: u32 = 2;
const UNSAFE_CONTEXT_PRECEDING_LINES: usize = 4;
const QEMU_TIMEOUT: Duration = Duration::from_secs(30);
const QEMU_POLL_INTERVAL: Duration = Duration::from_millis(20);
const NEGATIVE_CONTROLS: &[(&str, &str)] = &[
    ("--negative-control", "negative_control"),
    ("--panic-control", "panic reporting negative control"),
    ("--ownership-control", "physical pool already owned"),
    ("--retained-mapping-control", "mapped frame release"),
    ("--secondary-panic-control", "secondary CPU failure"),
    ("--retirement-control", "retiring frame release"),
    ("--shootdown-control", "remote TLB acknowledgement timeout"),
    (
        "--remote-tlbi-control",
        "kernel test failed: smp_remote_ack",
    ),
    ("--user-context-control", "user register context lost"),
    ("--user-root-control", "user address-space alias leaked"),
    (
        "--user-retirement-control",
        "user address space retains frame",
    ),
    ("--asid-reuse-control", "asid_reuse_requires_invalidation"),
];
const FOUNDATION_CONTROLS: &[(&str, &str, &str)] = &[(
    "--irq-simd-restore-control",
    "kernel-tests",
    "\"name\":\"irq_simd_context\",\"status\":\"fail\"",
)];
const SCHEDULER_CONTROLS: &[(&str, &str, &str)] = &[
    (
        "--checkpoint-publication-control",
        "kernel-tests",
        "CompletionPublicationTimeout",
    ),
    ("--process-exit-control", "kernel-tests", "Transition"),
    ("--process-rollback-control", "kernel-tests", "RollbackLeak"),
    ("--process-unlink-control", "kernel-tests", "NotQuiescent"),
    ("--process-start-control", "kernel-tests", "Transition"),
    ("--process-stale-control", "kernel-tests", "Stale"),
    ("--process-reclaim-control", "kernel-tests", "Transition"),
    (
        "--scheduler-aarch32-control",
        "kernel-tests",
        "InvalidUserContext",
    ),
    (
        "--scheduler-user-irq-control",
        "kernel-tests",
        "InvalidUserContext",
    ),
    (
        "--scheduler-context-control",
        "kernel-tests",
        "InvalidUserContext",
    ),
    (
        "--scheduler-inner-lock-control",
        "scheduler-inner-lock-input",
        "lock inside scheduler ownership contract",
    ),
    ("--scheduler-start-control", "kernel-tests", "WrongPhase"),
    (
        "--scheduler-task-control",
        "scheduler-task-input",
        "stale scheduler task",
    ),
    (
        "--scheduler-owner-control",
        "scheduler-owner-input",
        "duplicate running task",
    ),
    ("--scheduler-foreign-control", "kernel-tests", "ForeignCpu"),
    ("--scheduler-reentry-control", "kernel-tests", "Reentry"),
    (
        "--scheduler-stale-control",
        "kernel-tests",
        "StaleGeneration",
    ),
    ("--scheduler-reset-control", "kernel-tests", "WrongPhase"),
    ("--scheduler-inspect-control", "kernel-tests", "WrongPhase"),
    (
        "--scheduler-complete-control",
        "kernel-tests",
        "NotQuiescent",
    ),
    ("--scheduler-irq-control", "kernel-tests", "IrqEnabled"),
    (
        "--scheduler-lock-control",
        "scheduler-lock-input",
        "scheduler lock order contract",
    ),
];
const HANDLE_CONTROLS: &[(&str, &str, &str)] = &[
    (
        "--handle-generation-control",
        "handle-generation-negative",
        "\"name\":\"handle_el0_identity_type_generation_and_lifetime\",\"status\":\"fail\"",
    ),
    (
        "--handle-owner-control",
        "handle-owner-negative",
        "\"name\":\"handle_el0_identity_type_generation_and_lifetime\",\"status\":\"fail\"",
    ),
    (
        "--handle-type-control",
        "handle-type-negative",
        "\"name\":\"handle_el0_identity_type_generation_and_lifetime\",\"status\":\"fail\"",
    ),
    (
        "--handle-reuse-control",
        "handle-reuse-negative",
        "\"name\":\"handle_el0_identity_type_generation_and_lifetime\",\"status\":\"fail\"",
    ),
    (
        "--handle-retirement-control",
        "handle-retirement-negative",
        "\"event\":\"handle-retirement-reject\",\"status\":\"fail\"",
    ),
    (
        "--handle-transfer-rights-control",
        "handle-transfer-rights-negative",
        "\"name\":\"handle_el0_identity_type_generation_and_lifetime\",\"status\":\"fail\"",
    ),
];
const SECURITY_CONTROLS: &[(&str, &str, &str)] = &[
    (
        "--domain-budget-control",
        "domain-budget-negative",
        "\"name\":\"domain_memory_budget_enforced\",\"status\":\"fail\"",
    ),
    (
        "--domain-identity-control",
        "domain-identity-negative",
        "\"name\":\"capability_el0_scope_attenuation_and_denial\",\"status\":\"fail\"",
    ),
    (
        "--domain-teardown-control",
        "domain-teardown-negative",
        "\"name\":\"capability_el0_scope_attenuation_and_denial\",\"status\":\"fail\"",
    ),
    (
        "--capability-revoke-control",
        "capability-revoke-negative",
        "\"name\":\"capability_el0_scope_attenuation_and_denial\",\"status\":\"fail\"",
    ),
    (
        "--capability-scope-control",
        "capability-scope-negative",
        "\"name\":\"capability_el0_scope_attenuation_and_denial\",\"status\":\"fail\"",
    ),
];
const USER_COPY_CONTROLS: &[(&str, &str, &str)] = &[
    (
        "--user-copy-snapshot-control",
        "user-copy-snapshot-negative",
        "\"name\":\"user_copy_el0_boundary_and_snapshot\",\"status\":\"fail\"",
    ),
    (
        "--user-copy-recovery-control",
        "user-copy-recovery-negative",
        "\"event\":\"fatal\"",
    ),
];
const IPC_CONTROLS: &[(&str, &str, &str)] = &[
    (
        "--supervision-authority-control",
        "supervision-authority-negative",
        "supervision:Scenario",
    ),
    (
        "--supervision-stale-control",
        "supervision-stale-negative",
        "supervision:Scenario",
    ),
    (
        "--supervision-dependency-control",
        "supervision-dependency-negative",
        "supervision:DependencyNotRejected",
    ),
    (
        "--supervision-commit-control",
        "supervision-commit-negative",
        "supervision:CommitNotProven",
    ),
    (
        "--supervision-wait-control",
        "supervision-wait-negative",
        "supervision:WaitIdentityLost",
    ),
    (
        "--ipc-request-generation-control",
        "ipc-request-generation-negative",
        "ipc_payload_snapshot_result_id_and_stress",
    ),
    (
        "--ipc-double-charge-release-control",
        "ipc-double-charge-release-negative",
        "reject:ChargeAlreadyReleased",
    ),
    (
        "--ipc-storage-scope-control",
        "ipc-storage-scope-negative",
        "reject:StorageInsideScheduler",
    ),
    (
        "--ipc-duplicate-ready-control",
        "ipc-duplicate-ready-negative",
        "reject:DuplicateReady",
    ),
    (
        "--ipc-wrong-process-wake-control",
        "ipc-wrong-process-wake-negative",
        "reject:WrongProcessWake",
    ),
    (
        "--ipc-blocked-reclaim-control",
        "ipc-blocked-reclaim-negative",
        "reject:BlockedTaskReclaim",
    ),
    (
        "--ipc-wake-generation-control",
        "ipc-wake-generation-negative",
        "reject:WakeGenerationAccepted",
    ),
    (
        "--ipc-wait-recheck-control",
        "ipc-wait-recheck-negative",
        "reject:WaitRegistrationLost",
    ),
    (
        "--ipc-wake-publication-control",
        "ipc-wake-publication-negative",
        "reject:WakePublicationLost",
    ),
    (
        "--ipc-service-token-control",
        "ipc-service-token-negative",
        "ipc_el0_cpu0_to_cpu1",
    ),
    (
        "--ipc-charge-release-control",
        "ipc-charge-release-negative",
        "ipc_deadline_before_effect",
    ),
    (
        "--ipc-teardown-control",
        "ipc-teardown-negative",
        "reject:EndpointNotQuiescent",
    ),
    (
        "--ipc-cancel-control",
        "ipc-cancel-negative",
        "ipc_cancel_before_commit",
    ),
    (
        "--ipc-service-death-control",
        "ipc-service-death-negative",
        "ipc_service_death_queued",
    ),
    (
        "--ipc-double-terminal-control",
        "ipc-double-terminal-negative",
        "ipc_payload_snapshot_result_id_and_stress",
    ),
    (
        "--ipc-capacity-control",
        "ipc-capacity-negative",
        "ipc_queue_full_fifo_and_reclamation",
    ),
    (
        "--ipc-fifo-control",
        "ipc-fifo-negative",
        "ipc_queue_full_fifo_and_reclamation",
    ),
    (
        "--ipc-id-reuse-control",
        "ipc-id-reuse-negative",
        "ipc_payload_snapshot_result_id_and_stress",
    ),
    (
        "--ipc-receive-copy-control",
        "ipc-receive-copy-negative",
        "ipc_receive_copy_failure_retains_queue",
    ),
    (
        "--ipc-collect-copy-control",
        "ipc-collect-copy-negative",
        "ipc_collect_copy_failure_retains_result",
    ),
    (
        "--ipc-send-rights-control",
        "ipc-send-rights-negative",
        "ipc_authority_denial_revoke_and_retention",
    ),
    (
        "--ipc-revoke-control",
        "ipc-revoke-negative",
        "ipc_authority_denial_revoke_and_retention",
    ),
    (
        "--ipc-deadline-control",
        "ipc-deadline-negative",
        "ipc_deadline_before_effect",
    ),
];
#[path = "../../kernel/src/platform/config.rs"]
#[allow(dead_code)] // Layout constants are consumed by the target kernel.
mod platform_config;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const TESTS: &[&str] = &[
    "boot_el1",
    "completion_publication_state_inputs",
    "scheduler_foreign_cpu_rejected",
    "scheduler_unmasked_access_rejected",
    "scheduler_reentry_rejected",
    "scheduler_stale_generation_rejected",
    "scheduler_duplicate_start_rejected",
    "scheduler_live_reset_rejected",
    "scheduler_early_inspection_rejected",
    "process_aarch32_context_rejected",
    "process_masked_user_irq_rejected",
    "process_privileged_context_rejected",
    "process_unmasked_access_rejected",
    "process_duplicate_completion_rejected",
    "uart_mmio",
    "exception_vectors",
    "physical_discovery",
    "physical_allocator",
    "allocation_free",
    "physical_reuse",
    "physical_exhaustion",
    "page_map",
    "invalid_mapping_rejection",
    "page_unmap",
    "permissions",
    "execute_never",
    "mmu",
    "heap",
    "heap_exhaustion",
    "locking_exclusion",
    "locking_release",
    "monotonic_time",
    "interrupt_masking",
    "interrupt_delivery",
    "timer_rearm",
    "irq_simd_context",
    "smp_secondary_boot",
    "smp_cpu_identity",
    "smp_separate_stacks",
    "smp_ipi_forward",
    "smp_ipi_reverse",
    "smp_ipi_repeated",
    "smp_lock_publication",
    "smp_mapping_visible",
    "smp_retirement_pending",
    "smp_no_premature_reuse",
    "smp_remote_ack",
    "smp_future_ack_rejected",
    "smp_remote_tlb_invalidation",
    "smp_safe_reuse",
    "smp_simultaneous_timers",
    "smp_percpu_independent",
    "smp_orderly_shutdown",
    "el0_processes",
    "el0_timer_switches",
    "el0_user_stacks",
    "el0_memory_isolation",
    "el0_fault_containment",
    "el0_quiescent_reclamation",
    "el0_context_preservation",
    "user_context_invalid_states_rejected",
    "el0_smp_ownership",
    "el0_kernel_memory_rejected",
    "el0_foreign_memory_rejected",
    "el0_code_write_rejected",
    "el0_stack_guard",
    "el0_data_execute_rejected",
    "el0_privileged_instruction_rejected",
    "scheduler_generation_reuse",
    "process_registry_and_cpu_ownership",
    "process_preparation_and_admission",
    "process_live_reclaim_rejected",
    "process_unlinked_reclaim_rejected",
    "process_duplicate_start_rejected",
    "process_normal_exit_and_fault",
    "process_creation_rollback",
    "process_capacity_exhaustion",
    "asid_pool_exhaustion_both_cpus",
    "process_slot_generation_reuse",
    "process_terminal_rejections",
    "process_authority_boundary",
    "process_sparse_affinity",
    "process_bounded_stress",
    "asid_reuse_same_va_both_cpus",
    "asid_reuse_requires_invalidation",
    "process_el0_residency_initial",
    "process_el0_residency_monotonic",
    "process_el0_residency_accumulates",
    "process_quantum_return_and_peer_progress",
    "process_wait_block_and_wakeup",
    "elf_invalid_images_are_transactional",
    "elf_entry_alignment",
    "elf_creation_failure_is_transactional",
    "elf_executes_in_isolated_el0_spaces",
    "elf_bss_and_page_padding_are_zero",
    "elf_reclaims_all_frames",
    "process_resource_reclamation",
    "user_copy_el0_boundary_and_snapshot",
    "user_copy_lifetime_and_reclamation",
    "handle_cross_process_reference_isolation",
    "handle_el0_transfer_transaction_attenuation",
    "handle_el0_identity_type_generation_and_lifetime",
    "handle_exit_fault_cleanup_and_process_reuse",
    "domain_memory_budget_enforced",
    "domain_el0_request_and_queue_budgets",
    "ipc_el0_cpu0_to_cpu1",
    "supervision_el0_lifecycle",
    "ipc_el0_cpu1_to_cpu0",
    "ipc_el0_same_cpu0",
    "ipc_el0_same_cpu1",
    "ipc_deadline_before_effect",
    "ipc_deadline_after_commit",
    "ipc_service_death_queued",
    "ipc_service_death_delivered",
    "ipc_service_death_committed",
    "ipc_receive_copy_failure_retains_queue",
    "ipc_collect_copy_failure_retains_result",
    "ipc_cancel_before_commit",
    "ipc_cancel_after_commit",
    "ipc_requester_death_queued",
    "ipc_requester_death_delivered",
    "ipc_requester_death_committed",
    "ipc_authority_denial_revoke_and_retention",
    "ipc_payload_snapshot_result_id_and_stress",
    "ipc_queue_full_fifo_and_reclamation",
    "ipc_concurrent_producers_fifo_and_reclamation",
    "ipc_queued_head_nonhead_cancel_fifo",
    "ipc_unconsumed_terminal_domain_teardown",
    "ipc_concurrent_revoke_admission_retains_accepted",
    "ipc_both_peers_die_with_accepted_work",
    "ipc_shutdown_queued_and_blocked_requester",
    "ipc_empty_service_death_reclamation",
    "ipc_request_quota_failure_has_no_phantom_work",
    "capability_el0_scope_attenuation_and_denial",
    "capability_revocation_retains_admitted_effect",
    "domain_teardown_retains_accepted_notification",
    "domain_fault_peer_progress_and_reclamation",
    "domain_rebind_does_not_restore_grants",
    "capability_cross_cpu_revoke_admission",
    "domain_reclaimed_sender_retains_request_charge",
    "domain_service_fault_cancels_effect_and_releases_charges",
    "capability_service_rebind_preserves_scope",
    "domain_el0_observes_service_fault_cancellation",
];
fn main() {
    if let Err(e) = run() {
        eprintln!("xtask: {e}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    env::set_current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))?;
    fs::create_dir_all("target/kernel")?;
    let args = output::color_arguments(env::args().skip(1).collect())?;
    match args.first().map(String::as_str) {
        Some("docs" | "arena") => {
            let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
                .args(["run", "--locked", "-p", "repository-checks", "--", &args[0]])
                .args(&args[1..]).status()?;
            if status.success() { Ok(()) } else { Err("repository tooling command failed".into()) }
        }
        Some("app-smoke") => native_apps::smoke(&args[1..]),
        Some("selftest") => native_apps::selftest(&args[1..]),
        Some("service-run") => native_apps::runtime(&args[1..]),
        Some("lifecycle-test") => native_apps::lifecycle(&args[1..]),
        Some("native-controls") => native_apps::controls(&args[1..]),
        Some("audit") => audit(),
        Some("ipc-controls") if args.len() == 1 => matrix::run_ipc(),
        Some("matrix-plan") if args.len() == 1 => {
            println!("{}", serde_json::to_string_pretty(&matrix::plan_document()?)?);
            Ok(())
        }
        Some("matrix-task") if args.len() == 2 || args.len() == 3 && args[2] == "--prod" => {
            qemu()?;
            matrix::run_one(&args[1], args.len() == 3)
        }
        Some("matrix-shard") if args.len() == 3 => {
            let index: usize = args[1].parse()?;
            let count: usize = args[2].parse()?;
            qemu()?;
            audit()?;
            matrix::run(Some((index, count)))
        }
        Some("ipc-bench") if args.len()==1 => {
            let sources=source_inventory()?;
            for prod in [false,true] {
                let elf=build(prod,false,Some("ipc-benchmark"),true)?;
                execute(&elf,false,true)?;
            }
            if source_inventory()?!=sources {return Err("IPC measurement sources changed during run".into());}
            archive_ipc_benchmark(&sources)?;
            Ok(())
        },
        Some("routing") => {
            #[cfg(feature = "route-tools")]
            { routing_demo::run(&args[1..]) }
            #[cfg(not(feature = "route-tools"))]
            {
                let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
                    .args(["run", "--locked", "-p", "xtask", "--target-dir", "target/route-tools", "--features", "route-tools", "--"])
                    .args(&args).status()?;
                if status.success() { Ok(()) } else { Err("routing tool command failed".into()) }
            }
        }
        Some("compare") if args.len() == 3 => compare_measurements(Path::new(&args[1]), Path::new(&args[2])),
        Some("asid-bench") if args.len() == 1 => asid_bench(),
        Some("build") => {
            build(args.iter().any(|a| a == "--prod"), false, None, false)?;
            Ok(())
        }
        Some("run") => {
            let machine = args.iter().any(|a| a == "--machine");
            if args.iter().skip(1).any(|a| a != "--prod" && a != "--machine") { return Err("usage: cargo xtask run [--prod] [--machine]".into()); }
            let elf = build(args.iter().any(|a| a == "--prod"), false, None, machine)?;
            execute(&elf, false, machine)
        }
        Some("debug") => {
            let elf = build(false, false, None, false)?;
            let q = qemu()?;
            Command::new(q)
                .args(qemu_args(&elf))
                .args(["-S", "-gdb", "tcp:127.0.0.1:1234"])
                .status()?;
            Ok(())
        }
        Some("test") => {
            if args.iter().any(|arg| arg == "--positive-only") {
                if args.iter().skip(1).any(|arg| arg != "--positive-only" && arg != "--prod") { return Err("positive-only accepts only --prod".into()); }
                let elf = build(args.iter().any(|arg| arg == "--prod"), true, None, true)?;
                return execute(&elf, true, true);
            }
            if let Some(flag) = args.iter().find(|flag| matrix::observable_checks(flag).is_some()) { return matrix::run_observable(flag, args.iter().any(|arg| arg == "--prod")); }
            for &(flag, feature, _) in IPC_CONTROLS {
                if args.iter().any(|arg| arg == flag) {
                    let elf = build(args.iter().any(|arg| arg == "--prod"), true, Some(feature), true)?;
                    return execute(&elf, true, true);
                }
            }
            for &(flag, feature, _) in FOUNDATION_CONTROLS.iter().chain(USER_COPY_CONTROLS.iter()).chain(HANDLE_CONTROLS.iter()).chain(SECURITY_CONTROLS.iter()) {
                if args.iter().any(|arg| arg == flag) {
                    let elf = build(args.iter().any(|arg| arg == "--prod"), true, Some(feature), true)?;
                    return execute(&elf, true, true);
                }
            }
            for &(flag, feature, _) in SCHEDULER_CONTROLS {
                if args.iter().any(|arg| arg == flag) {
                    let elf = build(args.iter().any(|arg| arg == "--prod"), true, Some(feature), true)?;
                    return execute(&elf, true, true);
                }
            }
            for (flag, feature) in [("--secondary-panic-control", "secondary-panic-test"), ("--retirement-control", "retirement-negative")] {
                if args.iter().any(|a| a == flag) { let elf = build(false, true, Some(feature), true)?; return execute(&elf, true, true); }
            }
            if args.iter().any(|a| a == "--negative-control") {
                let elf = build(false, true, Some("negative-test"), true)?;
                return execute(&elf, true, true);
            }
            if args.iter().any(|a| a == "--panic-control") {
                let elf = build(false, true, Some("panic-test"), true)?;
                return execute(&elf, true, true);
            }
            if args.iter().any(|a| a == "--ownership-control") {
                let elf = build(false, true, Some("ownership-test"), true)?;
                return execute(&elf, true, true);
            }
            if args.iter().any(|a| a == "--retained-mapping-control") {
                let elf = build(false, true, Some("retained-mapping-test"), true)?;
                return execute(&elf, true, true);
            }
            let label = match args.as_slice() {
                [_] => None,
                [_, flag, label] if flag == "--record" => Some(label.as_str()),
                _ => return Err("usage: cargo xtask test [--record LABEL]".into()),
            };
            let starting_sources = source_inventory()?;
            qemu()?;
            audit()?;
            matrix::run(None)?;
            archive_measurements(label, &starting_sources)?;
            Ok(())
        }
        _ => Err("usage: cargo xtask selftest [--prod] | app-smoke [--prod] | service-run [--prod] [--live] | native-controls [--prod] | test [--record LABEL] | matrix-task FLAG [--prod] | matrix-shard INDEX COUNT | asid-bench | compare BASELINE CANDIDATE | build [--prod] | run [--prod] [--machine] | audit | debug".into()),
    }
}
fn archive_ipc_benchmark(sources: &Value) -> Result<()> {
    let mut profiles = serde_json::Map::new();
    for profile in ["dev", "prod"] {
        let events = read_json(format!(
            "target/kernel/{profile}-ipc-benchmark.results.json"
        ))?;
        let records = events.as_array().ok_or("IPC benchmark events absent")?;
        output::validate(records, false, &[], platform_config::ACTIVE_CPUS)?;
        let mut observations = Vec::new();
        for event in records
            .iter()
            .filter(|event| event["event"] == "ipc-measurement")
        {
            let samples = event["samples"]
                .as_array()
                .ok_or("IPC samples absent")?
                .iter()
                .map(|sample| sample.as_u64().ok_or("invalid IPC sample"))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let mean =
                samples.iter().map(|sample| *sample as f64).sum::<f64>() / samples.len() as f64;
            let stddev = (samples
                .iter()
                .map(|sample| (*sample as f64 - mean).powi(2))
                .sum::<f64>()
                / samples.len() as f64)
                .sqrt();
            let mut observation = event.clone();
            observation["mean"] = json!(mean);
            observation["stddev_population"] = json!(stddev);
            observations.push(observation);
        }
        if observations.len() != 144 {
            return Err("IPC performance groups incomplete".into());
        }
        profiles.insert(
            profile.into(),
            json!({"observations":observations,
            "build":read_json(format!("target/kernel/{profile}-ipc-benchmark-build.json"))?,
            "run":read_json(format!("target/kernel/{profile}-ipc-benchmark.run.json"))?,
            "events":events}),
        );
    }
    fs::create_dir_all("research/measurements")?;
    let artifact = json!({"schema_version":1,"scope":"QEMU TCG regression baseline; not physical ARM performance or a speed target",
        "clock":"boot-local architectural counter ticks; frequency recorded per observation",
        "measurement":"EL0 clock-probe envelopes include clock return/entry, argument setup and result checks; warmup 4, measured samples 16",
        "round_trip":"submit through terminal wait and successful collect, without intermediate operation timing/report probes",
        "receive":"Generic receive is mixed. Dedicated hot_ready_receive requires zero condition blocks; blocked_receive_wake and blocked_requester_wait require one condition block per operation, verified from owner-local CLOCK observations before/after every warmup and measured call. Queue-full requires explicit exhaustion with zero condition blocks.",
        "copy_path":["client user to initialized kernel request","kernel request to service user","service user to initialized kernel response","kernel response to client user"],
        "limitations":["No zero-copy","No hardware speed claim","No compatibility penalty or synthetic timing subtraction","Wake timing includes native publication/notification in submit and target resumption in receive/round-trip envelopes; no isolated IPI-only cost"],
        "sources":sources,"profiles":profiles});
    fs::write(
        "research/measurements/ipc-phase35-baseline.json",
        serde_json::to_string_pretty(&artifact)?,
    )?;
    println!("IPC baseline: research/measurements/ipc-phase35-baseline.json");
    Ok(())
}
fn build(prod: bool, tests: bool, extra: Option<&str>, machine: bool) -> Result<PathBuf> {
    build_mode(prod, tests, extra, machine, 0, None)
}
fn build_mode(
    prod: bool,
    tests: bool,
    extra: Option<&str>,
    machine: bool,
    argument: u64,
    native_images: Option<&[PathBuf; 3]>,
) -> Result<PathBuf> {
    let mut c = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    c.args([
        "build",
        "--locked",
        "-p",
        "kolvrt-kernel",
        "--target",
        "aarch64-unknown-none",
        "--no-default-features",
    ]);
    if prod {
        c.arg("--release");
    }
    let mut features = Vec::new();
    if machine {
        features.push("machine-events");
    }
    if !prod {
        features.push("diagnostics");
    }
    if tests {
        features.push("kernel-tests");
    }
    if let Some(e) = extra {
        features.push(e);
        if e == "boot-payload" {
            c.env(
                "KOLVRT_BOOT_PAYLOAD",
                fs::canonicalize("target/kernel/payload.bin")?,
            );
        }
    }
    if extra.is_some_and(|f| f.split(',').any(|part| part == "native-apps")) {
        c.env_remove("CARGO_TARGET_DIR");
        let images = native_images.ok_or("native build requires immutable original ELF inputs")?;
        for (index, (name, _role)) in [
            ("KOLVRT_NATIVE_ROOT_ELF", "root"),
            ("KOLVRT_NATIVE_SERVICE_ELF", "service"),
            ("KOLVRT_NATIVE_CLIENT_ELF", "client"),
        ]
        .into_iter()
        .enumerate()
        {
            c.env(name, fs::canonicalize(&images[index])?);
        }
        c.env("KOLVRT_NATIVE_ARGUMENT", argument.to_string()).env(
            "KOLVRT_NATIVE_WATCHDOG_MS",
            if argument == 1 { "0" } else { "20000" },
        );
    }
    if !features.is_empty() {
        c.args(["--features", &features.join(",")]);
    }
    let observation = timing::Observation::start("cargo-build", format!("{c:?}"));
    let status = c.status();
    observation.finish(if status.as_ref().is_ok_and(|status| status.success()) {
        "success"
    } else {
        "failure"
    });
    if !status?.success() {
        return Err("kernel build failed".into());
    }
    let source = format!(
        "target/aarch64-unknown-none/{}/kolvrt-kernel",
        if prod { "release" } else { "debug" }
    );
    let name = format!(
        "{}-{}",
        if prod { "prod" } else { "dev" },
        extra.unwrap_or(if tests { "tests" } else { "boot" })
    );
    let dest = PathBuf::from(format!("target/kernel/{name}.elf"));
    let source = PathBuf::from(&source);
    fs::copy(source, &dest)?;
    let bytes = fs::read(&dest)?;
    let u16at = |o: usize| {
        u16::from_le_bytes(
            bytes[o..o + core::mem::size_of::<u16>()]
                .try_into()
                .unwrap(),
        ) as usize
    };
    let u64at = |o: usize| {
        u64::from_le_bytes(
            bytes[o..o + core::mem::size_of::<u64>()]
                .try_into()
                .unwrap(),
        )
    };
    if &bytes[..b"\x7fELF".len()] != b"\x7fELF"
        || bytes[ELF_CLASS_OFFSET] != ELF_CLASS_64
        || u16at(ELF_MACHINE_OFFSET) != ELF_MACHINE_AARCH64
    {
        return Err("wrong ELF architecture".into());
    }
    let mut loaded = 0;
    let mut memory = 0;
    let mut wx = false;
    for i in 0..u16at(ELF_PROGRAM_COUNT_OFFSET) {
        let o = u64at(ELF_PROGRAM_TABLE_OFFSET) as usize + i * u16at(ELF_PROGRAM_ENTRY_SIZE_OFFSET);
        let kind = u32::from_le_bytes(
            bytes[o..o + core::mem::size_of::<u32>()]
                .try_into()
                .unwrap(),
        );
        if kind == ELF_PT_LOAD {
            let flags = u32::from_le_bytes(
                bytes[o + ELF_FLAGS_OFFSET..o + ELF_FLAGS_OFFSET + core::mem::size_of::<u32>()]
                    .try_into()
                    .unwrap(),
            );
            loaded += u64at(o + ELF_FILE_SIZE_OFFSET);
            memory += u64at(o + ELF_MEMORY_SIZE_OFFSET);
            wx |= flags & (ELF_WRITE | ELF_EXECUTE) == (ELF_WRITE | ELF_EXECUTE);
        }
    }
    if wx {
        return Err("ELF has writable executable segment".into());
    }
    if prod
        && !tests
        && bytes
            .windows(b"negative_control".len())
            .any(|s| s == b"negative_control")
    {
        return Err("production includes negative test".into());
    }
    let report = json!({"artifact":dest,"elf_bytes":bytes.len(),"load_bytes":loaded,"memory_bytes":memory,"features":features,"sha256":(Sha256::digest(&bytes)).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),"compiler":"1.99.0","target":"aarch64-unknown-none"});
    fs::write(
        format!("target/kernel/{name}-build.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    print!(
        "{}",
        output::console_text(
            &format!(
                "[OK] build: {} ({} bytes ELF; {} bytes loaded)\n",
                dest.display(),
                bytes.len(),
                loaded
            ),
            output::stdout_color()
        )
    );
    Ok(dest)
}
fn qemu() -> Result<PathBuf> {
    let path = env::var_os("QEMU_AARCH64")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if cfg!(windows) {
                PathBuf::from(".toolchains/qemu/bin/qemu-system-aarch64.exe")
            } else {
                PathBuf::from("qemu-system-aarch64")
            }
        });
    let version = Command::new(&path)
        .arg("--version")
        .output()
        .map_err(|e| format!("QEMU 10.1.0 unavailable at {}: {e}", path.display()))?;
    let version = String::from_utf8(version.stdout)?;
    if !version.starts_with("QEMU emulator version 10.1.0 ")
        && !version.starts_with("QEMU emulator version 10.1.0\n")
    {
        return Err(format!("QEMU pin mismatch: {version}").into());
    }
    Ok(path)
}
fn qemu_args(elf: &Path) -> Vec<String> {
    vec![
        "-machine".into(),
        "virt-10.1,gic-version=3,virtualization=on,its=off,dtb-randomness=off".into(),
        "-cpu".into(),
        env::var("QEMU_CPU").unwrap_or_else(|_| "cortex-a57".into()),
        "-accel".into(),
        "tcg".into(),
        "-smp".into(),
        platform_config::CONFIGURED_CPUS.to_string(),
        "-m".into(),
        format!("{}M", platform_config::RAM_BYTES / (1024 * 1024)),
        "-display".into(),
        "none".into(),
        "-serial".into(),
        "stdio".into(),
        "-monitor".into(),
        "none".into(),
        "-nic".into(),
        "none".into(),
        "-no-reboot".into(),
        "-kernel".into(),
        elf.to_string_lossy().into(),
    ]
}
fn execute(elf: &Path, tests: bool, machine: bool) -> Result<()> {
    execute_mode(elf, tests, machine, false, true)
}
fn execute_quiet(elf: &Path, tests: bool, machine: bool) -> Result<()> {
    execute_mode(elf, tests, machine, false, false)
}
fn execute_mode(
    elf: &Path,
    tests: bool,
    machine: bool,
    payload: bool,
    show_console: bool,
) -> Result<()> {
    execute_validated(elf, tests, machine, payload, show_console, false)
}
fn execute_native(elf: &Path) -> Result<()> {
    execute_validated(elf, false, true, false, true, true)
}
fn execute_validated(
    elf: &Path,
    tests: bool,
    machine: bool,
    payload: bool,
    show_console: bool,
    native: bool,
) -> Result<()> {
    let log = elf.with_extension("log");
    let err = elf.with_extension("stderr");
    // Never leave an earlier run's successful evidence beside a failed/human run.
    let result = elf.with_extension("results.json");
    if result.exists() {
        fs::remove_file(&result)?;
    }
    let emulator = qemu()?;
    let version = Command::new(&emulator).arg("--version").output()?;
    fs::write(
        elf.with_extension("run.json"),
        serde_json::to_string_pretty(
            &json!({"qemu_version":String::from_utf8(version.stdout)?,"arguments":qemu_args(elf),"active_cpus":platform_config::ACTIVE_CPUS,"configured_cpus":platform_config::CONFIGURED_CPUS,"elf_sha256":(Sha256::digest(fs::read(elf)?)).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),"timeout_seconds":QEMU_TIMEOUT.as_secs(),"accelerator":"TCG","measurement_claim":"emulator timer ticks; not hardware throughput"}),
        )?,
    )?;
    let observation =
        timing::Observation::start("qemu", format!("{} {:?}", elf.display(), qemu_args(elf)));
    let mut child = Command::new(emulator)
        .args(qemu_args(elf))
        .stdin(Stdio::null())
        .stdout(fs::File::create(&log)?)
        .stderr(fs::File::create(&err)?)
        .spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            observation.finish(if status.success() {
                "success"
            } else {
                "failure"
            });
            if !status.success() {
                return Err(format!("QEMU failed: {}", fs::read_to_string(&err)?).into());
            }
            break;
        }
        if start.elapsed() > QEMU_TIMEOUT {
            child.kill()?;
            child.wait()?;
            observation.finish("timeout");
            return Err(format!("QEMU timeout: {}", fs::read_to_string(&log)?).into());
        }
        thread::sleep(QEMU_POLL_INTERVAL);
    }
    let text = fs::read_to_string(&log)?;
    if show_console {
        print!("{}", output::console_text(&text, output::stdout_color()));
    }
    if !machine {
        if text.contains("@KOLVRT") || text.contains("[FAIL]") {
            return Err("human console failure or unexpected machine record".into());
        }
        return Ok(());
    }
    let events = output::parse(&text)?;
    fs::write(
        elf.with_extension("results.json"),
        serde_json::to_string_pretty(&events)?,
    )?;
    let native_events: Vec<_> = events
        .iter()
        .filter(|event| {
            !payload || !matches!(event["event"].as_str(), Some("user-report" | "user-result"))
        })
        .cloned()
        .collect();
    if native {
        native_apps::validate(&native_events)?;
    } else {
        output::validate(&native_events, tests, TESTS, platform_config::ACTIVE_CPUS)?;
    }

    Ok(())
}
fn walk(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for e in fs::read_dir(path)? {
        let p = e?.path();
        if p.is_dir() {
            walk(&p, out)?;
        } else {
            out.push(p);
        }
    }
    Ok(())
}
fn source_inventory() -> Result<Value> {
    let mut files = Vec::new();
    for directory in [
        "apps",
        "tests/native-apps",
        "crates/kernel",
        "crates/kernel-core",
        "crates/xtask",
        "crates/routing",
        "crates/window-compat",
        "crates/routing-demo",
    ] {
        if Path::new(directory).exists() {
            walk(Path::new(directory), &mut files)?;
        }
    }
    files.extend(
        [
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            ".cargo/config.toml",
            "assets/branding/boot-logo.txt",
        ]
        .map(PathBuf::from),
    );
    files.sort();
    let mut inventory = Vec::new();
    for file in files {
        let bytes = fs::read(&file)?;
        let path = file.to_string_lossy().replace('\\', "/");
        if let Ok(text) = std::str::from_utf8(&bytes) {
            let normalized = text.replace("\r\n", "\n");
            inventory.push(json!({"path":path,"sha256_lf":(Sha256::digest(normalized.as_bytes())).iter().map(|byte| format!("{byte:02x}")).collect::<String>()}));
        } else {
            inventory.push(json!({"path":path,"sha256":(Sha256::digest(bytes)).iter().map(|byte| format!("{byte:02x}")).collect::<String>()}));
        }
    }
    Ok(json!(inventory))
}
fn read_json(path: impl AsRef<Path>) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn asid_bench() -> Result<()> {
    const PAIRS: usize = 8;
    let sources = source_inventory()?;
    let tagged = build(false, true, None, true)?;
    let baseline = build(false, true, Some("asid-baseline"), true)?;
    let mut pairs = Vec::with_capacity(PAIRS);
    for pair in 0..PAIRS {
        let order = if pair % 2 == 0 {
            [("baseline", &baseline), ("tagged", &tagged)]
        } else {
            [("tagged", &tagged), ("baseline", &baseline)]
        };
        let mut results = serde_json::Map::new();
        for (mode, elf) in order {
            execute_quiet(elf, true, true)?;
            let path = elf.with_extension("results.json");
            let events = read_json(path)?;
            let measurement = events
                .as_array()
                .ok_or("invalid ASID QEMU event list")?
                .iter()
                .find(|event| event["event"] == "asid-measurement")
                .cloned()
                .ok_or("missing ASID measurement event")?;
            results.insert(mode.into(), measurement);
        }
        let tagged_switches = results["tagged"]["cpus"]
            .as_array()
            .ok_or("missing tagged CPU counters")?
            .iter()
            .map(|cpu| {
                cpu["switches"]
                    .as_u64()
                    .ok_or("invalid tagged switch count")
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let baseline_switches = results["baseline"]["cpus"]
            .as_array()
            .ok_or("missing baseline CPU counters")?
            .iter()
            .map(|cpu| {
                cpu["switches"]
                    .as_u64()
                    .ok_or("invalid baseline switch count")
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if tagged_switches != baseline_switches {
            return Err(format!("paired scheduler work differed in pair {}", pair + 1).into());
        }
        pairs.push(json!({"pair":pair + 1,"order":order.map(|(mode, _)| mode),"results":results}));
        println!("ASID paired QEMU run {}/{} passed", pair + 1, PAIRS);
    }
    if source_inventory()? != sources {
        return Err("source changed during ASID paired measurement; results rejected".into());
    }
    let deltas: Vec<i64> = pairs
        .iter()
        .map(|pair| {
            let baseline = pair["results"]["baseline"]["elapsed_ticks"]
                .as_i64()
                .unwrap();
            let tagged = pair["results"]["tagged"]["elapsed_ticks"].as_i64().unwrap();
            baseline - tagged
        })
        .collect();
    let mut sorted = deltas.clone();
    sorted.sort_unstable();
    let median_delta = if sorted.len() % 2 == 1 {
        sorted[sorted.len() / 2] as f64
    } else {
        (sorted[sorted.len() / 2 - 1] as f64 + sorted[sorted.len() / 2] as f64) / 2.0
    };
    let digest = (Sha256::digest(serde_json::to_vec(&sources)?))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let record = json!({
        "schema_version": 1,
        "issue": 18,
        "source_inventory_sha256": digest,
        "source_files": sources,
        "platform": {"qemu_version": String::from_utf8(Command::new(qemu()?).arg("--version").output()?.stdout)?, "accelerator": "TCG", "cpus": platform_config::ACTIVE_CPUS, "warmup_cycles": 0, "measured_cycles": 32, "pair_order": "counterbalanced", "repeats": PAIRS},
        "measurement": {"scope": "process lifecycle stress on both pinned CPUs", "units": "cntpct_el0 timer_ticks", "paired_delta_direction": "baseline minus tagged; positive favors tagged", "median_paired_delta_ticks": median_delta, "paired_deltas_ticks": deltas, "claim": "QEMU TCG comparison only; not hardware throughput or a universal performance claim"},
        "builds": {"tagged": read_json("target/kernel/dev-tests-build.json")?, "baseline": read_json("target/kernel/dev-asid-baseline-build.json")?, "tagged_run": read_json(tagged.with_extension("run.json"))?, "baseline_run": read_json(baseline.with_extension("run.json"))?},
        "pairs": pairs
    });
    fs::write(
        "research/results/issue18-asid-measurements.json",
        serde_json::to_string_pretty(&record)?,
    )?;
    println!("Retained paired ASID results: research/results/issue18-asid-measurements.json");
    Ok(())
}
fn validate_samples(measurement: &Value) -> Result<()> {
    if measurement["units"] != "timer_ticks"
        || measurement["frequency"].as_u64().is_none_or(|v| v == 0)
    {
        return Err("invalid measurement units/frequency".into());
    }
    let mut samples = measurement["samples"]
        .as_array()
        .ok_or("missing raw samples")?
        .iter()
        .map(|v| v.as_u64().ok_or("invalid sample"))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if measurement["iterations"].as_u64() != Some(samples.len() as u64)
        || measurement["warmup"].as_u64().is_none()
    {
        return Err("invalid iteration/warmup metadata".into());
    }
    let quantiles = kernel_core::quantiles(&mut samples).ok_or("empty samples")?;
    for (name, expected) in ["median", "p95", "p99"].into_iter().zip(quantiles) {
        if measurement[name].as_u64() != Some(expected) {
            return Err("quantile disagrees with raw samples".into());
        }
    }
    Ok(())
}
fn archive_measurements(label: Option<&str>, starting_sources: &Value) -> Result<()> {
    if source_inventory()? != *starting_sources {
        return Err("source changed during kernel matrix; measurement rejected".into());
    }
    let mut profiles = serde_json::Map::new();
    for profile in ["dev", "prod"] {
        let events = read_json(format!("target/kernel/{profile}-tests.results.json"))?;
        let measurement = events
            .as_array()
            .ok_or("invalid event list")?
            .iter()
            .find(|event| event["event"] == "measurement")
            .ok_or("missing real kernel measurement")?
            .clone();
        validate_samples(&measurement)?;
        for event in events.as_array().unwrap().iter().filter(|e| {
            e["event"] == "measurement"
                && e["scope"]
                    .as_str()
                    .is_some_and(|s| s.starts_with("user_copy_"))
        }) {
            validate_samples(event)?;
        }
        profiles.insert(profile.into(), json!({"measurement":measurement,"test_build":read_json(format!("target/kernel/{profile}-tests-build.json"))?,"boot_build":read_json(format!("target/kernel/{profile}-boot-build.json"))?,"run":read_json(format!("target/kernel/{profile}-tests.run.json"))?,"test_events":events}));
    }
    let revision = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let mut ipc_mutations = Vec::new();
    for profile in ["dev", "prod"] {
        for &(flag, feature, expected) in IPC_CONTROLS {
            let sidecar = read_json(format!("target/kernel/{profile}-{feature}.results.json"))?;
            let failures: Vec<_> = sidecar
                .as_array()
                .ok_or("IPC sidecar is not an event list")?
                .iter()
                .filter(|event| {
                    event["status"] == "fail"
                        && ((event["event"] == "test" && event["name"] == expected)
                            || expected.strip_prefix("reject:").is_some_and(|error| {
                                event["event"] == "ipc-reject" && event["error"] == error
                            })
                            || expected.strip_prefix("supervision:").is_some_and(|error| {
                                event["event"] == "supervision-reject" && event["error"] == error
                            }))
                })
                .cloned()
                .collect();
            if failures.is_empty() {
                return Err("IPC exact rejection missing during archival".into());
            }
            ipc_mutations.push(json!({
                "profile":profile,"flag":flag,"feature":feature,"expected":expected,
                "status":"rejected",
                "exact_rejections":failures,
                "build":read_json(format!("target/kernel/{profile}-{feature}-build.json"))?,
                "run":read_json(format!("target/kernel/{profile}-{feature}.run.json"))?
            }));
        }
    }
    let record = json!({"schema_version":1,"label":label.unwrap_or("latest"),"timestamp_unix_ms":timestamp,"git_commit":String::from_utf8(revision.stdout)?.trim(),"worktree_dirty":!dirty.stdout.is_empty(),"source_files":starting_sources,"profiles":profiles,"correctness":{"matrix":"passed","tests_per_profile":TESTS.len(),"required_control_tasks":NEGATIVE_CONTROLS.len() + (FOUNDATION_CONTROLS.len() + SCHEDULER_CONTROLS.len() + USER_COPY_CONTROLS.len() + HANDLE_CONTROLS.len() + SECURITY_CONTROLS.len() + IPC_CONTROLS.len()) * 2,"ipc_negative_controls":IPC_CONTROLS.len()*2,"coverage_kinds":matrix::plan_document()?["tasks"].as_array().unwrap().iter().fold(serde_json::Map::<String,Value>::new(), |mut counts, task| { let kind=task["coverage_kind"].as_str().unwrap(); let count=counts.get(kind).and_then(Value::as_u64).unwrap_or(0); counts.insert(kind.to_owned(), json!(count+1)); counts })},"ipc_negative_controls":ipc_mutations,"claim":"TCG timer observations; not proof of fastest algorithm or hardware throughput","method_review":"docs/architecture/implementation-review.md"});
    let text = serde_json::to_string_pretty(&record)?;
    fs::write("target/kernel/measurement.json", &text)?;
    if let Some(label) = label {
        if label.is_empty()
            || label.len() > 64
            || !label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(
                "record label must be 1..64 ASCII letters/digits/hyphens/underscores".into(),
            );
        }
        fs::create_dir_all("research/measurements/runs")?;
        let digest = (Sha256::digest(serde_json::to_vec(starting_sources)?))
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let path = format!(
            "research/measurements/runs/{timestamp}-{label}-{}.json",
            &digest[..12]
        );
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(text.as_bytes())?;
        println!("Recorded verified kernel measurements: {path}");
    }
    Ok(())
}
fn compare_measurements(baseline: &Path, candidate: &Path) -> Result<()> {
    let baseline = read_json(baseline)?;
    let candidate = read_json(candidate)?;
    if baseline["schema_version"] != 1 || candidate["schema_version"] != 1 {
        return Err("unknown measurement schema".into());
    }
    for profile in ["dev", "prod"] {
        let old = &baseline["profiles"][profile];
        let new = &candidate["profiles"][profile];
        for record in [&baseline, &candidate] {
            if record["correctness"]["matrix"] != "passed" {
                return Err("comparison requires recorded correctness checks".into());
            }
        }
        for field in [
            "qemu_version",
            "arguments",
            "active_cpus",
            "configured_cpus",
            "accelerator",
        ] {
            if old["run"][field].is_null() || old["run"][field] != new["run"][field] {
                return Err(format!("incomparable {profile} environment: {field}").into());
            }
        }
        for field in ["compiler", "target", "features"] {
            if old["test_build"][field].is_null()
                || old["test_build"][field] != new["test_build"][field]
            {
                return Err(format!("incomparable build: {field}").into());
            }
        }
        let old = &old["measurement"];
        let new = &new["measurement"];
        validate_samples(old)?;
        validate_samples(new)?;
        for field in ["scope", "units", "frequency", "warmup", "iterations"] {
            if old[field].is_null() || old[field] != new[field] {
                return Err(format!("incomparable measurement: {field}").into());
            }
        }
        let mut deltas = serde_json::Map::new();
        for quantile in ["median", "p95", "p99"] {
            let a = old[quantile].as_u64().unwrap();
            let b = new[quantile].as_u64().unwrap();
            deltas.insert(quantile.into(),json!({"baseline_ticks":a,"candidate_ticks":b,"delta_ticks":(i128::from(b)-i128::from(a)).to_string(),"delta_percent":if a==0 {None} else {Some((b as f64/a as f64-1.0)*100.0)}}));
        }
        println!(
            "{}",
            json!({"profile":profile,"baseline":baseline["label"],"candidate":candidate["label"],"observed_differences":deltas,"claim":"observations only; repeated comparative runs and reliability review required before adoption"})
        );
    }
    Ok(())
}
fn audit() -> Result<()> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let tree = Command::new(&cargo)
        .args([
            "tree",
            "--locked",
            "-p",
            "kolvrt-kernel",
            "--target",
            "aarch64-unknown-none",
            "--edges",
            "normal,build",
            "--all-features",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ])
        .output()?;
    if !tree.status.success() {
        return Err("native dependency audit failed".into());
    }
    let tree = String::from_utf8(tree.stdout)?;
    let names = tree
        .lines()
        .map(|line| line.split_whitespace().next().unwrap_or(""))
        .collect::<BTreeSet<_>>();
    if names != BTreeSet::from(["kernel-core", "kolvrt-kernel"]) {
        return Err(format!("unexpected native dependency closure: {tree}").into());
    }
    let mut files = Vec::new();
    for path in ["crates/kernel/src", "crates/kernel-core/src"] {
        walk(Path::new(path), &mut files)?;
    }
    files.sort();
    let mut locations = Vec::new();
    let mut assembly = Vec::new();
    let mut generated_wrappers = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file)?;
        enforce_native_source(&source)?;
        generated_wrappers.extend(
            source
                .lines()
                .map(str::trim)
                .filter(|line| line.starts_with("read_reg!("))
                .map(|line| format!("{})", line.split(',').next().unwrap())),
        );
        if file.extension().is_some_and(|e| e == "S") {
            assembly.push(json!({"path":file,"sha256":(Sha256::digest(source.as_bytes())).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),"review":"INV-ENTRY, INV-VECTOR, INV-PROBE, INV-USER-CONTEXT, INV-USER-IMAGE"}));
        }
        for (i, line) in source.lines().enumerate() {
            if line.contains("unsafe") && !line.trim_start().starts_with("//") {
                let context = source
                    .lines()
                    .skip(i.saturating_sub(UNSAFE_CONTEXT_PRECEDING_LINES))
                    .take(UNSAFE_CONTEXT_PRECEDING_LINES + 1)
                    .collect::<Vec<_>>()
                    .join("\n");
                locations
                    .push(json!({"path":file,"line":i+1,"text":line.trim(),"context":context}));
            }
        }
    }
    let sysroot = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()?;
    if !sysroot.status.success() {
        return Err("compiler inventory failed".into());
    }
    let sysroot = String::from_utf8(sysroot.stdout)?;
    let libraries = Path::new(sysroot.trim()).join("lib/rustlib/aarch64-unknown-none/lib");
    let mut runtime = Vec::new();
    for e in fs::read_dir(libraries)? {
        let p = e?.path();
        let name = p.file_name().unwrap().to_string_lossy();
        if p.extension().is_some_and(|e| e == "rlib")
            && ["libcore-", "liballoc-", "libcompiler_builtins-"]
                .iter()
                .any(|prefix| name.starts_with(prefix))
        {
            runtime.push(json!({"artifact":name,"sha256":(Sha256::digest(fs::read(&p)?)).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),"source_unsafe_coverage":"compiler-trusted; not individually proven"}));
        }
    }
    let report = json!({"scope":"first-party source inventory with local invariant context; assembly and compiled runtime artifacts included; not a safety proof","locations":locations,"assembly":assembly,"native_dependency_tree":tree,"compiler_runtime":runtime,"generated_wrappers":generated_wrappers,"compiler":"Rust 1.99.0","dependency_boundary":"compiler runtime and generated instructions require compiler trust; no compatibility dependency"});
    fs::write(
        "target/kernel/unsafe-audit.json",
        serde_json::to_string_pretty(&report)?,
    )?;
    println!("unsafe inventory: target/kernel/unsafe-audit.json");
    Ok(())
}

fn enforce_native_source(source: &str) -> Result<()> {
    for line in source
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
    {
        if [
            "routing::",
            "use routing",
            "extern crate routing",
            "window_compat::",
            "use window_compat",
            "window-compat",
            "feature = \"compat",
            "feature=\"compat",
            "Input::Encoded",
            "Route::Inclusive",
            "Route::Counted",
            "Route::EmptyFirst",
        ]
        .iter()
        .any(|pattern| line.contains(pattern))
        {
            return Err("native source contains compatibility import, type or conditional".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod native_architecture_tests {
    use serde_json::json;
    #[test]
    fn snapshot_and_handle_controls_require_the_expected_failure_not_a_pass_then_panic() {
        for &(flag, _, marker) in super::USER_COPY_CONTROLS
            .iter()
            .chain(super::HANDLE_CONTROLS.iter())
        {
            if flag == "--user-copy-recovery-control" {
                continue; // Recovery deliberately causes a fatal architectural exception.
            }
            let (event, name) = if flag == "--handle-retirement-control" {
                ("handle-retirement-reject", None)
            } else if flag == "--user-copy-snapshot-control" {
                ("test", Some("user_copy_el0_boundary_and_snapshot"))
            } else {
                (
                    "test",
                    Some("handle_el0_identity_type_generation_and_lifetime"),
                )
            };
            let mut expected = json!({"event":event,"status":"fail"});
            if let Some(name) = name {
                expected["name"] = json!(name);
            }
            assert!(
                expected.to_string().contains(marker),
                "missing precise failure marker: {flag}"
            );
            expected["status"] = json!("pass");
            let unrelated = format!(
                "{expected}\n{}",
                json!({"event":"panic","status":"fail","message":"unrelated"})
            );
            assert!(
                !unrelated.contains(marker),
                "pass then unrelated panic accepted: {flag}"
            );
            expected["status"] = json!("fail");
            expected["event"] = json!("unrelated");
            if name.is_some() {
                expected["name"] = json!("unrelated_test");
            }
            assert!(
                !expected.to_string().contains(marker),
                "unrelated failure accepted: {flag}"
            );
        }
    }
    #[test]
    fn reject_legacy_types_imports_and_conditionals() {
        for code in [
            "use routing as r;",
            "extern crate routing;",
            "fn run() { window_compat::v1(&[], &[]); }",
            "#[cfg(feature = \"compat-v1\")] fn special() {}",
            "let input = Input::Encoded(bytes);",
        ] {
            assert!(super::enforce_native_source(code).is_err(), "{code}");
        }
        assert!(
            super::enforce_native_source(
                "// routing:: is outside native core\nuse kernel_core::window::Span;"
            )
            .is_ok()
        );
    }
}
