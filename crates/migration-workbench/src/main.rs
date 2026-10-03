#![forbid(unsafe_code)]
use ed25519_dalek::{Signer, SigningKey};
use migration_advisor::{
    Capability, Package, Requirement, SemanticVersion, advisor::*, plan_digest, provenance::*,
    solver::Plan, statistics::StatisticalPolicy,
};
use migration_workbench::{CONTRACT, ITERATIONS, PROTOCOL, Receipt};
use rand_core::OsRng;
use serde::Serialize;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const PAIRS: usize = 100;
const CHILD_TIMEOUT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(10);
const MAX_RECEIPT_BYTES: u64 = 8192;

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
            || u128::from(receipt.native_admissions) + u128::from(receipt.compat_admissions)
                != u128::from(ITERATIONS))
    {
        return Err("fixture oracle or accounting failed".into());
    }
    Ok(receipt)
}
fn attest<T: Serialize>(key: &SigningKey, role: Role, payload: &T) -> Attestation {
    let digest = subject_digest(&serde_json::to_vec(payload).unwrap());
    let signature = key.sign(&signing_message(role, &digest));
    Attestation {
        role,
        subject_digest: digest,
        key_id: "ephemeral-local-experiment".into(),
        signature: hex(&signature.to_bytes()),
    }
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
            context: context.clone(),
            passed: true,
        });
    }
    let statistics = StatisticalPolicy {
        min_resampling_blocks: PAIRS,
        resampling_block_length: 1,
        bootstrap_resamples: 2000,
        confidence_basis_points: 9500,
        seed: 7,
        improvement_margin_ns: 1000,
        p95_regression_budget_ns: 100000,
        p99_regression_budget_ns: None,
    };
    let observed = Instant::now();
    let mut pairs = Vec::new();
    let mut raw = Vec::new();
    for i in 0..PAIRS {
        let first = if i % 2 == 0 { 0 } else { 1 };
        let second = 1 - first;
        let a = execute(
            &binaries[first],
            "measure",
            &root.join(format!("pair-{i:03}-{first}.json")),
        )?;
        let b = execute(
            &binaries[second],
            "measure",
            &root.join(format!("pair-{i:03}-{second}.json")),
        )?;
        let (baseline, candidate) = if first == 0 { (a, b) } else { (b, a) };
        if baseline.mode != "compat"
            || candidate.mode != "native"
            || baseline.compat_admissions != ITERATIONS
            || candidate.native_admissions != ITERATIONS
        {
            return Err("fixture route mismatch".into());
        }
        pairs.push(Pair {
            baseline_ns: baseline.elapsed_ns,
            candidate_ns: candidate.elapsed_ns,
            baseline_compat_admissions: baseline.compat_admissions,
            candidate_compat_admissions: candidate.compat_admissions,
            useful_units: ITERATIONS,
            baseline_first: first == 0,
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
    let migration=MigrationPlan {candidate:identities[1].clone(),plan_digest:candidate_digest,context:context.clone(),required_changes:vec!["change client from LE16 inclusive encoding to native half-open Span in a separately authorized next process launch".into()],
        persistent_data_change:false,rollback_strategy:RollbackStrategy::ReinstallPrevious {previous_plan:baseline_digest},rollback_preconditions:vec!["retained old executable hash and immutable dataset match; rollback process contract/oracle check passed".into()],irreversible_changes:vec![]};
    let key = SigningKey::generate(&mut OsRng);
    let mut attestations = Vec::new();
    for p in &catalog {
        attestations.push(attest(&key, Role::Catalog, p));
    }
    for t in &contracts {
        attestations.push(attest(&key, Role::ContractTest, t));
    }
    attestations.push(attest(&key, Role::Runtime, &runtime));
    attestations.push(attest(&key, Role::Benchmark, &benchmark));
    attestations.push(attest(&key, Role::Proposal, &migration));
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
        attestations,
        migration_plans: vec![migration],
    };
    let policy = TrustPolicy {
        schema: 1,
        keys: vec![TrustedKey {
            key_id: "ephemeral-local-experiment".into(),
            public_key: hex(&key.verifying_key().to_bytes()),
            roles: vec![
                Role::Catalog,
                Role::Runtime,
                Role::ContractTest,
                Role::Benchmark,
                Role::Proposal,
            ],
            revoked: false,
        }],
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
        &migration_advisor::solver::ExhaustiveSolver { max_states: 1024 },
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
