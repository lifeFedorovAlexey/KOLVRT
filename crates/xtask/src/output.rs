use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Value, json};
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

/// Fresh machine-mode execution must carry its one validated boot inventory.
/// Historical receipts continue to use `validate`, where inventory is optional.
pub fn require_current_device_observation(events: &[Value]) -> Result<()> {
    let mut matching = events
        .iter()
        .filter(|event| event["event"] == "device-observation");
    let event = matching
        .next()
        .ok_or("missing current device observation")?;
    if matching.next().is_some() {
        return Err("duplicate current device observation".into());
    }
    validate_device_observation(event)
}

fn validate_device_observation(event: &Value) -> Result<()> {
    // This runner targets the pinned QEMU platform, not arbitrary firmware.
    let expected = kernel_core::platform::discover(include_bytes!(
        "../../../research/fixtures/virt-10.1.dtb"
    ))?
    .boot_console()
    .map_err(|_| "validated console descriptor absent")?;
    if event["status"] != "pass"
        || event["kind"] != "pl011"
        || event["reservation"] != "boot-console"
        || event["scope"].as_u64() != Some(1)
        || event["generation"].as_u64() != Some(1)
        || event["mmio_base"].as_u64() != Some(expected.mmio_base())
        || event["mmio_size"].as_u64() != Some(expected.mmio_size())
        || event["irq"].as_u64() != Some(u64::from(expected.irq()))
        || event["interrupt_controller"].as_u64()
            != Some(u64::from(expected.interrupt_controller()))
    {
        return Err("invalid or duplicate device observation".into());
    }
    Ok(())
}

