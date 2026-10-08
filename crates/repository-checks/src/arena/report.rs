//! Human-readable views over the existing Arena assessor; no separate admission rules.
use super::*;
use std::{fmt::Write as _, path::PathBuf};
const DEFAULT_CLOCK: &str = "research/arena/runs/clock-query-2978ad9";

pub struct Rendered {
    pub text: String,
    pub admissible: bool,
}
fn number(value: f64) -> String {
    if value == 0.0 {
        "0".into()
    } else if value.abs() < 0.001 || value.abs() >= 1e12 {
        format!("{value:.6e}")
    } else {
        format!("{value:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}
fn run_paths(target: &Path) -> CheckResult<Vec<PathBuf>> {
    if target.is_file() {
        return Ok(vec![target.to_owned()]);
    }
    let mut paths = fs::read_dir(target)
        .map_err(|e| format!("Cannot read Arena bundle {}: {e}", target.display()))?
        .map(|entry| entry.map(|e| e.path()).map_err(|e| e.to_string()))
        .collect::<CheckResult<Vec<_>>>()?;
    paths.retain(|p| {
        p.is_file()
            && p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.ends_with("-run.json"))
    });
    paths.sort();
    if paths.is_empty() {
        return Err(format!("No *-run.json passports in {}", target.display()));
    }
    Ok(paths)
}
fn observation<'a>(run: &'a Value, id: &str) -> Option<&'a Value> {
    a(run, "observations")
        .iter()
        .find(|o| s(o, "metric_id") == id)
}
fn retained_frequency(bundle: &Path, run: &Value) -> Option<u64> {
    // The assessor has already checked artifact hashes. Only actual retained pair
    // observations agreeing with the manifest justify a unit conversion here.
    let manifest =
        input(&evidence_path(bundle, s(&run["environment_manifest"], "path")).ok()?).ok()?;
    let expected = manifest["expected_counter_frequency"]
        .as_u64()
        .filter(|n| *n > 0)?;
    let mut found = false;
    for artifact in a(run, "artifacts")
        .iter()
        .filter(|a| s(a, "path").ends_with("/pair.json") || s(a, "path") == "pair.json")
    {
        let pair = input(&evidence_path(bundle, s(artifact, "path")).ok()?).ok()?;
        let attempts = pair["attempts"].as_array()?;
        if attempts.len() != 2 {
            return None;
        }
        for attempt in attempts {
            if attempt["status"] != "passed"
                || attempt["result"]["frequency"].as_u64() != Some(expected)
            {
                return None;
            }
        }
        found = true;
    }
    found.then_some(expected)
}
fn metric_value(value: &Value, units: &str, frequency: Option<u64>) -> String {
    let Some(value) = value.as_f64() else {
        return "unavailable".into();
    };
    let mut result = format!("{} {units}", number(value));
    if units == "timer_ticks"
        && let Some(f) = frequency
    {
        let _ = write!(result, " ({} us)", number(value * 1_000_000.0 / f as f64));
    }
    result
}
/// Report every selected passport, retaining rejection reasons and nonzero status.
pub fn render(root: &Path, target: Option<&Path>) -> CheckResult<Rendered> {
    let selected = target
        .map(Path::to_owned)
        .unwrap_or_else(|| root.join(DEFAULT_CLOCK));
    let paths = run_paths(&selected)?;
    let mut text = String::from("ARENA MEASUREMENTS\n");
    if target.is_none() {
        writeln!(text,"Default: retained CLOCK snapshot {DEFAULT_CLOCK}; not newest or current-source evidence.").unwrap();
    }
    writeln!(text,"Bundle: {}\nNo baseline selected: this report does not compare revisions or establish an optimization.\n",selected.display()).unwrap();
    let mut rows = Vec::new();
    let mut admissible = true;
    for path in paths {
        match input(&path).and_then(|run| {
            assess(root, bundle_base(&path), &run).map(|assessment| (run, assessment))
        }) {
            Ok((run, assessment)) => rows.push((path, run, assessment)),
            Err(error) => {
                admissible = false;
                writeln!(text, "INVALID {}\n  {error}\n", path.display()).unwrap();
            }
        }
    }
    rows.sort_by(|left, right| {
        (
            s(
                &left.1["profile_snapshot"]["environment"],
                "execution_profile",
            ),
            s(&left.1["profile_snapshot"], "id"),
            &left.0,
        )
            .cmp(&(
                s(
                    &right.1["profile_snapshot"]["environment"],
                    "execution_profile",
                ),
                s(&right.1["profile_snapshot"], "id"),
                &right.0,
            ))
    });
    let mut group = String::new();
    for (path, run, assessment) in rows {
        let profile = &run["profile_snapshot"];
        let env = &profile["environment"];
        if group != s(&assessment, "comparison_class") {
            group = s(&assessment, "comparison_class").to_owned();
            writeln!(
                text,
                "{} | {} / {}",
                s(env, "execution_profile"),
                s(env, "platform_kind"),
                s(env, "architecture")
            )
            .unwrap();
            writeln!(
                text,
                "Operation: {}",
                s(&profile["workload"], "useful_operation")
            )
            .unwrap();
            writeln!(
                text,
                "Workload: {} @ {}; profile {} @ {}",
                s(&profile["workload"], "id"),
                s(&profile["workload"], "version"),
                s(profile, "id"),
                s(profile, "version")
            )
            .unwrap();
            writeln!(
                text,
                "Environment: {} @ {}; compiler {} {}",
                s(&env["hardware_profile"], "id"),
                s(&env["hardware_profile"], "version"),
                s(&env["toolchain"], "compiler"),
                s(&env["toolchain"], "version")
            )
            .unwrap();
            if s(env, "platform_kind") == "QEMU" {
                text.push_str(
                    "timer_ticks: virtual QEMU counter ticks, not physical CPU cycles.
",
                );
            }
            for (index, metric) in a(profile, "metrics").iter().enumerate() {
                writeln!(
                    text,
                    "#{} {} [{}]: {} (direction {})",
                    index + 1,
                    s(metric, "statistic"),
                    s(metric, "id"),
                    s(metric, "definition"),
                    s(metric, "direction")
                )
                .unwrap();
            }
            writeln!(text, "Provenance: {}", s(&run["provenance"], "submission")).unwrap();
            text.push_str(
                "Run | statistic | value | N / warmup | losses (fail/drop/unfinished) | admission
",
            );
        }
        let admitted = assessment["admission_state"] == "STRUCTURALLY_ADMISSIBLE";
        admissible &= admitted;
        let frequency = retained_frequency(bundle_base(&path), &run);
        let name = path.file_name().and_then(|p| p.to_str()).unwrap_or("run");
        for (index, metric) in a(profile, "metrics").iter().enumerate() {
            let id = s(metric, "id");
            let measured = &assessment["measurements"][id];
            if measured["state"] == "AVAILABLE" {
                let o = observation(&run, id).ok_or("assessed metric observation absent")?;
                writeln!(
                    text,
                    "{name} | #{} {} | {} | N={} / {} | failures={} / {} / {} | {}",
                    index + 1,
                    s(metric, "statistic"),
                    metric_value(&measured["value"], s(metric, "units"), frequency),
                    measured["sample_count"],
                    a(o, "warmup_samples").len(),
                    o["failures"],
                    o["dropped"],
                    o["unfinished"],
                    s(&assessment, "admission_state")
                )
                .unwrap();
            } else {
                writeln!(
                    text,
                    "{name} | #{} {} | {}: {} | {}",
                    index + 1,
                    s(metric, "statistic"),
                    s(measured, "state"),
                    s(measured, "reason"),
                    s(&assessment, "admission_state")
                )
                .unwrap();
            }
        }
        let overhead = &run["overhead_result"];
        let off = overhead["off_samples"].as_array().map_or(0, Vec::len);
        let on = overhead["on_samples"].as_array().map_or(0, Vec::len);
        writeln!(
            text,
            "  source commit {} / snapshot {}; recorder OFF/ON {} (N={off}/{on})",
            s(&run["provenance"], "exact_commit")
                .chars()
                .take(12)
                .collect::<String>(),
            s(&run["provenance"], "source_sha256")
                .chars()
                .take(12)
                .collect::<String>(),
            s(overhead, "state")
        )
        .unwrap();
        if overhead["state"] != "AVAILABLE" {
            writeln!(text, "  Recorder reason: {}", s(overhead, "reason")).unwrap();
        }
        for reason in a(&assessment, "reasons") {
            writeln!(
                text,
                "  Admission reason: {}",
                reason.as_str().unwrap_or("unspecified")
            )
            .unwrap();
        }
        writeln!(text, "  Raw passport: {}", path.display()).unwrap();
    }
    text.push_str("
Recorder OFF/ON compares recorder envelopes, not isolated CPU cost or a revision improvement. Microseconds, when shown, use matching retained pair frequencies and manifest; no frequency is assumed.
");
    text.push_str("Admission checks artifact integrity and declared evidence; it is not current-source execution attestation.\nRecord eligible: false. Descriptive observations do not prove statistical significance or superiority.\n");
    Ok(Rendered { text, admissible })
}
fn change(before: f64, after: f64, direction: &str) -> String {
    let delta = after - before;
    let relative = if before == 0.0 {
        "N/A (zero baseline)".into()
    } else {
        format!("{:+.4}%", delta / before * 100.0)
    };
    let trend = if delta == 0.0 {
        "unchanged"
    } else if (direction == "MIN" && delta < 0.0) || (direction == "MAX" && delta > 0.0) {
        "observed favorable direction"
    } else {
        "observed unfavorable direction"
    };
    format!("absolute {delta:+.6}; relative {relative}; {trend}")
}
/// A readable descriptive comparison, with exactly the existing compatibility/admission gates.
pub fn diff(root: &Path, left: &Path, right: &Path) -> CheckResult<String> {
    let l = input(left)?;
    let r = input(right)?;
    let la = assess(root, bundle_base(left), &l)?;
    let ra = assess(root, bundle_base(right), &r)?;
    compatible(
        root,
        &l["registry_snapshot"],
        &l["profile_snapshot"],
        &r["registry_snapshot"],
        &r["profile_snapshot"],
    )?;
    if la["admission_state"] != "STRUCTURALLY_ADMISSIBLE"
        || ra["admission_state"] != "STRUCTURALLY_ADMISSIBLE"
    {
        return Err(format!(
            "Cannot compare invalid/incomplete runs. Baseline: {} {}; candidate: {} {}. Use arena report RUN for details.",
            s(&la, "admission_state"),
            la["reasons"],
            s(&ra, "admission_state"),
            ra["reasons"]
        ));
    }
    let mut text = String::from("ARENA DESCRIPTIVE COMPARISON\n");
    writeln!(
        text,
        "Operation: {}\nBaseline: {}\nCandidate: {}",
        s(&l["profile_snapshot"]["workload"], "useful_operation"),
        left.display(),
        right.display()
    )
    .unwrap();
    writeln!(
        text,
        "Source commits: {} -> {}",
        s(&l["provenance"], "exact_commit"),
        s(&r["provenance"], "exact_commit")
    )
    .unwrap();
    if l["provenance"]["source_sha256"] == r["provenance"]["source_sha256"] {
        text.push_str(
            "Same retained source snapshot: repeat comparison, not evidence of an optimization.\n",
        );
    } else {
        text.push_str(
            "Different retained source snapshots; direction below describes observations only.\n",
        );
    }
    writeln!(
        text,
        "Execution profile: {} | comparison class: {}",
        s(&l["profile_snapshot"]["environment"], "execution_profile"),
        s(&la, "comparison_class")
    )
    .unwrap();
    for metric in a(&l["profile_snapshot"], "metrics") {
        let id = s(metric, "id");
        let before = &la["measurements"][id];
        let after = &ra["measurements"][id];
        let (a, b) = (
            before["value"]
                .as_f64()
                .ok_or("baseline metric unavailable")?,
            after["value"]
                .as_f64()
                .ok_or("candidate metric unavailable")?,
        );
        writeln!(
            text,
            "{}: {}\n  {} -> {} {} | N={} -> {}\n  {} ({})",
            s(metric, "statistic"),
            s(metric, "definition"),
            number(a),
            number(b),
            s(metric, "units"),
            before["sample_count"],
            after["sample_count"],
            change(a, b, s(metric, "direction")),
            s(metric, "units")
        )
        .unwrap();
    }
    text.push_str("Not the recorder OFF/ON comparison. No statistical significance, proven improvement, regression threshold or superiority claim. Record eligible: false.\n");
    Ok(text)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptive_direction_and_zero_baseline_are_explicit() {
        assert!(change(10.0, 8.0, "MIN").ends_with("; observed favorable direction"));
        assert!(change(10.0, 8.0, "MAX").ends_with("; observed unfavorable direction"));
        assert!(change(0.0, 1.0, "MAX").contains("zero baseline"));
        assert!(change(1.0, 1.0, "MIN").contains("unchanged"));
    }
    #[test]
    fn no_assumed_counter_frequency() {
        assert_eq!(
            metric_value(&json!(100), "timer_ticks", None),
            "100 timer_ticks"
        );
        assert_eq!(
            metric_value(&json!(100), "timer_ticks", Some(1000000)),
            "100 timer_ticks (100 us)"
        );
    }
}
