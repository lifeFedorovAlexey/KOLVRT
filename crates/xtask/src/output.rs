use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::collections::BTreeSet;
use std::io::{self, IsTerminal};

const PREFIX: &str = "@KOLVRT/1 ";
const RECORD_BYTES: usize = 4096;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

pub fn color_arguments(args: Vec<String>) -> Result<Vec<String>> {
    let mut chosen = false;
    let mut remaining = Vec::new();
    for arg in args {
        if arg.starts_with("--color") {
            if chosen {
                return Err("duplicate --color option".into());
            }
            match arg.as_str() {
                "--color=auto" | "--color=always" | "--color=never" => {}
                _ => return Err("use --color=auto|always|never".into()),
            }
            chosen = true;
        } else {
            remaining.push(arg);
        }
    }
    Ok(remaining)
}

fn color_enabled(mode: ColorMode, terminal: bool, no_color: bool, dumb: bool) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => terminal && !no_color && !dumb,
    }
}

pub fn stdout_color() -> bool {
    let mode = std::env::args()
        .find_map(|arg| match arg.as_str() {
            "--color=always" => Some(ColorMode::Always),
            "--color=never" => Some(ColorMode::Never),
            "--color=auto" => Some(ColorMode::Auto),
            _ => None,
        })
        .unwrap_or(ColorMode::Auto);
    color_enabled(
        mode,
        io::stdout().is_terminal(),
        std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()),
        std::env::var("TERM").is_ok_and(|value| value == "dumb"),
    )
}

/// Color is presentation only; labels and evidence are unchanged.
pub fn console_text(text: &str, color: bool) -> String {
    let plain = human_text(text);
    if !color {
        return plain;
    }
    let mut rendered = String::new();
    for line in plain.split_inclusive('\n') {
        if let Some(rest) = line.strip_prefix("KOLVRT |") {
            rendered.push_str("\x1b[1;32mKOLVRT\x1b[0m |");
            rendered.push_str(rest);
            continue;
        }
        let badge = line.strip_prefix('[').and_then(|s| s.split_once(']'));
        let style = badge.and_then(|(label, _)| match label {
            "OK" | "NATIVE" => Some("32"),
            "FAIL" | "COMPAT" => Some("31"),
            "WARN" | "MIXED" => Some("33"),
            "MOSTLY_NATIVE" => Some("92"),
            "LEGACY" => Some("38;5;130"),
            "INFO" => Some("36"),
            "DEBUG" | "UNKNOWN" => Some("90"),
            _ => None,
        });
        if let (Some((label, rest)), Some(style)) = (badge, style) {
            rendered.push_str(&format!("\x1b[{style}m[{label}]\x1b[0m{rest}"));
        } else {
            rendered.push_str(line);
        }
    }
    rendered
}

/// Human presentation never prints the machine stream, including malformed versions.
pub fn human_text(text: &str) -> String {
    text.split_inclusive('\n')
        .filter(|line| !line.contains("@KOLVRT"))
        .collect()
}

/// Complete newline-framed records only. Human prose is not an evidence source.
pub fn parse(text: &str) -> Result<Vec<Value>> {
    let mut events = Vec::new();
    for line in text.split_inclusive('\n') {
        if !line.starts_with("@KOLVRT") {
            if line.contains("@KOLVRT") {
                return Err("kernel event marker outside record boundary".into());
            }
            continue;
        }
        if !line.ends_with('\n') || line.len() > RECORD_BYTES {
            return Err("truncated or oversized kernel event".into());
        }
        let payload = line
            .strip_prefix(PREFIX)
            .ok_or("unsupported kernel event framing/version")?;
        let event = serde_json::from_str::<UniqueJson>(payload)?.0;
        if !event.is_object() || event["event"].as_str().is_none() {
            return Err("invalid kernel event object".into());
        }
        events.push(event);
    }
    Ok(events)
}