pub fn validate(events: &[Value], tests: bool, expected: &[&str], cpus: usize) -> Result<()> {
    let mut ipc_runs = BTreeSet::new();
    let mut ipc_deadlines = BTreeSet::new();
    let mut ipc_lifetimes = BTreeSet::new();
    let mut requester_deaths = BTreeSet::new();
    let mut authority_runs = BTreeSet::new();
    let mut payload_runs = BTreeSet::new();
    let mut queue_runs = BTreeSet::new();
    let mut producer_runs = BTreeSet::new();
    let mut queued_cancels = BTreeSet::new();
    let mut terminal_deaths = BTreeSet::new();
    let mut revoke_races = BTreeSet::new();
    let mut closing_runs = BTreeSet::new();
    let mut empty_service_deaths = BTreeSet::new();
    let mut quota_failures = BTreeSet::new();
    let mut ipc_measurements = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut terminal = false;
    let mut measurements = BTreeSet::new();
    let mut asid_measurement = false;
    let asid_required = expected.iter().any(|name| name.starts_with("asid_"));
    let mut el0 = false;
    let mut device_observation = false;
    for event in events {
        if matches!(event["event"].as_str(), Some("fatal" | "panic")) || event["status"] == "fail" {
            return Err(format!("kernel failure: {event}").into());
        }
        if terminal {
            return Err("event after terminal kernel result".into());
        }
        match event["event"].as_str() {
            Some("device-observation") => {
                if device_observation {
                    return Err("duplicate device observation".into());
                }
                validate_device_observation(event)?;
                device_observation = true;
            }
            Some("ipc-measurement") => {
                let scope = event["scope"]
                    .as_str()
                    .ok_or("IPC measurement scope absent")?;
                let client = event["client_cpu"]
                    .as_u64()
                    .ok_or("measurement client absent")?;
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("measurement service absent")?;
                let payload = event["payload"]
                    .as_u64()
                    .ok_or("measurement payload absent")?;
                let controlled = [
                    "hot_ready_receive",
                    "blocked_receive_wake",
                    "blocked_requester_wait",
                    "queue_full_rejection",
                ]
                .contains(&scope);
                if !["submit", "receive", "reply", "collect", "round_trip"].contains(&scope)
                    && !controlled
                    || event["controlled_path"] != controlled
                    || client >= cpus as u64
                    || service >= cpus as u64
                    || ![0, 8, 64, 256].contains(&payload)
                    || event["units"] != "timer_ticks"
                    || event["frequency"]
                        .as_u64()
                        .is_none_or(|frequency| frequency == 0)
                    || event["warmup"] != 4
                    || event["iterations"] != 16
                    || !ipc_measurements.insert((scope, client, service, payload))
                {
                    return Err("invalid or duplicate IPC measurement".into());
                }
                let mut samples = event["samples"]
                    .as_array()
                    .ok_or("IPC samples absent")?
                    .iter()
                    .map(|sample| sample.as_u64().ok_or("invalid IPC sample"))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                if samples.len() != 16 {
                    return Err("incomplete IPC samples".into());
                }
                let quantiles = kernel_core::quantiles(&mut samples).unwrap();
                for (name, value) in ["median", "p95", "p99"].into_iter().zip(quantiles) {
                    if event[name].as_u64() != Some(value) {
                        return Err("IPC quantile mismatch".into());
                    }
                }
            }
            Some("ipc-multiple-producers") => {
                let first = event["first_cpu"]
                    .as_u64()
                    .ok_or("first producer CPU absent")?;
                let second = event["second_cpu"]
                    .as_u64()
                    .ok_or("second producer CPU absent")?;
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("producer service CPU absent")?;
                if ![(0, 0, 0), (1, 1, 1), (0, 1, 0), (0, 1, 1)].contains(&(first, second, service))
                    || event["status"] != "pass"
                    || event["capacity"] != 4
                    || event["requests"] != 32
                    || event["fifo"] != true
                    || event["reclaimed"] != true
                    || !producer_runs.insert((first, second, service))
                {
                    return Err("invalid concurrent producer evidence".into());
                }
            }
            Some("ipc-queued-cancel") => {
                let client = event["client_cpu"]
                    .as_u64()
                    .ok_or("cancel client CPU absent")?;
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("cancel service CPU absent")?;
                if client >= cpus as u64
                    || service >= cpus as u64
                    || event["status"] != "pass"
                    || event["capacity"] != 4
                    || event["delivered"] != 2
                    || event["fifo"] != true
                    || event["full_rejected"] != true
                    || event["reclaimed"] != true
                    || !queued_cancels.insert((client, service))
                {
                    return Err("invalid queued head/non-head cancel evidence".into());
                }
            }
            Some("ipc-terminal-death") => {
                let client = event["client_cpu"]
                    .as_u64()
                    .ok_or("terminal death client absent")?;
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("terminal death service absent")?;
                if client >= cpus as u64
                    || service >= cpus as u64
                    || event["status"] != "pass"
                    || event["completed_unconsumed"] != true
                    || event["reclaimed"] != true
                    || !terminal_deaths.insert((client, service))
                {
                    return Err("invalid unconsumed terminal death evidence".into());
                }
            }
            Some("ipc-revoke-race") => {
                let first = event["first_cpu"]
                    .as_u64()
                    .ok_or("revoke issuer CPU absent")?;
                let second = event["second_cpu"]
                    .as_u64()
                    .ok_or("revoke sender CPU absent")?;
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("revoke service CPU absent")?;
                if ![(0, 0, 0), (1, 1, 1), (0, 1, 0), (0, 1, 1)].contains(&(first, second, service))
                    || event["status"] != "pass"
                    || event["attempts_per_client"] != 16
                    || event["issuer_post_revoke_denials"] != 14
                    || event["accepted_completed"]
                        .as_u64()
                        .is_none_or(|count| count > 4)
                    || event["reclaimed"] != true
                    || !revoke_races.insert((first, second, service))
                {
                    return Err("invalid revoke/admission race evidence".into());
                }
            }
            Some(name @ ("ipc-both-death" | "ipc-shutdown-load")) => {
                let client = event["client_cpu"]
                    .as_u64()
                    .ok_or("closing client absent")?;
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("closing service absent")?;
                if client >= cpus as u64
                    || service >= cpus as u64
                    || event["status"] != "pass"
                    || event["capacity"] != 4
                    || event["delivered"] != 0
                    || event["reclaimed"] != true
                    || (name == "ipc-shutdown-load"
                        && event["client_terminal_blocks"]
                            .as_u64()
                            .is_none_or(|count| count == 0))
                    || !closing_runs.insert((name, client, service))
                {
                    return Err("invalid closing/death evidence".into());
                }
            }
            Some("ipc-empty-service-death") => {
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("empty service CPU absent")?;
                if service >= cpus as u64
                    || event["status"] != "pass"
                    || event["reclaimed"] != true
                    || !empty_service_deaths.insert(service)
                {
                    return Err("invalid empty service death evidence".into());
                }
            }
            Some("ipc-request-quota") => {
                let client = event["client_cpu"].as_u64().ok_or("quota client absent")?;
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("quota service absent")?;
                if client >= cpus as u64
                    || service >= cpus as u64
                    || event["status"] != "pass"
                    || event["requests_limit"] != 0
                    || event["exhausted"] != true
                    || event["reclaimed"] != true
                    || !quota_failures.insert((client, service))
                {
                    return Err("invalid request quota rollback evidence".into());
                }
            }
            Some("ipc-queue") => {
                let client = event["client_cpu"]
                    .as_u64()
                    .ok_or("queue client CPU absent")?;
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("queue service CPU absent")?;
                let capacity = event["capacity"].as_u64().ok_or("queue capacity absent")?;
                if event["status"] != "pass"
                    || client >= cpus as u64
                    || service >= cpus as u64
                    || ![1, 4].contains(&capacity)
                    || event["delivered"].as_u64() != Some(capacity)
                    || event["full_rejected"] != true
                    || event["fifo"] != true
                    || event["reclaimed"] != true
                    || !queue_runs.insert((client, service, capacity))
                {
                    return Err("invalid or duplicate bounded queue evidence".into());
                }
            }
            Some("ipc-payload-stress") => {
                let client = event["client_cpu"]
                    .as_u64()
                    .ok_or("payload client CPU absent")?;
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("payload service CPU absent")?;
                let capacity = event["capacity"]
                    .as_u64()
                    .ok_or("payload capacity absent")?;
                if event["status"] != "pass"
                    || client >= cpus as u64
                    || service >= cpus as u64
                    || ![1, 4].contains(&capacity)
                    || event["requests"].as_u64() != Some(24)
                    || event["payload_sizes"] != json!([0, 1, 8, 64, 255, 256])
                    || event["reclaimed"] != true
                    || !payload_runs.insert((client, service, capacity))
                {
                    return Err("invalid or duplicate payload stress evidence".into());
                }
            }
            Some("ipc-authority") => {
                let client = event["client_cpu"]
                    .as_u64()
                    .ok_or("authority client CPU absent")?;
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("authority service CPU absent")?;
                if event["status"] != "pass"
                    || client >= cpus as u64
                    || service >= cpus as u64
                    || event["denied_probes"].as_u64() != Some(13)
                    || event["accepted_after_revoke_close"] != true
                    || event["reclaimed"] != true
                    || !authority_runs.insert((client, service))
                {
                    return Err("invalid or duplicate IPC authority evidence".into());
                }
            }
            Some("ipc-requester-death") => {
                let name = event["name"]
                    .as_str()
                    .ok_or("requester death name absent")?;
                if ![
                    "ipc_requester_death_queued",
                    "ipc_requester_death_delivered",
                    "ipc_requester_death_committed",
                ]
                .contains(&name)
                {
                    return Err("unknown requester death stage".into());
                }
                let client = event["client_cpu"].as_u64().ok_or("requester CPU absent")?;
                let service = event["service_cpu"].as_u64().ok_or("service CPU absent")?;
                let survivor = event["survivor_cpu"]
                    .as_u64()
                    .ok_or("survivor CPU absent")?;
                if event["status"] != "pass"
                    || client >= cpus as u64
                    || service >= cpus as u64
                    || survivor >= cpus as u64
                    || survivor == client
                    || event["reclaimed"] != true
                    || event["survivor_completed"] != true
                    || !requester_deaths.insert((name, client, service))
                {
                    return Err("invalid or duplicate requester death evidence".into());
                }
            }
            Some("ipc-lifetime") => {
                let name = event["name"].as_str().ok_or("lifetime case absent")?;
                let outcome = match name {
                    "ipc_service_death_queued"
                    | "ipc_service_death_delivered"
                    | "ipc_cancel_before_commit" => 4,
                    "ipc_service_death_committed" | "ipc_cancel_after_commit" => 5,
                    "ipc_receive_copy_failure_retains_queue"
                    | "ipc_collect_copy_failure_retains_result" => 0,
                    _ => return Err("unknown IPC lifetime case".into()),
                };
                let client = event["client_cpu"]
                    .as_u64()
                    .ok_or("lifetime client CPU absent")?;
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("lifetime service CPU absent")?;
                if event["status"] != "pass"
                    || client >= cpus as u64
                    || service >= cpus as u64
                    || event["outcome"].as_u64() != Some(outcome)
                    || event["reclaimed"] != true
                    || !ipc_lifetimes.insert((name, client, service))
                {
                    return Err("invalid or duplicate IPC lifetime evidence".into());
                }
                for peer in ["client", "service"] {
                    let blocks = event[format!("{peer}_blocks")]
                        .as_u64()
                        .ok_or("lifetime block count absent")?;
                    let wakes = event[format!("{peer}_wakes")]
                        .as_u64()
                        .ok_or("lifetime wake count absent")?;
                    if blocks != wakes {
                        return Err("lifetime exact wake count mismatch".into());
                    }
                }
            }
            Some("ipc-deadline") => {
                let outcome = match event["name"].as_str() {
                    Some("ipc_deadline_before_effect") => 6,
                    Some("ipc_deadline_after_commit") => 5,
                    _ => return Err("unknown IPC deadline case".into()),
                };
                let client = event["client_cpu"]
                    .as_u64()
                    .ok_or("deadline client CPU absent")?;
                let service = event["service_cpu"]
                    .as_u64()
                    .ok_or("deadline service CPU absent")?;
                if event["status"] != "pass"
                    || client >= cpus as u64
                    || service >= cpus as u64
                    || client == service
                    || event["outcome"].as_u64() != Some(outcome)
                    || event["all_blocked"].as_u64().is_none_or(|count| count == 0)
                    || event["reclaimed"] != true
                    || !ipc_deadlines.insert((outcome, client, service))
                {
                    return Err("invalid or duplicate blocked deadline evidence".into());
                }
                for peer in ["client", "service"] {
                    let blocks = event[format!("{peer}_blocks")]
                        .as_u64()
                        .ok_or("deadline block count absent")?;
                    let wakes = event[format!("{peer}_wakes")]
                        .as_u64()
                        .ok_or("deadline wake count absent")?;
                    if blocks == 0 || blocks != wakes {
                        return Err("deadline did not wake exact blocked peer".into());
                    }
                }
            }
            Some("supervision") => {
                if event["status"] != "pass"
                    || event["coverage"] != 255
                    || event["reclaimed"] != true
                    || event["owners_released"] != true
                {
                    return Err("invalid supervision evidence".into());
                }
            }
            Some("ipc") => {
                let (client, service) = match event["name"].as_str() {
                    Some("ipc_el0_cpu0_to_cpu1") => (0, 1),
                    Some("ipc_el0_cpu1_to_cpu0") => (1, 0),
                    Some("ipc_el0_same_cpu0") => (0, 0),
                    Some("ipc_el0_same_cpu1") => (1, 1),
                    _ => return Err("unknown IPC matrix case".into()),
                };
                let capacity = event["capacity"].as_u64().ok_or("IPC capacity absent")?;
                if event["status"] != "pass"
                    || ![1, 4].contains(&capacity)
                    || event["client_cpu"].as_u64() != Some(client)
                    || event["service_cpu"].as_u64() != Some(service)
                    || client >= cpus as u64
                    || service >= cpus as u64
                    || !ipc_runs.insert((client, service, capacity))
                {
                    return Err("invalid or duplicate IPC matrix evidence".into());
                }
                for peer in ["client", "service"] {
                    let generation = event[format!("{peer}_generation")]
                        .as_u64()
                        .ok_or("IPC generation absent")?;
                    let blocks = event[format!("{peer}_blocks")]
                        .as_u64()
                        .ok_or("IPC block count absent")?;
                    let wakes = event[format!("{peer}_wakes")]
                        .as_u64()
                        .ok_or("IPC wake count absent")?;
                    if generation == 0 || blocks != wakes {
                        return Err("IPC exact wake count mismatch".into());
                    }
                }
            }
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
    // Historical output-policy receipts remain readable. Current boot declares
    // the IPC matrix explicitly; current suites require its named real tests.
    let ipc_required = expected.contains(&"ipc_el0_cpu0_to_cpu1")
        || events
            .iter()
            .any(|event| event["event"] == "boot" && event.get("ipc_runs").is_some());
    if ipc_required
        && (ipc_runs.len() != 8
            || events
                .iter()
                .any(|event| event["event"] == "boot" && event["ipc_runs"].as_u64() != Some(8)))
    {
        return Err("missing or invalid current IPC matrix".into());
    }
    let deadline_required = expected.contains(&"ipc_deadline_before_effect")
        || events
            .iter()
            .any(|event| event["event"] == "boot" && event.get("ipc_deadline_runs").is_some());
    if deadline_required
        && (ipc_deadlines.len() != 4
            || events.iter().any(|event| {
                event["event"] == "boot" && event["ipc_deadline_runs"].as_u64() != Some(4)
            }))
    {
        return Err("missing current blocked deadline matrix".into());
    }
    let lifetime_required = expected.contains(&"ipc_service_death_queued")
        || events
            .iter()
            .any(|event| event["event"] == "boot" && event.get("ipc_lifetime_runs").is_some());
    if lifetime_required
        && (ipc_lifetimes.len() != 28
            || events.iter().any(|event| {
                event["event"] == "boot" && event["ipc_lifetime_runs"].as_u64() != Some(28)
            }))
    {
        return Err("missing current IPC lifetime matrix".into());
    }
    if !ipc_measurements.is_empty() && ipc_measurements.len() != 144 {
        return Err("incomplete IPC performance matrix".into());
    }
    if expected.contains(&"ipc_concurrent_producers_fifo_and_reclamation")
        && producer_runs.len() != 4
    {
        return Err("incomplete concurrent producer matrix".into());
    }
    if expected.contains(&"ipc_queued_head_nonhead_cancel_fifo") && queued_cancels.len() != 4 {
        return Err("incomplete queued head/non-head cancel matrix".into());
    }
    if expected.contains(&"ipc_unconsumed_terminal_domain_teardown") && terminal_deaths.len() != 4 {
        return Err("incomplete unconsumed terminal domain teardown matrix".into());
    }
    if expected.contains(&"ipc_concurrent_revoke_admission_retains_accepted")
        && revoke_races.len() != 4
    {
        return Err("incomplete IPC revoke/admission race matrix".into());
    }
    if expected.contains(&"ipc_both_peers_die_with_accepted_work") && closing_runs.len() != 8 {
        return Err("incomplete shutdown/both-death matrix".into());
    }
    if expected.contains(&"ipc_empty_service_death_reclamation")
        && empty_service_deaths.len() != cpus
    {
        return Err("incomplete empty service death matrix".into());
    }
    if expected.contains(&"ipc_request_quota_failure_has_no_phantom_work")
        && quota_failures.len() != 4
    {
        return Err("incomplete IPC quota rollback matrix".into());
    }
    let queue_required = expected.contains(&"ipc_queue_full_fifo_and_reclamation")
        || events
            .iter()
            .any(|event| event["event"] == "boot" && event.get("ipc_queue_runs").is_some());
    if queue_required
        && (queue_runs.len() != 8
            || events.iter().any(|event| {
                event["event"] == "boot" && event["ipc_queue_runs"].as_u64() != Some(8)
            }))
    {
        return Err("missing current queue matrix".into());
    }
    let payload_required = expected.contains(&"ipc_payload_snapshot_result_id_and_stress")
        || events
            .iter()
            .any(|event| event["event"] == "boot" && event.get("ipc_payload_runs").is_some());
    if payload_required
        && (payload_runs.len() != 8
            || events.iter().any(|event| {
                event["event"] == "boot" && event["ipc_payload_runs"].as_u64() != Some(8)
            }))
    {
        return Err("missing current payload stress matrix".into());
    }
    let authority_required = expected.contains(&"ipc_authority_denial_revoke_and_retention")
        || events
            .iter()
            .any(|event| event["event"] == "boot" && event.get("ipc_authority_runs").is_some());
    if authority_required
        && (authority_runs.len() != 4
            || events.iter().any(|event| {
                event["event"] == "boot" && event["ipc_authority_runs"].as_u64() != Some(4)
            }))
    {
        return Err("missing current authority matrix".into());
    }
    let requester_required = expected.contains(&"ipc_requester_death_queued")
        || events.iter().any(|event| {
            event["event"] == "boot" && event.get("ipc_requester_death_runs").is_some()
        });
    if requester_required
        && (requester_deaths.len() != 12
            || events.iter().any(|event| {
                event["event"] == "boot" && event["ipc_requester_death_runs"].as_u64() != Some(12)
            }))
    {
        return Err("missing current requester death matrix".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn blocked_deadline_and_lifetime_receipts_reject_missing_or_corrupt_controls() {
        let mut events = vec![el0()];
        for (name, outcome) in [
            ("ipc_deadline_before_effect", 6),
            ("ipc_deadline_after_commit", 5),
        ] {
            for (client, service) in [(0, 1), (1, 0)] {
                events.push(json!({"event":"ipc-deadline", "status":"pass", "name":name,
                    "client_cpu":client, "service_cpu":service, "outcome":outcome,
                    "all_blocked":1, "client_blocks":2, "client_wakes":2,
                    "service_blocks":1, "service_wakes":1, "reclaimed":true}));
            }
        }
        for (name, outcome) in [
            ("ipc_service_death_queued", 4),
            ("ipc_service_death_delivered", 4),
            ("ipc_service_death_committed", 5),
            ("ipc_receive_copy_failure_retains_queue", 0),
            ("ipc_collect_copy_failure_retains_result", 0),
            ("ipc_cancel_before_commit", 4),
            ("ipc_cancel_after_commit", 5),
        ] {
            for (client, service) in [(0, 1), (1, 0), (0, 0), (1, 1)] {
                events.push(json!({"event":"ipc-lifetime", "status":"pass", "name":name,
                    "client_cpu":client, "service_cpu":service, "outcome":outcome,
                    "client_blocks":1, "client_wakes":1, "service_blocks":0,
                    "service_wakes":0, "reclaimed":true}));
            }
        }
        let mut terminal = boot();
        terminal["ipc_deadline_runs"] = json!(4);
        terminal["ipc_lifetime_runs"] = json!(28);
        events.push(terminal);
        validate(&events, false, &[], 2).unwrap();
        for (index, field, bad) in [
            (1, "all_blocked", json!(0)),
            (1, "outcome", json!(0)),
            (1, "client_blocks", json!(0)),
            (1, "client_wakes", json!(1)),
            (1, "client_cpu", json!(1)),
            (5, "outcome", json!(6)),
            (5, "reclaimed", json!(false)),
            (5, "service_wakes", json!(1)),
            (5, "service_cpu", json!(2)),
            (5, "name", json!("unknown")),
        ] {
            for value in [bad, Value::Null, json!("wrong-type")] {
                let mut corrupt = events.clone();
                corrupt[index][field] = value;
                assert!(
                    validate(&corrupt, false, &[], 2).is_err(),
                    "accepted invalid {index}.{field}"
                );
            }
        }
        for index in [1, 5] {
            let mut missing = events.clone();
            missing.remove(index);
            assert!(validate(&missing, false, &[], 2).is_err());
            let mut duplicate = events.clone();
            duplicate.insert(index, events[index].clone());
            assert!(validate(&duplicate, false, &[], 2).is_err());
        }
    }
    #[test]
    fn ipc_receipt_requires_complete_matrix_and_exact_typed_wake_counts() {
        let mut events = vec![el0()];
        for (name, client, service) in [
            ("ipc_el0_cpu0_to_cpu1", 0, 1),
            ("ipc_el0_cpu1_to_cpu0", 1, 0),
            ("ipc_el0_same_cpu0", 0, 0),
            ("ipc_el0_same_cpu1", 1, 1),
        ] {
            for capacity in [1, 4] {
                events.push(json!({"event":"ipc", "status":"pass", "name":name,
                    "capacity":capacity, "client_cpu":client, "service_cpu":service,
                    "client_generation":1, "service_generation":2,
                    "client_blocks":1, "client_wakes":1, "service_blocks":0, "service_wakes":0}));
            }
        }
        let mut terminal = boot();
        terminal["ipc_runs"] = json!(8);
        events.push(terminal);
        validate(&events, false, &[], 2).unwrap();
        for (field, bad) in [
            ("capacity", json!(2)),
            ("client_cpu", json!(1)),
            ("service_cpu", json!(0)),
            ("client_generation", json!(0)),
            ("service_generation", json!(0)),
            ("client_wakes", json!(2)),
            ("service_wakes", json!(1)),
            ("status", json!("fail")),
            ("name", json!("other")),
        ] {
            for value in [bad, Value::Null, json!("wrong-type")] {
                let mut corrupt = events.clone();
                corrupt[1][field] = value;
                assert!(
                    validate(&corrupt, false, &[], 2).is_err(),
                    "accepted invalid IPC {field}"
                );
            }
        }
        let mut missing = events.clone();
        missing.remove(1);
        assert!(validate(&missing, false, &[], 2).is_err());
        let mut duplicate = events.clone();
        duplicate.insert(1, events[1].clone());
        assert!(validate(&duplicate, false, &[], 2).is_err());
    }
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
    fn device_inventory_is_optional_but_present_fields_and_uniqueness_are_checked() {
        let descriptor = kernel_core::platform::discover(include_bytes!(
            "../../../research/fixtures/virt-10.1.dtb"
        ))
        .unwrap()
        .boot_console()
        .unwrap();
        let inventory = json!({"event":"device-observation","status":"pass",
            "kind":"pl011","reservation":"boot-console","scope":1,"generation":1,
            "mmio_base":descriptor.mmio_base(),"mmio_size":descriptor.mmio_size(),
            "irq":descriptor.irq(),"interrupt_controller":descriptor.interrupt_controller()});
        validate(&[inventory.clone(), el0(), boot()], false, &[], 2).unwrap();
        require_current_device_observation(&[inventory.clone(), el0(), boot()]).unwrap();
        assert!(require_current_device_observation(&[el0(), boot()]).is_err());
        assert!(
            require_current_device_observation(&[inventory.clone(), inventory.clone()]).is_err()
        );
        validate(&[el0(), boot()], false, &[], 2).unwrap();
        assert!(
            validate(
                &[inventory.clone(), inventory.clone(), el0(), boot()],
                false,
                &[],
                2
            )
            .is_err()
        );
        for (field, bad) in [
            ("status", json!("unknown")),
            ("kind", json!("virtio")),
            ("reservation", json!("driver")),
            ("scope", json!(2)),
            ("generation", json!(2)),
            ("mmio_base", json!(descriptor.mmio_base() + 4096)),
            ("mmio_size", json!(descriptor.mmio_size() + 4096)),
            ("irq", json!(descriptor.irq() + 1)),
            (
                "interrupt_controller",
                json!(descriptor.interrupt_controller() + 1),
            ),
        ] {
            for value in [bad, Value::Null, json!("wrong-type")] {
                let mut invalid = inventory.clone();
                invalid[field] = value;
                assert!(require_current_device_observation(&[invalid.clone()]).is_err());
                assert!(
                    validate(&[invalid, el0(), boot()], false, &[], 2).is_err(),
                    "accepted {field}"
                );
            }
            let mut missing = inventory.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(validate(&[missing, el0(), boot()], false, &[], 2).is_err());
        }
        let stream = format!("[OK] console: UART ready\n{PREFIX}{inventory}\n");
        assert_eq!(parse(&stream).unwrap(), vec![inventory]);
        assert_eq!(console_text(&stream, false), "[OK] console: UART ready\n");
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
