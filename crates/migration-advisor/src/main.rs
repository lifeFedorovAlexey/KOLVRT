use migration_advisor::{
    advisor::{Request, advise, advise_with_verifier},
    authorization::{SignedDeploymentAuthorization, authorize},
    provenance::{SignedArtifactStore, TrustPolicy},
    solver::ClosureSolver,
};
use std::{
    env, fs,
    io::{self, Write},
    process::ExitCode,
};

const MAX_INPUT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SOLVER_STATES: u64 = 1 << 19;

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    let valid_args = match args.first().map(String::as_str) {
        Some("advise") => matches!(args.len(), 2 | 6 | 8),
        Some("inspect-routing") => args.len() == 2,
        Some("authorize") => matches!(args.len(), 7 | 9),
        _ => false,
    };
    if !valid_args {
        return Err("usage: migration-advisor advise REQUEST.json [--trust-policy TRUST.json --artifact-root DIR] | inspect-routing ROUTING-RESULTS.json | authorize REQUEST.json AUTHORIZATION.json --trust-policy TRUST.json --artifact-root DIR".into());
    }
    let file = fs::File::open(&args[1]).map_err(|e| e.to_string())?;
    use std::io::Read;
    let mut bytes = Vec::new();
    file.take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("request exceeds 4 MiB".into());
    }
    if args[0] == "inspect-routing" {
        let report = migration_advisor::routing_evidence::inspect(&bytes)?;
        let stdout = io::stdout();
        let mut output = stdout.lock();
        serde_json::to_writer_pretty(&mut output, &report).map_err(|e| e.to_string())?;
        return writeln!(output).map_err(|e| e.to_string());
    }
    let request: Request = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let solver = ClosureSolver {
        max_states: MAX_SOLVER_STATES,
    };
    if args[0] == "authorize" {
        if args[3] != "--trust-policy" || args[5] != "--artifact-root" {
            return Err("usage: authorize REQUEST.json AUTHORIZATION.json --trust-policy TRUST.json --artifact-root DIR".into());
        }
        let auth_file = fs::File::open(&args[2]).map_err(|e| e.to_string())?;
        let mut auth_bytes = Vec::new();
        auth_file
            .take(MAX_INPUT_BYTES + 1)
            .read_to_end(&mut auth_bytes)
            .map_err(|e| e.to_string())?;
        if auth_bytes.len() as u64 > MAX_INPUT_BYTES {
            return Err("deployment authorization exceeds 4 MiB".into());
        }
        let signed: SignedDeploymentAuthorization =
            serde_json::from_slice(&auth_bytes).map_err(|e| e.to_string())?;
        let policy_file = fs::File::open(&args[4]).map_err(|e| e.to_string())?;
        let mut policy_bytes = Vec::new();
        policy_file
            .take(MAX_INPUT_BYTES + 1)
            .read_to_end(&mut policy_bytes)
            .map_err(|e| e.to_string())?;
        if policy_bytes.len() as u64 > MAX_INPUT_BYTES {
            return Err("trust policy exceeds 4 MiB".into());
        }
        let policy: TrustPolicy =
            serde_json::from_slice(&policy_bytes).map_err(|e| e.to_string())?;
        let verifier = configured_verifier(policy, &args[6], &args[7..])?;
        let receipt = authorize(&request, &signed, &solver, &verifier)?;
        let stdout = io::stdout();
        let mut output = stdout.lock();
        serde_json::to_writer_pretty(&mut output, &receipt).map_err(|e| e.to_string())?;
        return writeln!(output).map_err(|e| e.to_string());
    }
    let report = if args.len() >= 6 {
        if args[2] != "--trust-policy" || args[4] != "--artifact-root" {
            return Err(
                "usage: advise REQUEST.json --trust-policy TRUST.json --artifact-root DIR".into(),
            );
        }
        let file = fs::File::open(&args[3]).map_err(|e| e.to_string())?;
        let mut policy_bytes = Vec::new();
        file.take(MAX_INPUT_BYTES + 1)
            .read_to_end(&mut policy_bytes)
            .map_err(|e| e.to_string())?;
        if policy_bytes.len() as u64 > MAX_INPUT_BYTES {
            return Err("trust policy exceeds 4 MiB".into());
        }
        let policy: TrustPolicy =
            serde_json::from_slice(&policy_bytes).map_err(|e| e.to_string())?;
        let verifier = configured_verifier(policy, &args[5], &args[6..])?;
        advise_with_verifier(&request, &solver, &verifier)?
    } else {
        advise(&request, &solver)?
    };
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer_pretty(&mut output, &report).map_err(|e| e.to_string())?;
    writeln!(output).map_err(|e| e.to_string())
}

fn configured_verifier(
    policy: TrustPolicy,
    artifact_root: &str,
    session_args: &[String],
) -> Result<SignedArtifactStore, String> {
    match (policy.schema, session_args) {
        (1, []) => SignedArtifactStore::new(policy, artifact_root.into()),
        (2, [flag, ledger]) if flag == "--session-ledger" => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| "system UTC clock is before UNIX epoch")?
                .as_secs();
            SignedArtifactStore::provisioned(policy, artifact_root.into(), ledger.into(), now)
        }
        _ => Err("schema 1 uses fixture policy; schema 2 requires --session-ledger DIR".into()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("[FAIL] migration-advisor: {e}");
            ExitCode::FAILURE
        }
    }
}
