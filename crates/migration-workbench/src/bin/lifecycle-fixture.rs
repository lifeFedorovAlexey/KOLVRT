#![forbid(unsafe_code)]

use migration_advisor::provenance::subject_digest;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, process::ExitCode};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StateV1 {
    schema: u8,
    records: Vec<i64>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StateV2 {
    schema: u8,
    records: Vec<i64>,
    checksum: i64,
}

#[derive(Serialize)]
struct Receipt {
    operation: String,
    passed: bool,
    state_digest: String,
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("next");
    let mut file = fs::File::create(&temporary).map_err(|error| error.to_string())?;
    use std::io::Write;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    fs::rename(&temporary, path).map_err(|error| error.to_string())
}

fn run(args: &[String]) -> Result<Receipt, String> {
    let [operation, directory] = args else {
        return Err("usage: lifecycle-fixture seed|migrate|verify-v1|verify-v2 <state-dir>".into());
    };
    let root = Path::new(directory);
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    match operation.as_str() {
        "seed" => {
            let path = root.join("state-v1.json");
            if path.exists() {
                return Err("seed requires an empty state directory".into());
            }
            let state = StateV1 {
                schema: 1,
                records: vec![3, 5, 8, 13],
            };
            let bytes = serde_json::to_vec(&state).map_err(|error| error.to_string())?;
            write_atomic(&path, &bytes)?;
            Ok(Receipt {
                operation: operation.clone(),
                passed: true,
                state_digest: subject_digest(&bytes),
            })
        }
        "migrate" => {
            let old_path = root.join("state-v1.json");
            let bytes = fs::read(&old_path).map_err(|error| error.to_string())?;
            let old: StateV1 = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            if old.schema != 1 || old.records != [3, 5, 8, 13] {
                return Err("unsupported or invalid v1 persistent state".into());
            }
            let state = StateV2 {
                schema: 2,
                checksum: old.records.iter().sum(),
                records: old.records,
            };
            let migrated = serde_json::to_vec(&state).map_err(|error| error.to_string())?;
            write_atomic(&root.join("state-v2.json"), &migrated)?;
            Ok(Receipt {
                operation: operation.clone(),
                passed: true,
                state_digest: subject_digest(&migrated),
            })
        }
        "verify-v1" => {
            let path = root.join("state-v1.json");
            let bytes = fs::read(path).map_err(|error| error.to_string())?;
            let state: StateV1 =
                serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            if state.schema != 1 || state.records != [3, 5, 8, 13] {
                return Err("v1 state restoration check failed".into());
            }
            Ok(Receipt {
                operation: operation.clone(),
                passed: true,
                state_digest: subject_digest(&bytes),
            })
        }
        "verify-v2" => {
            let path = root.join("state-v2.json");
            let bytes = fs::read(path).map_err(|error| error.to_string())?;
            let state: StateV2 =
                serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            if state.schema != 2
                || state.records != [3, 5, 8, 13]
                || state.checksum != state.records.iter().sum::<i64>()
            {
                return Err("v2 restart/persistence check failed".into());
            }
            Ok(Receipt {
                operation: operation.clone(),
                passed: true,
                state_digest: subject_digest(&bytes),
            })
        }
        _ => Err("unknown lifecycle-fixture operation".into()),
    }
}

fn main() -> ExitCode {
    match run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(receipt) => {
            println!("{}", serde_json::to_string(&receipt).unwrap());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("[FAIL] lifecycle-fixture: {error}");
            ExitCode::FAILURE
        }
    }
}
