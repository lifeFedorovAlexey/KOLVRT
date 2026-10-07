//! External IPC pilot through the ordinary native ELF pipeline. No qualifying record is inferred.
use super::*;
use crate::arena_common::{AttemptSpec, compiler_identity, digest, git, run_attempt, write_json};
const MAGIC: u64 = 0x0049_5043_3301;
const HEADER: usize = 128;
const RECORD: usize = 8;
const EXHAUSTED: u64 = 12;
const EXPIRED: u64 = 16;
const ORDERS: [[u64; 2]; 2] = [[1, 2], [2, 1]];
fn counts(case: u64) -> (u64, u64) {
    if case == 9 { (3, 33) } else { (4, 32) }
}
fn clients(case: u64) -> u64 {
    if matches!(case, 2 | 5 | 8 | 11) { 2 } else { 1 }
}
fn payload(case: u64) -> u64 {
    if case < 9 {
        [0, 8, 256][case as usize / 3]
    } else if case == 9 {
        8
    } else {
        16
    }
}
fn argument(case: u64, mode: u64) -> u64 {
    let (warm, measured) = counts(case);
    case | (mode << 8) | (measured << 16) | (warm << 32)
}
fn summary(values: &[u64]) -> Value {
    if values.is_empty() {
        return json!({"state":"UNAVAILABLE","reason":"no successful observations","count":0});
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    json!({"state":"AVAILABLE","count":n,"p50":sorted[(n*50).div_ceil(100)-1],"p95":sorted[(n*95).div_ceil(100)-1],"p99":sorted[(n*99).div_ceil(100)-1],"tail_adequacy":"INCONCLUSIVE","interpretation":"descriptive per-boot nearest-rank; correlated samples, not independent tail evidence"})
}
fn overlap(records: &[Value], clients: u64) -> Value {
    let mut successful = 0u64;
    let mut admitted = 0u64;
    let mut refusals = 0u64;
    for (i, a) in records.iter().enumerate() {
        if a["phase"] != 1 {
            continue;
        }
        if a["stage"] == 1 && a["status"] == EXHAUSTED {
            refusals += 1;
        }
        for b in &records[i + 1..] {
            if b["phase"] != 1 || a["client"] == b["client"] {
                continue;
            }
            let intersects = a["start_ticks"]
                .as_u64()
                .unwrap()
                .max(b["start_ticks"].as_u64().unwrap())
                < a["end_ticks"]
                    .as_u64()
                    .unwrap()
                    .min(b["end_ticks"].as_u64().unwrap());
            if intersects && a["stage"] != 1 && b["stage"] != 1 {
                admitted += 1;
                if a["stage"] == 0 && b["stage"] == 0 {
                    successful += 1;
                }
            }
        }
    }
    json!({"successful_envelope_pairs":successful,"admitted_outcome_envelope_pairs":admitted,"measured_exhausted_count":refusals,"scope":"measured cross-client userspace CLOCK envelopes only; not proof of simultaneous kernel-admitted work","observation":if clients<2 {"single requester; cross-client overlap not applicable"} else if admitted==0 {"two-requester offered load; measured admitted-outcome envelopes did not overlap (observed serialized envelopes), no kernel overlap claim"} else {"cross-client userspace envelope overlap observed; internal kernel-inflight overlap remains unproven"}})
}
pub(super) fn validate(events: &[Value]) -> Result<Value> {
    if events
        .iter()
        .any(|e| matches!(e["event"].as_str(), Some("panic" | "fatal")) || e["status"] == "fail")
    {
        return Err("IPC pilot kernel failure".into());
    }
    let unique = |name: &str| -> Result<&Value> {
        let found: Vec<_> = events.iter().filter(|e| e["event"] == name).collect();
        if found.len() != 1 {
            return Err(format!("IPC pilot requires unique {name}").into());
        }
        Ok(found[0])
    };
    let loader = unique("native-root-image")?;
    if loader["format"] != "elf64" || loader["generation"].as_u64().is_none_or(|v| v == 0) {
        return Err("IPC original ELF witness missing".into());
    }
    let complete = unique("native-boot")?;
    if complete["status"] != "complete"
        || complete["root_exit"] != 0
        || complete["owners_released"] != true
        || complete["frames_restored"] != true
        || complete["live_processes"] != 0
        || complete["live_domains"] != 0
    {
        return Err("IPC pilot root/reclamation failed".into());
    }
    let words: Vec<u64> = native_apps::logical_report(events)?
        .iter()
        .map(|v| v.as_u64().ok_or("IPC report requires exact u64"))
        .collect::<std::result::Result<_, _>>()?;
    validate_words(&words)
}
fn validate_words(words: &[u64]) -> Result<Value> {
    if words.len() < HEADER
        || words[0] != MAGIC
        || words[1] != 1
        || words[2] > 11
        || !matches!(words[3], 1 | 2)
        || words[4] == 0
    {
        return Err("IPC report identity/schema invalid".into());
    }
    let case = words[2];
    let (warm, measured) = counts(case);
    let total = (warm + measured) as usize;
    let peers = clients(case);
    if words[5] != warm
        || words[6] != measured
        || words[7] != peers
        || words[8] != total as u64
        || words.len() != HEADER + total * RECORD
        || words[11] != total as u64
        || words[20] != 0
        || words[21] != payload(case)
        || words[22] != u64::from(case < 9 && case.is_multiple_of(3))
        || words[23] != 0
        || words[28] != 1
        || words[29..HEADER].iter().any(|v| *v != 0)
    {
        return Err("IPC header/count/ledger/reclamation mismatch".into());
    }
    if words[24] != total as u64 / peers
        || words[25] != if peers == 2 { total as u64 / 2 } else { 0 }
        || words[26] != warm / peers
        || words[27] != if peers == 2 { warm / 2 } else { 0 }
    {
        return Err("IPC client population mismatch".into());
    }
    let elapsed = words[10]
        .checked_sub(words[9])
        .filter(|v| *v > 0)
        .ok_or("IPC aggregate time invalid")?;
    let mut ids = BTreeSet::new();
    let mut sequences = BTreeSet::new();
    let (mut admitted, mut successes, mut exhausted, mut expired, mut other) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    let (mut warm_seen, mut measured_seen) = ([0u64; 2], [0u64; 2]);
    let (mut successful_samples, mut warm_samples, mut rejected_samples) =
        (Vec::new(), Vec::new(), Vec::new());
    let mut records = Vec::new();
    let mut measured_successes = 0u64;
    let mut previous_end = [0u64; 2];
    let mut ledger_hash = 0u64;
    for (index, record) in words[HEADER..].as_chunks::<RECORD>().0.iter().enumerate() {
        let (client, phase) = (record[0] >> 32, record[0] & 0xffff_ffff);
        if client >= peers
            || phase > 1
            || client != index as u64 / (total as u64 / peers)
            || record[1] != ((client << 32) | (index as u64 % (total as u64 / peers) + 1))
            || !ids.insert((client, record[1]))
        {
            return Err("IPC duplicate/foreign request or record order".into());
        }
        let expected_phase = u64::from(index as u64 % (total as u64 / peers) >= warm / peers);
        if phase != expected_phase || (case != 9 && record[2] < previous_end[client as usize]) {
            return Err("IPC phase/order inconsistent".into());
        }
        previous_end[client as usize] = record[3];
        let wall = record[3]
            .checked_sub(record[2])
            .ok_or("IPC backwards clock")?;
        let residual = wall
            .checked_sub(record[4])
            .and_then(|v| v.checked_sub(record[5]))
            .ok_or("IPC attribution exceeds wall")?;
        if record[5] != 0 {
            return Err("IPC CLOCK read-window attribution is not IPC cost".into());
        }
        if phase == 1 {
            if record[2] < words[9] || record[3] > words[10] {
                return Err("IPC sample outside aggregate interval".into());
            }
            measured_seen[client as usize] += 1;
        } else {
            warm_seen[client as usize] += 1;
        }
        let (stage, code) = (record[6] >> 32, record[6] & 0xffff_ffff);
        if stage > 4 || (stage == 0) != (code == 0) {
            return Err("IPC malformed status stage".into());
        }
        if stage != 1 {
            admitted += 1;
        }
        if stage == 0 {
            successes += 1;
            ledger_hash =
                ledger_hash.wrapping_add(record[1].wrapping_add(record[7].rotate_left(17)));
            if record[7] == 0 || !sequences.insert(record[7]) {
                return Err("IPC fake/repeated successful result sequence".into());
            }
            if phase == 1 {
                successful_samples.push(wall);
                measured_successes += 1;
            } else {
                warm_samples.push(wall);
            }
        } else {
            if record[7] != 0 {
                return Err("failed IPC attempt contains successful result".into());
            }
            if stage == 1 && code == EXHAUSTED {
                exhausted += 1;
            } else if code == EXPIRED {
                expired += 1;
            } else {
                other += 1;
            }
            if phase == 1 {
                rejected_samples.push(wall);
            }
        }
        records.push(json!({"client":client,"phase":phase,"request_id":record[1],"start_ticks":record[2],"end_ticks":record[3],"wall_ticks":wall,"execution_window_ticks":record[4],"read_window_ticks":record[5],"unattributed_ticks":residual,"stage":stage,"status":code,"sequence":record[7]}));
    }
    if words[18] != ledger_hash
        || words[12..17] != [admitted, successes, exhausted, expired, other]
        || warm_seen.iter().sum::<u64>() != warm
        || measured_seen.iter().sum::<u64>() != measured
        || sequences.iter().copied().ne(1..=successes)
    {
        return Err("IPC outcome counts or useful sequence ledger mismatch".into());
    }
    if expired != 0 || other != 0 || (!matches!(case, 2 | 5 | 8 | 9 | 11) && exhausted != 0) {
        return Err("unexpected IPC functional outcome; raw attempt retained".into());
    }
    if case == 9 {
        if exhausted != total as u64 / 3 || successes != total as u64 * 2 / 3 {
            return Err("capacity-one rejection/drainage missing".into());
        }
        for cycle in words[HEADER..].as_chunks::<{ RECORD * 3 }>().0 {
            if cycle[RECORD + 2] < cycle[2]
                || cycle[RECORD + 3] > cycle[3]
                || cycle[2 * RECORD + 2] < cycle[3]
                || cycle[6] != 0
                || cycle[RECORD + 6] != (1u64 << 32 | EXHAUSTED)
                || cycle[2 * RECORD + 6] != 0
            {
                return Err("saturation requires admitted/rejected/recovered ordering".into());
            }
        }
    }
    if case >= 10 {
        if words[19] != successes {
            return Err(
                "production counter final state differs from real successful additions".into(),
            );
        }
    } else if words[17] != successes {
        return Err("external server execution count missing".into());
    }
    Ok(
        json!({"case":case,"mode":words[3],"frequency":words[4],"warmup_offered":warm,"measured_offered":measured,"clients":peers,"offered":total,"admitted":admitted,"successful":successes,"exhausted":exhausted,"expired":expired,"other_errors":other,"measured_successes":measured_successes,"aggregate_start_ticks":words[9],"aggregate_end_ticks":words[10],"aggregate_elapsed_ticks":elapsed,"throughput":{"successful_operations":measured_successes,"elapsed_ticks":elapsed,"frequency":words[4],"successful_operations_per_second":measured_successes as f64*words[4] as f64/elapsed as f64,"definition":"successes * frequency / elapsed ticks; includes declared DONE control, excludes bulk report transfer"},"successful_wall_ticks":successful_samples,"warmup_successful_wall_ticks":warm_samples,"rejected_wall_ticks":rejected_samples,"successful_summary":summary(&successful_samples),"userspace_overlap":overlap(&records,peers),"records":records,"raw_words":words,"functional_status":"passed","admission_state":"INELIGIBLE","admission_reason":"functional pilot only; variable successful populations and all capacity losses retained; no qualifying performance or completed SEC claim","record_eligible":false,"owners_released":true,"frames_restored":true}),
    )
}
fn configuration_overrides_build(path: &Path) -> Result<bool> {
    if !path.is_file() {
        return Ok(false);
    }
    let text = fs::read_to_string(path)?;
    // Registry/network/alias settings alone do not alter this compilation class.
    // Additional build/target/profile/env sections need an explicitly reviewed class.
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .any(|line| {
            [
                "[build",
                "[target",
                "[profile",
                "[env",
                "[unstable",
                "rustflags",
                "rustc",
                "linker",
            ]
            .iter()
            .any(|key| line.starts_with(key))
        }))
}
fn verify_build_environment(manifest: &Value, compiler: &str) -> Result<()> {
    if compiler.split_whitespace().next() != Some("rustc")
        || compiler.split_whitespace().nth(1) != manifest["compiler_version"].as_str()
    {
        return Err("actual rustc version differs from IPC class".into());
    }
    let bytes = fs::read(".cargo/config.toml")?;
    if manifest["cargo_config_sha256"] != digest(&bytes) {
        return Err("Cargo configuration differs from IPC declared raw-byte digest".into());
    }
    let text = String::from_utf8(bytes)?;
    let flags_line = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("rustflags = "))
        .ok_or("fixed target rustflags absent")?;
    let flags: Value = serde_json::from_str(flags_line)?;
    if flags != manifest["rustflags"] {
        return Err("IPC declared flags differ from target Cargo configuration".into());
    }
    let cwd = env::current_dir()?;
    for ancestor in cwd.ancestors() {
        for name in ["config", "config.toml"] {
            let candidate = ancestor.join(".cargo").join(name);
            if candidate != cwd.join(".cargo/config.toml")
                && configuration_overrides_build(&candidate)?
            {
                return Err(format!(
                    "fixed IPC build rejects additional Cargo configuration {}",
                    candidate.display()
                )
                .into());
            }
        }
    }
    if let Some(home) = env::var_os("CARGO_HOME") {
        for name in ["config", "config.toml"] {
            let candidate = Path::new(&home).join(name);
            if configuration_overrides_build(&candidate)? {
                return Err(format!(
                    "fixed IPC build rejects additional Cargo-home configuration {}",
                    candidate.display()
                )
                .into());
            }
        }
    }
    for (name, _) in env::vars_os() {
        let name = name.to_string_lossy();
        if name.starts_with("CARGO_TARGET_") && name != "CARGO_TARGET_DIR"
            || matches!(
                name.as_ref(),
                "CARGO_BUILD_TARGET"
                    | "CARGO_BUILD_RUSTC"
                    | "CARGO_BUILD_RUSTC_WRAPPER"
                    | "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER"
            )
        {
            return Err(format!("fixed IPC build rejects override {name}").into());
        }
    }
    Ok(())
}
fn definition_snapshot() -> Result<Value> {
    let directory = Path::new("research/arena/ipc-query");
    let _ = fs::read(directory.join("protocol.json"))?;
    let mut paths = Vec::new();
    walk(directory, &mut paths)?;
    paths.sort();
    let mut files = Vec::new();
    for path in paths {
        let bytes = fs::read(&path)?;
        files
            .push(json!({"path":path.to_string_lossy().replace('\\',"/"),"sha256":digest(&bytes)}));
    }
    Ok(json!(files))
}
pub(super) fn run(args: &[String]) -> Result<()> {
    let smoke = match args.first().map(String::as_str) {
        Some("ipc-pilot") if args.len() == 1 => None,
        Some("ipc-smoke") if args.len() == 2 || (args.len() == 3 && args[2] == "--prod") => {
            Some((args[1].parse::<u64>()?, args.len() == 3))
        }
        _ => return Err("usage: cargo xtask arena run ipc-pilot | ipc-smoke CASE [--prod]".into()),
    };
    if smoke.is_some_and(|(case, _)| case > 11) {
        return Err("IPC case outside0..11".into());
    }
    let compiler = compiler_identity()?;
    let environments = [
        read_json("research/arena/ipc-query/dev-environment.json")?,
        read_json("research/arena/ipc-query/prod-environment.json")?,
    ];
    for environment in &environments {
        verify_build_environment(environment, &compiler)?;
    }
    let root = PathBuf::from(format!(
        "target/kernel/arena-ipc/{}-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        std::process::id()
    ));
    fs::create_dir_all(root.parent().ok_or("campaign parent absent")?)?;
    fs::create_dir(&root)?;
    let sources = source_inventory()?;
    let definitions = definition_snapshot()?;
    write_json(root.join("source-files.json"), &sources)?;
    write_json(root.join("definitions.json"), &definitions)?;
    let revision = git(&["rev-parse", "HEAD"])?;
    let dirty = !git(&["status", "--porcelain"])?.is_empty();
    let mut plan = Vec::new();
    if let Some((case, prod)) = smoke {
        plan.push((case, prod, 0, 0, 1));
    } else {
        for case in 0..12 {
            for prod in [false, true] {
                for (pair, order) in ORDERS.iter().enumerate() {
                    for (position, &mode) in order.iter().enumerate() {
                        plan.push((case, prod, pair, position, mode));
                    }
                }
            }
        }
    }
    let mut attempts = Vec::new();
    let mut unchanged = true;
    for &(case, prod, pair, position, mode) in &plan {
        if source_inventory()? != sources || definition_snapshot()? != definitions {
            unchanged = false;
            break;
        }
        let profile = if prod { "prod" } else { "dev" };
        let directory = root.join(format!("case-{case}/{profile}/pair-{pair}/mode-{mode}"));
        let service = if case >= 10 {
            "counter-service"
        } else {
            "selftest-ipc-measure-client"
        };
        let client = if case < 9 && case.is_multiple_of(3) {
            "selftest-ipc-measure-client"
        } else {
            "selftest-ipc-measure-peer"
        };
        let mut attempt = run_attempt(
            AttemptSpec {
                prod,
                images: ["selftest-ipc-measure-root", service, client],
                argument: argument(case, mode),
                directory: &directory,
            },
            json!({"case":case,"profile":profile,"pair":pair,"order":position,"mode":mode,"argument":argument(case,mode)}),
            validate,
            &[("case", case), ("mode", mode)],
        )?;
        if attempt["status"] == "passed"
            && !crate::arena_common::manifest_matches(&attempt, &environments[usize::from(prod)])
        {
            attempt["status"] = json!("failed");
            attempt["error"] = json!(
                "actual QEMU arguments/version, counter frequency or kernel build disagree with declared IPC environment"
            );
            write_json(directory.join("attempt.json"), &attempt)?;
        }
        attempts.push(attempt);
    }
    unchanged &= source_inventory()? == sources && definition_snapshot()? == definitions;
    let passed = attempts.iter().filter(|a| a["status"] == "passed").count();
    let success = unchanged && passed == plan.len() && attempts.len() == plan.len();
    let campaign = json!({"schema_version":1,"stage":if smoke.is_some(){"partial-functional-smoke"}else{"functional-pilot"},"exact_commit":revision,"worktree_dirty":dirty,"compiler_version":compiler,"source_files":sources,"definition_snapshot":definitions,"source_unchanged":unchanged,"planned_attempts":plan.len(),"attempted":attempts.len(),"passed":passed,"functional_status":if success{"passed"}else{"failed"},"admission_state":"INELIGIBLE","admission_reason":"pilot does not establish main sample plan, tail adequacy or current full SEC evidence; losses preserved","record_eligible":false,"attempts":attempts});
    write_json(root.join("campaign.json"), &campaign)?;
    println!(
        "IPC functional evidence: {}; Arena admission INELIGIBLE (pilot only)",
        root.display()
    );
    if !success {
        return Err(format!(
            "IPC functional pilot failed; all attempted evidence retained at {}",
            root.display()
        )
        .into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u64> {
        let mut words = vec![0; HEADER];
        words[..29].copy_from_slice(&[
            MAGIC, 1, 0, 1, 62500000, 4, 32, 1, 36, 80, 1000, 36, 36, 36, 0, 0, 0, 36, 0, 0, 0, 0,
            1, 0, 36, 0, 4, 0, 1,
        ]);
        for i in 0..36 {
            words.extend([
                u64::from(i >= 4),
                i + 1,
                10 + i * 20,
                20 + i * 20,
                2,
                0,
                0,
                i + 1,
            ]);
        }
        words[18] = (1u64..=36)
            .map(|v| v.wrapping_add(v.rotate_left(17)))
            .fold(0, u64::wrapping_add);
        words
    }
    #[test]
    fn ipc_oracle_keeps_actual_success_population() {
        let result = validate_words(&fixture()).unwrap();
        assert_eq!(result["measured_successes"], 32);
        assert_eq!(result["admission_state"], "INELIGIBLE");
    }
    #[test]
    fn ipc_oracle_rejects_forged_execution_and_counts() {
        for (index, value) in [
            (13, 35),
            (17, 0),
            (28, 0),
            (128 + 7, 0),
            (128 + 8 + 7, 1),
            (128 + 3, 0),
            (128 + 4, 20),
            (128 + 5, 1),
            (29, 1),
        ] {
            let mut w = fixture();
            w[index] = value;
            assert!(validate_words(&w).is_err(), "{index}");
        }
        let mut w = fixture();
        w.pop();
        assert!(validate_words(&w).is_err());
    }
    #[test]
    fn ipc_case_schedule_and_arguments_are_bounded() {
        for case in 0..12 {
            let a = argument(case, 2);
            assert_eq!(a & 255, case);
            assert_eq!((a >> 8) & 255, 2);
            assert_eq!(((a >> 16) & 65535) + ((a >> 32) & 65535), 36);
        }
        assert_eq!(ORDERS, [[1, 2], [2, 1]]);
    }
    fn update_ledger(words: &mut [u64]) {
        let mut admitted = 0;
        let mut successes = 0;
        let mut rejected = 0;
        let mut hash = 0u64;
        for r in words[HEADER..].as_chunks::<RECORD>().0 {
            if r[6] >> 32 != 1 {
                admitted += 1;
            }
            if r[6] == 0 {
                successes += 1;
                hash = hash.wrapping_add(r[1].wrapping_add(r[7].rotate_left(17)));
            } else {
                rejected += 1;
            }
        }
        words[12..19].copy_from_slice(&[admitted, successes, rejected, 0, 0, successes, hash]);
    }
    #[test]
    fn ipc_oracle_retains_capacity_losses_as_ineligible_population() {
        let mut w = fixture();
        w.truncate(HEADER);
        w[2] = 2;
        w[7] = 2;
        w[9] = 100;
        w[22] = 0;
        w[24] = 18;
        w[25] = 18;
        w[26] = 2;
        w[27] = 2;
        for client in 0u64..2 {
            for local in 0u64..18 {
                let phase = u64::from(local >= 2);
                let start = if phase == 0 {
                    10 + local * 20 + client * 5
                } else {
                    100 + (local - 2) * 20 + client * 10
                };
                let mut seq = if phase == 0 {
                    1 + client * 2 + local
                } else {
                    5 + (local - 2) * 2 + client
                };
                let rejected = client == 0 && local == 3;
                if seq > 7 {
                    seq -= 1;
                }
                w.extend([
                    (client << 32) | phase,
                    (client << 32) | (local + 1),
                    start,
                    start + 8,
                    2,
                    0,
                    if rejected { (1 << 32) | EXHAUSTED } else { 0 },
                    if rejected { 0 } else { seq },
                ]);
            }
        }
        update_ledger(&mut w);
        let result = validate_words(&w).unwrap();
        assert_eq!(result["exhausted"], 1);
        assert_eq!(result["measured_successes"], 31);
        assert_eq!(
            result["successful_wall_ticks"].as_array().unwrap().len(),
            31
        );
        assert_eq!(result["rejected_wall_ticks"].as_array().unwrap().len(), 1);
        assert_eq!(result["admission_state"], "INELIGIBLE");
        w[14] = 0;
        assert!(validate_words(&w).is_err());
    }
    #[test]
    fn ipc_oracle_requires_actual_saturation_rejection_and_recovery() {
        let mut w = fixture();
        w.truncate(HEADER);
        w[2] = 9;
        w[5] = 3;
        w[6] = 33;
        w[9] = 100;
        w[21] = 8;
        w[22] = 0;
        w[26] = 3;
        for cycle in 0u64..12 {
            let base = if cycle == 0 {
                10
            } else {
                100 + (cycle - 1) * 60
            };
            let phase = u64::from(cycle > 0);
            for step in 0u64..3 {
                let (start, end) = match step {
                    0 => (base, base + 30),
                    1 => (base + 10, base + 20),
                    _ => (base + 31, base + 40),
                };
                w.extend([
                    phase,
                    cycle * 3 + step + 1,
                    start,
                    end,
                    2,
                    0,
                    if step == 1 { (1 << 32) | EXHAUSTED } else { 0 },
                    if step == 1 {
                        0
                    } else {
                        cycle * 2 + 1 + u64::from(step == 2)
                    },
                ]);
            }
        }
        update_ledger(&mut w);
        let result = validate_words(&w).unwrap();
        assert_eq!(result["exhausted"], 12);
        assert_eq!(result["measured_successes"], 22);
        w[HEADER + RECORD + 6] = 0;
        assert!(validate_words(&w).is_err());
    }
    #[test]
    fn ipc_overlap_distinguishes_disjoint_envelopes_from_queue_refusals() {
        let r = |client, start, end, stage, status| json!({"client":client,"phase":1,"start_ticks":start,"end_ticks":end,"stage":stage,"status":status});
        let mut records = vec![
            r(0, 10, 20, 0, 0),
            r(1, 20, 30, 0, 0),
            r(1, 12, 15, 1, EXHAUSTED),
        ];
        let disjoint = overlap(&records, 2);
        assert_eq!(disjoint["successful_envelope_pairs"], 0);
        assert_eq!(disjoint["measured_exhausted_count"], 1);
        records[1]["start_ticks"] = json!(19);
        let observed = overlap(&records, 2);
        assert_eq!(observed["successful_envelope_pairs"], 1);
        assert_eq!(observed["admitted_outcome_envelope_pairs"], 1);
        assert_eq!(observed["measured_exhausted_count"], 1);
        records[1]["phase"] = json!(0);
        assert_eq!(overlap(&records, 2)["successful_envelope_pairs"], 0);
    }
}
