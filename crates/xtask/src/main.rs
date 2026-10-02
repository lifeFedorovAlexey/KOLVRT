mod output;
#[cfg(feature = "route-tools")]
mod routing_demo;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
// ELF64 little-endian header and PT_LOAD fields.
const ELF_CLASS_OFFSET: usize = 4;
const ELF_CLASS_64: u8 = 2;
const ELF_MACHINE_OFFSET: usize = 18;
const ELF_MACHINE_AARCH64: usize = 183;
const ELF_PROGRAM_TABLE_OFFSET: usize = 32;
const ELF_PROGRAM_ENTRY_SIZE_OFFSET: usize = 54;
const ELF_PROGRAM_COUNT_OFFSET: usize = 56;
const ELF_PT_LOAD: u32 = 1;
const ELF_FLAGS_OFFSET: usize = 4;
const ELF_FILE_SIZE_OFFSET: usize = 32;
const ELF_MEMORY_SIZE_OFFSET: usize = 40;
const ELF_EXECUTE: u32 = 1;
const ELF_WRITE: u32 = 2;
const UNSAFE_CONTEXT_PRECEDING_LINES: usize = 4;
const QEMU_TIMEOUT: Duration = Duration::from_secs(30);
const QEMU_POLL_INTERVAL: Duration = Duration::from_millis(20);
const NEGATIVE_CONTROLS: &[(&str, &str)] = &[
    ("--negative-control", "negative_control"),
    ("--panic-control", "panic reporting negative control"),
    ("--ownership-control", "physical pool already owned"),
    ("--retained-mapping-control", "mapped frame release"),
    ("--secondary-panic-control", "secondary CPU failure"),
    ("--retirement-control", "retiring frame release"),
    ("--shootdown-control", "remote TLB acknowledgement timeout"),
    ("--remote-tlbi-control", "secondary CPU failure"),
    ("--user-context-control", "user register context lost"),
    ("--user-root-control", "user address-space alias leaked"),
    (
        "--user-retirement-control",
        "user address space retains frame",
    ),
];
#[path = "../../kernel/src/platform/config.rs"]
#[allow(dead_code)] // Layout constants are consumed by the target kernel.
mod platform_config;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const TESTS: &[&str] = &[
    "boot_el1",
    "uart_mmio",
    "exception_vectors",
    "physical_discovery",
    "physical_allocator",
    "allocation_free",
    "physical_reuse",
    "physical_exhaustion",
    "page_map",
    "invalid_mapping_rejection",
    "page_unmap",
    "permissions",
    "execute_never",
    "mmu",
    "heap",
    "heap_exhaustion",
    "locking_exclusion",
    "locking_release",
    "monotonic_time",
    "interrupt_masking",
    "interrupt_delivery",
    "timer_rearm",
    "irq_simd_context",
    "smp_secondary_boot",
    "smp_cpu_identity",
    "smp_separate_stacks",
    "smp_ipi_forward",
    "smp_ipi_reverse",
    "smp_ipi_repeated",
    "smp_lock_publication",
    "smp_mapping_visible",
    "smp_retirement_pending",
    "smp_no_premature_reuse",
    "smp_remote_ack",
    "smp_remote_tlb_invalidation",
    "smp_safe_reuse",
    "smp_simultaneous_timers",
    "smp_percpu_independent",
    "smp_orderly_shutdown",
    "el0_processes",
    "el0_timer_switches",
    "el0_user_stacks",
    "el0_memory_isolation",
    "el0_fault_containment",
    "el0_quiescent_reclamation",
    "el0_context_preservation",
    "el0_smp_ownership",
    "el0_kernel_memory_rejected",
    "el0_foreign_memory_rejected",
    "el0_code_write_rejected",
    "el0_stack_guard",
    "el0_data_execute_rejected",
    "el0_privileged_instruction_rejected",
];
fn main() {
    if let Err(e) = run() {
        eprintln!("xtask: {e}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    env::set_current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))?;
    fs::create_dir_all("target/kernel")?;
    let args = output::color_arguments(env::args().skip(1).collect())?;
    match args.first().map(String::as_str) {
        Some("audit") => audit(),
        Some("routing") => {
            #[cfg(feature = "route-tools")]
            { routing_demo::run(&args[1..]) }
            #[cfg(not(feature = "route-tools"))]
            {
                let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
                    .args(["run", "--locked", "-p", "xtask", "--target-dir", "target/route-tools", "--features", "route-tools", "--"])
                    .args(&args).status()?;
                if status.success() { Ok(()) } else { Err("routing tool command failed".into()) }
            }
        }
        Some("compare") if args.len() == 3 => compare_measurements(Path::new(&args[1]), Path::new(&args[2])),
        Some("build") => {
            build(args.iter().any(|a| a == "--prod"), false, None, false)?;
            Ok(())
        }
        Some("run") => {
            let machine = args.iter().any(|a| a == "--machine");
            if args.iter().skip(1).any(|a| a != "--prod" && a != "--machine") { return Err("usage: cargo xtask run [--prod] [--machine]".into()); }
            let elf = build(args.iter().any(|a| a == "--prod"), false, None, machine)?;
            execute(&elf, false, machine)
        }
        Some("debug") => {
            let elf = build(false, false, None, false)?;
            let q = qemu()?;
            Command::new(q)
                .args(qemu_args(&elf))
                .args(["-S", "-gdb", "tcp:127.0.0.1:1234"])
                .status()?;
            Ok(())
        }
        Some("test") => {
            for (flag, feature) in [("--secondary-panic-control", "secondary-panic-test"), ("--retirement-control", "retirement-negative"), ("--shootdown-control", "shootdown-negative"), ("--remote-tlbi-control", "remote-tlbi-negative"), ("--user-context-control", "user-context-negative"), ("--user-root-control", "user-root-negative"), ("--user-retirement-control", "user-retirement-negative")] {
                if args.iter().any(|a| a == flag) { let elf = build(false, true, Some(feature), true)?; return execute(&elf, true, true); }
            }
            if args.iter().any(|a| a == "--negative-control") {
                let elf = build(false, true, Some("negative-test"), true)?;
                return execute(&elf, true, true);
            }
            if args.iter().any(|a| a == "--panic-control") {
                let elf = build(false, true, Some("panic-test"), true)?;
                return execute(&elf, true, true);
            }
            if args.iter().any(|a| a == "--ownership-control") {
                let elf = build(false, true, Some("ownership-test"), true)?;
                return execute(&elf, true, true);
            }
            if args.iter().any(|a| a == "--retained-mapping-control") {
                let elf = build(false, true, Some("retained-mapping-test"), true)?;
                return execute(&elf, true, true);
            }
            let starting_sources = source_inventory()?;
            qemu()?;
            audit()?;
            for prod in [false, true] {
                let elf = build(prod, true, None, true)?;
                execute(&elf, true, true)?;
                let elf = build(prod, false, None, true)?;
                execute(&elf, false, true)?;
            }
            for &(flag, marker) in NEGATIVE_CONTROLS {
                let output = Command::new(env::current_exe()?)
                    .args(["test", flag])
                    .output()?;
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                fs::write(format!("target/kernel/{flag}.log"), &text)?;
                if output.status.success() || !text.contains(marker) {
                    return Err(format!(
                        "host failure propagation control did not fail correctly: {flag}"
                    )
                    .into());
                }
                println!(
                    "host failure propagation verified: {flag} exit={:?}",
                    output.status.code()
                );
            }
            println!("EL0 foundation kernel matrix passed (two active CPUs; compatibility not connected).");
            let label = match args.as_slice() {
                [_] => None,
                [_, flag, label] if flag == "--record" => Some(label.as_str()),
                _ => return Err("usage: cargo xtask test [--record LABEL]".into()),
            };
            archive_measurements(label, &starting_sources)?;
            Ok(())
        }
        _ => Err("usage: cargo xtask test [--record LABEL] | compare BASELINE CANDIDATE | build [--prod] | run [--prod] [--machine] | audit | debug".into()),
    }
}
fn build(prod: bool, tests: bool, extra: Option<&str>, machine: bool) -> Result<PathBuf> {
    let mut c = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    c.args([
        "build",
        "--locked",
        "-p",
        "kolvrt-kernel",
        "--target",
        "aarch64-unknown-none",
        "--no-default-features",
    ]);
    if prod {
        c.arg("--release");
    }
    let mut features = Vec::new();
    if machine {
        features.push("machine-events");
    }
    if !prod {
        features.push("diagnostics");
    }
    if tests {
        features.push("kernel-tests");
    }
    if let Some(e) = extra {
        features.push(e);
        if e == "boot-payload" {
            c.env(
                "KOLVRT_BOOT_PAYLOAD",
                fs::canonicalize("target/kernel/payload.bin")?,
            );
        }
    }
    if !features.is_empty() {
        c.args(["--features", &features.join(",")]);
    }
    if !c.status()?.success() {
        return Err("kernel build failed".into());
    }
    let source = format!(
        "target/aarch64-unknown-none/{}/kolvrt-kernel",
        if prod { "release" } else { "debug" }
    );
    let name = format!(
        "{}-{}",
        if prod { "prod" } else { "dev" },
        extra.unwrap_or(if tests { "tests" } else { "boot" })
    );
    let dest = PathBuf::from(format!("target/kernel/{name}.elf"));
    fs::copy(source, &dest)?;
    let bytes = fs::read(&dest)?;
    let u16at = |o: usize| {
        u16::from_le_bytes(
            bytes[o..o + core::mem::size_of::<u16>()]
                .try_into()
                .unwrap(),
        ) as usize
    };
    let u64at = |o: usize| {
        u64::from_le_bytes(
            bytes[o..o + core::mem::size_of::<u64>()]
                .try_into()
                .unwrap(),
        )
    };
    if &bytes[..b"\x7fELF".len()] != b"\x7fELF"
        || bytes[ELF_CLASS_OFFSET] != ELF_CLASS_64
        || u16at(ELF_MACHINE_OFFSET) != ELF_MACHINE_AARCH64
    {
        return Err("wrong ELF architecture".into());
    }
    let mut loaded = 0;
    let mut memory = 0;
    let mut wx = false;
    for i in 0..u16at(ELF_PROGRAM_COUNT_OFFSET) {
        let o = u64at(ELF_PROGRAM_TABLE_OFFSET) as usize + i * u16at(ELF_PROGRAM_ENTRY_SIZE_OFFSET);
        let kind = u32::from_le_bytes(
            bytes[o..o + core::mem::size_of::<u32>()]
                .try_into()
                .unwrap(),
        );
        if kind == ELF_PT_LOAD {
            let flags = u32::from_le_bytes(
                bytes[o + ELF_FLAGS_OFFSET..o + ELF_FLAGS_OFFSET + core::mem::size_of::<u32>()]
                    .try_into()
                    .unwrap(),
            );
            loaded += u64at(o + ELF_FILE_SIZE_OFFSET);
            memory += u64at(o + ELF_MEMORY_SIZE_OFFSET);
            wx |= flags & (ELF_WRITE | ELF_EXECUTE) == (ELF_WRITE | ELF_EXECUTE);
        }
    }
    if wx {
        return Err("ELF has writable executable segment".into());
    }
    if prod
        && !tests
        && bytes
            .windows(b"negative_control".len())
            .any(|s| s == b"negative_control")
    {
        return Err("production includes negative test".into());
    }
    let report = json!({"artifact":dest,"elf_bytes":bytes.len(),"load_bytes":loaded,"memory_bytes":memory,"features":features,"sha256":format!("{:x}",Sha256::digest(&bytes)),"compiler":"1.99.0","target":"aarch64-unknown-none"});
    fs::write(
        format!("target/kernel/{name}-build.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    print!(
        "{}",
        output::console_text(
            &format!(
                "[OK] build: {} ({} bytes ELF; {} bytes loaded)\n",
                dest.display(),
                bytes.len(),
                loaded
            ),
            output::stdout_color()
        )
    );
    Ok(dest)
}
fn qemu() -> Result<PathBuf> {
    let path = env::var_os("QEMU_AARCH64")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if cfg!(windows) {
                PathBuf::from(".toolchains/qemu/bin/qemu-system-aarch64.exe")
            } else {
                PathBuf::from("qemu-system-aarch64")
            }
        });
    let version = Command::new(&path)
        .arg("--version")
        .output()
        .map_err(|e| format!("QEMU 10.1.0 unavailable at {}: {e}", path.display()))?;
    let version = String::from_utf8(version.stdout)?;
    if !version.starts_with("QEMU emulator version 10.1.0 ")
        && !version.starts_with("QEMU emulator version 10.1.0\n")
    {
        return Err(format!("QEMU pin mismatch: {version}").into());
    }
    Ok(path)
}
fn qemu_args(elf: &Path) -> Vec<String> {
    vec![
        "-machine".into(),
        "virt-10.1,gic-version=3,virtualization=on,its=off,dtb-randomness=off".into(),
        "-cpu".into(),
        "cortex-a57".into(),
        "-accel".into(),
        "tcg".into(),
        "-smp".into(),
        platform_config::CONFIGURED_CPUS.to_string(),
        "-m".into(),
        format!("{}M", platform_config::RAM_BYTES / (1024 * 1024)),
        "-display".into(),
        "none".into(),
        "-serial".into(),
        "stdio".into(),
        "-monitor".into(),
        "none".into(),
        "-nic".into(),
        "none".into(),
        "-no-reboot".into(),
        "-kernel".into(),
        elf.to_string_lossy().into(),
    ]
}
fn execute(elf: &Path, tests: bool, machine: bool) -> Result<()> {
    execute_mode(elf, tests, machine, false)
}
fn execute_mode(elf: &Path, tests: bool, machine: bool, payload: bool) -> Result<()> {
    let log = elf.with_extension("log");
    let err = elf.with_extension("stderr");
    // Never leave an earlier run's successful evidence beside a failed/human run.
    let result = elf.with_extension("results.json");
    if result.exists() {
        fs::remove_file(&result)?;
    }
    let emulator = qemu()?;
    let version = Command::new(&emulator).arg("--version").output()?;
    fs::write(
        elf.with_extension("run.json"),
        serde_json::to_string_pretty(
            &json!({"qemu_version":String::from_utf8(version.stdout)?,"arguments":qemu_args(elf),"active_cpus":platform_config::ACTIVE_CPUS,"configured_cpus":platform_config::CONFIGURED_CPUS,"elf_sha256":format!("{:x}",Sha256::digest(fs::read(elf)?)),"timeout_seconds":QEMU_TIMEOUT.as_secs(),"accelerator":"TCG","measurement_claim":"emulator timer ticks; not hardware throughput"}),
        )?,
    )?;
    let mut child = Command::new(emulator)
        .args(qemu_args(elf))
        .stdin(Stdio::null())
        .stdout(fs::File::create(&log)?)
        .stderr(fs::File::create(&err)?)
        .spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                return Err(format!("QEMU failed: {}", fs::read_to_string(&err)?).into());
            }
            break;
        }
        if start.elapsed() > QEMU_TIMEOUT {
            child.kill()?;
            child.wait()?;
            return Err(format!("QEMU timeout: {}", fs::read_to_string(&log)?).into());
        }
        thread::sleep(QEMU_POLL_INTERVAL);
    }
    let text = fs::read_to_string(&log)?;
    print!("{}", output::console_text(&text, output::stdout_color()));
    if !machine {
        if text.contains("@KOLVRT") || text.contains("[FAIL]") {
            return Err("human console failure or unexpected machine record".into());
        }
        return Ok(());
    }
    let events = output::parse(&text)?;
    fs::write(
        elf.with_extension("results.json"),
        serde_json::to_string_pretty(&events)?,
    )?;
    let native_events: Vec<_> = events
        .iter()
        .filter(|event| {
            !payload || !matches!(event["event"].as_str(), Some("user-report" | "user-result"))
        })
        .cloned()
        .collect();
    output::validate(&native_events, tests, TESTS, platform_config::ACTIVE_CPUS)?;

    Ok(())
}
fn walk(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for e in fs::read_dir(path)? {
        let p = e?.path();
        if p.is_dir() {
            walk(&p, out)?;
        } else {
            out.push(p);
        }
    }
    Ok(())
}
fn source_inventory() -> Result<Value> {
    let mut files = Vec::new();
    for directory in [
        "crates/kernel",
        "crates/kernel-core",
        "crates/xtask",
        "crates/routing",
        "crates/window-compat",
        "crates/routing-demo",
    ] {
        if Path::new(directory).exists() {
            walk(Path::new(directory), &mut files)?;
        }
    }
    files.extend(
        [
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            ".cargo/config.toml",
            "assets/branding/boot-logo.txt",
        ]
        .map(PathBuf::from),
    );
    files.sort();
    let mut inventory = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file)?.replace("\r\n", "\n");
        inventory.push(json!({"path":file.to_string_lossy().replace('\\',"/"),"sha256_lf":format!("{:x}",Sha256::digest(text.as_bytes()))}));
    }
    Ok(json!(inventory))
}
fn read_json(path: impl AsRef<Path>) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn validate_samples(measurement: &Value) -> Result<()> {
    if measurement["units"] != "timer_ticks"
        || measurement["frequency"].as_u64().is_none_or(|v| v == 0)
    {
        return Err("invalid measurement units/frequency".into());
    }
    let mut samples = measurement["samples"]
        .as_array()
        .ok_or("missing raw samples")?
        .iter()
        .map(|v| v.as_u64().ok_or("invalid sample"))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if measurement["iterations"].as_u64() != Some(samples.len() as u64)
        || measurement["warmup"].as_u64().is_none()
    {
        return Err("invalid iteration/warmup metadata".into());
    }
    let quantiles = kernel_core::quantiles(&mut samples).ok_or("empty samples")?;
    for (name, expected) in ["median", "p95", "p99"].into_iter().zip(quantiles) {
        if measurement[name].as_u64() != Some(expected) {
            return Err("quantile disagrees with raw samples".into());
        }
    }
    Ok(())
}
fn archive_measurements(label: Option<&str>, starting_sources: &Value) -> Result<()> {
    if source_inventory()? != *starting_sources {
        return Err("source changed during kernel matrix; measurement rejected".into());
    }
    let mut profiles = serde_json::Map::new();
    for profile in ["dev", "prod"] {
        let events = read_json(format!("target/kernel/{profile}-tests.results.json"))?;
        let measurement = events
            .as_array()
            .ok_or("invalid event list")?
            .iter()
            .find(|event| event["event"] == "measurement")
            .ok_or("missing real kernel measurement")?
            .clone();
        validate_samples(&measurement)?;
        profiles.insert(profile.into(), json!({"measurement":measurement,"test_build":read_json(format!("target/kernel/{profile}-tests-build.json"))?,"boot_build":read_json(format!("target/kernel/{profile}-boot-build.json"))?,"run":read_json(format!("target/kernel/{profile}-tests.run.json"))?,"test_events":events}));
    }
    let revision = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let record = json!({"schema_version":1,"label":label.unwrap_or("latest"),"timestamp_unix_ms":timestamp,"git_commit":String::from_utf8(revision.stdout)?.trim(),"worktree_dirty":!dirty.stdout.is_empty(),"source_files":starting_sources,"profiles":profiles,"correctness":{"matrix":"passed","tests_per_profile":TESTS.len(),"negative_host_controls":NEGATIVE_CONTROLS.len()},"claim":"TCG timer observations; not proof of fastest algorithm or hardware throughput","method_review":"docs/architecture/implementation-review.md"});
    let text = serde_json::to_string_pretty(&record)?;
    fs::write("target/kernel/measurement.json", &text)?;
    if let Some(label) = label {
        if label.is_empty()
            || label.len() > 64
            || !label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(
                "record label must be 1..64 ASCII letters/digits/hyphens/underscores".into(),
            );
        }
        fs::create_dir_all("research/measurements/runs")?;
        let digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(starting_sources)?)
        );
        let path = format!(
            "research/measurements/runs/{timestamp}-{label}-{}.json",
            &digest[..12]
        );
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(text.as_bytes())?;
        println!("Recorded verified kernel measurements: {path}");
    }
    Ok(())
}
fn compare_measurements(baseline: &Path, candidate: &Path) -> Result<()> {
    let baseline = read_json(baseline)?;
    let candidate = read_json(candidate)?;
    if baseline["schema_version"] != 1 || candidate["schema_version"] != 1 {
        return Err("unknown measurement schema".into());
    }
    for profile in ["dev", "prod"] {
        let old = &baseline["profiles"][profile];
        let new = &candidate["profiles"][profile];
        for record in [&baseline, &candidate] {
            if record["correctness"]["matrix"] != "passed" {
                return Err("comparison requires recorded correctness checks".into());
            }
        }
        for field in [
            "qemu_version",
            "arguments",
            "active_cpus",
            "configured_cpus",
            "accelerator",
        ] {
            if old["run"][field].is_null() || old["run"][field] != new["run"][field] {
                return Err(format!("incomparable {profile} environment: {field}").into());
            }
        }
        for field in ["compiler", "target", "features"] {
            if old["test_build"][field].is_null()
                || old["test_build"][field] != new["test_build"][field]
            {
                return Err(format!("incomparable build: {field}").into());
            }
        }
        let old = &old["measurement"];
        let new = &new["measurement"];
        validate_samples(old)?;
        validate_samples(new)?;
        for field in ["scope", "units", "frequency", "warmup", "iterations"] {
            if old[field].is_null() || old[field] != new[field] {
                return Err(format!("incomparable measurement: {field}").into());
            }
        }
        let mut deltas = serde_json::Map::new();
        for quantile in ["median", "p95", "p99"] {
            let a = old[quantile].as_u64().unwrap();
            let b = new[quantile].as_u64().unwrap();
            deltas.insert(quantile.into(),json!({"baseline_ticks":a,"candidate_ticks":b,"delta_ticks":(i128::from(b)-i128::from(a)).to_string(),"delta_percent":if a==0 {None} else {Some((b as f64/a as f64-1.0)*100.0)}}));
        }
        println!(
            "{}",
            json!({"profile":profile,"baseline":baseline["label"],"candidate":candidate["label"],"observed_differences":deltas,"claim":"observations only; repeated comparative runs and reliability review required before adoption"})
        );
    }
    Ok(())
}
fn audit() -> Result<()> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let tree = Command::new(&cargo)
        .args([
            "tree",
            "--locked",
            "-p",
            "kolvrt-kernel",
            "--target",
            "aarch64-unknown-none",
            "--edges",
            "normal,build",
            "--all-features",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ])
        .output()?;
    if !tree.status.success() {
        return Err("native dependency audit failed".into());
    }
    let tree = String::from_utf8(tree.stdout)?;
    let names = tree
        .lines()
        .map(|line| line.split_whitespace().next().unwrap_or(""))
        .collect::<BTreeSet<_>>();
    if names != BTreeSet::from(["kernel-core", "kolvrt-kernel"]) {
        return Err(format!("unexpected native dependency closure: {tree}").into());
    }
    let mut files = Vec::new();
    for path in ["crates/kernel/src", "crates/kernel-core/src"] {
        walk(Path::new(path), &mut files)?;
    }
    files.sort();
    let mut locations = Vec::new();
    let mut assembly = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file)?;
        enforce_native_source(&source)?;
        if file.extension().is_some_and(|e| e == "S") {
            assembly.push(json!({"path":file,"sha256":format!("{:x}",Sha256::digest(source.as_bytes())),"review":"INV-ENTRY, INV-VECTOR, INV-PROBE, INV-USER-CONTEXT, INV-USER-IMAGE"}));
        }
        for (i, line) in source.lines().enumerate() {
            if line.contains("unsafe") && !line.trim_start().starts_with("//") {
                let context = source
                    .lines()
                    .skip(i.saturating_sub(UNSAFE_CONTEXT_PRECEDING_LINES))
                    .take(UNSAFE_CONTEXT_PRECEDING_LINES + 1)
                    .collect::<Vec<_>>()
                    .join("\n");
                locations
                    .push(json!({"path":file,"line":i+1,"text":line.trim(),"context":context}));
            }
        }
    }
    let sysroot = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()?;
    if !sysroot.status.success() {
        return Err("compiler inventory failed".into());
    }
    let sysroot = String::from_utf8(sysroot.stdout)?;
    let libraries = Path::new(sysroot.trim()).join("lib/rustlib/aarch64-unknown-none/lib");
    let mut runtime = Vec::new();
    for e in fs::read_dir(libraries)? {
        let p = e?.path();
        let name = p.file_name().unwrap().to_string_lossy();
        if p.extension().is_some_and(|e| e == "rlib")
            && ["libcore-", "liballoc-", "libcompiler_builtins-"]
                .iter()
                .any(|prefix| name.starts_with(prefix))
        {
            runtime.push(json!({"artifact":name,"sha256":format!("{:x}",Sha256::digest(fs::read(&p)?)),"source_unsafe_coverage":"compiler-trusted; not individually proven"}));
        }
    }
    let report = json!({"scope":"first-party source inventory with local invariant context; assembly and compiled runtime artifacts included; not a safety proof","locations":locations,"assembly":assembly,"native_dependency_tree":tree,"compiler_runtime":runtime,"generated_wrappers":["read_reg!(el)","read_reg!(ticks)","read_reg!(frequency)","read_reg!(sctlr)","read_reg!(acknowledge)"],"compiler":"Rust 1.99.0","dependency_boundary":"compiler runtime and generated instructions require compiler trust; no compatibility dependency"});
    fs::write(
        "target/kernel/unsafe-audit.json",
        serde_json::to_string_pretty(&report)?,
    )?;
    println!("unsafe inventory: target/kernel/unsafe-audit.json");
    Ok(())
}

fn enforce_native_source(source: &str) -> Result<()> {
    for line in source
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
    {
        if [
            "routing::",
            "use routing",
            "extern crate routing",
            "window_compat::",
            "use window_compat",
            "window-compat",
            "feature = \"compat",
            "feature=\"compat",
            "Input::Encoded",
            "Route::Inclusive",
            "Route::Counted",
            "Route::EmptyFirst",
        ]
        .iter()
        .any(|pattern| line.contains(pattern))
        {
            return Err("native source contains compatibility import, type or conditional".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod native_architecture_tests {
    #[test]
    fn reject_legacy_types_imports_and_conditionals() {
        for code in [
            "use routing as r;",
            "extern crate routing;",
            "fn run() { window_compat::v1(&[], &[]); }",
            "#[cfg(feature = \"compat-v1\")] fn special() {}",
            "let input = Input::Encoded(bytes);",
        ] {
            assert!(super::enforce_native_source(code).is_err(), "{code}");
        }
        assert!(
            super::enforce_native_source(
                "// routing:: is outside native core\nuse kernel_core::window::Span;"
            )
            .is_ok()
        );
    }
}
