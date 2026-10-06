//! One task inventory for both serial acceptance and partitioned CI execution.
use super::*;

#[derive(Clone)]
struct Task {
    id: String,
    prod: bool,
    tests: bool,
    flag: Option<&'static str>,
    feature: Option<&'static str>,
    marker: Option<String>,
    ipc_test: Option<&'static str>,
}

fn tasks() -> Vec<Task> {
    let mut tasks = Vec::new();
    for prod in [false, true] {
        for tests in [true, false] {
            tasks.push(Task {
                id: format!(
                    "{}-{}",
                    if prod { "prod" } else { "dev" },
                    if tests { "tests" } else { "boot" }
                ),
                prod,
                tests,
                flag: None,
                feature: None,
                marker: None,
                ipc_test: None,
            });
        }
    }
    for &(flag, marker) in NEGATIVE_CONTROLS {
        let feature = match flag {
            "--negative-control" => "negative-test",
            "--panic-control" => "panic-test",
            "--ownership-control" => "ownership-test",
            "--retained-mapping-control" => "retained-mapping-test",
            "--secondary-panic-control" => "secondary-panic-test",
            "--retirement-control" => "retirement-negative",
            "--shootdown-control" => "shootdown-negative",
            "--remote-tlbi-control" => "remote-tlbi-negative",
            "--user-context-control" => "user-context-negative",
            "--user-root-control" => "user-root-negative",
            "--user-retirement-control" => "user-retirement-negative",
            "--asid-reuse-control" => "asid-reuse-negative",
            _ => unreachable!("unknown registered negative control"),
        };
        tasks.push(control(false, flag, feature, Some(marker.to_owned()), None));
    }
    for prod in [false, true] {
        for &(flag, feature, error) in SCHEDULER_CONTROLS {
            tasks.push(control(
                prod,
                flag,
                feature,
                Some(format!("\"error\":\"{error}\"")),
                None,
            ));
        }
        for &(flag, feature, marker) in FOUNDATION_CONTROLS
            .iter()
            .chain(USER_COPY_CONTROLS)
            .chain(HANDLE_CONTROLS)
            .chain(SECURITY_CONTROLS)
        {
            tasks.push(control(prod, flag, feature, Some(marker.to_owned()), None));
        }
        for &(flag, feature, test) in IPC_CONTROLS {
            tasks.push(control(prod, flag, feature, None, Some(test)));
        }
    }
    tasks
}

fn control(
    prod: bool,
    flag: &'static str,
    feature: &'static str,
    marker: Option<String>,
    ipc_test: Option<&'static str>,
) -> Task {
    Task {
        id: format!("{}-{flag}", if prod { "prod" } else { "dev" }),
        prod,
        tests: true,
        flag: Some(flag),
        feature: Some(feature),
        marker,
        ipc_test,
    }
}

fn describe(task: &Task) -> Value {
    json!({"id":task.id,"profile":if task.prod {"prod"} else {"dev"},"tests":task.tests,"flag":task.flag,"feature":task.feature,"expected_marker":task.marker,"expected_ipc_test":task.ipc_test})
}

pub fn plan_document() -> Result<Value> {
    Ok(
        json!({"schema_version":1,"source_files":source_inventory()?,"qemu_arguments":qemu_args(Path::new("<ELF>")),"tasks":tasks().iter().map(describe).collect::<Vec<_>>()}),
    )
}

fn selected(position: usize, shard: Option<(usize, usize)>) -> bool {
    shard.is_none_or(|(index, count)| position % count == index)
}

fn ipc_failure(events: &Value, test: &str) -> Result<bool> {
    Ok(events
        .as_array()
        .ok_or("IPC control events absent")?
        .iter()
        .any(|event| {
            ((event["event"] == "test" && event["name"] == test)
                || test
                    .strip_prefix("reject:")
                    .is_some_and(|error| event["event"] == "ipc-reject" && event["error"] == error)
                || test.strip_prefix("supervision:").is_some_and(|error| {
                    event["event"] == "supervision-reject" && event["error"] == error
                }))
                && event["status"] == "fail"
        }))
}

