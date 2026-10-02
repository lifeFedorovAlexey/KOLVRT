use migration_advisor::{
    advisor::{Request, advise, advise_with_verifier},
    provenance::{SignedArtifactStore, TrustPolicy},
    solver::ExhaustiveSolver,
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
    if !matches!(args.len(), 2 | 6) || !matches!(args[0].as_str(), "advise" | "inspect-routing") {
        return Err("usage: migration-advisor advise REQUEST.json (JSON report on stdout)".into());
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
        if args.len() != 2 {
            return Err("usage: inspect-routing ROUTING-RESULTS.json".into());
        }
        let report = migration_advisor::routing_evidence::inspect(&bytes)?;
        let stdout = io::stdout();
        let mut output = stdout.lock();
        serde_json::to_writer_pretty(&mut output, &report).map_err(|e| e.to_string())?;
        return writeln!(output).map_err(|e| e.to_string());
    }
    let request: Request = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let solver = ExhaustiveSolver {
        max_states: MAX_SOLVER_STATES,
    };
    let report = if args.len() == 6 {
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
        let verifier = SignedArtifactStore::new(policy, args[5].clone().into())?;
        advise_with_verifier(&request, &solver, &verifier)?
    } else {
        advise(&request, &solver)?
    };
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer_pretty(&mut output, &report).map_err(|e| e.to_string())?;
    writeln!(output).map_err(|e| e.to_string())
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