/// Reject ambiguous evidence rather than letting the last duplicate key win.
struct UniqueJson(Value);
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON with unique object keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<UniqueJson, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<UniqueJson, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<UniqueJson, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<UniqueJson, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| UniqueJson(Value::Number(n)))
                    .ok_or_else(|| E::custom("nonfinite number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<UniqueJson, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<UniqueJson, E> {
                Ok(UniqueJson(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<UniqueJson, A::Error> {
                let mut values = Vec::new();
                while let Some(UniqueJson(v)) = a.next_element()? {
                    values.push(v);
                }
                Ok(UniqueJson(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<UniqueJson, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = a.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom(format!("duplicate event key: {key}")));
                    }
                    let UniqueJson(value) = a.next_value()?;
                    values.insert(key, value);
                }
                Ok(UniqueJson(Value::Object(values)))
            }
        }
        d.deserialize_any(UniqueVisitor)
    }
}

pub fn validate(events: &[Value], tests: bool, expected: &[&str], cpus: usize) -> Result<()> {
    let mut names = BTreeSet::new();
    let mut terminal = false;
    let mut measurements = BTreeSet::new();
    let mut asid_measurement = false;
    let asid_required = expected.iter().any(|name| name.starts_with("asid_"));
    let mut el0 = false;
    for event in events {
        if matches!(event["event"].as_str(), Some("fatal" | "panic")) || event["status"] == "fail" {
            return Err(format!("kernel failure: {event}").into());
        }
        if terminal {
            return Err("event after terminal kernel result".into());
        }
        match event["event"].as_str() {
            Some("test") if tests => {
                let name = event["name"].as_str().ok_or("test missing name")?;
                if event["status"] != "pass" || !expected.contains(&name) || !names.insert(name) {
                    return Err("invalid or duplicate test event".into());
                }
            }
            Some("measurement") if tests => {
                let scope = event["scope"].as_str().ok_or("measurement missing scope")?;
                let owner = if scope == "lock_uncontended" {
                    if !event["cpu"].is_null() {
                        return Err("lock measurement has CPU selector".into());
                    }
                    None
                } else if [
                    "user_copy_small",
                    "user_copy_page",
                    "user_copy_three_pages",
                    "user_copy_failure",
                ]
                .contains(&scope)
                    && expected.contains(&"user_copy_el0_boundary_and_snapshot")
                    || [
                        "handle_lookup_success",
                        "handle_lookup_failure",
                        "handle_create",
                        "handle_close",
                        "handle_slot_reuse",
                    ]
                    .contains(&scope)
                        && expected.contains(&"handle_el0_identity_type_generation_and_lifetime")
                {
                    let cpu = event["cpu"]
                        .as_u64()
                        .ok_or("copy measurement missing CPU")?;
                    if cpu >= cpus as u64 {
                        return Err("foreign copy measurement CPU".into());
                    }
                    Some(cpu)
                } else {
                    return Err("unknown measurement scope".into());
                };
                if !measurements.insert((scope, owner))
                    || event["units"] != "timer_ticks"
                    || event["frequency"].as_u64().is_none_or(|v| v == 0)
                    || ["warmup", "iterations", "median", "p95", "p99"]
                        .iter()
                        .any(|field| event[field].as_u64().is_none())
                    || event["samples"].as_array().is_none_or(|samples| {
                        samples.is_empty() || samples.iter().any(|v| v.as_u64().is_none())
                    })
                {
                    return Err("invalid or duplicate measurement event".into());
                }
                crate::validate_samples(event)?;
            }
            Some("asid-measurement") if tests => {
                let mode = event["mode"]
                    .as_str()
                    .ok_or("ASID measurement missing mode")?;
                let reuse_invalidation = event["reuse_invalidation"]
                    .as_bool()
                    .ok_or("ASID measurement missing reuse invalidation status")?;
                let cpu_events = event["cpus"]
                    .as_array()
                    .ok_or("ASID measurement missing CPU counters")?;
                if asid_measurement
                    || event["elapsed_ticks"]
                        .as_u64()
                        .is_none_or(|ticks| ticks == 0)
                    || cpu_events.len() != cpus
                    || !matches!(mode, "tagged" | "asid-zero-baseline")
                    || !matches!(event["hardware_asid_bits"].as_u64(), Some(0 | 8 | 16))
                    || (mode == "tagged" && event["hardware_asid_bits"] == 0)
                {
                    return Err("invalid or duplicate ASID measurement".into());
                }
                let isolation = event["same_va_isolation"]
                    .as_array()
                    .ok_or("ASID measurement missing isolation checks")?;
                let backing = event["distinct_backing"]
                    .as_array()
                    .ok_or("ASID measurement missing backing checks")?;
                let reused_asid = event["same_asid_reused"]
                    .as_array()
                    .ok_or("ASID measurement missing repeated-ASID checks")?;
                if isolation.len() != cpus
                    || backing.len() != cpus
                    || reused_asid.len() != cpus
                    || (reuse_invalidation
                        && (isolation.iter().any(|v| v != true)
                            || backing.iter().any(|v| v != true)
                            || reused_asid.iter().any(|v| v != true)))
                {
                    return Err("ASID same-VA or physical-backing check failed".into());
                }
                for (index, cpu) in cpu_events.iter().enumerate() {
                    if cpu["cpu"].as_u64() != Some(index as u64)
                        || ["switches", "full_tlbi", "asid_tlbi", "reuses"]
                            .iter()
                            .any(|field| cpu[field].as_u64().is_none())
                        || cpu["switches"].as_u64().is_none_or(|count| count == 0)
                        || (mode == "tagged"
                            && cpu["reuses"].as_u64().is_none_or(|count| count == 0))
                        || (mode == "tagged"
                            && (cpu["full_tlbi"] != 0
                                || (reuse_invalidation
                                    && cpu["asid_tlbi"].as_u64().is_none_or(|count| count == 0))
                                || (!reuse_invalidation && cpu["asid_tlbi"] != 0)))
                        || (mode == "asid-zero-baseline"
                            && cpu["full_tlbi"].as_u64().is_none_or(|count| count == 0))
                    {
                        return Err("invalid ASID CPU switch/invalidation counters".into());
                    }
                }
                asid_measurement = true;
            }
            Some("el0") if !tests && event["status"] == "pass" => {
                let processes = cpus * crate::platform_config::USER_PROCESSES_PER_CPU;
                if el0
                    || event["reclaimed"] != true
                    || event["processes"].as_u64() != Some(processes as u64)
                    || event["workers"].as_u64() != Some(cpus as u64)
                    || event["faults"].as_u64() != Some((processes - cpus) as u64)
                    || event["switches"]
                        .as_u64()
                        .is_none_or(|switches| switches <= processes as u64)
                {
                    return Err("invalid or duplicate EL0 event".into());
                }
                el0 = true;
            }
            Some("suite") if tests => {
                let groups = usize::from(expected.contains(&"user_copy_el0_boundary_and_snapshot"))
                    * 4
                    + usize::from(
                        expected.contains(&"handle_el0_identity_type_generation_and_lifetime"),
                    ) * 5;
                if groups != 0 && measurements.len() != 1 + cpus * groups {
                    return Err("missing user-copy measurements".into());
                }
                if event["status"] != "pass"
                    || event["tests"].as_u64() != Some(expected.len() as u64)
                    || names != expected.iter().copied().collect()
                    || (asid_required && !asid_measurement)
                {
                    return Err("missing or unexpected real kernel tests".into());
                }
                terminal = true;
            }
            Some("boot") if !tests => {
                if event["status"] != "pass"
                    || !el0
                    || event["el"] != 1
                    || event["timer_irq"] != true
                    || event["active_cpus"].as_u64() != Some(cpus as u64)
                    || event["secondary_shutdown_verified"] != true
                {
                    return Err("invalid boot result".into());
                }
                terminal = true;
            }
            _ => return Err("unknown or out-of-scope kernel event".into()),
        }
    }
    if !terminal {
        return Err("missing terminal kernel result".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn colors_preserve_labels_and_never_enter_evidence() {
        let text = "KOLVRT | DEV | AArch64 | EL1\n[OK] console: ready\n[FAIL] panic: halted\n[LEGACY] declared dependency\n[COMPAT] observed scope\n[MIXED] observed scope\n[MOSTLY_NATIVE] budget met\n[NATIVE] verified scope\n@KOLVRT/1 {\"event\":\"panic\",\"status\":\"fail\"}\n";
        let plain = console_text(text, false);
        assert!(!plain.contains('\x1b'));
        assert!(!plain.contains("@KOLVRT"));
        let colored = console_text(text, true);
        for (label, style) in [
            ("OK", "32"),
            ("FAIL", "31"),
            ("LEGACY", "38;5;130"),
            ("COMPAT", "31"),
            ("MIXED", "33"),
            ("MOSTLY_NATIVE", "92"),
            ("NATIVE", "32"),
        ] {
            assert!(colored.contains(&format!("\x1b[{style}m[{label}]\x1b[0m")));
        }
        assert!(!colored.contains("@KOLVRT"));
        assert_eq!(
            parse(text).unwrap(),
            vec![json!({"event":"panic","status":"fail"})]
        );
        assert_eq!(
            console_text("[INFO] state: NATIVE unknown\n", true),
            "\x1b[36m[INFO]\x1b[0m state: NATIVE unknown\n"
        );
    }
    #[test]
    fn color_modes_respect_redirects_and_explicit_overrides() {
        assert!(color_enabled(ColorMode::Auto, true, false, false));
        for (terminal, no_color, dumb) in [
            (false, false, false),
            (true, true, false),
            (true, false, true),
        ] {
            assert!(!color_enabled(ColorMode::Auto, terminal, no_color, dumb));
        }
        assert!(color_enabled(ColorMode::Always, false, true, true));
        assert!(!color_enabled(ColorMode::Never, true, false, false));
        assert_eq!(
            color_arguments(vec!["run".into(), "--prod".into(), "--color=always".into()]).unwrap(),
            vec!["run", "--prod"]
        );
        assert!(color_arguments(vec!["run".into(), "--color=magic".into()]).is_err());
        assert!(color_arguments(vec!["--color=auto".into(), "--color=never".into()]).is_err());
    }
    fn boot() -> Value {
        json!({"event":"boot","status":"pass","el":1,"timer_irq":true,"active_cpus":2,"secondary_shutdown_verified":true})
    }
    fn el0() -> Value {
        let cpus = crate::platform_config::ACTIVE_CPUS;
        let processes = crate::platform_config::USER_PROCESSES;
        json!({"event":"el0","status":"pass","processes":processes,"workers":cpus,"faults":processes-cpus,"switches":processes * 2,"reclaimed":true})
    }
    #[test]
    fn framing_rejects_lost_or_malformed_evidence() {
        let valid = format!(
            "[OK] console: ready\n{PREFIX}{}\n{PREFIX}{}\n",
            el0(),
            boot()
        );
        let events = parse(&valid).unwrap();
        validate(&events, false, &[], 2).unwrap();
        assert_eq!(human_text(&valid), "[OK] console: ready\n");
        for text in [
            valid.trim_end().to_owned(),
            "@KOLVRT/2 {}\n".into(),
            format!("{PREFIX}{{\n"),
            format!("{PREFIX}[]\n"),
            format!("{PREFIX}{}\n", "x".repeat(RECORD_BYTES)),
        ] {
            assert!(parse(&text).is_err());
        }
        assert!(validate(&parse("{\"event\":\"boot\"}\n").unwrap(), false, &[], 2).is_err());
    }
    #[test]
    fn ambiguous_keys_and_embedded_frames_are_rejected() {
        for payload in [
            r#"{"event":"panic","status":"fail","status":"pass"}"#,
            r#"{"event":"panic","status":"fail","detail":{"owner":1,"owner":2}}"#,
            r#"{"event":"panic","status":"fail","detail":[{"owner":1,"owner":2}]}"#,
        ] {
            assert!(parse(&format!("{PREFIX}{payload}\n")).is_err());
        }
        let partial =
            format!("[OK] partial line{PREFIX}{{\"event\":\"panic\",\"status\":\"fail\"}}\n");
        assert!(parse(&partial).is_err());
        assert!(human_text(&partial).is_empty());
        assert!(parse(&format!("{PREFIX}{{\"event\":\"panic\",\"status\":\"fail\",\"detail\":{{\"note\":\"escaped \\\"quote\\\"\"}}}}\n")).is_ok());
    }
    #[test]
    fn terminal_is_not_overwritable_and_failures_win() {
        validate(&[el0(), boot()], false, &[], 2).unwrap();
        for events in [
            vec![],
            vec![el0(), boot(), boot()],
            vec![el0(), boot(), json!({"event":"panic","status":"fail"})],
            vec![el0(), json!({"event":"panic","status":"fail"}), boot()],
            vec![el0(), boot(), el0()],
            vec![json!({"event":"unknown"})],
        ] {
            assert!(validate(&events, false, &[], 2).is_err());
        }
        let mut wrong = boot();
        wrong["secondary_shutdown_verified"] = json!(false);
        assert!(validate(&[el0(), wrong], false, &[], 2).is_err());
    }
    #[test]
    fn boot_and_el0_fields_are_independently_required() {
        let valid = vec![el0(), boot()];
        validate(&valid, false, &[], 2).unwrap();
        for (index, field, bad) in [
            (0, "status", json!("fail")),
            (0, "reclaimed", json!(false)),
            (0, "processes", json!(0)),
            (0, "workers", json!(0)),
            (0, "faults", json!(0)),
            (0, "switches", json!(crate::platform_config::USER_PROCESSES)),
            (1, "status", json!("fail")),
            (1, "el", json!(0)),
            (1, "timer_irq", json!(false)),
            (1, "active_cpus", json!(1)),
            (1, "secondary_shutdown_verified", json!(false)),
        ] {
            for value in [bad, Value::Null, json!("invalid-type")] {
                let mut corrupt = valid.clone();
                corrupt[index][field] = value.clone();
                assert!(
                    validate(&corrupt, false, &[], 2).is_err(),
                    "accepted {index}.{field}={value}"
                );
            }
            let mut missing = valid.clone();
            missing[index].as_object_mut().unwrap().remove(field);
            assert!(
                validate(&missing, false, &[], 2).is_err(),
                "accepted missing {index}.{field}"
            );
        }
        assert!(validate(&[boot()], false, &[], 2).is_err());
        assert!(validate(&[el0(), el0(), boot()], false, &[], 2).is_err());
    }
    #[test]
    fn retained_qemu_output_still_validates_without_human_json() {
        // Revalidate actual retained observations, without claiming a new QEMU run.
        let evidence: Value =
            serde_json::from_str(include_str!("../../../research/results/output-policy.json"))
                .unwrap();
        for profile in ["dev", "prod"] {
            for kind in ["boot", "tests"] {
                let capture = &evidence["profiles"][profile][kind];
                let events = capture["events"].as_array().unwrap();
                let stream: String = events.iter().map(|e| format!("{PREFIX}{e}\n")).collect();
                let parsed = parse(&stream).unwrap();
                let names: Vec<_> = events
                    .iter()
                    .filter(|e| e["event"] == "test")
                    .map(|e| e["name"].as_str().unwrap())
                    .collect();
                validate(&parsed, kind == "tests", &names, 2).unwrap();
                assert!(human_text(&stream).is_empty());
            }
        }
        let panic = evidence["panic"]["events"].as_array().unwrap();
        assert!(validate(panic, false, &[], 2).is_err());
    }
    #[test]
    fn suite_requires_actual_unique_tests_before_terminal() {
        let test = json!({"event":"test","name":"real","status":"pass"});
        let suite = json!({"event":"suite","status":"pass","tests":1});
        validate(&[test.clone(), suite.clone()], true, &["real"], 2).unwrap();
        for events in [
            vec![suite.clone()],
            vec![test.clone(), test.clone(), suite.clone()],
            vec![test.clone(), suite.clone(), test.clone()],
        ] {
            assert!(validate(&events, true, &["real"], 2).is_err());
        }
    }
    #[test]
    fn asid_evidence_requires_each_cpu_isolation_and_invalidation_counter() {
        let expected = ["asid_reuse_requires_invalidation"];
        let measurement = json!({
            "event":"asid-measurement", "mode":"tagged", "hardware_asid_bits":16,
            "elapsed_ticks":100, "reuse_invalidation":true,
            "same_va_isolation":[true,true], "distinct_backing":[true,true],
            "same_asid_reused":[true,true],
            "cpus":[
                {"cpu":0,"switches":64,"full_tlbi":0,"asid_tlbi":32,"reuses":32},
                {"cpu":1,"switches":64,"full_tlbi":0,"asid_tlbi":32,"reuses":32}
            ]
        });
        let valid = vec![
            json!({"event":"test","name":expected[0],"status":"pass"}),
            measurement,
            json!({"event":"suite","status":"pass","tests":1}),
        ];
        validate(&valid, true, &expected, 2).unwrap();
        for bits in [8, 16] {
            let mut supported = valid.clone();
            supported[1]["hardware_asid_bits"] = json!(bits);
            validate(&supported, true, &expected, 2).unwrap();
        }
        for (field, value) in [
            ("mode", json!("unknown")),
            ("hardware_asid_bits", json!(0)),
            ("hardware_asid_bits", json!(12)),
            ("elapsed_ticks", json!(0)),
            ("reuse_invalidation", json!("true")),
            ("cpus", json!([])),
        ] {
            let mut corrupt = valid.clone();
            corrupt[1][field] = value;
            assert!(
                validate(&corrupt, true, &expected, 2).is_err(),
                "accepted ASID {field}"
            );
        }
        for field in ["same_va_isolation", "distinct_backing", "same_asid_reused"] {
            for value in [
                json!([]),
                json!([true]),
                json!([true, true, true]),
                json!([false, true]),
                json!([true, false]),
                json!([true, "true"]),
            ] {
                let mut corrupt = valid.clone();
                corrupt[1][field] = value;
                assert!(
                    validate(&corrupt, true, &expected, 2).is_err(),
                    "accepted ASID {field}"
                );
            }
        }
        for cpu in 0..2 {
            for (field, value) in [
                ("cpu", json!(2)),
                ("switches", json!(0)),
                ("full_tlbi", json!(1)),
                ("asid_tlbi", json!(0)),
                ("reuses", json!(0)),
            ] {
                let mut corrupt = valid.clone();
                corrupt[1]["cpus"][cpu][field] = value;
                assert!(
                    validate(&corrupt, true, &expected, 2).is_err(),
                    "accepted CPU{cpu}.{field}"
                );
                let mut missing = valid.clone();
                missing[1]["cpus"][cpu]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
                assert!(
                    validate(&missing, true, &expected, 2).is_err(),
                    "accepted missing CPU{cpu}.{field}"
                );
            }
        }
        let mut baseline = valid.clone();
        baseline[1]["mode"] = json!("asid-zero-baseline");
        baseline[1]["hardware_asid_bits"] = json!(0);
        for cpu in baseline[1]["cpus"].as_array_mut().unwrap() {
            cpu["full_tlbi"] = json!(64);
            cpu["asid_tlbi"] = json!(0);
            cpu["reuses"] = json!(0);
        }
        validate(&baseline, true, &expected, 2).unwrap();
        for cpu in 0..2 {
            let mut corrupt = baseline.clone();
            corrupt[1]["cpus"][cpu]["full_tlbi"] = json!(0);
            assert!(validate(&corrupt, true, &expected, 2).is_err());
        }
        let mut missing = valid.clone();
        missing.remove(1);
        assert!(validate(&missing, true, &expected, 2).is_err());
        let mut duplicate = valid.clone();
        duplicate.insert(1, valid[1].clone());
        assert!(validate(&duplicate, true, &expected, 2).is_err());
    }
    #[test]
    fn copy_measurements_require_complete_unique_attribution_and_valid_samples() {
        let expected = ["user_copy_el0_boundary_and_snapshot"];
        let sample = |scope: &str, cpu: Option<usize>| {
            let mut event = json!({"event":"measurement","scope":scope,"units":"timer_ticks","frequency":62500000,"warmup":4,"iterations":2,"median":1,"p95":2,"p99":2,"samples":[1,2]});
            if let Some(cpu) = cpu {
                event["cpu"] = json!(cpu);
            }
            event
        };
        let mut events = vec![
            json!({"event":"test","name":expected[0],"status":"pass"}),
            sample("lock_uncontended", None),
        ];
        for cpu in 0..2 {
            for scope in [
                "user_copy_small",
                "user_copy_page",
                "user_copy_three_pages",
                "user_copy_failure",
            ] {
                events.push(sample(scope, Some(cpu)));
            }
        }
        events.push(json!({"event":"suite","status":"pass","tests":1}));
        validate(&events, true, &expected, 2).unwrap();
        let mut missing = events.clone();
        missing.remove(2);
        assert!(validate(&missing, true, &expected, 2).is_err());
        let mut duplicate = events.clone();
        duplicate.insert(2, events[2].clone());
        assert!(validate(&duplicate, true, &expected, 2).is_err());
        for (field, value) in [
            ("cpu", json!(2)),
            ("scope", json!("user_copy_unknown")),
            ("median", json!(9)),
            ("iterations", json!(3)),
        ] {
            let mut corrupt = events.clone();
            corrupt[2][field] = value;
            assert!(
                validate(&corrupt, true, &expected, 2).is_err(),
                "accepted corrupt {field}"
            );
        }
    }
}
