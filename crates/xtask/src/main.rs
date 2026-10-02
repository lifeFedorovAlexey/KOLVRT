use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
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
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("audit") => audit(),
        Some("build") => {
            build(args.iter().any(|a| a == "--prod"), false, None)?;
            Ok(())
        }
        Some("debug") => {
            let elf = build(false, false, None)?;
            let q = qemu()?;
            Command::new(q)
                .args(qemu_args(&elf))
                .args(["-S", "-gdb", "tcp:127.0.0.1:1234"])
                .status()?;
            Ok(())
        }
        Some("test") => {
            if args.iter().any(|a| a == "--negative-control") {
                let elf = build(false, true, Some("negative-test"))?;
                return execute(&elf, true);
            }
            if args.iter().any(|a| a == "--panic-control") {
                let elf = build(false, true, Some("panic-test"))?;
                return execute(&elf, true);
            }
            if args.iter().any(|a| a == "--ownership-control") {
                let elf = build(false, true, Some("ownership-test"))?;
                return execute(&elf, true);
            }
            if args.iter().any(|a| a == "--retained-mapping-control") {
                let elf = build(false, true, Some("retained-mapping-test"))?;
                return execute(&elf, true);
            }
            qemu()?;
            audit()?;
            for prod in [false, true] {
                let elf = build(prod, true, None)?;
                execute(&elf, true)?;
                let elf = build(prod, false, None)?;
                execute(&elf, false)?;
            }
            for (flag, marker) in [
                ("--negative-control", "negative_control"),
                ("--panic-control", "panic reporting negative control"),
                ("--ownership-control", "physical pool already owned"),
                ("--retained-mapping-control", "mapped frame release"),
            ] {
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
            println!("Phase 1 kernel matrix passed (single active CPU; SMP deferred).");
            Ok(())
        }
        _ => Err("usage: cargo xtask test | build [--prod] | audit | debug".into()),
    }
}
fn build(prod: bool, tests: bool, extra: Option<&str>) -> Result<PathBuf> {
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
    if !prod {
        features.push("diagnostics");
    }
    if tests {
        features.push("kernel-tests");
    }
    if let Some(e) = extra {
        features.push(e);
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
    println!("{report}");
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
fn execute(elf: &Path, tests: bool) -> Result<()> {
    let log = elf.with_extension("log");
    let err = elf.with_extension("stderr");
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
    print!("{text}");
    let mut names = BTreeSet::new();
    let mut suite = false;
    let mut boot = false;
    let mut events = Vec::new();
    for line in text.lines().filter(|l| l.starts_with('{')) {
        let event: Value = serde_json::from_str(line)?;
        if event["event"] == "fatal" || event["event"] == "panic" || event["status"] == "fail" {
            events.push(event.clone());
            fs::write(
                elf.with_extension("results.json"),
                serde_json::to_string_pretty(&events)?,
            )?;
            return Err(format!("kernel failure: {line}").into());
        }
        if event["event"] == "test" {
            let name = event["name"].as_str().ok_or("test missing name")?;
            if event["status"] != "pass" || !names.insert(name.to_string()) {
                return Err("invalid test event".into());
            }
        }
        if event["event"] == "suite" {
            if suite || event["status"] != "pass" || event["tests"] != TESTS.len() {
                return Err("invalid suite".into());
            }
            suite = true;
        }
        if event["event"] == "boot" {
            boot = event["status"] == "pass" && event["el"] == 1 && event["timer_irq"] == true;
        }
        events.push(event);
    }
    if tests {
        let expected = TESTS
            .iter()
            .map(|name| name.to_string())
            .collect::<BTreeSet<_>>();
        if !suite || names != expected {
            return Err("missing or unexpected real kernel tests".into());
        }
    } else if !boot {
        return Err("missing actual boot result".into());
    }
    fs::write(
        elf.with_extension("results.json"),
        serde_json::to_string_pretty(&events)?,
    )?;
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
        if file.extension().is_some_and(|e| e == "S") {
            assembly.push(json!({"path":file,"sha256":format!("{:x}",Sha256::digest(source.as_bytes())),"review":"INV-ENTRY, INV-VECTOR, INV-PROBE"}));
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