fn execute_task(task: &Task) -> Result<Value> {
    let profile = if task.prod { "prod" } else { "dev" };
    let artifact = format!(
        "target/kernel/{profile}-{}",
        task.feature
            .unwrap_or(if task.tests { "tests" } else { "boot" })
    );
    // Retained metadata must have been produced by this actual task invocation.
    for suffix in [".run.json", ".results.json", "-build.json"] {
        match fs::remove_file(format!("{artifact}{suffix}")) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    if let Some(flag) = task.flag {
        let mut command = Command::new(env::current_exe()?);
        command.args(["test", flag]);
        if task.prod {
            command.arg("--prod");
        }
        let output = command.output()?;
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        fs::write(format!("target/kernel/{}.host.log", task.id), &text)?;
        let witnessed = if task.feature == Some("irq-simd-restore-negative") {
            irq_context_failure(&read_json(format!("{artifact}.results.json"))?)?
        } else if task.feature == Some("checkpoint-publication-negative") {
            let events = read_json(format!("{artifact}.results.json"))?;
            let events = events
                .as_array()
                .ok_or("checkpoint control events absent")?;
            let held = events.iter().any(|event| {
                event["event"] == "scheduler-reject"
                    && event["error"] == "CheckpointPublicationHeld"
                    && event["quiescent"] == true
                    && event["status"] == "fail"
            });
            let bounded_failure = events.iter().any(|event| {
                event["event"] == "scheduler-reject"
                    && event["error"] == "CompletionPublicationTimeout"
                    && event["status"] == "fail"
            });
            held && bounded_failure
        } else if let Some(test) = task.ipc_test {
            ipc_failure(&read_json(format!("{artifact}.results.json"))?, test)?
        } else {
            task.marker
                .as_ref()
                .is_some_and(|marker| text.contains(marker))
        };
        if output.status.success() || !witnessed {
            return Err(format!("required negative control not witnessed: {}", task.id).into());
        }
    } else {
        let elf = build(task.prod, task.tests, None, true)?;
        execute(&elf, task.tests, true)?;
    }
    println!("Matrix task passed: {} (executed)", task.id);
    Ok(
        json!({"id":task.id,"outcome":"passed","observation":"executed","build":read_json(format!("{artifact}-build.json"))?,"run":read_json(format!("{artifact}.run.json"))?}),
    )
}

pub fn run(shard: Option<(usize, usize)>) -> Result<()> {
    if shard.is_some_and(|(index, count)| count == 0 || count > 16 || index >= count) {
        return Err("invalid matrix shard; require 0 <= index < count <= 16".into());
    }
    let destination = shard.map_or_else(
        || "target/kernel/matrix-execution.json".to_owned(),
        |(index, _)| format!("target/kernel/shard-{index}.json"),
    );
    if Path::new(&destination).exists() {
        fs::remove_file(&destination)?;
    }
    let plan = plan_document()?;
    let mut completed = Vec::new();
    for (position, task) in tasks().iter().enumerate() {
        if selected(position, shard) {
            completed.push(execute_task(task)?);
        }
    }
    if plan["source_files"] != source_inventory()? {
        return Err("matrix sources changed during execution".into());
    }
    fs::write(
        destination,
        serde_json::to_string_pretty(
            &json!({"schema_version":1,"plan":plan,"shard_index":shard.map(|v|v.0),"shard_count":shard.map(|v|v.1),"completed":completed,"scope":if shard.is_some() {"partial; requires exact-source aggregate"} else {"complete serial matrix"}}),
        )?,
    )?;
    Ok(())
}

pub fn run_ipc() -> Result<()> {
    for task in tasks().iter().filter(|task| task.ipc_test.is_some()) {
        execute_task(task)?;
    }
    Ok(())
}

fn irq_context_failure(events: &Value) -> Result<bool> {
    let events = events.as_array().ok_or("IRQ control events absent")?;
    Ok(events
        .iter()
        .any(|e| e["event"] == "test" && e["name"] == "irq_simd_context" && e["status"] == "fail")
        && events.iter().any(|e| {
            e["event"] == "irq-context"
                && e["status"] == "fail"
                && e["probe_result"] == 0
                && e["deliveries"].as_u64().is_some_and(|n| n > 0)
        }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn irq_control_requires_named_failure_and_real_delivery() {
        let failure = json!({"event":"test","name":"irq_simd_context","status":"fail"});
        let evidence =
            json!({"event":"irq-context","status":"fail","probe_result":0,"deliveries":1});
        assert!(irq_context_failure(&json!([failure.clone(), evidence.clone()])).unwrap());
        assert!(!irq_context_failure(&json!([failure.clone()])).unwrap());
        assert!(
            !irq_context_failure(&json!([evidence.clone(),{"event":"panic","status":"fail"}]))
                .unwrap()
        );
        assert!(!irq_context_failure(&json!([failure,{"event":"irq-context","status":"fail","probe_result":0,"deliveries":0}])).unwrap());
    }
    #[test]
    fn shards_partition_every_positive_and_negative_task_once() {
        let plan = tasks();
        let expected = 4
            + NEGATIVE_CONTROLS.len()
            + 2 * (FOUNDATION_CONTROLS.len()
                + SCHEDULER_CONTROLS.len()
                + USER_COPY_CONTROLS.len()
                + HANDLE_CONTROLS.len()
                + SECURITY_CONTROLS.len()
                + IPC_CONTROLS.len());
        assert_eq!(plan.len(), expected);
        assert_eq!(
            plan.iter().map(|t| &t.id).collect::<BTreeSet<_>>().len(),
            expected
        );
        for count in [1, 2, 4, 8] {
            for position in 0..plan.len() {
                assert_eq!(
                    (0..count)
                        .filter(|index| selected(position, Some((*index, count))))
                        .count(),
                    1
                );
            }
        }
    }
    #[test]
    fn ipc_requires_the_registered_failure_not_an_unrelated_panic() {
        assert!(
            !ipc_failure(
                &json!([{"event":"test","name":"other","status":"fail"}]),
                "actual"
            )
            .unwrap()
        );
        assert!(
            !ipc_failure(
                &json!([{"event":"test","name":"actual","status":"pass"}]),
                "actual"
            )
            .unwrap()
        );
        assert!(
            ipc_failure(
                &json!([{"event":"test","name":"actual","status":"fail"}]),
                "actual"
            )
            .unwrap()
        );
        assert!(
            ipc_failure(
                &json!([{"event":"ipc-reject","error":"WrongProcessWake","status":"fail"}]),
                "reject:WrongProcessWake"
            )
            .unwrap()
        );
    }
    #[test]
    fn supervision_requires_its_exact_failure_kind_and_error() {
        let exact =
            json!([{"event":"supervision-reject","error":"WaitIdentityLost","status":"fail"}]);
        assert!(ipc_failure(&exact, "supervision:WaitIdentityLost").unwrap());
        for event in [
            json!({"event":"supervision-reject","error":"Other","status":"fail"}),
            json!({"event":"ipc-reject","error":"WaitIdentityLost","status":"fail"}),
            json!({"event":"supervision-reject","error":"WaitIdentityLost","status":"pass"}),
            json!({"event":"panic","status":"fail"}),
        ] {
            assert!(!ipc_failure(&json!([event]), "supervision:WaitIdentityLost").unwrap());
        }
        assert!(ipc_failure(&json!({}), "supervision:WaitIdentityLost").is_err());
    }
}
