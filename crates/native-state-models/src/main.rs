#![forbid(unsafe_code)]
use native_protocol_model::{HEADER, MAX_PAYLOAD, OPERATION_SEND, request_fields, response_fields};
use native_state_models::{Mutation, binding, lifetime, startup, wait};
use serde_json::{Value, json};
use std::{
    env, fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const WORKER_TIMEOUT: Duration = Duration::from_secs(5);
const WORKER_POLL_INTERVAL: Duration = Duration::from_millis(2);

const EXPERIMENT_PAYLOAD: &[u8] = b"ping";
const EXPERIMENT_DEADLINE: u64 = 10;
const WORKER_CRASH_EXIT: i32 = 71;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn packet() -> Vec<u8> {
    let size = HEADER + EXPERIMENT_PAYLOAD.len();
    let mut b = vec![0; size];
    b[request_fields::OPERATION].copy_from_slice(&OPERATION_SEND.to_le_bytes());
    b[request_fields::TOTAL_SIZE].copy_from_slice(&(size as u32).to_le_bytes());
    b[request_fields::HANDLE].copy_from_slice(&1u64.to_le_bytes());
    b[request_fields::REQUEST_ID].copy_from_slice(&1u64.to_le_bytes());
    b[request_fields::DEADLINE].copy_from_slice(&EXPERIMENT_DEADLINE.to_le_bytes());
    b[request_fields::PAYLOAD_SIZE]
        .copy_from_slice(&(EXPERIMENT_PAYLOAD.len() as u32).to_le_bytes());
    b[HEADER..].copy_from_slice(EXPERIMENT_PAYLOAD);
    b
}
fn echo(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = native_protocol_model::decode(input, 0).map_err(|e| format!("{e:?}"))?;
    let mut frame = [0; native_protocol_model::RESPONSE_HEADER + MAX_PAYLOAD];
    let len = native_protocol_model::encode_response(
        request.request_id,
        native_protocol_model::Status::Completed,
        request.payload(),
        &mut frame,
    )
    .map_err(|e| format!("{e:?}"))?;
    Ok(frame[..len].to_vec())
}
fn worker(mode: &str) -> Result<(), String> {
    let mut data = Vec::new();
    std::io::stdin()
        .take((HEADER + MAX_PAYLOAD + 1) as u64)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    let mut response = echo(&data)?;
    if mode == "--worker-crash" {
        std::process::exit(WORKER_CRASH_EXIT);
    }
    if mode == "--worker-malformed" {
        response[response_fields::REQUEST_ID.start] ^= 1;
    }
    std::io::stdout()
        .write_all(&response)
        .map_err(|e| e.to_string())
}
fn isolated(input: &[u8], mode: &str) -> Result<Vec<u8>, String> {
    let request = native_protocol_model::decode(input, 0).map_err(|e| format!("{e:?}"))?;
    let mut command = Command::new(env::current_exe().map_err(|e| e.to_string())?);
    command
        .arg(mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    if let Err(e) = child.stdin.take().unwrap().write_all(input) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e.to_string());
    }
    // Bounded request and response fit the pipes. No wait is unbounded.
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err("SERVICE_LOST_EFFECT_UNKNOWN".into());
                }
                break;
            }
            Ok(None) if start.elapsed() < WORKER_TIMEOUT => {
                std::thread::sleep(WORKER_POLL_INTERVAL)
            }
            other => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("worker deadline or host error: {other:?}"));
            }
        }
    }
    let mut response = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .take((native_protocol_model::RESPONSE_HEADER + MAX_PAYLOAD + 1) as u64)
        .read_to_end(&mut response)
        .map_err(|e| e.to_string())?;
    native_protocol_model::decode_response(&response, request.request_id)
        .map_err(|_| "INVALID_RESPONSE".to_string())?;
    Ok(response)
}
fn experiment() -> Result<Value, String> {
    let good = [
        ("lifetime", lifetime(Mutation::None)),
        ("wait", wait(false)),
        ("binding", binding(false)),
        ("startup", startup(false)),
    ];
    let bad = [
        ("premature_free", lifetime(Mutation::FreeOnClose)),
        ("duplicate_effect", lifetime(Mutation::DoubleEffect)),
        ("lost_charge", lifetime(Mutation::LoseCharge)),
        ("lost_wakeup", wait(true)),
        ("premature_retirement", binding(true)),
        ("shutdown_with_requests", startup(true)),
    ];
    let mut models = serde_json::Map::new();
    for (name, result) in good {
        if result.counterexample.is_some() {
            return Err(format!("{name}: unexpected counterexample"));
        }
        models.insert(name.into(),json!({"reachable_states":result.states,"transitions":result.transitions,"violations":0}));
    }
    let mut controls = serde_json::Map::new();
    for (name, result) in bad {
        controls.insert(
            name.into(),
            json!(
                result
                    .counterexample
                    .ok_or(format!("negative control {name} survived"))?
            ),
        );
    }
    let input = packet();
    let expected = echo(&input)?;
    let response = isolated(&input, "--worker")?;
    if response != expected {
        return Err("placement paths disagree".into());
    }
    if isolated(&input, "--worker-crash") != Err("SERVICE_LOST_EFFECT_UNKNOWN".into()) {
        return Err("crash was not contained".into());
    }
    if isolated(&input, "--worker-malformed") != Err("INVALID_RESPONSE".into()) {
        return Err("malformed response was accepted".into());
    }
    if isolated(&input, "--worker")? != expected {
        return Err("fresh service cannot recover".into());
    }
    Ok(
        json!({"schema_version":1,"scope":"Finite host models and process-boundary experiment; no kernel or hardware proof", "models":models,"negative_controls":controls,"placement":{"same_payload_result":true,"child_failure_observed":true,"supervisor_survived":true,"fresh_service_succeeded":true,"malformed_response_rejected":true,"sandbox_claim":false,"performance_claim":false}}),
    )
}
fn run() -> Result<(), String> {
    let arg = env::args().nth(1).unwrap_or_default();
    if ["--worker", "--worker-crash", "--worker-malformed"].contains(&arg.as_str()) {
        return worker(&arg);
    }
    if !["", "--check", "--record"].contains(&arg.as_str()) {
        return Err("usage: native-state-models [--check | --record]".into());
    }
    let result = experiment()?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let path = root.join("research/results/native-state-models.json");
    if arg == "--check" {
        let recorded: Value =
            serde_json::from_str(&fs::read_to_string(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if recorded != result {
            return Err("model evidence changed: review then record the new result".into());
        }
        println!(
            "Model evidence reproduced; all negative controls detected; process failure contained."
        );
    } else {
        let text = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())? + "\n";
        if arg == "--record" {
            fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::write(path, text).map_err(|e| e.to_string())?;
        } else {
            print!("{text}");
        }
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
