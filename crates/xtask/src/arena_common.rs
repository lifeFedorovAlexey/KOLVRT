//! Shared retention and ordinary ELF execution for actual Arena producers.
use super::*;
pub(super) fn write_json(path: impl AsRef<Path>, value: &Value) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}
pub(super) fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        return Err("Arena source provenance git query failed".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub(super) fn compiler_identity() -> Result<String> {
    for (name, _) in env::vars_os() {
        let name = name.to_string_lossy();
        if matches!(
            name.as_ref(),
            "RUSTFLAGS"
                | "CARGO_ENCODED_RUSTFLAGS"
                | "CARGO_BUILD_RUSTFLAGS"
                | "CARGO_TARGET_AARCH64_UNKNOWN_NONE_RUSTFLAGS"
                | "RUSTC"
                | "RUSTC_WRAPPER"
                | "RUSTC_WORKSPACE_WRAPPER"
        ) || name.starts_with("CARGO_PROFILE_")
        {
            return Err(format!("fixed Arena class rejects build override {name}").into());
        }
    }
    let compiler = Command::new("rustc").arg("--version").output()?;
    if !compiler.status.success() {
        return Err("cannot observe actual rustc version".into());
    }
    Ok(String::from_utf8(compiler.stdout)?)
}

pub(super) fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub(super) fn value_digest(value: &Value) -> Result<String> {
    Ok(digest(serde_json::to_string(value)?.as_bytes()))
}
pub(super) struct AttemptSpec<'a> {
    pub prod: bool,
    pub images: [&'a str; 3],
    pub argument: u64,
    pub directory: &'a Path,
}
pub(super) fn run_attempt(
    spec: AttemptSpec<'_>,
    mut attempt: Value,
    validator: EventValidator,
    expected: &[(&str, u64)],
) -> Result<Value> {
    let AttemptSpec {
        prod,
        images,
        argument,
        directory,
    } = spec;
    let profile = if prod { "prod" } else { "dev" };
    fs::create_dir_all(directory)?;
    attempt["directory"] = json!(directory);
    attempt["status"] = json!("failed");
    let outcome = (|| -> Result<()> {
        let (built, applications) = native_apps::build_images(prod, images, argument)?;
        attempt["applications"] = applications;
        let elf = directory.join("kernel.elf");
        fs::copy(&built, &elf)?;
        let hash = digest(&fs::read(&elf)?);
        attempt["kernel"] = json!({"artifact":elf,"sha256":hash});
        let build = read_json(built.with_file_name(format!("{profile}-native-apps-build.json")))?;
        write_json(directory.join("kernel-build.json"), &build)?;
        attempt["kernel_build"] = build;
        execute_validated(&elf, false, true, false, true, Some(validator))?;
        let run = read_json(elf.with_extension("run.json"))?;
        if run["elf_sha256"] != hash {
            return Err("Arena kernel identity changed during execution".into());
        }
        let events = read_json(elf.with_extension("results.json"))?;
        let result = validator(events.as_array().ok_or("Arena events absent")?)?;
        for &(key, value) in expected {
            if result[key] != value {
                return Err(
                    format!("Arena observed {key} does not match scheduled invocation").into(),
                );
            }
        }
        attempt["result"] = result;
        attempt["run"] = run;
        attempt["events"] = events;
        Ok(())
    })();
    if let Err(error) = outcome {
        attempt["error"] = json!(error.to_string());
    } else {
        attempt["status"] = json!("passed");
    }
    for (key, suffix) in [("run", "run.json"), ("events", "results.json")] {
        let path = directory.join("kernel.elf").with_extension(suffix);
        if path.exists() {
            match read_json(path) {
                Ok(value) => attempt[key] = value,
                Err(error) => {
                    attempt["status"] = json!("failed");
                    attempt[format!("{key}_error")] = json!(error.to_string());
                }
            }
        }
    }
    write_json(directory.join("attempt.json"), &attempt)?;
    Ok(attempt)
}

pub(super) fn manifest_matches(attempt: &Value, manifest: &Value) -> bool {
    let Some(args) = attempt["run"]["arguments"].as_array() else {
        return false;
    };
    let expected = manifest["qemu_arguments_without_kernel"].as_array();
    let arguments_match = expected.is_some_and(|expected| {
        args.len() == expected.len() + 2
            && &args[..expected.len()] == expected.as_slice()
            && args[expected.len()] == "-kernel"
            && args.last() == Some(&attempt["kernel"]["artifact"])
    });
    arguments_match
        && attempt["result"]["frequency"] == manifest["expected_counter_frequency"]
        && attempt["run"]["configured_cpus"] == manifest["configured_cpus"]
        && attempt["run"]["active_cpus"] == manifest["active_cpus"]
        && attempt["kernel_build"]["compiler"] == manifest["compiler_version"]
        && attempt["kernel_build"]["target"] == manifest["target"]
        && attempt["kernel_build"]["features"] == manifest["kernel_features"]
        && attempt["run"]["qemu_version"]
            .as_str()
            .is_some_and(|version| {
                version.lines().next().is_some_and(|line| {
                    line.split_whitespace().nth(3) == manifest["qemu_version"].as_str()
                })
            })
}
