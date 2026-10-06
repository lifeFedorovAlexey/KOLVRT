//! Fault injection in an isolated source/build tree; ordinary entry.S stays pure.
use super::*;

pub(super) fn supports(feature: &str) -> bool {
    feature
        .split(',')
        .any(|item| item == "irq-simd-restore-negative" || mutation_specs().get(item).is_some())
}
fn mutation_specs() -> Value {
    serde_json::from_str(include_str!("mutations.json")).expect("reviewed mutation specs")
}
pub(super) fn tree(features: &str, prod: bool) -> Result<PathBuf> {
    let feature = features
        .split(',')
        .find(|item| *item == "irq-simd-restore-negative" || mutation_specs().get(*item).is_some())
        .ok_or("selected mutation missing")?;
    let mut input = json!({"source_files":source_inventory()?});
    input["test_fragment"] = json!(fs::read_to_string(
        "crates/xtask/src/test_support/irq_corruption.S"
    )?);
    input["prod"] = json!(prod);
    input["feature"] = json!(feature);
    input["mutation_specs"] = mutation_specs();
    let digest = Sha256::digest(serde_json::to_vec(&input)?)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let root = env::current_dir()?;
    let destination = root.join(format!("target/tb/{}", &digest[..16]));
    if !destination.starts_with(root.join("target/tb")) {
        return Err("test build escaped workspace".into());
    }
    if !destination.exists() {
        fs::create_dir_all(&destination)?;
        for directory in [
            "crates/kernel",
            "crates/kernel-core",
            "assets",
            "research/fixtures",
            ".cargo",
        ] {
            copy(&PathBuf::from(directory), &destination.join(directory))?;
        }
        for file in ["Cargo.lock", "rust-toolchain.toml"] {
            fs::copy(file, destination.join(file))?;
        }
        let workspace = fs::read_to_string("Cargo.toml")?;
        let profiles = workspace
            .find("[profile.dev]")
            .ok_or("workspace profiles absent")?;
        fs::write(
            destination.join("Cargo.toml"),
            format!(
                "[workspace]\nmembers=[\"crates/kernel\",\"crates/kernel-core\"]\nresolver=\"3\"\n{}",
                &workspace[profiles..]
            ),
        )?;
        if feature == "irq-simd-restore-negative" {
            let entry = destination.join("crates/kernel/src/arch/aarch64/entry.S");
            let normal = fs::read_to_string(&entry)?.replace("\r\n", "\n");
            let anchor = "irq_vector:\n save\n bl interrupt_entry\n restore";
            if normal.matches(anchor).count() != 1 {
                return Err("IRQ mutation anchor changed; review required".into());
            }
            let mutation = fs::read_to_string("crates/xtask/src/test_support/irq_corruption.S")?;
            fs::write(
                entry,
                normal.replace(
                    anchor,
                    &format!("irq_vector:\n save\n bl interrupt_entry\n{mutation} restore"),
                ),
            )?;
        } else {
            let specs = mutation_specs();
            for change in specs[feature]
                .as_array()
                .ok_or("unknown isolated mutation")?
            {
                let relative = change["path"].as_str().ok_or("mutation path absent")?;
                if !relative.starts_with("crates/kernel-core/src/")
                    && !relative.starts_with("crates/kernel/src/")
                {
                    return Err("mutation target not approved".into());
                }
                let file = destination.join(relative);
                let normal = fs::read_to_string(&file)?.replace("\r\n", "\n");
                let from = change["from"].as_str().ok_or("mutation anchor absent")?;
                let to = change["to"].as_str().ok_or("mutation replacement absent")?;
                if !normal.contains(from) {
                    return Err(format!("mutation anchor changed: {feature} {relative}").into());
                }
                fs::write(file, normal.replace(from, to))?;
            }
        }
        let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .current_dir(&destination)
            .args(["metadata", "--offline", "--format-version", "1"])
            .stdout(Stdio::null())
            .status()?;
        if !status.success() {
            return Err("isolated mutation lock resolution failed".into());
        }
        fs::write(
            destination.join("mutation.json"),
            serde_json::to_string_pretty(
                &json!({"kind":feature,"original_sources":input,"production_source_modified":false}),
            )?,
        )?;
    }
    Ok(destination)
}
fn copy(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
