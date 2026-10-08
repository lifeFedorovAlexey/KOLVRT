//! A bounded view of one canonical architecture document, with validated retained measurements.
use super::*;
use std::{fmt::Write as _, path::PathBuf, process::Command};
const DOCUMENT: &str = "docs/architecture/kernel-component-map.md";
const MARKER: &str = "<!-- arena-architecture -->";
const CLOCK: &str = "research/arena/runs/clock-query-2978ad9";
const TEMPLATE: &str = include_str!("architecture.html");
fn text<'a>(value: &'a Value, key: &str) -> CheckResult<&'a str> {
    value[key]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("architecture {key} must be nonempty text"))
}
fn list<'a>(value: &'a Value, key: &str) -> CheckResult<&'a Vec<Value>> {
    value[key]
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or_else(|| format!("architecture {key} must be a nonempty array"))
}
fn safe_file(root: &Path, path: &str) -> CheckResult<PathBuf> {
    let p = Path::new(path);
    if path.contains('\\')
        || p.is_absolute()
        || p.components().any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("unsafe architecture source path: {path}"));
    }
    let resolved =
        fs::canonicalize(root.join(p)).map_err(|e| format!("architecture source {path}: {e}"))?;
    if !resolved.starts_with(fs::canonicalize(root).map_err(|e| e.to_string())?)
        || !resolved.is_file()
    {
        return Err(format!(
            "architecture source escapes repository or is not a file: {path}"
        ));
    }
    Ok(resolved)
}
fn parse(document: &str) -> CheckResult<Value> {
    if document.matches(MARKER).count() != 1 {
        return Err("architecture document must contain one model marker".into());
    }
    let rest = document.split_once(MARKER).unwrap().1.trim_start();
    let rest = rest
        .strip_prefix("```json")
        .ok_or("architecture marker must precede JSON fence")?;
    let (json, _) = rest
        .split_once("```")
        .ok_or("architecture JSON fence not closed")?;
    parse_json(json.trim())
}
fn validate(root: &Path, model: &Value, known: &BTreeSet<String>) -> CheckResult<BTreeSet<String>> {
    if model["schema_version"] != 1 || !matches!(model["version"].as_str(), Some("1" | "2")) {
        return Err("unsupported architecture model schema/version".into());
    }
    let nodes = list(model, "nodes")?;
    let edges = list(model, "edges")?;
    if nodes.len() > 200 || edges.len() > 1000 {
        return Err("architecture view exceeds bounded node/edge limits".into());
    }
    let mut ids = BTreeSet::new();
    let mut all_member_ids = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for node in nodes {
        let id = text(node, "id")?;
        if !ids.insert(id.to_owned()) || !known.contains(id) {
            return Err(format!(
                "duplicate or unknown canonical architecture node: {id}"
            ));
        }
        text(node, "label")?;
        text(node, "responsibility")?;
        if !matches!(text(node, "layer")?, "EL1" | "EL0") {
            return Err("architecture layer must be EL1 or EL0".into());
        }
        for contract in list(node, "contracts")? {
            let id = contract.as_str().ok_or("contract ID must be text")?;
            if !known.contains(id) {
                return Err(format!("unknown architecture contract: {id}"));
            }
        }
        let mut members = BTreeSet::new();
        let mut member_ids = BTreeSet::new();
        let member_list = list(node, "members")?;
        if member_list.is_empty() || member_list.len() > 200 {
            return Err("architecture group must have 1..200 members".into());
        }
        for member in member_list {
            if model["version"] == "2" {
                let member_id = text(member, "id")?;
                let prefix = format!("{id}.");
                if !member_id.starts_with(&prefix)
                    || member_id.len() == prefix.len()
                    || member_id.len() > 240
                    || !member_id.bytes().all(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'.' || c == b'-'
                    })
                    || !all_member_ids.insert(member_id.to_owned())
                {
                    return Err("invalid or duplicate scoped architecture member ID".into());
                }
                member_ids.insert(member_id);
            }
            if !members.insert(text(member, "name")?) {
                return Err("duplicate member in architecture group".into());
            }
            for source in list(member, "sources")? {
                let path = source.as_str().ok_or("source path must be text")?;
                safe_file(root, path)?;
                sources.insert(path.to_owned());
            }
        }
        if model["version"] == "2" {
            let relations = node["member_relations"]
                .as_array()
                .ok_or("architecture member_relations must be an array")?;
            if relations.len() > 1000 {
                return Err("architecture member relations exceed bounded limit".into());
            }
            let mut seen = BTreeSet::new();
            for relation in relations {
                let from = text(relation, "from")?;
                let to = text(relation, "to")?;
                let kind = text(relation, "kind")?;
                let label = text(relation, "label")?;
                if !member_ids.contains(from) || !member_ids.contains(to) {
                    return Err("member relation escapes its architecture group".into());
                }
                if !matches!(kind, "call" | "data" | "authority" | "lifetime")
                    || !seen.insert((from, to, kind, label))
                {
                    return Err("unsupported or duplicate architecture member relation".into());
                }
                let evidence = list(relation, "evidence")?;
                if evidence.is_empty() {
                    return Err("architecture member relation requires source evidence".into());
                }
                for source in evidence {
                    let path = source.as_str().ok_or("member evidence path must be text")?;
                    safe_file(root, path)?;
                    sources.insert(path.to_owned());
                }
            }
        }
    }
    let mut links = BTreeSet::new();
    for edge in edges {
        let (from, to, kind) = (text(edge, "from")?, text(edge, "to")?, text(edge, "kind")?);
        if !ids.contains(from) || !ids.contains(to) {
            return Err("dangling architecture edge".into());
        }
        if !matches!(kind, "call" | "data" | "authority" | "lifetime") {
            return Err("unsupported architecture edge kind".into());
        }
        if model["version"] == "2" {
            let mut endpoint_pairs = BTreeSet::new();
            for pair in list(edge, "member_endpoints")? {
                let source = text(pair, "from_member")?;
                let target = text(pair, "to_member")?;
                if !all_member_ids.contains(source)
                    || !all_member_ids.contains(target)
                    || !source.starts_with(&format!("{from}."))
                    || !target.starts_with(&format!("{to}."))
                    || !endpoint_pairs.insert((source, target))
                {
                    return Err("invalid architecture boundary member endpoints".into());
                }
            }
        }
        text(edge, "label")?;
        if !links.insert((from, to, kind, text(edge, "label")?)) {
            return Err("duplicate architecture edge".into());
        }
        for source in list(edge, "evidence")? {
            let path = source.as_str().ok_or("edge evidence must be text")?;
            safe_file(root, path)?;
            sources.insert(path.to_owned());
        }
    }
    Ok(sources)
}
fn git(root: &Path, args: &[&str]) -> CheckResult<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "architecture source identity unavailable: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}
fn recognized_clock(run: &Value, profile: &str) -> CheckResult<()> {
    let p = &run["profile_snapshot"];
    if p["target"]["block_id"] != "kolvrt.clock.query"
        || p["target"]["contract_id"] != "kolvrt.clock.query.api"
        || p["target"]["functional_version"] != "native-clock-snapshot-1"
        || p["workload"]["id"] != "arena.workload.clock-query"
        || p["workload"]["version"] != "1"
        || p["environment"]["execution_profile"] != profile.to_uppercase()
        || p["environment"]["platform_kind"] != "QEMU"
    {
        return Err("retained file is not the declared CLOCK workload/profile".into());
    }
    let metrics = p["metrics"].as_array().ok_or("CLOCK metric absent")?;
    if metrics.len() != 1
        || metrics[0]["id"] != "arena.metric.clock-query-envelope"
        || metrics[0]["statistic"] != "MEDIAN"
        || metrics[0]["units"] != "timer_ticks"
    {
        return Err("unrecognized CLOCK envelope metric".into());
    }
    Ok(())
}
fn file_uri(root: &Path) -> CheckResult<String> {
    let path = fs::canonicalize(root)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/");
    let encoded = path
        .as_bytes()
        .iter()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~:/".contains(b) {
                (*b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect::<String>();
    Ok(format!(
        "file://{}{encoded}/",
        if encoded.starts_with('/') { "" } else { "/" }
    ))
}
fn stamp(model: &mut Value) {
    model["view"]
        .as_object_mut()
        .unwrap()
        .remove("snapshot_sha256");
    model["view"]["snapshot_sha256"] = json!(digest(model));
}
fn fresh_sources(root: &Path, model: &Value, sources: &BTreeSet<String>) -> CheckResult<()> {
    text(model, "reviewed_base")?;
    let reviewed = model["source_files"]
        .as_object()
        .ok_or("architecture reviewed source_files map absent")?;
    if reviewed.keys().cloned().collect::<BTreeSet<_>>() != *sources {
        return Err("architecture reviewed source map does not exactly cover member and edge evidence files".into());
    }
    let mut stale = Vec::new();
    for source in sources {
        let bytes = fs::read(safe_file(root, source)?).map_err(|e| e.to_string())?;
        let hash = if let Ok(text) = std::str::from_utf8(&bytes) {
            bytes_digest(text.replace("\r\n", "\n").as_bytes())
        } else {
            bytes_digest(&bytes)
        };
        if reviewed[source] != hash {
            stale.push(source.as_str());
        }
    }
    if !stale.is_empty() {
        return Err(format!(
            "STALE architecture sources: {}. Обновите модель после review.",
            stale.join(", ")
        ));
    }
    Ok(())
}
fn measurements(root: &Path, nodes: &mut [Value]) -> Vec<Value> {
    let mut warnings = Vec::new();
    for profile in ["dev", "prod"] {
        for pair in 0..3 {
            let relative = format!("{CLOCK}/{profile}-{pair}-run.json");
            let path = root.join(&relative);
            let checked = (|| -> CheckResult<Value> {
                let run = input(&path)?;
                recognized_clock(&run, profile)?;
                let result = assess(root, bundle_base(&path), &run)?;
                if result["admission_state"] != "STRUCTURALLY_ADMISSIBLE" {
                    return Err(format!(
                        "retained measurement INELIGIBLE: {}",
                        result["reasons"]
                    ));
                }
                let p = &run["profile_snapshot"];
                let frequency = super::report::retained_frequency(bundle_base(&path), &run);
                let values=a(p,"metrics").iter().map(|metric|{let m=&result["measurements"][s(metric,"id")];json!({"id":metric["id"],"label":metric["definition"],"statistic":metric["statistic"],"units":metric["units"],"value":m["value"],"sample_count":m["sample_count"],"microseconds":if metric["units"]=="timer_ticks"{m["value"].as_f64().zip(frequency).map(|(v,f)|v*1_000_000.0/f as f64)}else{None}})}).collect::<Vec<_>>();
                Ok(
                    json!({"path":relative,"run_id":run["run_id"],"execution_profile":p["environment"]["execution_profile"],"platform":p["environment"]["platform_kind"],"block_id":p["target"]["block_id"],"contract_id":p["target"]["contract_id"],"useful_operation":p["workload"]["useful_operation"],"exact_commit":run["provenance"]["exact_commit"],"source_sha256":run["provenance"]["source_sha256"],"frequency":frequency,"metrics":values,"scope":"Historical CLOCK query envelope; not exclusive component CPU cost or current-source acceptance","comparison_class":result["comparison_class"],"admission_state":result["admission_state"],"record_eligible":false}),
                )
            })();
            match checked {
                Ok(value) => {
                    let mut matched = false;
                    for node in nodes.iter_mut() {
                        if node["contracts"].as_array().is_some_and(|contracts| {
                            contracts.contains(&value["block_id"])
                                || contracts.contains(&value["contract_id"])
                        }) {
                            node["measurements"]
                                .as_array_mut()
                                .unwrap()
                                .push(value.clone());
                            matched = true;
                        }
                    }
                    if !matched {
                        warnings.push(json!({"path":relative,"reason":"validated retained target is not mapped to this view"}));
                    }
                }
                Err(error) => warnings.push(json!({"path":relative,"reason":error})),
            }
        }
    }
    warnings
}
pub fn snapshot(root: &Path) -> CheckResult<Value> {
    let raw = fs::read_to_string(root.join(DOCUMENT)).map_err(|e| e.to_string())?;
    let mut model = parse(&raw)?;
    let knowledge = Knowledge::build(root)?;
    let known = knowledge.graph["nodes"]
        .as_object()
        .ok_or("canonical graph nodes absent")?
        .keys()
        .cloned()
        .collect();
    let mut sources = validate(root, &model, &known)?;
    fresh_sources(root, &model, &sources)?;
    sources.insert(DOCUMENT.into());
    let files = sources
        .iter()
        .map(|p| {
            fs::read(root.join(p))
                .map(|bytes| json!({"path":p,"sha256":bytes_digest(&bytes)}))
                .map_err(|e| e.to_string())
        })
        .collect::<CheckResult<Vec<_>>>()?;
    let nodes = model["nodes"].as_array_mut().unwrap();
    for n in nodes.iter_mut() {
        n["measurements"] = json!([]);
        let locations = n["contracts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| {
                let path =
                    knowledge.graph["nodes"][id.as_str().unwrap()]["locations"]["en"]["path"]
                        .as_str()
                        .ok_or("canonical contract location absent")?;
                safe_file(root, path)?;
                Ok(json!({"id":id,"path":path}))
            })
            .collect::<CheckResult<Vec<_>>>()?;
        n["contract_locations"] = json!(locations);
    }
    let warnings = measurements(root, nodes);
    let el1 = nodes.iter().filter(|n| n["layer"] == "EL1").count();
    let el0 = nodes.len() - el1;
    model["view"] = json!({"canonical_document":DOCUMENT,"source_commit":git(root,&["rev-parse","HEAD"] )?,"worktree_dirty":!git(root,&["status","--porcelain"] )?.is_empty(),"source_files":files,"repository_href":file_uri(root)?,"el1_groups":el1,"el0_groups":el0,"edge_count":model["edges"].as_array().unwrap().len(),"scope":"Source-backed component groups and typed relations; overlapping implementation files are intentional; not runtime traffic or cost attribution","measurement_warnings":warnings});
    stamp(&mut model);
    Ok(model)
}
fn short(id: &str) -> &str {
    id.rsplit('.').next().unwrap_or(id)
}
pub fn terminal(model: &Value) -> String {
    let mut out = format!(
        "КАРТА ЯДРА — {} групп EL1 + {} групп EL0\nИсточник: {} · {}{}\n",
        model["view"]["el1_groups"],
        model["view"]["el0_groups"],
        s(&model["view"], "canonical_document"),
        s(&model["view"], "source_commit")
            .chars()
            .take(12)
            .collect::<String>(),
        if model["view"]["worktree_dirty"] == true {
            " + рабочие изменения"
        } else {
            ""
        }
    );
    for layer in ["EL1", "EL0"] {
        writeln!(out, "\n{layer}").unwrap();
        for n in a(model, "nodes").iter().filter(|n| n["layer"] == layer) {
            let members = a(n, "members")
                .iter()
                .map(|m| s(m, "name"))
                .collect::<Vec<_>>()
                .join(" + ");
            let links = a(model, "edges")
                .iter()
                .filter(|e| e["from"] == n["id"])
                .map(|e| format!("{}→{}", s(e, "kind"), short(s(e, "to"))))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(
                out,
                "  {}: {} | {} | {}",
                s(n, "label"),
                members,
                if a(n, "measurements").is_empty() {
                    "замеров нет"
                } else {
                    "есть исторический замер пути"
                },
                links
            )
            .unwrap();
        }
    }
    for n in a(model, "nodes")
        .iter()
        .filter(|n| !a(n, "measurements").is_empty())
    {
        writeln!(
            out,
            "\n{} — CLOCK: 2 граничных вызова + 1 запрос + общий учёт harness",
            s(n, "label")
        )
        .unwrap();
        for profile in ["DEV", "PROD"] {
            let runs: Vec<_> = a(n, "measurements")
                .iter()
                .filter(|r| r["execution_profile"] == profile)
                .collect();
            let values = runs
                .iter()
                .map(|r| {
                    r["metrics"][0]["microseconds"].as_f64().map_or_else(
                        || {
                            format!(
                                "{} {}",
                                r["metrics"][0]["value"],
                                s(&r["metrics"][0], "units")
                            )
                        },
                        |v| format!("{v:.3} мкс"),
                    )
                })
                .collect::<Vec<_>>()
                .join(" / ");
            let ns = runs
                .iter()
                .map(|r| r["metrics"][0]["sample_count"].to_string())
                .collect::<Vec<_>>()
                .join("/");
            writeln!(out, "  {profile}: медианы повторов {values}; N={ns}").unwrap();
        }
    }
    writeln!(out,"\nСвязи: call / data / authority / lifetime; веса и отдельная стоимость компонентов не измерены.").unwrap();
    out.push_str("CLOCK: исторический путь, не эксклюзивная стоимость блока. Изменение: нет базового замера.\n");
    for warning in a(&model["view"], "measurement_warnings") {
        writeln!(
            out,
            "Недоступен замер {}: {}",
            s(warning, "path"),
            s(warning, "reason")
        )
        .unwrap();
    }
    out
}
fn embedded_json(value: &Value) -> CheckResult<String> {
    Ok(serde_json::to_string(value)
        .map_err(|e| e.to_string())?
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029"))
}
pub fn html(model: &Value) -> CheckResult<String> {
    Ok(TEMPLATE.replace("__ARENA_MODEL__", &embedded_json(model)?))
}
pub fn cli(root: &Path, args: &[String]) -> CheckResult<()> {
    let output = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => None,
        ["--html"] => Some(root.join("target/arena-map.html")),
        ["--html", path] => Some(PathBuf::from(path)),
        _ => return Err("usage: cargo xtask arena map [--html [OUTPUT]]".into()),
    };
    let mut view = snapshot(root)?;
    if let Some(path) = output {
        if path.parent().is_some_and(|p| p == root.join("target")) {
            view["view"]["repository_href"] = json!("../");
            stamp(&mut view);
        }
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, html(&view)?).map_err(|e| e.to_string())?;
        println!("Карта архитектуры: {}", path.display());
    } else {
        print!("{}", terminal(&view));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedding_cannot_close_script_or_create_markup() {
        let value = json!({"label":"</script><img src=x onerror=alert(1)>&\u{2028}"});
        let encoded = embedded_json(&value).unwrap();
        assert!(!encoded.contains('<'));
        assert!(!encoded.contains('&'));
        assert_eq!(parse_json(&encoded).unwrap(), value);
        let page = html(&value).unwrap();
        assert!(page.contains("application/json"));
        assert!(!page.contains("innerHTML"));
        assert!(!page.contains("<img src=x"));
    }
    #[test]
    fn clock_attachment_rejects_relabeling_valid_other_workload() {
        let mut run = json!({"profile_snapshot":{"target":{"block_id":"kolvrt.clock.query","contract_id":"kolvrt.clock.query.api","functional_version":"native-clock-snapshot-1"},"workload":{"id":"arena.workload.clock-query","version":"1"},"environment":{"execution_profile":"DEV","platform_kind":"QEMU"},"metrics":[{"id":"arena.metric.clock-query-envelope","statistic":"MEDIAN","units":"timer_ticks"}]}});
        assert!(recognized_clock(&run, "dev").is_ok());
        assert!(recognized_clock(&run, "prod").is_err());
        run["profile_snapshot"]["workload"]["id"] = json!("other.workload");
        assert!(recognized_clock(&run, "dev").is_err());
    }
    #[test]
    fn reviewed_sources_reject_change_missing_and_extra() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let sources = ["Cargo.toml".to_owned()].into_iter().collect();
        let raw = fs::read_to_string(root.join("Cargo.toml")).unwrap();
        let mut model = json!({"reviewed_base":"reviewed-test-input","source_files":{"Cargo.toml":bytes_digest(raw.replace("\r\n","\n").as_bytes())}});
        assert!(fresh_sources(&root, &model, &sources).is_ok());
        model["source_files"]["Cargo.toml"] = json!("stale");
        assert!(
            fresh_sources(&root, &model, &sources)
                .unwrap_err()
                .contains("STALE")
        );
        model["source_files"]["extra.rs"] = json!("hash");
        assert!(fresh_sources(&root, &model, &sources).is_err());
    }
    #[test]
    fn parser_rejects_ambiguous_model_or_wrong_fence() {
        assert!(parse("no marker").is_err());
        assert!(parse(&format!("{MARKER}\ntext")).is_err());
        assert!(parse(&format!("{MARKER}\n```json\n{{}}\n```\n{MARKER}")).is_err());
    }
    #[test]
    fn member_relations_require_local_identity_and_actual_evidence() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let known = ["test.one".to_owned(), "test.contract".to_owned()]
            .into_iter()
            .collect();
        let valid = json!({"schema_version":1,"version":"2","nodes":[{"id":"test.one","label":"One","layer":"EL1","responsibility":"scope","contracts":["test.contract"],"members":[{"id":"test.one.first","name":"First","sources":["Cargo.toml"]},{"id":"test.one.second","name":"Second","sources":["Cargo.toml"]}],"member_relations":[{"from":"test.one.first","to":"test.one.second","kind":"call","label":"Actual call","evidence":["Cargo.toml"]}]}],"edges":[{"from":"test.one","to":"test.one","kind":"data","label":"Owned state","evidence":["Cargo.toml"],"member_endpoints":[{"from_member":"test.one.first","to_member":"test.one.second"}]}]});
        assert!(
            validate(&root, &valid, &known)
                .unwrap()
                .contains("Cargo.toml")
        );
        for bad in ["test.other.member", "test.one.absent"] {
            let mut changed = valid.clone();
            changed["nodes"][0]["member_relations"][0]["to"] = json!(bad);
            assert!(validate(&root, &changed, &known).is_err());
        }
        let mut changed = valid.clone();
        changed["nodes"][0]["member_relations"][0]["evidence"] = json!([]);
        assert!(validate(&root, &changed, &known).is_err());
        let mut changed = valid.clone();
        changed["nodes"][0]["members"][1]["id"] = json!("test.one.first");
        assert!(validate(&root, &changed, &known).is_err());
        for target in ["test.other.member", "test.one.absent"] {
            let mut changed = valid.clone();
            changed["edges"][0]["member_endpoints"][0]["to_member"] = json!(target);
            assert!(validate(&root, &changed, &known).is_err());
        }
        let mut changed = valid.clone();
        changed["edges"][0]["member_endpoints"] = json!([]);
        assert!(validate(&root, &changed, &known).is_err());
        let mut changed = valid;
        changed["nodes"][0]["member_relations"][0]["evidence"] = json!(["../outside"]);
        assert!(validate(&root, &changed, &known).is_err());
    }
    #[test]
    fn model_rejects_dangling_unknown_and_unsafe_sources() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let known = [
            "test.one".to_owned(),
            "test.two".to_owned(),
            "test.contract".to_owned(),
        ]
        .into_iter()
        .collect();
        let valid = json!({"schema_version":1,"version":"1","nodes":[{"id":"test.one","label":"One","layer":"EL1","responsibility":"actual scope","members":[{"name":"module","sources":["Cargo.toml"]}],"contracts":["test.contract"]},{"id":"test.two","label":"Two","layer":"EL0","responsibility":"actual client","members":[{"name":"app","sources":["Cargo.toml"]}],"contracts":["test.contract"]}],"edges":[{"from":"test.one","to":"test.two","kind":"call","label":"actual relation","evidence":["Cargo.toml"]}]});
        assert!(validate(&root, &valid, &known).is_ok());
        for (field, bad) in [("to", "missing"), ("kind", "weight")] {
            let mut x = valid.clone();
            x["edges"][0][field] = json!(bad);
            assert!(validate(&root, &x, &known).is_err());
        }
        let mut x = valid.clone();
        x["nodes"][0]["contracts"] = json!(["unknown"]);
        assert!(validate(&root, &x, &known).is_err());
        for path in ["../outside", "/absolute", "missing.rs"] {
            let mut x = valid.clone();
            x["nodes"][0]["members"][0]["sources"] = json!([path]);
            assert!(validate(&root, &x, &known).is_err());
        }
        let mut x = valid.clone();
        x["nodes"][1]["id"] = json!("test.one");
        assert!(validate(&root, &x, &known).is_err());
    }
}
