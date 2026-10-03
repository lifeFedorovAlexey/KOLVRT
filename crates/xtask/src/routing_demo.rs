//! Optional EL0 image build and evidence decoding. Kernel sees only native operations.
use super::*;
const USER_BASE: u64 = platform_config::USER_PAYLOAD_BASE as u64;
const ELF_SEGMENT_FILE_OFFSET: usize = 8;
const ELF_SEGMENT_ADDRESS: usize = 16;
const ELF_HEADER_BYTES: usize = 64;
const REPORT_HEADER: u64 = 0x4b56_5233;
const REPORT_BENCH: u64 = 0x4245_4e43;
const REPORT_FINAL: u64 = 0x444f_4e45;
const FIXED_HEADER_WORDS: usize = 15;
const COUNTER_WORDS: usize = 9;
const BENCH_SAMPLES: usize = 128;
const BENCH_WARMUP: u64 = 16;
const SHA256_BYTES: usize = 32;
const SHA256_HEX_BYTES: usize = SHA256_BYTES * 2;
const PROFILE_GENERATION: u32 = 1;
const USER_EXITED_STATE: u64 = 2; // Native bootstrap user-result event state.
const USER_SUCCESS_EXIT: u64 = 1;
const CAPTURE_REQUIRED_SLICES: u64 = 3;
const MIN_CONFORMANCE_CHECKS: u64 = 12;
const USEFUL_WINDOW_WORDS: u64 = 2;
const BENCH_HEADER_WORDS: usize = 3; // Tag, route, sample count.
const INCLUSIVE_RECORD_BYTES: u64 = 2 * core::mem::size_of::<u16>() as u64;
const COUNTED_RECORD_BYTES: u64 = 2 * core::mem::size_of::<u32>() as u64;
const CHECKED_CONVERSIONS: u64 = 3; // Decode two fields and normalize endpoints.
const BUG_CONVERSIONS: u64 = CHECKED_CONVERSIONS + 1; // Explicit zero-count rewrite.
mod header {
    pub const TAG: usize = 0;
    pub const CONSUMER: usize = 1;
    pub const ROUTE: usize = 2;
    pub const GENERATION: usize = 3;
    pub const CONFORMANCE: usize = 4;
    pub const SUM: usize = 5;
    pub const WORDS: usize = 6;
    pub const FREQUENCY: usize = 7;
    pub const TRANSLATIONS: usize = 8;
    pub const COPIED_BYTES: usize = 9;
    pub const BACKEND_CALLS: usize = 10;
    pub const DIGEST: usize = 11;
}
mod counter {
    pub const NATIVE: usize = 0;
    pub const COMPAT: usize = 1;
    pub const BACKEND_CALLS: usize = 2;
    pub const TRANSLATIONS: usize = 3;
    pub const COPIES: usize = 4;
    pub const COPIED_BYTES: usize = 5;
    pub const CONVERSIONS: usize = 6;
    pub const ERRORS: usize = 7;
    pub const FALLBACKS: usize = 8;
}
fn field(bytes: &[u8], at: usize, len: usize) -> Result<u64> {
    let mut value = [0u8; core::mem::size_of::<u64>()];
    value[..len].copy_from_slice(
        bytes
            .get(at..at.checked_add(len).ok_or("ELF offset overflow")?)
            .ok_or("truncated ELF")?,
    );
    Ok(u64::from_le_bytes(value))
}

