//! Decode existing KOLVRT EL0 reports, rather than manufacturing independent A/B
//! pairs from sequential request samples. Imported assertions remain unattested.
use crate::{digest_valid, provenance::subject_digest};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const HEADER: u64 = 0x4b56_5232;
const BENCH: u64 = 0x4245_4e43;
const FINAL: u64 = 0x444f_4e45;
const HEADER_WORDS: usize = 15;
const WARMUP: usize = 16;
const SAMPLES: usize = 128;
const COUNTERS: usize = 9;
const ROUTE_NAMES: [&str; 4] = [
    "window.native/1.0.0",
    "window.inclusive/1.0.0",
    "window.counted/2.0.0",
    "bug.window.empty-first/1.0.0",
];

#[derive(Debug, Serialize)]
pub struct Observation {
    pub consumer: u64,
    pub cpu: u64,
    pub declared_route: String,
    pub generation: u64,
    pub conformance_checks: u64,
    pub timer_frequency_hz: u64,
    pub measured_routes: Vec<MeasuredRoute>,
}
#[derive(Debug, Serialize)]
pub struct MeasuredRoute {
    pub route: String,
    pub native_admissions: u64,
    pub compat_admissions: u64,
    pub translations: u64,
    pub copies: u64,
    pub copied_bytes: u64,
    pub conversions: u64,
    pub warmup_ticks: Vec<u64>,
    pub sample_ticks: Vec<u64>,
    pub observed_preemptions: u64,
    pub median_wall_ns: u64,
    pub adapter_cpu_ns: Option<u64>,
}
#[derive(Debug, Serialize)]
pub struct Run {
    pub profile: String,
    pub kernel_digest: String,
    pub package_digest: String,
    pub observations: Vec<Observation>,
}
#[derive(Debug, Serialize)]
pub struct RoutingReport {
    pub schema: u32,
    pub input_digest: String,
    pub environment: String,
    pub provenance_verified: bool,
    pub independent_paired_runs: bool,
    pub preferred_migration: Option<String>,
    pub automatic_replacement: bool,
    pub gaps: Vec<String>,
    pub runs: Vec<Run>,
}

fn number(value: &Value, key: &str) -> Result<u64, String> {
    value[key]
        .as_u64()
        .ok_or_else(|| format!("missing/invalid {key}"))
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing/invalid {key}"))
}
fn words(value: &Value) -> Result<Vec<u64>, String> {
    value
        .as_array()
        .ok_or("missing word array")?
        .iter()
        .map(|v| v.as_u64().ok_or("invalid report word".into()))
        .collect()
}
fn route(id: u64) -> Result<&'static str, String> {
    ROUTE_NAMES
        .get(id as usize)
        .copied()
        .ok_or("unknown route".into())
}
fn take<'a>(words: &'a [u64], cursor: &mut usize, count: usize) -> Result<&'a [u64], String> {
    let end = cursor.checked_add(count).ok_or("report length overflow")?;
    let chunk = words.get(*cursor..end).ok_or("truncated EL0 report")?;
    *cursor = end;
    Ok(chunk)
}

