#![forbid(unsafe_code)]
use ed25519_dalek::{Signer, SigningKey};
use migration_advisor::{
    Capability, Package, Requirement, SemanticVersion, advisor::*, plan_digest, provenance::*,
    solver::Plan, statistics::StatisticalPolicy,
};
use migration_workbench::{CONTRACT, ITERATIONS, PROTOCOL, Receipt};
use rand_core::{OsRng, RngCore};
use serde::Serialize;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const PAIRS: usize = 100;
const PILOT_PAIRS: usize = 100;
const EXPECTED_MEAN_GAIN_NS: u64 = 25_000;
const CONSUMER_ID: &str = "fixture-window-consumer";
const CHILD_TIMEOUT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(10);
const MAX_RECEIPT_BYTES: u64 = 32768;

fn store(root: &Path, bytes: &[u8]) -> Result<String, String> {
    let digest = subject_digest(bytes);
    fs::write(root.join(&digest), bytes).map_err(|e| e.to_string())?;
    Ok(digest)
}
fn store_json<T: Serialize>(root: &Path, value: &T) -> Result<String, String> {
    store(root, &serde_json::to_vec(value).map_err(|e| e.to_string())?)
}
fn execute(path: &Path, mode: &str, log: &Path) -> Result<Receipt, String> {
    let mut child = Command::new(path)
        .arg(mode)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let begin = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if begin.elapsed() >= CHILD_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            fs::write(log, b"timed-out fixture; experiment aborted").map_err(|e| e.to_string())?;
            return Err("fixture timeout; retained failure; no benchmark result".into());
        }
        std::thread::sleep(POLL);
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .take(MAX_RECEIPT_BYTES + 1)
        .read_to_end(&mut stdout)
        .map_err(|e| e.to_string())?;
    child
        .stderr
        .take()
        .unwrap()
        .take(MAX_RECEIPT_BYTES + 1)
        .read_to_end(&mut stderr)
        .map_err(|e| e.to_string())?;
    fs::write(log, &stdout).map_err(|e| e.to_string())?;
    fs::write(log.with_extension("stderr"), &stderr).map_err(|e| e.to_string())?;
    if !status.success() || stdout.len() as u64 > MAX_RECEIPT_BYTES || !stderr.is_empty() {
        return Err("fixture failed; raw evidence retained".into());
    }
    let receipt: Receipt = serde_json::from_slice(&stdout).map_err(|e| e.to_string())?;
    if !receipt.passed || receipt.checks < 12 {
        return Err("fixture contracts failed".into());
    }
    if mode == "measure"
        && (receipt.iterations != ITERATIONS
            || receipt.checksum != migration_workbench::expected_checksum()
            || receipt.elapsed_ns == 0
            || receipt.request_latency_samples_ns.len()
                != ((ITERATIONS - 1) / migration_workbench::REQUEST_LATENCY_SAMPLE_STRIDE + 1)
                    as usize
            || receipt.request_latency_samples_ns.contains(&0)
            || u128::from(receipt.native_admissions) + u128::from(receipt.compat_admissions)
                != u128::from(ITERATIONS))
    {
        return Err("fixture oracle or accounting failed".into());
    }
    Ok(receipt)
}
/// Local fixture-only signer. Provisioned producer private keys are never loaded
/// by the advisor or workbench; production evidence enters through external policy.
struct EphemeralFixtureSigner(SigningKey);
impl EphemeralFixtureSigner {
    fn generate() -> Self {
        let mut secret = [0_u8; 32];
        OsRng.fill_bytes(&mut secret);
        Self(SigningKey::from_bytes(&secret))
    }
    fn attest<T: Serialize>(&self, role: Role, payload: &T) -> Attestation {
        let digest = subject_digest(&serde_json::to_vec(payload).unwrap());
        let signature = self.0.sign(&signing_message(role, &digest));
        Attestation {
            role,
            subject_digest: digest,
            key_id: "ephemeral-local-experiment".into(),
            signature: hex(&signature.to_bytes()),
            session: None,
            issued_at_unix_seconds: None,
            session_id: None,
        }
    }
    fn public_key(&self) -> String {
        hex(&self.0.verifying_key().to_bytes())
    }
}
fn measure_pair(
    binaries: &[PathBuf],
    root: &Path,
    series: &str,
    index: usize,
) -> Result<(Receipt, Receipt, bool), String> {
    let first = if index.is_multiple_of(2) { 0 } else { 1 };
    let second = 1 - first;
    let a = execute(
        &binaries[first],
        "measure",
        &root.join(format!("{series}-{index:03}-{first}.json")),
    )?;
    let b = execute(
        &binaries[second],
        "measure",
        &root.join(format!("{series}-{index:03}-{second}.json")),
    )?;
    let (baseline, candidate) = if first == 0 { (a, b) } else { (b, a) };
    if baseline.mode != "compat"
        || candidate.mode != "native"
        || baseline.compat_admissions != ITERATIONS
        || candidate.native_admissions != ITERATIONS
        || baseline.copied_bytes != ITERATIONS * 4
        || candidate.copied_bytes != 0
        || baseline.peak_memory_bytes.bytes().is_none()
        || candidate.peak_memory_bytes.bytes().is_none()
    {
        return Err("fixture route or resource accounting mismatch".into());
    }
    Ok((baseline, candidate, first == 0))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn run() -> Result<PathBuf, String> {
    if std::env::args().skip(1).collect::<Vec<_>>() != ["run"] {
        return Err("usage: migration-workbench run; first build --release --bins".into());
    }
    let directory = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .unwrap()
        .to_path_buf();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    let root = PathBuf::from(format!(
        "target/migration-workbench/{stamp}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let artifacts = root.join("artifacts");
    fs::create_dir(&artifacts).map_err(|e| e.to_string())?;
    let suffix = std::env::consts::EXE_SUFFIX;
    let lifecycle_evidence =
        execute_lifecycle(&directory.join(format!("lifecycle-fixture{suffix}")), &root)?;
    let lifecycle_digest = store_json(&artifacts, &lifecycle_evidence)?;
    fs::write(
        root.join("lifecycle-evidence.json"),
        serde_json::to_vec_pretty(&lifecycle_evidence).unwrap(),
    )
    .map_err(|error| error.to_string())?;
    let mut binaries = Vec::new();
    let mut identities = Vec::new();
    for name in ["compat-fixture", "native-fixture"] {
        let bytes = fs::read(directory.join(format!("{name}{suffix}")))
            .map_err(|e| format!("build fixture binaries first: {e}"))?;
        let digest = store(&artifacts, &bytes)?;
        let path = root.join(format!("{name}{suffix}"));
        fs::write(&path, &bytes).map_err(|e| e.to_string())?;
        binaries.push(path);
        identities.push(digest);
    }
    if identities[0] == identities[1] {
        return Err("fixture executables unexpectedly have identical identities".into());
    }
    let suite = store(
        &artifacts,
        &fs::read("crates/routing/src/conformance.rs").map_err(|e| e.to_string())?,
    )?;
    let contract = store(&artifacts, CONTRACT.as_bytes())?;
    let workload = store_json(&artifacts, &(0..256u32).collect::<Vec<_>>())?;
    let protocol = store(&artifacts, PROTOCOL.as_bytes())?;
    let system = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let kernel_bytes = fs::read(Path::new(&system).join("System32/ntoskrnl.exe"))
        .map_err(|e| format!("Windows host kernel identity unavailable: {e}"))?;
    let kernel = store(&artifacts, &kernel_bytes)?;
    let context = Context {
        machine: format!(
            "physical-host/{}/{}/{}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::env::var("COMPUTERNAME").unwrap_or_else(|_| "local".into())
        ),
        workload,
        kernel,
        protocol,
        source: EvidenceSource::HostProcess,
    };
    let mut contracts = Vec::new();
    for (i, path) in binaries.iter().enumerate() {
        execute(path, "verify", &root.join(format!("contract-{i}.json")))?;
        contracts.push(ContractTest {
            package: identities[i].clone(),
            contract: contract.clone(),
            suite: suite.clone(),
            execution_receipt: (i == 1).then(|| lifecycle_digest.clone()),
            execution_artifacts: if i == 1 {
                vec![
                    lifecycle_evidence.worker_digest.clone(),
                    lifecycle_evidence.snapshot_digest.clone(),
                    lifecycle_evidence.invalid_migration_stderr_digest.clone(),
                    lifecycle_evidence.invalid_path_stderr_digest.clone(),
                ]
            } else {
                vec![]
            },
            context: context.clone(),
            passed: true,
        });
    }
    let improvement_margin_ns = 1000;
    let mut pilot_raw = Vec::new();
    let mut pilot_gains_ns = Vec::new();
    for i in 0..PILOT_PAIRS {
        let (baseline, candidate, _) = measure_pair(&binaries, &root, "power-pilot", i)?;
        let gain = i128::from(baseline.elapsed_ns) - i128::from(candidate.elapsed_ns);
        pilot_gains_ns.push(i64::try_from(gain).map_err(|_| "pilot gain overflow")?);
        pilot_raw.push((baseline, candidate));
        if (i + 1) % 10 == 0 {
            eprintln!(
                "[INFO] migration-workbench: retained {} independent power-pilot pairs",
                i + 1
            );
        }
    }
    let power_pilot_artifact = store_json(&artifacts, &pilot_raw)?;
    let pilot_mean_gain =
        pilot_gains_ns.iter().map(|gain| *gain as f64).sum::<f64>() / pilot_gains_ns.len() as f64;
    let pilot_variance = pilot_gains_ns
        .iter()
        .map(|gain| (*gain as f64 - pilot_mean_gain).powi(2))
        .sum::<f64>()
        / (pilot_gains_ns.len() - 1) as f64;
    let power_pilot = (pilot_variance.is_finite() && pilot_variance > 0.0).then(|| {
        migration_advisor::advisor::PowerPilot {
            artifact: power_pilot_artifact,
            context: context.clone(),
            paired_gains_ns: pilot_gains_ns,
        }
    });
    let prospective_power =
        power_pilot.as_ref().map(
            |pilot| migration_advisor::statistics::ProspectivePowerPlan {
                pilot_digest: subject_digest(&serde_json::to_vec(pilot).unwrap()),
                expected_mean_gain_ns: EXPECTED_MEAN_GAIN_NS,
                target_power_basis_points: 9000,
                planned_effective_blocks: PAIRS,
            },
        );
    let statistics = StatisticalPolicy {
        min_resampling_blocks: PAIRS,
        resampling_block_length: 1,
        bootstrap_resamples: 2000,
        confidence_basis_points: 9500,
        seed: 7,
        improvement_margin_ns,
        p95_regression_budget_ns: 100000,
        p99_regression_budget_ns: None,
        p95_memory_regression_budget_bytes: Some(0),
        p95_copied_bytes_regression_budget: Some(0),
        p95_energy_regression_budget_uj: None,
        consumer_resource_budgets: vec![migration_advisor::statistics::ConsumerResourceBudget {
            consumer_id: CONSUMER_ID.into(),
            p95_memory_regression_budget_bytes: Some(0),
            p95_copied_bytes_regression_budget: Some(0),
            p95_energy_regression_budget_uj: None,
        }],
        prospective_power,
    };
    let observed = Instant::now();
    let mut pairs = Vec::new();
    let mut raw = Vec::new();
    for i in 0..PAIRS {
        let (baseline, candidate, baseline_first) =
            measure_pair(&binaries, &root, "measurement", i)?;
        pairs.push(Pair {
            consumer_id: Some(CONSUMER_ID.into()),
            baseline_ns: baseline.elapsed_ns,
            candidate_ns: candidate.elapsed_ns,
            request_latency: Some(migration_advisor::advisor::RequestLatencySamples {
                sample_stride: migration_workbench::REQUEST_LATENCY_SAMPLE_STRIDE,
                sample_width: migration_workbench::REQUEST_LATENCY_SAMPLE_WIDTH,
                baseline_ns: baseline.request_latency_samples_ns.clone(),
                candidate_ns: candidate.request_latency_samples_ns.clone(),
            }),
            baseline_memory_bytes: baseline.peak_memory_bytes.bytes(),
            candidate_memory_bytes: candidate.peak_memory_bytes.bytes(),
            baseline_copied_bytes: Some(baseline.copied_bytes),
            candidate_copied_bytes: Some(candidate.copied_bytes),
            baseline_energy_uj: None,
            candidate_energy_uj: None,
            baseline_compat_admissions: baseline.compat_admissions,
            candidate_compat_admissions: candidate.compat_admissions,
            useful_units: ITERATIONS,
            baseline_first,
            oracle_passed: true,
        });
        raw.push((baseline, candidate));
        if (i + 1) % 10 == 0 {
            eprintln!("[INFO] migration-workbench: retained {} paired runs", i + 1);
        }
    }
    let elapsed: u64 = observed
        .elapsed()
        .as_nanos()
        .try_into()
        .map_err(|_| "observation clock overflow")?;
    // Isolated rollback: relaunch the retained old executable, check its contracts and
    // original data checksum. This workload has no persistent state to migrate.
    let rollback = execute(&binaries[0], "measure", &root.join("rollback.json"))?;
    if rollback.compat_admissions != ITERATIONS {
        return Err("rollback route was not restored".into());
    }
    for (i, path) in binaries.iter().enumerate() {
        if subject_digest(&fs::read(path).map_err(|e| e.to_string())?) != identities[i] {
            return Err("executable changed during experiment".into());
        }
    }
    let version = SemanticVersion {
        major: 1,
        minor: 0,
        patch: 0,
    };
    let catalog: Vec<_> = identities
        .iter()
        .enumerate()
        .map(|(i, id)| Package {
            name: if i == 0 {
                "window-compat-fixture"
            } else {
                "window-native-fixture"
            }
            .into(),
            version: 1,
            identity: id.clone(),
            variant: None,
            provides: vec![Capability {
                capability_id: "workload.window.reduce".into(),
                semantic_version: version,
                contract_digest: contract.clone(),
            }],
            requires: vec![],
            conflicts: vec![],
        })
        .collect();
    let installed = Plan {
        packages: vec![identities[0].clone()],
    };
    let candidate_plan = Plan {
        packages: vec![identities[1].clone()],
    };
    let baseline_digest = plan_digest(&installed);
    let candidate_digest = plan_digest(&candidate_plan);
    let runtime = RuntimeEvidence {
        artifact: store_json(&artifacts, &raw)?,
        package: identities[0].clone(),
        context: context.clone(),
        window_start_ns: 0,
        window_end_ns: elapsed,
        complete: true,
        lost_events: 0,
        routes: vec![RouteEvidence {
            route: RouteName::COMPAT.into(),
            adapter: Some(identities[0].clone()),
            generation: 1,
            native_admissions: 0,
            compat_admissions: PAIRS as u64 * ITERATIONS,
            failed: 0,
            adapter_cpu_ns: None,
            copied_bytes: Some(raw.iter().map(|(a, _)| a.copied_bytes).sum()),
        }],
    };
    let benchmark = Benchmark {
        artifact: store_json(&artifacts, &raw)?,
        baseline_plan: baseline_digest.clone(),
        candidate_plan: candidate_digest.clone(),
        context: context.clone(),
        statistics: statistics.clone(),
        stabilized: raw.iter().all(|(a, b)| a.stabilized && b.stabilized),
        failures: 0,
        timeouts: 0,
        unfinished: 0,
        pairs,
    };
    let migration=MigrationPlan {candidate:identities[1].clone(),plan_digest:candidate_digest,context:context.clone(),required_changes:vec!["change client from LE16 inclusive encoding to native half-open Span in a separately authorized next process launch".into(), "migrate persistent fixture state from schema v1 to schema v2 and verify the candidate restart".into()],
        persistent_data_change:true,rollback_strategy:RollbackStrategy::RestoreSnapshot {snapshot_digest:lifecycle_evidence.snapshot_digest.clone()},rollback_execution_receipt:Some(lifecycle_digest.clone()),rollback_preconditions:vec![format!("the candidate lifecycle receipt {lifecycle_digest} records a successful v1-to-v2 restart and byte-identical snapshot restoration before authorization")],irreversible_changes:vec![]};
    let signer = EphemeralFixtureSigner::generate();
    let mut attestations = Vec::new();
    for p in &catalog {
        attestations.push(signer.attest(Role::Catalog, p));
    }
    for t in &contracts {
        attestations.push(signer.attest(Role::ContractTest, t));
    }
    if let Some(pilot) = &power_pilot {
        attestations.push(signer.attest(Role::Benchmark, pilot));
    }
    attestations.push(signer.attest(Role::Runtime, &runtime));
    attestations.push(signer.attest(Role::Benchmark, &benchmark));
    attestations.push(signer.attest(Role::Proposal, &migration));
    let request = Request {
        schema: migration_advisor::SCHEMA,
        catalog,
        current: identities[0].clone(),
        installed,
        requirements: vec![vec![Requirement {
            capability_id: "workload.window.reduce".into(),
            min_version: version,
            max_version: version,
            contract_digest: contract,
        }]],
        retained: vec![],
        context,
        runtime: Some(runtime),
        tests: contracts,
        benchmarks: vec![benchmark],
        statistics,
        power_pilot,
        attestations,
        session: None,
        migration_plans: vec![migration],
        scoped_resolutions: None,
    };
    let policy = TrustPolicy {
        schema: 1,
        keys: vec![TrustedKey {
            key_id: "ephemeral-local-experiment".into(),
            public_key: signer.public_key(),
            roles: vec![
                Role::Catalog,
                Role::Runtime,
                Role::ContractTest,
                Role::Benchmark,
                Role::Proposal,
            ],
            revoked: false,
            compromised: false,
            producer: None,
            valid_from_unix: None,
            valid_until_unix: None,
        }],
        session: None,
        independence: vec![],
        required_session_id: None,
        max_attestation_age_seconds: None,
        max_clock_skew_seconds: None,
    };
    fs::write(
        root.join("request.json"),
        serde_json::to_vec_pretty(&request).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        root.join("trust.json"),
        serde_json::to_vec_pretty(&policy).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    let verifier = SignedArtifactStore::new(policy, artifacts)?;
    let exploratory =
        migration_advisor::statistics::analyze(&request.benchmarks[0].pairs, &request.statistics)?;
    fs::write(
        root.join("descriptive-statistics.json"),
        serde_json::to_vec_pretty(&exploratory).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    let report = advise_with_verifier(
        &request,
        &migration_advisor::solver::ClosureSolver { max_states: 1024 },
        &verifier,
    )?;
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    Ok(root)
}
struct RouteName;
impl RouteName {
    const COMPAT: &'static str = "host.window.inclusive";
}

#[derive(Serialize)]
struct LifecycleEvidence {
    schema: u32,
    scenario: String,
    worker_digest: String,
    worker_receipts: Vec<serde_json::Value>,
    snapshot_digest: String,
    restored_state_digest: String,
    invalid_migration_stderr_digest: String,
    invalid_path_stderr_digest: String,
    failed_migration_rejected: bool,
    invalid_state_path_rejected: bool,
}

fn lifecycle_step(
    worker: &Path,
    operation: &str,
    state_dir: &Path,
    log: &Path,
) -> Result<(bool, Option<serde_json::Value>), String> {
    let mut child = Command::new(worker)
        .arg(operation)
        .arg(state_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            break status;
        }
        if started.elapsed() >= CHILD_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            fs::write(log, b"timed-out lifecycle worker; experiment aborted")
                .map_err(|error| error.to_string())?;
            return Err(format!("lifecycle worker timed out in {operation}"));
        }
        std::thread::sleep(POLL);
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .take(MAX_RECEIPT_BYTES + 1)
        .read_to_end(&mut stdout)
        .map_err(|error| error.to_string())?;
    child
        .stderr
        .take()
        .unwrap()
        .take(MAX_RECEIPT_BYTES + 1)
        .read_to_end(&mut stderr)
        .map_err(|error| error.to_string())?;
    fs::write(log, &stdout).map_err(|error| error.to_string())?;
    fs::write(log.with_extension("stderr"), &stderr).map_err(|error| error.to_string())?;
    if stdout.len() as u64 > MAX_RECEIPT_BYTES {
        return Err("lifecycle worker output exceeded its limit".into());
    }
    if !status.success() {
        return Ok((false, None));
    }
    if !stderr.is_empty() {
        return Err("successful lifecycle worker wrote to stderr".into());
    }
    let receipt: serde_json::Value =
        serde_json::from_slice(&stdout).map_err(|error| error.to_string())?;
    if receipt["passed"] != true || receipt["operation"] != operation {
        return Err(format!(
            "lifecycle receipt failed validation for {operation}"
        ));
    }
    Ok((true, Some(receipt)))
}

fn execute_lifecycle(worker: &Path, root: &Path) -> Result<LifecycleEvidence, String> {
    let run_dir = root.join("lifecycle");
    let state_dir = run_dir.join("state");
    let snapshot = run_dir.join("snapshot-v1.json");
    let worker_bytes =
        fs::read(worker).map_err(|error| format!("lifecycle worker unavailable: {error}"))?;
    let worker_digest = store(&root.join("artifacts"), &worker_bytes)?;
    fs::create_dir_all(&state_dir).map_err(|error| error.to_string())?;
    let mut receipts = Vec::new();
    for (operation, name) in [
        ("seed", "seed"),
        ("verify-v1", "startup-v1"),
        ("migrate", "migrate-v1-to-v2"),
        ("verify-v2", "restart-v2"),
    ] {
        let (passed, receipt) = lifecycle_step(
            worker,
            operation,
            &state_dir,
            &run_dir.join(format!("{name}.json")),
        )?;
        if !passed {
            return Err(format!("lifecycle step {name} failed"));
        }
        receipts.push(receipt.unwrap());
        if operation == "verify-v1" {
            fs::copy(state_dir.join("state-v1.json"), &snapshot)
                .map_err(|error| error.to_string())?;
        }
    }
    let snapshot_digest = store(
        &root.join("artifacts"),
        &fs::read(&snapshot).map_err(|error| error.to_string())?,
    )?;

    fs::remove_dir_all(&state_dir).map_err(|error| error.to_string())?;
    fs::create_dir_all(&state_dir).map_err(|error| error.to_string())?;
    fs::copy(&snapshot, state_dir.join("state-v1.json")).map_err(|error| error.to_string())?;
    let (restored, receipt) = lifecycle_step(
        worker,
        "verify-v1",
        &state_dir,
        &run_dir.join("rollback-verify.json"),
    )?;
    if !restored {
        return Err("restored state failed the old application restart check".into());
    }
    let rollback_receipt = receipt.unwrap();
    let restored_state_digest = rollback_receipt["state_digest"]
        .as_str()
        .ok_or("rollback receipt has no state digest")?
        .to_owned();
    if restored_state_digest != snapshot_digest {
        return Err("restored state differs from the retained snapshot".into());
    }
    receipts.push(rollback_receipt);

    let invalid = run_dir.join("invalid-state");
    fs::create_dir_all(&invalid).map_err(|error| error.to_string())?;
    fs::write(
        invalid.join("state-v1.json"),
        b"{\"schema\":1,\"records\":[999]}",
    )
    .map_err(|error| error.to_string())?;
    let (failed_migration_rejected, _) = lifecycle_step(
        worker,
        "migrate",
        &invalid,
        &run_dir.join("invalid-migration.json"),
    )?;
    if failed_migration_rejected || invalid.join("state-v2.json").exists() {
        return Err("invalid persistent state was accepted by the migrator".into());
    }
    let invalid_migration_stderr =
        fs::read(run_dir.join("invalid-migration.stderr")).map_err(|error| error.to_string())?;
    if invalid_migration_stderr.is_empty() {
        return Err("invalid migration rejection did not retain an error message".into());
    }
    let invalid_migration_stderr_digest =
        store(&root.join("artifacts"), &invalid_migration_stderr)?;

    let invalid_path = run_dir.join("not-a-directory");
    fs::write(&invalid_path, b"file").map_err(|error| error.to_string())?;
    let (invalid_state_path_rejected, _) = lifecycle_step(
        worker,
        "seed",
        &invalid_path,
        &run_dir.join("invalid-path.json"),
    )?;
    if invalid_state_path_rejected {
        return Err("invalid state path was not rejected".into());
    }
    let invalid_path_stderr =
        fs::read(run_dir.join("invalid-path.stderr")).map_err(|error| error.to_string())?;
    if invalid_path_stderr.is_empty() {
        return Err("invalid state path rejection did not retain an error message".into());
    }
    let invalid_path_stderr_digest = store(&root.join("artifacts"), &invalid_path_stderr)?;
    let worker_after = fs::read(worker).map_err(|error| error.to_string())?;
    if subject_digest(&worker_after) != worker_digest {
        return Err("lifecycle worker changed while executing the scenario".into());
    }
    Ok(LifecycleEvidence {
        schema: 1,
        scenario: "separate-process persistent-state v1-to-v2 migration, restart, invalid-input rejection, and snapshot rollback".into(),
        worker_digest,
        worker_receipts: receipts,
        snapshot_digest,
        restored_state_digest,
        invalid_migration_stderr_digest,
        invalid_path_stderr_digest,
        failed_migration_rejected: true,
        invalid_state_path_rejected: true,
    })
}

fn main() {
    match run() {
        Ok(root) => println!(
            "[OK] migration-workbench: host library experiment retained at {}; not a KOLVRT/ARM64 deployment recommendation",
            root.display()
        ),
        Err(e) => {
            eprintln!("[FAIL] migration-workbench: {e}");
            std::process::exit(1);
        }
    }
}
