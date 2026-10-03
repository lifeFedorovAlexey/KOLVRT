//! Decode existing KOLVRT EL0 reports, rather than manufacturing independent A/B
//! pairs from sequential request samples. Imported assertions remain unattested.
use crate::{digest_valid, provenance::subject_digest};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const HEADER: u64 = 0x4b56_5233;
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
    pub process_slot: u64,
    pub process_generation: u64,
    pub owner_cpu: u64,
    pub resident_pages: u64,
    pub el0_residency_counter_ticks: u64,
    pub native_window_service_counter_ticks: u64,
    pub counter_frequency_hz: u64,
    pub estimated_el0_residency_ns: u64,
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
    pub route_el0_cpu_ticks: Vec<u64>,
    pub native_service_cpu_ticks: Vec<u64>,
    pub observed_preemptions: u64,
    pub median_wall_ns: u64,
    pub route_el0_cpu_ns: Option<u64>,
    pub native_service_cpu_ns: u64,
    pub measured_segments_cpu_ns: u64,
}
#[derive(Debug, Serialize)]
pub struct Run {
    pub profile: String,
    pub kernel_digest: String,
    pub package_digest: String,
    pub expected_consumers: u64,
    pub completed_consumers: u64,
    pub missing_consumers: u64,
    pub validated_report_chunks: u64,
    pub lost_report_chunks: u64,
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
    let evidence_schema = number(&input, "schema_version")?;
    if !matches!(evidence_schema, 4..=6) {
        return Err("unsupported routing evidence schema".into());
    }
    // v4 used adapter_* names for this same EL0 route-call interval. Normalize the
    // legacy labels without attributing the interval to adapter-only execution.
    let (route_el0_cpu_samples_key, route_el0_cpu_ns_key) = if evidence_schema == 4 {
        ("adapter_cpu_samples", "adapter_cpu_ns")
    } else {
        ("route_el0_cpu_samples", "route_el0_cpu_ns")
    };
    let measured_segments_cpu_ns_key = if evidence_schema >= 6 {
        "measured_segments_cpu_ns"
    } else {
        "exclusive_cpu_ns"
    };
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
        let mut process_results = BTreeMap::<u64, (u64, u64, u64, u64, u64, u64, u64)>::new();
        let mut report_chunks = 0u64;
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
                    report_chunks = report_chunks
                        .checked_add(1)
                        .ok_or("report chunk count overflow")?;
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
                    let process_slot = number(event, "process_slot")?;
                    let process_generation = number(event, "process_generation")?;
                    let owner_cpu = number(event, "owner_cpu")?;
                    let resident_pages = number(event, "resident_pages")?;
                    let el0_residency_ticks = number(event, "el0_residency_ticks")?;
                    let native_window_service_ticks = number(event, "native_window_service_ticks")?;
                    let counter_frequency_hz = number(event, "counter_frequency_hz")?;
                    if !finished.insert(id)
                        || number(event, "state")? != 2
                        || number(event, "exit")? != 1
                        || number(event, "fault")? != 0
                        || number(event, "length")? != raw.get(&id).map_or(0, Vec::len) as u64
                        || process_slot != id
                        || process_generation == 0
                        || resident_pages == 0
                        || el0_residency_ticks == 0
                        || native_window_service_ticks == 0
                        || counter_frequency_hz == 0
                        || process_results
                            .insert(
                                id,
                                (
                                    process_slot,
                                    process_generation,
                                    owner_cpu,
                                    resident_pages,
                                    el0_residency_ticks,
                                    native_window_service_ticks,
                                    counter_frequency_hz,
                                ),
                            )
                            .is_some()
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
        let accounting = &run["scope_accounting"];
        let expected_consumers = number(accounting, "expected_consumers")?;
        let completed_consumers = number(accounting, "completed_consumers")?;
        let missing_consumers = number(accounting, "missing_consumers")?;
        let validated_report_chunks = number(accounting, "validated_report_chunks")?;
        let lost_report_chunks = number(accounting, "lost_report_chunks")?;
        if !boot
            || consumers.is_empty()
            || expected_consumers != consumers.len() as u64
            || completed_consumers != finished.len() as u64
            || missing_consumers != expected_consumers.saturating_sub(completed_consumers)
            || validated_report_chunks != report_chunks
            || lost_report_chunks != 0
            || processes != Some(consumers.len() as u64)
            || finished.len() != consumers.len()
            || raw.len() != consumers.len()
        {
            return Err("incomplete run".into());
        }
        let mut observations = Vec::new();
        let mut seen = BTreeSet::new();
        let mut seen_process_ids = BTreeSet::new();
        for consumer in consumers {
            let id = number(consumer, "id")?;
            if !seen.insert(id) {
                return Err("duplicate consumer summary".into());
            }
            let process_slot = number(&consumer["process_identity"], "process_slot")?;
            let process_generation = number(&consumer["process_identity"], "process_generation")?;
            let owner_cpu = number(&consumer["process_identity"], "owner_cpu")?;
            let resident_pages = number(&consumer["process_identity"], "resident_pages")?;
            let el0_residency_ticks = number(consumer, "el0_residency_ticks")?;
            let native_window_service_ticks = number(consumer, "native_window_service_ticks")?;
            let counter_frequency_hz = number(consumer, "counter_frequency_hz")?;
            if process_slot != id
                || process_generation == 0
                || owner_cpu != number(consumer, "cpu")?
                || resident_pages == 0
                || el0_residency_ticks == 0
                || native_window_service_ticks == 0
                || counter_frequency_hz == 0
                || !seen_process_ids.insert((process_slot, process_generation))
                || process_results.get(&id)
                    != Some(&(
                        process_slot,
                        process_generation,
                        owner_cpu,
                        resident_pages,
                        el0_residency_ticks,
                        native_window_service_ticks,
                        counter_frequency_hz,
                    ))
            {
                return Err("kernel process identity/accounting mismatch".into());
            }
            let data = raw.get(&id).ok_or("summary without raw report")?;
            let mut cursor = 0;
            let header = take(data, &mut cursor, HEADER_WORDS)?;
            if header[0] != HEADER
                || header[7] != counter_frequency_hz
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
            let mut measured_native_service_ticks = 0u64;
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
                let route_el0_cpu_ticks = take(data, &mut cursor, SAMPLES)?.to_vec();
                let native_service_cpu_ticks = take(data, &mut cursor, SAMPLES)?.to_vec();
                let preemptions = take(data, &mut cursor, 1)?[0];
                let counters = take(data, &mut cursor, COUNTERS)?;
                if samples != words(&benchmark["samples"])?
                    || warmup != words(&benchmark["warmup_samples"])?
                    || route_el0_cpu_ticks != words(&benchmark[route_el0_cpu_samples_key])?
                    || native_service_cpu_ticks != words(&benchmark["native_service_cpu_samples"])?
                    || counters != words(&benchmark["counters"])?
                    || preemptions != number(benchmark, "observed_preemptions")?
                {
                    return Err("summary/raw measurement mismatch".into());
                }
                if route_el0_cpu_ticks.contains(&0) || native_service_cpu_ticks.contains(&0) {
                    return Err("invalid route-EL0/native CPU accounting".into());
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
                let median_ticks = |values: &[u64]| {
                    let mut sorted = values.to_vec();
                    sorted.sort_unstable();
                    sorted[(values.len() - 1) / 2]
                };
                let route_el0_ticks = median_ticks(&route_el0_cpu_ticks);
                let native_ticks = median_ticks(&native_service_cpu_ticks);
                let measured_segments_cpu_samples = route_el0_cpu_ticks
                    .iter()
                    .zip(&native_service_cpu_ticks)
                    .map(|(&route_el0, &native)| {
                        route_el0
                            .checked_add(native)
                            .ok_or_else(|| "measured CPU segment sum overflow".to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let measured_segments_cpu_ticks = median_ticks(&measured_segments_cpu_samples);
                let to_ns = |ticks: u64| -> Result<u64, String> {
                    (u128::from(ticks) * 1_000_000_000 / u128::from(header[7]))
                        .try_into()
                        .map_err(|_| "CPU time conversion overflow".into())
                };
                if number(benchmark, route_el0_cpu_ns_key)? != to_ns(route_el0_ticks)?
                    || number(benchmark, "native_service_cpu_ns")? != to_ns(native_ticks)?
                    || number(benchmark, measured_segments_cpu_ns_key)?
                        != to_ns(measured_segments_cpu_ticks)?
                {
                    return Err("exclusive CPU summary disagrees with raw counters".into());
                }
                let route_native_service_ticks = native_service_cpu_ticks
                    .iter()
                    .try_fold(0u64, |sum, ticks| sum.checked_add(*ticks))
                    .ok_or("native service counter sum overflow")?;
                measured_native_service_ticks = measured_native_service_ticks
                    .checked_add(route_native_service_ticks)
                    .ok_or("native service counter sum overflow")?;
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
                    route_el0_cpu_ticks,
                    native_service_cpu_ticks,
                    observed_preemptions: preemptions,
                    median_wall_ns: ns.try_into().map_err(|_| "latency conversion overflow")?,
                    route_el0_cpu_ns: Some(to_ns(route_el0_ticks)?),
                    native_service_cpu_ns: to_ns(native_ticks)?,
                    measured_segments_cpu_ns: to_ns(measured_segments_cpu_ticks)?,
                });
            }
            if native_window_service_ticks < measured_native_service_ticks {
                return Err("process native-service total is below measured samples".into());
            }
            if take(data, &mut cursor, 1)? != [FINAL] || cursor != data.len() {
                return Err("missing/trailing report data".into());
            }
            observations.push(Observation {
                consumer: id,
                cpu: number(consumer, "cpu")?,
                process_slot,
                process_generation,
                owner_cpu,
                resident_pages,
                el0_residency_counter_ticks: el0_residency_ticks,
                native_window_service_counter_ticks: native_window_service_ticks,
                counter_frequency_hz,
                estimated_el0_residency_ns: (u128::from(el0_residency_ticks) * 1_000_000_000
                    / u128::from(counter_frequency_hz))
                .try_into()
                .map_err(|_| "EL0 residency conversion overflow")?,
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
            expected_consumers,
            completed_consumers,
            missing_consumers,
            validated_report_chunks,
            lost_report_chunks,
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
            "exclusive CPU covers the EL0 call region and READ_WINDOW body; shared SVC and other kernel overhead remain unattributed".into(),
            "snapshot rollback execution is not part of this routing exercise".into(),
        ],
        runs: output,
    })
}