/// Raw frames are authoritative for counters/samples; the saved summary is checked
/// against them. This validates structure, not producer identity or physical performance.
pub fn inspect(bytes: &[u8]) -> Result<RoutingReport, String> {
    let input: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if number(&input, "schema_version")? != 1 {
        return Err("unsupported routing evidence schema".into());
    }
    let runs = input["runs"]
        .as_array()
        .filter(|r| !r.is_empty())
        .ok_or("missing routing runs")?;
    let mut profiles = BTreeSet::new();
    let mut output = Vec::new();
    for run in runs {
        let profile = text(run, "profile")?;
        if !profiles.insert(profile) {
            return Err("duplicate routing profile".into());
        }
        let kernel = text(&run["kernel_build"], "sha256")?;
        let package = text(&run["user_artifact"], "elf_sha256")?;
        if !digest_valid(kernel)
            || !digest_valid(package)
            || text(&run["run"], "elf_sha256")? != kernel
            || text(&run["run"], "accelerator")? != "TCG"
        {
            return Err("invalid identities or unsupported execution environment".into());
        }
        let mut raw = BTreeMap::<u64, Vec<u64>>::new();
        let mut finished = BTreeSet::new();
        let mut boot = false;
        let mut processes = None;
        for event in run["events"].as_array().ok_or("missing raw events")? {
            if boot {
                return Err("event after terminal boot".into());
            }
            if event["status"] == "fail"
                || matches!(event["event"].as_str(), Some("panic" | "fatal"))
            {
                return Err("kernel/EL0 failure".into());
            }
            match text(event, "event")? {
                "user-report" => {
                    let id = number(event, "id")?;
                    if finished.contains(&id) {
                        return Err("report after completion".into());
                    }
                    let data = raw.entry(id).or_default();
                    if number(event, "offset")? != data.len() as u64 {
                        return Err("lost/duplicate report chunk".into());
                    }
                    data.extend(words(&event["words"])?);
                }
                "user-result" => {
                    let id = number(event, "id")?;
                    if !finished.insert(id)
                        || number(event, "state")? != 2
                        || number(event, "exit")? != 1
                        || number(event, "fault")? != 0
                        || number(event, "length")? != raw.get(&id).map_or(0, Vec::len) as u64
                    {
                        return Err("missing/failed/duplicate consumer completion".into());
                    }
                }
                "el0" => {
                    if processes.is_some()
                        || event["status"] != "pass"
                        || event["reclaimed"] != true
                    {
                        return Err("invalid EL0 terminal evidence".into());
                    }
                    processes = Some(number(event, "processes")?);
                }
                "boot" => {
                    if event["status"] != "pass"
                        || number(event, "el")? != 1
                        || event["secondary_shutdown_verified"] != true
                    {
                        return Err("invalid boot terminal evidence".into());
                    }
                    boot = true;
                }
                _ => {}
            }
        }
        let consumers = run["consumers"]
            .as_array()
            .ok_or("missing consumer summaries")?;
        if !boot
            || consumers.is_empty()
            || processes != Some(consumers.len() as u64)
            || finished.len() != consumers.len()
            || raw.len() != consumers.len()
        {
            return Err("incomplete run".into());
        }
        let mut observations = Vec::new();
        let mut seen = BTreeSet::new();
        for consumer in consumers {
            let id = number(consumer, "id")?;
            if !seen.insert(id) {
                return Err("duplicate consumer summary".into());
            }
            let data = raw.get(&id).ok_or("summary without raw report")?;
            let mut cursor = 0;
            let header = take(data, &mut cursor, HEADER_WORDS)?;
            if header[0] != HEADER
                || header[1] != id
                || header[2] != number(consumer, "route")?
                || header[3] != number(consumer, "generation")?
                || header[3] == 0
                || header[4] != number(consumer, "conformance_checks")?
                || header[4] < 12
                || header[5] != number(consumer, "oracle_sum")?
                || header[5] == 0
                || header[6] != 2
                || header[7] != number(consumer, "frequency")?
                || header[7] == 0
                || header[10] != 1
                || header[11..15] != words(&consumer["profile_digest_words"])?
            {
                return Err("summary/raw contract result mismatch".into());
            }
            let declared = route(header[2])?;
            let mut measured = Vec::new();
            let mut measured_ids = BTreeSet::new();
            for benchmark in consumer["benchmarks"]
                .as_array()
                .ok_or("missing benchmark summaries")?
            {
                let frame = take(data, &mut cursor, 3)?;
                if frame[0] != BENCH
                    || frame[1] != number(benchmark, "route")?
                    || frame[2] != SAMPLES as u64
                    || !measured_ids.insert(frame[1])
                {
                    return Err("invalid/duplicate benchmark frame".into());
                }
                let name = route(frame[1])?;
                let warmup = take(data, &mut cursor, WARMUP)?.to_vec();
                let samples = take(data, &mut cursor, SAMPLES)?.to_vec();
                let preemptions = take(data, &mut cursor, 1)?[0];
                let counters = take(data, &mut cursor, COUNTERS)?;
                if samples != words(&benchmark["samples"])?
                    || warmup != words(&benchmark["warmup_samples"])?
                    || counters != words(&benchmark["counters"])?
                    || preemptions != number(benchmark, "observed_preemptions")?
                {
                    return Err("summary/raw measurement mismatch".into());
                }
                let calls = (SAMPLES + WARMUP) as u64;
                let compat = frame[1] != 0;
                if counters[0] != if compat { 0 } else { calls }
                    || counters[1] != if compat { calls } else { 0 }
                    || counters[2] != calls
                    || counters[3] != if compat { calls } else { 0 }
                    || counters[4] != counters[3]
                    || counters[5] != counters[3] * if frame[1] == 1 { 4 } else { 8 }
                    || counters[6] != counters[3] * if frame[1] == 3 { 4 } else { 3 }
                    || counters[7] != 0
                    || counters[8] != 0
                {
                    return Err("invalid native/compat accounting".into());
                }
                let mut sorted = samples.clone();
                sorted.sort_unstable();
                let ticks = sorted[(SAMPLES - 1) / 2];
                let ns = u128::from(ticks) * 1_000_000_000 / u128::from(header[7]);
                measured.push(MeasuredRoute {
                    route: name.into(),
                    native_admissions: counters[0],
                    compat_admissions: counters[1],
                    translations: counters[3],
                    copies: counters[4],
                    copied_bytes: counters[5],
                    conversions: counters[6],
                    warmup_ticks: warmup,
                    sample_ticks: samples,
                    observed_preemptions: preemptions,
                    median_wall_ns: ns.try_into().map_err(|_| "latency conversion overflow")?,
                    adapter_cpu_ns: None,
                });
            }
            if take(data, &mut cursor, 1)? != [FINAL] || cursor != data.len() {
                return Err("missing/trailing report data".into());
            }
            observations.push(Observation {
                consumer: id,
                cpu: number(consumer, "cpu")?,
                declared_route: declared.into(),
                generation: header[3],
                conformance_checks: header[4],
                timer_frequency_hz: header[7],
                measured_routes: measured,
            });
        }
        output.push(Run {
            profile: profile.into(),
            kernel_digest: kernel.into(),
            package_digest: package.into(),
            observations,
        });
    }
    Ok(RoutingReport {
        schema: crate::SCHEMA,
        input_digest: subject_digest(bytes),
        environment: "qemu_tcg".into(),
        provenance_verified: false,
        independent_paired_runs: false,
        preferred_migration: None,
        automatic_replacement: false,
        gaps: vec![
            "saved producer assertions are not authenticated".into(),
            "sequential request samples are not independent alternating paired runs".into(),
            "physical ARM64 workload measurements are absent".into(),
            "exclusive adapter CPU attribution is unavailable".into(),
            "snapshot rollback execution is not part of this routing exercise".into(),
        ],
        runs: output,
    })
}