fn median(values: &[u64]) -> Result<u64> {
    if values.is_empty() {
        return Err("cannot summarize empty counter samples".into());
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    Ok(sorted[(sorted.len() - 1) / 2])
}

fn ticks_to_ns(ticks: u64, frequency: u64) -> Result<u64> {
    if frequency == 0 {
        return Err("counter frequency is zero".into());
    }
    (u128::from(ticks) * 1_000_000_000 / u128::from(frequency))
        .try_into()
        .map_err(|_| "counter conversion overflow".into())
}

fn extract(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() < ELF_HEADER_BYTES
        || &bytes[..4] != b"\x7fELF"
        || bytes[ELF_CLASS_OFFSET] != ELF_CLASS_64
        || field(bytes, ELF_MACHINE_OFFSET, 2)? != ELF_MACHINE_AARCH64 as u64
    {
        return Err("invalid user ELF".into());
    }
    let table = field(bytes, ELF_PROGRAM_TABLE_OFFSET, 8)? as usize;
    let width = field(bytes, ELF_PROGRAM_ENTRY_SIZE_OFFSET, 2)? as usize;
    let count = field(bytes, ELF_PROGRAM_COUNT_OFFSET, 2)? as usize;
    const ELF_PROGRAM_HEADER_BYTES: usize = 56;
    if width < ELF_PROGRAM_HEADER_BYTES
        || table
            .checked_add(width.checked_mul(count).ok_or("ELF table overflow")?)
            .is_none_or(|end| end > bytes.len())
    {
        return Err("truncated ELF program table".into());
    }
    let mut image = Vec::new();
    for index in 0..count {
        let at = table
            .checked_add(index.checked_mul(width).ok_or("ELF table overflow")?)
            .ok_or("ELF table overflow")?;
        if field(bytes, at, 4)? != ELF_PT_LOAD as u64 {
            continue;
        }
        let flags = field(bytes, at + ELF_FLAGS_OFFSET, 4)?;
        if flags & ELF_WRITE as u64 != 0 {
            return Err("user image contains writable globals; unsupported".into());
        }
        let address = field(bytes, at + ELF_SEGMENT_ADDRESS, 8)?
            .checked_sub(USER_BASE)
            .ok_or("wrong user load address")? as usize;
        let file_offset = field(bytes, at + ELF_SEGMENT_FILE_OFFSET, 8)? as usize;
        let file_bytes = field(bytes, at + ELF_FILE_SIZE_OFFSET, 8)? as usize;
        let memory_bytes = field(bytes, at + ELF_MEMORY_SIZE_OFFSET, 8)? as usize;
        let end = address
            .checked_add(memory_bytes)
            .ok_or("ELF memory overflow")?;
        if end > platform_config::USER_PAYLOAD_BYTES || file_bytes > memory_bytes {
            return Err("user image budget exceeded".into());
        }
        let source = bytes
            .get(
                file_offset
                    ..file_offset
                        .checked_add(file_bytes)
                        .ok_or("ELF file overflow")?,
            )
            .ok_or("truncated user segment")?;
        if address < image.len() {
            return Err("overlapping user segments".into());
        }
        image.resize(end, 0);
        image[address..address + file_bytes].copy_from_slice(source);
    }
    if image.is_empty() {
        return Err("empty user image".into());
    }
    Ok(image)
}
fn guest(features: &[&str], evidence: bool) -> Result<Value> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let selected = std::iter::once("guest")
        .chain(evidence.then_some("evidence"))
        .chain(features.iter().copied())
        .collect::<Vec<_>>()
        .join(",");
    let routes = [
        routing::Route::Native,
        if features.contains(&"v1") {
            routing::Route::Inclusive
        } else {
            routing::Route::Native
        },
        if features.contains(&"v2") {
            routing::Route::Counted
        } else {
            routing::Route::Native
        },
        if features.contains(&"bug") {
            routing::Route::EmptyFirst
        } else {
            routing::Route::Native
        },
    ];
    let profile = routing::Profile::new(PROFILE_GENERATION, routes)
        .map_err(|e| format!("profile creation: {e:?}"))?
        .encode();
    let digest = format!(
        "{:x}",
        Sha256::digest(&profile[..profile.len() - SHA256_BYTES])
    );
    fs::write("target/kernel/routing-profile.bin", profile)?;
    if !Command::new(&cargo)
        .env(
            "KOLVRT_ROUTING_PROFILE",
            fs::canonicalize("target/kernel/routing-profile.bin")?,
        )
        .env("KOLVRT_ROUTING_DIGEST", &digest)
        .args([
            "build",
            "--locked",
            "-p",
            "routing-demo",
            "--target",
            "aarch64-unknown-none",
            "--release",
            "--no-default-features",
            "--features",
            &selected,
        ])
        .status()?
        .success()
    {
        return Err("user image build failed".into());
    }
    let elf = fs::read("target/aarch64-unknown-none/release/routing-demo")?;
    let image = extract(&elf)?;
    fs::write("target/kernel/payload.bin", &image)?;
    let tree = Command::new(cargo)
        .args([
            "tree",
            "--locked",
            "-p",
            "routing-demo",
            "--target",
            "aarch64-unknown-none",
            "--edges",
            "normal",
            "--no-default-features",
            "--features",
            &selected,
        ])
        .output()?;
    if !tree.status.success() {
        return Err("user dependency inspection failed".into());
    }
    let dependencies = String::from_utf8(tree.stdout)?;
    if features.is_empty() && dependencies.contains("window-compat") {
        return Err("native-only user depends on adapter".into());
    }
    Ok(
        json!({"elf_sha256":format!("{:x}",Sha256::digest(&elf)),"image_sha256":format!("{:x}",Sha256::digest(&image)),"image_bytes":image.len(),"elf_bytes":elf.len(),"features":features,"evidence":evidence,"profile_sha256":digest,"profile_bytes":profile.to_vec(),"dependencies":dependencies}),
    )
}
fn reports(events: &[Value], expected: [u64; 4], dev: bool) -> Result<Value> {
    let mut routes = [routing::Route::Native; routing::CONSUMERS];
    for (route, id) in routes.iter_mut().zip(expected) {
        *route = routing::Route::parse(u8::try_from(id)?)
            .map_err(|e| format!("invalid expected route: {e:?}"))?;
    }
    let profile = routing::Profile::new(PROFILE_GENERATION, routes)
        .map_err(|e| format!("invalid expected profile: {e:?}"))?
        .encode();
    let digest = &profile[profile.len() - SHA256_BYTES..];
    let mut words = vec![Vec::<u64>::new(); platform_config::USER_PROCESSES];
    let mut finished = BTreeSet::new();
    let mut native_attempts = [0u64; platform_config::USER_PROCESSES];
    let mut process_identities = vec![None::<Value>; platform_config::USER_PROCESSES];
    let mut process_residency = vec![None::<(u64, u64, u64)>; platform_config::USER_PROCESSES];
    let mut seen_process_generations = BTreeSet::new();
    let mut terminal = false;
    for event in events {
        terminal |= event["event"] == "boot";
        if !matches!(event["event"].as_str(), Some("user-report" | "user-result")) {
            continue;
        }
        if terminal {
            return Err("user evidence after terminal boot".into());
        }
        let id = event["id"].as_u64().ok_or("missing user ID")? as usize;
        let data = words.get_mut(id).ok_or("foreign report ID")?;
        if finished.contains(&id) {
            return Err("report after user completion".into());
        }
        if event["event"] == "user-result" {
            if event["state"] != USER_EXITED_STATE
                || event["exit"] != USER_SUCCESS_EXIT
                || event["fault"] != 0
                || event["slices"]
                    .as_u64()
                    .is_none_or(|v| v < CAPTURE_REQUIRED_SLICES)
                || event["length"].as_u64() != Some(data.len() as u64)
            {
                return Err(format!("EL0 workload failed: {event}").into());
            }
            finished.insert(id);
            native_attempts[id] = event["native_attempts"]
                .as_u64()
                .ok_or("missing independent native accounting")?;
            let process_slot = event["process_slot"]
                .as_u64()
                .ok_or("missing kernel process slot")?;
            let process_generation = event["process_generation"]
                .as_u64()
                .filter(|generation| *generation != 0)
                .ok_or("missing kernel process generation")?;
            let owner_cpu = event["owner_cpu"]
                .as_u64()
                .ok_or("missing kernel process owner")?;
            let resident_pages = event["resident_pages"]
                .as_u64()
                .filter(|pages| *pages != 0)
                .ok_or("missing kernel resident frame charge")?;
            let el0_residency_ticks = event["el0_residency_ticks"]
                .as_u64()
                .filter(|ticks| *ticks != 0)
                .ok_or("missing kernel-counted EL0 residency")?;
            let native_window_service_ticks = event["native_window_service_ticks"]
                .as_u64()
                .filter(|ticks| *ticks != 0)
                .ok_or("missing kernel-counted native service time")?;
            let counter_frequency_hz = event["counter_frequency_hz"]
                .as_u64()
                .filter(|frequency| *frequency != 0)
                .ok_or("missing EL0 counter frequency")?;
            if process_slot != id as u64
                || owner_cpu != (id / platform_config::USER_PROCESSES_PER_CPU) as u64
                || !seen_process_generations.insert((process_slot, process_generation))
            {
                return Err("kernel process identity does not match the consumer slot".into());
            }
            process_identities[id] = Some(json!({
                "process_slot":process_slot,
                "process_generation":process_generation,
                "owner_cpu":owner_cpu,
                "resident_pages":resident_pages
            }));
            process_residency[id] = Some((
                el0_residency_ticks,
                native_window_service_ticks,
                counter_frequency_hz,
            ));
        } else {
            if event["offset"].as_u64() != Some(data.len() as u64) {
                return Err("missing or duplicate report chunk".into());
            }
            let chunk = event["words"].as_array().ok_or("missing report data")?;
            if chunk.is_empty()
                || data
                    .len()
                    .checked_add(chunk.len())
                    .is_none_or(|length| length > kernel_core::execution::REPORT_WORDS)
            {
                return Err("report budget exceeded".into());
            }
            for value in chunk {
                data.push(value.as_u64().ok_or("invalid report word")?);
            }
        }
    }
    if finished.len() != platform_config::USER_PROCESSES {
        return Err("missing actual user completion".into());
    }
    let mut consumers = Vec::new();
    for (id, words) in words.iter().enumerate() {
        if words.len() < FIXED_HEADER_WORDS + 1
            || words[header::TAG] != REPORT_HEADER
            || words[header::CONSUMER] != id as u64
            || words[header::ROUTE] != expected[id % routing::CONSUMERS]
            || words[header::GENERATION] != u64::from(PROFILE_GENERATION)
            || words[header::CONFORMANCE] < MIN_CONFORMANCE_CHECKS
            || words[header::SUM] == 0
            || words[header::WORDS] != USEFUL_WINDOW_WORDS
            || words[header::FREQUENCY] == 0
            || words[header::BACKEND_CALLS] != 1
            || words.last() != Some(&REPORT_FINAL)
        {
            return Err("invalid route result header".into());
        }
        for (actual, bytes) in words[header::DIGEST..FIXED_HEADER_WORDS]
            .iter()
            .zip(digest.as_chunks::<{ core::mem::size_of::<u64>() }>().0)
        {
            if *actual != u64::from_le_bytes(*bytes) {
                return Err("report profile does not match launch selection".into());
            }
        }
        if words[header::TRANSLATIONS] != u64::from(words[header::ROUTE] != 0)
            || words[header::COPIED_BYTES]
                != match words[header::ROUTE] {
                    0 => 0,
                    1 => INCLUSIVE_RECORD_BYTES,
                    _ => COUNTED_RECORD_BYTES,
                }
        {
            return Err("incorrect translation work".into());
        }
        let mut cursor = FIXED_HEADER_WORDS;
        let mut benchmarks = Vec::new();
        while cursor < words.len() - 1 {
            if !dev
                || words[cursor] != REPORT_BENCH
                || words.get(cursor + 2) != Some(&(BENCH_SAMPLES as u64))
            {
                return Err("invalid benchmark frame".into());
            }
            let route = words[cursor + 1];
            cursor += BENCH_HEADER_WORDS;
            let warmup = words
                .get(cursor..cursor + BENCH_WARMUP as usize)
                .ok_or("missing warmup")?
                .to_vec();
            cursor += BENCH_WARMUP as usize;
            let samples = words
                .get(cursor..cursor + BENCH_SAMPLES)
                .ok_or("missing samples")?
                .to_vec();
            cursor += BENCH_SAMPLES;
            let adapter_cpu_samples = words
                .get(cursor..cursor + BENCH_SAMPLES)
                .ok_or("missing exclusive EL0 CPU samples")?
                .to_vec();
            cursor += BENCH_SAMPLES;
            let native_service_cpu_samples = words
                .get(cursor..cursor + BENCH_SAMPLES)
                .ok_or("missing native service CPU samples")?
                .to_vec();
            cursor += BENCH_SAMPLES;
            let preemptions = *words.get(cursor).ok_or("missing preemption accounting")?;
            cursor += 1;
            let c = words
                .get(cursor..cursor + COUNTER_WORDS)
                .ok_or("missing counters")?;
            cursor += COUNTER_WORDS;
            let calls = BENCH_WARMUP + BENCH_SAMPLES as u64;
            if route > routing::Route::EmptyFirst as u64
                || adapter_cpu_samples.contains(&0)
                || (route == routing::Route::Native as u64
                    && native_service_cpu_samples.contains(&0))
                || (route != routing::Route::Native as u64
                    && native_service_cpu_samples.iter().any(|&ticks| ticks != 0))
                || c[counter::NATIVE] != if route == 0 { calls } else { 0 }
                || c[counter::COMPAT] != if route == 0 { 0 } else { calls }
                || c[counter::BACKEND_CALLS] != calls
                || c[counter::TRANSLATIONS] != if route == 0 { 0 } else { calls }
                || c[counter::COPIES] != c[counter::TRANSLATIONS]
                || c[counter::COPIED_BYTES]
                    != c[counter::TRANSLATIONS]
                        * if route == routing::Route::Inclusive as u64 {
                            INCLUSIVE_RECORD_BYTES
                        } else {
                            COUNTED_RECORD_BYTES
                        }
                || c[counter::CONVERSIONS]
                    != c[counter::TRANSLATIONS]
                        * if route == routing::Route::EmptyFirst as u64 {
                            BUG_CONVERSIONS
                        } else {
                            CHECKED_CONVERSIONS
                        }
                || c[counter::ERRORS] != 0
                || c[counter::FALLBACKS] != 0
            {
                return Err("route accounting mismatch".into());
            }
            let mut sorted = samples.clone();
            let [wall_median, p95, p99] = kernel_core::quantiles(&mut sorted).unwrap();
            let mean = samples.iter().map(|&v| v as f64).sum::<f64>() / samples.len() as f64;
            let variance = samples
                .iter()
                .map(|&v| (v as f64 - mean).powi(2))
                .sum::<f64>()
                / (samples.len() - 1) as f64;
            let adapter_cpu_ticks = median(&adapter_cpu_samples)?;
            let native_service_cpu_ticks = median(&native_service_cpu_samples)?;
            let exclusive_cpu_samples = adapter_cpu_samples
                .iter()
                .zip(&native_service_cpu_samples)
                .map(|(&adapter, &native)| {
                    adapter
                        .checked_add(native)
                        .ok_or_else(|| "exclusive CPU sample overflow".to_string())
                })
                .collect::<Result<Vec<_>>>()?;
            let exclusive_cpu_ticks = median(&exclusive_cpu_samples)?;
            benchmarks.push(json!({
                "route":route,
                "warmup_samples":warmup,
                "samples":samples,
                "adapter_cpu_samples":adapter_cpu_samples,
                "native_service_cpu_samples":native_service_cpu_samples,
                "median_ticks":wall_median,
                "p95_ticks":p95,
                "p99_ticks":p99,
                "mean_ticks":mean,
                "sample_variance_ticks_squared":variance,
                "standard_deviation_ticks":variance.sqrt(),
                "observed_preemptions":preemptions,
                "adapter_cpu_ns":ticks_to_ns(adapter_cpu_ticks, words[header::FREQUENCY])?,
                "native_service_cpu_ns":ticks_to_ns(native_service_cpu_ticks, words[header::FREQUENCY])?,
                "exclusive_cpu_ns":ticks_to_ns(exclusive_cpu_ticks, words[header::FREQUENCY])?,
                "throughput":null,
                "counters":c,
                "allocations":0,
                "locks":0,
                "fallbacks":0,
                "memory":"stack-owned bounded synchronous request"
            }));
        }
        if dev && benchmarks.len() != expected.iter().copied().collect::<BTreeSet<_>>().len() {
            return Err("missing benchmark route".into());
        }
        if dev {
            let actual: BTreeSet<_> = benchmarks
                .iter()
                .map(|b| b["route"].as_u64().unwrap())
                .collect();
            if actual != expected.iter().copied().collect() || actual.len() != benchmarks.len() {
                return Err("duplicate or missing benchmark version".into());
            }
        }
        const BASE_NATIVE_ATTEMPTS: u64 = 7; // Pre-capture/width/bounds denials, oracle, initial route and corrected empty behavior.
        const BUG_NATIVE_ATTEMPTS: u64 = 2; // Historical zero count and native one-word oracle.
        const DEV_NATIVE_ATTEMPTS: u64 = 2; // Switched native route and repeated empty oracle.
        let expected_attempts = BASE_NATIVE_ATTEMPTS
            + if expected.contains(&(routing::Route::EmptyFirst as u64)) {
                BUG_NATIVE_ATTEMPTS
            } else {
                0
            }
            + if dev { DEV_NATIVE_ATTEMPTS } else { 0 }
            + benchmarks
                .iter()
                .map(|b| b["counters"][counter::BACKEND_CALLS].as_u64().unwrap())
                .sum::<u64>();
        if native_attempts[id] != expected_attempts {
            return Err(
                "EL0 accounting disagrees with independently counted native admissions".into(),
            );
        }
        let process_identity = process_identities[id]
            .as_ref()
            .ok_or("consumer is missing its kernel-owned process identity")?;
        let (el0_residency_ticks, native_window_service_ticks, counter_frequency_hz) =
            process_residency[id].ok_or("consumer is missing kernel process CPU accounting")?;
        consumers.push(json!({"id":id,"cpu":id / platform_config::USER_PROCESSES_PER_CPU,"process_identity":process_identity,"el0_residency_ticks":el0_residency_ticks,"native_window_service_ticks":native_window_service_ticks,"counter_frequency_hz":counter_frequency_hz,"route":words[header::ROUTE],"generation":words[header::GENERATION],"conformance_checks":words[header::CONFORMANCE],"oracle_sum":words[header::SUM],"frequency":words[header::FREQUENCY],"native_attempts":native_attempts[id],"profile_digest_words":&words[header::DIGEST..FIXED_HEADER_WORDS],"benchmarks":benchmarks}));
    }
    Ok(json!(consumers))
}
pub fn run(args: &[String]) -> Result<()> {
    if let [command, routes, path] = args
        && command == "profile"
    {
        let selected: Vec<_> = routes
            .split(',')
            .map(|name| match name {
                "native" => Ok(routing::Route::Native),
                "inclusive" => Ok(routing::Route::Inclusive),
                "counted" => Ok(routing::Route::Counted),
                "bug" => Ok(routing::Route::EmptyFirst),
                _ => Err("unsupported route name"),
            })
            .collect::<std::result::Result<_, _>>()?;
        let routes: [routing::Route; routing::CONSUMERS] = selected
            .try_into()
            .map_err(|_| "profile requires four explicit consumers")?;
        let bytes = routing::Profile::new(PROFILE_GENERATION, routes)
            .map_err(|e| format!("invalid profile: {e:?}"))?
            .encode();
        fs::write(path, bytes)?;
        println!(
            "Profile: {path}; schema={}; trusted expected digest={:x}; source identities are not executable signatures",
            routing::PROFILE_SCHEMA,
            Sha256::digest(&bytes[..bytes.len() - SHA256_BYTES])
        );
        return Ok(());
    }
    if let [command, path, expected] = args
        && command == "validate"
    {
        if expected.len() != SHA256_HEX_BYTES || !expected.bytes().all(|v| v.is_ascii_hexdigit()) {
            return Err("expected SHA-256 must contain 64 hex characters".into());
        }
        let mut digest = [0u8; SHA256_BYTES];
        for (index, value) in digest.iter_mut().enumerate() {
            *value = u8::from_str_radix(&expected[index * 2..index * 2 + 2], 16)?;
        }
        let profile = routing::Profile::decode(&fs::read(path)?, digest)
            .map_err(|e| format!("profile rejected: {e:?}"))?;
        println!(
            "Validated generation {}: {:?}",
            profile.generation(),
            profile.routes()
        );
        return Ok(());
    }
    if matches!(
        args.first().map(String::as_str),
        Some("status" | "inspect" | "compare" | "top")
    ) {
        let command = &args[0];
        let record = read_json("research/results/routing-phase2.json")?;
        let consumers = record["runs"][0]["consumers"]
            .as_array()
            .ok_or("missing verified consumers")?;
        if command == "status" || command == "top" {
            for run in record["runs"].as_array().ok_or("missing runs")? {
                let list = run["consumers"].as_array().ok_or("missing consumers")?;
                let native = list.iter().filter(|c| c["route"] == 0).count();
                println!(
                    "{}: native consumers {}; compatibility consumers {}; observed concurrent CPUs {}",
                    run["profile"].as_str().unwrap_or("unknown"),
                    native,
                    list.len() - native,
                    platform_config::ACTIVE_CPUS
                );
                for route in 1..=3 {
                    let bindings = list.iter().filter(|c| c["route"] == route).count();
                    println!(
                        "  {}: {} declared bindings",
                        routing::Route::parse(route).unwrap().name(),
                        bindings
                    );
                }
            }
        } else {
            let id: usize = args
                .get(1)
                .ok_or("inspect/compare requires consumer ID")?
                .parse()?;
            let consumer = consumers.get(id).ok_or("unknown consumer")?;
            let route =
                routing::Route::parse(consumer["route"].as_u64().ok_or("missing route")? as u8)
                    .unwrap();
            println!(
                "Consumer {id}, CPU {}, generation {}: [{}] {}",
                consumer["cpu"],
                consumer["generation"],
                if route == routing::Route::Native {
                    "NATIVE"
                } else {
                    "COMPAT"
                },
                route.name()
            );
            println!(
                "Reason: {}; native replacement: {}; migration: docs/kernel/routing.md",
                route.reason(),
                routing::Route::Native.name()
            );
            println!(
                "Observations are isolated benchmark admissions, not production workload cost. No scalar score."
            );
            for benchmark in consumer["benchmarks"]
                .as_array()
                .ok_or("missing DEV benchmarks")?
            {
                println!(
                    "  route {}: median {} / p95 {} / p99 {} timer ticks; raw samples retained; CPU attribution unavailable",
                    benchmark["route"],
                    benchmark["median_ticks"],
                    benchmark["p95_ticks"],
                    benchmark["p99_ticks"]
                );
            }
        }
        return Ok(());
    }
    if let [command, control] = args
        && command == "control"
    {
        let feature = match control.as_str() {
            "profile" => "profile-negative",
            "adapter" => "adapter-negative",
            "report" => "report-negative",
            _ => return Err("unknown routing control".into()),
        };
        guest(&["v1", feature], true)?;
        let elf = build(true, false, Some("boot-payload"), true)?;
        execute_mode(&elf, false, true, true)?;
        let events =
            serde_json::from_slice::<Vec<Value>>(&fs::read(elf.with_extension("results.json"))?)?;
        reports(&events, [0, 1, 0, 0], false)?;
        return Err("negative routing control unexpectedly passed".into());
    }
    if args != ["test"] {
        return Err("usage: cargo xtask routing test".into());
    }
    let starting_sources = super::source_inventory()?;
    super::audit()?;
    let mut runs = Vec::new();
    for (label, prod, features, routes) in [
        (
            "dev-full",
            false,
            vec!["dev", "v1", "v2", "bug"],
            [0, 1, 2, 3],
        ),
        ("prod-full", true, vec!["v1", "v2", "bug"], [0, 1, 2, 3]),
        ("prod-v1", true, vec!["v1"], [0, 1, 0, 0]),
        ("prod-v2", true, vec!["v2"], [0, 0, 2, 0]),
        ("prod-bug", true, vec!["bug"], [0, 0, 0, 3]),
        ("prod-native", true, vec![], [0, 0, 0, 0]),
    ] {
        let artifact = guest(&features, true)?;
        let elf = build(prod, false, Some("boot-payload"), true)?;
        let retained = PathBuf::from(format!("target/kernel/routing-{label}.elf"));
        fs::copy(&elf, &retained)?;
        execute_mode(&retained, false, true, true)?;
        let events: Vec<Value> =
            serde_json::from_slice(&fs::read(retained.with_extension("results.json"))?)?;
        let consumers = reports(&events, routes, !prod)?;
        let report_chunks = events
            .iter()
            .filter(|event| event["event"] == "user-report")
            .count();
        let expected_consumers = platform_config::USER_PROCESSES as u64;
        let completed_consumers = consumers
            .as_array()
            .ok_or("missing parsed consumers")?
            .len() as u64;
        let missing_consumers = expected_consumers.saturating_sub(completed_consumers);
        runs.push(json!({"profile":label,"user_artifact":artifact,"kernel_build":read_json(elf.with_file_name(if prod {"prod-boot-payload-build.json"} else {"dev-boot-payload-build.json"}))?,"run":read_json(retained.with_extension("run.json"))?,"scope_accounting":{"expected_consumers":expected_consumers,"completed_consumers":completed_consumers,"missing_consumers":missing_consumers,"validated_report_chunks":report_chunks,"lost_report_chunks":0},"consumers":consumers,"events":events}));
        println!(
            "routing: {label} verified on {} EL0 processes",
            platform_config::USER_PROCESSES
        );
    }
    let stripped = guest(&["v1"], false)?;
    let elf = build(true, false, Some("boot-payload"), false)?;
    let retained = PathBuf::from("target/kernel/routing-prod-stripped.elf");
    fs::copy(&elf, &retained)?;
    execute_mode(&retained, false, false, true)?;
    let stripped_kernel = read_json("target/kernel/prod-boot-payload-build.json")?;
    for (control, marker) in [
        ("profile", "EL0 workload failed"),
        ("adapter", "EL0 workload failed"),
        ("report", "incorrect translation work"),
    ] {
        let output = Command::new(env::current_exe()?)
            .args(["routing", "control", control])
            .output()?;
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        fs::write(
            format!("target/kernel/routing-negative-{control}.log"),
            &text,
        )?;
        if output.status.success() || !text.contains(marker) {
            return Err(format!("routing control failed to detect {control}: {text}").into());
        }
    }
    if starting_sources != super::source_inventory()? {
        return Err("sources changed during routing run".into());
    }
    let result = json!({"schema_version":4,"scope":"real fixed-affinity EL0 consumers; kernel-owned ProcessId slot/generation, CPU owner, resident frame charge and EL0 counter residency are joined to the exact loaded image digest and raw route report; native kernel contains no route or legacy decoder","runs":runs,"negative_controls":["profile integrity", "adapter fault containment", "accounting report corruption"],"stripped_production":{"user_artifact":stripped,"diagnostic_features":[],"kernel_build":stripped_kernel,"verification":"human-console boot smoke; semantic checks use matched PROD evidence image"},"source_files":starting_sources,"claim":"TCG timer latency and EL0 residency-counter observations; exception-entry overhead remains in EL0 residency; tail uncertainty and physical hardware unverified"});
    fs::write(
        "target/kernel/routing-results.json",
        serde_json::to_string_pretty(&result)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_image_rejects_truncated_overlapping_writable_and_oversized_input() {
        const PROGRAM_BYTES: usize = 56;
        const DATA_OFFSET: usize = ELF_HEADER_BYTES + PROGRAM_BYTES;
        let mut elf = vec![0u8; DATA_OFFSET + 4];
        elf[..4].copy_from_slice(b"\x7fELF");
        elf[ELF_CLASS_OFFSET] = ELF_CLASS_64;
        let set = |bytes: &mut [u8], at: usize, value: u64, len: usize| {
            bytes[at..at + len].copy_from_slice(&value.to_le_bytes()[..len])
        };
        set(&mut elf, ELF_MACHINE_OFFSET, ELF_MACHINE_AARCH64 as u64, 2);
        set(
            &mut elf,
            ELF_PROGRAM_TABLE_OFFSET,
            ELF_HEADER_BYTES as u64,
            8,
        );
        set(
            &mut elf,
            ELF_PROGRAM_ENTRY_SIZE_OFFSET,
            PROGRAM_BYTES as u64,
            2,
        );
        set(&mut elf, ELF_PROGRAM_COUNT_OFFSET, 1, 2);
        set(&mut elf, ELF_HEADER_BYTES, ELF_PT_LOAD as u64, 4);
        set(
            &mut elf,
            ELF_HEADER_BYTES + ELF_FLAGS_OFFSET,
            ELF_EXECUTE as u64,
            4,
        );
        set(
            &mut elf,
            ELF_HEADER_BYTES + ELF_SEGMENT_FILE_OFFSET,
            DATA_OFFSET as u64,
            8,
        );
        set(
            &mut elf,
            ELF_HEADER_BYTES + ELF_SEGMENT_ADDRESS,
            USER_BASE,
            8,
        );
        set(&mut elf, ELF_HEADER_BYTES + ELF_FILE_SIZE_OFFSET, 4, 8);
        set(&mut elf, ELF_HEADER_BYTES + ELF_MEMORY_SIZE_OFFSET, 4, 8);
        assert_eq!(extract(&elf).unwrap(), [0; 4]);
        for length in 0..elf.len() {
            assert!(extract(&elf[..length]).is_err());
        }
        let mut bad = elf.clone();
        set(
            &mut bad,
            ELF_HEADER_BYTES + ELF_FLAGS_OFFSET,
            (ELF_WRITE | ELF_EXECUTE) as u64,
            4,
        );
        assert!(extract(&bad).is_err());
        let mut bad = elf.clone();
        set(
            &mut bad,
            ELF_HEADER_BYTES + ELF_MEMORY_SIZE_OFFSET,
            platform_config::USER_PAYLOAD_BYTES as u64 + 1,
            8,
        );
        assert!(extract(&bad).is_err());
        let mut bad = elf.clone();
        set(&mut bad, ELF_PROGRAM_COUNT_OFFSET, 2, 2);
        assert!(extract(&bad).is_err());
        let mut bad = elf;
        set(
            &mut bad,
            ELF_HEADER_BYTES + ELF_SEGMENT_ADDRESS,
            USER_BASE - 1,
            8,
        );
        assert!(extract(&bad).is_err());
    }
    #[test]
    fn report_decoder_rejects_foreign_identity_missing_and_duplicate_chunks() {
        assert!(reports(&[], [0, 1, 2, 3], true).is_err());
        assert!(reports(&[json!({"event":"user-report","id":platform_config::USER_PROCESSES,"offset":0,"words":[1]})],[0,1,2,3],true).is_err());
        assert!(
            reports(
                &[json!({"event":"user-report","id":0,"offset":1,"words":[1]})],
                [0, 1, 2, 3],
                true
            )
            .is_err()
        );
        let chunk = json!({"event":"user-report","id":0,"offset":0,"words":[1]});
        assert!(reports(&[chunk.clone(), chunk], [0, 1, 2, 3], true).is_err());
    }
}
