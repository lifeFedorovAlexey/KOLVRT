//! Read-only projections of validated COST-L declarations, never runtime authority.
use crate::{CheckResult, cost_l};
use serde_json::{Value, json};
use std::{
    cmp::Ordering,
    collections::BTreeSet,
    path::{Path, PathBuf},
};

const MAX_PAGE_ROWS: usize = 64;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const METRICS: &[&str] = &[
    "cpu",
    "latency",
    "memory",
    "copies",
    "allocations",
    "context_switches",
    "throughput",
];
const USAGE: &str = "cost-l show ID | consumers ID | deps CONSUMER | list | top [--directory PATH] [--json] [--limit 1..64] [--offset N] [--sort id|reach|METRIC] [--measurement-scope EXACT --denominator EXACT] [--status STATE] [--category TEXT] [--security TEXT] [--maintenance TEXT] [--migration TEXT]";

struct Options {
    command: String,
    selector: Option<String>,
    directory: PathBuf,
    json: bool,
    limit: usize,
    offset: usize,
    sort: String,
    filters: Vec<(String, String)>,
    measurement_scope: Option<String>,
    denominator: Option<String>,
}

fn options(root: &Path, args: &[String]) -> CheckResult<Options> {
    let command = args.first().ok_or(USAGE)?.clone();
    if !matches!(
        command.as_str(),
        "show" | "consumers" | "deps" | "list" | "top"
    ) {
        return Err(USAGE.into());
    }
    let mut at = 1;
    let selector = if matches!(command.as_str(), "show" | "consumers" | "deps") {
        let value = args
            .get(at)
            .filter(|s| !s.starts_with("--"))
            .ok_or(USAGE)?
            .clone();
        at += 1;
        if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            return Err("invalid query identity".into());
        }
        Some(value)
    } else {
        None
    };
    let mut out = Options {
        command,
        selector,
        directory: root.join("research/cost-l"),
        json: false,
        limit: 20,
        offset: 0,
        sort: "id".into(),
        filters: Vec::new(),
        measurement_scope: None,
        denominator: None,
    };
    if out.command == "top" {
        out.sort = "reach".into();
    }
    let mut seen = BTreeSet::new();
    while at < args.len() {
        let flag = args[at].as_str();
        if !seen.insert(flag.to_owned()) {
            return Err(format!("duplicate option {flag}"));
        }
        at += 1;
        if flag == "--json" {
            out.json = true;
            continue;
        }
        let value = args.get(at).filter(|s| !s.starts_with("--")).ok_or(USAGE)?;
        at += 1;
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err("empty or control-containing option".into());
        }
        match flag {
            "--directory" => out.directory = PathBuf::from(value),
            "--limit" => out.limit = value.parse().map_err(|_| "invalid limit")?,
            "--offset" => out.offset = value.parse().map_err(|_| "invalid offset")?,
            "--sort" => out.sort = value.clone(),
            "--measurement-scope" => out.measurement_scope = Some(value.clone()),
            "--denominator" => out.denominator = Some(value.clone()),
            "--status" | "--category" | "--security" | "--maintenance" | "--migration" => {
                out.filters.push((flag.into(), value.clone()))
            }
            _ => return Err(USAGE.into()),
        }
    }
    if !(1..=MAX_PAGE_ROWS).contains(&out.limit) {
        return Err("limit must be 1..64".into());
    }
    let ranked = matches!(out.command.as_str(), "list" | "top");
    if !ranked
        && (!out.filters.is_empty()
            || seen.contains("--sort")
            || out.measurement_scope.is_some()
            || out.denominator.is_some())
    {
        return Err("ranking/filter options require list or top".into());
    }
    if out.sort != "id" && out.sort != "reach" && !METRICS.contains(&out.sort.as_str()) {
        return Err("unknown independent sort dimension".into());
    }
    if METRICS.contains(&out.sort.as_str()) {
        if out.measurement_scope.is_none() || out.denominator.is_none() {
            return Err("metric ranking requires exact measurement-scope and denominator; unrelated observations are not comparable".into());
        }
    } else if out.measurement_scope.is_some() || out.denominator.is_some() {
        return Err("measurement scope requires metric ranking".into());
    }
    Ok(out)
}

fn reach(record: &Value) -> usize {
    record["affected_consumers"].as_array().unwrap().len()
}

fn projection(record: &Value) -> Value {
    json!({"id":record["id"],"title":record["title"],"status":record["status"],"category":record["category"],"subsystem":record["subsystem"],"declared_reach":reach(record),"global_reach":"UNKNOWN","compat_modules":record["compat_modules"],"affected_consumers":record["affected_consumers"],"runtime_cost":record["runtime_cost"],"security_impact":record["security_impact"],"complexity_impact":record["complexity_impact"],"support":record["support"],"native":record["native"],"authority_implications":record["authority_implications"],"software_state":if record["compat_modules"].as_array().unwrap().is_empty(){"UNDETERMINED_NO_MODULE_DECLARATION"}else{"COMPAT_DECLARATION"},"hardware_classification":record["history"]["classification"]})
}

fn matches_filters(record: &Value, filters: &[(String, String)]) -> bool {
    filters.iter().all(|(flag, wanted)| {
        let actual = match flag.as_str() {
            "--status" => record["status"].as_str().unwrap().to_owned(),
            "--category" => record["category"].as_str().unwrap().to_owned(),
            "--security" => record["security_impact"].as_str().unwrap().to_owned(),
            "--maintenance" => format!(
                "{} {}",
                record["complexity_impact"].as_str().unwrap(),
                record["support"]["removal_condition"].as_str().unwrap()
            ),
            "--migration" => format!(
                "{} {}",
                record["support"]["migration_target"].as_str().unwrap(),
                record["support"]["migration_strategy"].as_str().unwrap()
            ),
            _ => unreachable!(),
        };
        if matches!(flag.as_str(), "--status" | "--category") {
            actual == *wanted
        } else {
            actual.to_lowercase().contains(&wanted.to_lowercase())
        }
    })
}

/// All queries validate the whole selected registry before producing any output.
pub fn query(root: &Path, args: &[String]) -> CheckResult<(Value, bool)> {
    let opts = options(root, args)?;
    let records = cost_l::validate(root, &opts.directory)?;
    let mut rows: Vec<Value> = match opts.command.as_str() {
        "show" | "consumers" => {
            let selector = opts.selector.as_ref().unwrap();
            if selector.len() != 11
                || !selector.starts_with("COST-L-")
                || !selector[7..].bytes().all(|b| b.is_ascii_digit())
                || selector == "COST-L-0000"
            {
                return Err("malformed or reserved COST-L identity".into());
            }
            let record = records
                .iter()
                .find(|r| r["id"] == *selector)
                .ok_or("unknown COST-L identity")?;
            if opts.command == "show" {
                vec![projection(record)]
            } else {
                record["affected_consumers"].as_array().unwrap().iter().map(|c| json!({"debt_id":selector,"consumer":c,"evidence_kind":"DECLARED_RELATION","transitive_path":"NOT_RECORDED"})).collect()
            }
        }
        "deps" => {
            let selector = opts.selector.as_ref().unwrap();
            let rows: Vec<_> = records.iter().flat_map(|r| r["affected_consumers"].as_array().unwrap().iter().filter(|c| c["id"] == *selector).map(|c| json!({"debt_id":r["id"],"consumer":c,"debt_status":r["status"],"evidence_kind":"DECLARED_RELATION","transitive_path":"NOT_RECORDED"}))).collect();
            if rows.is_empty() {
                return Err(
                    "unknown consumer in selected declarations; global absence is not established"
                        .into(),
                );
            }
            rows
        }
        "list" | "top" => records
            .iter()
            .filter(|r| matches_filters(r, &opts.filters))
            .map(projection)
            .collect(),
        _ => unreachable!(),
    };
    if METRICS.contains(&opts.sort.as_str()) {
        let mut units = BTreeSet::new();
        for row in &mut rows {
            let metric = &row["runtime_cost"][&opts.sort];
            let comparable = metric["state"] == "MEASURED"
                && metric["scope"].as_str() == opts.measurement_scope.as_deref()
                && metric["denominator"].as_str() == opts.denominator.as_deref();
            if comparable {
                units.insert(metric["unit"].as_str().unwrap().to_owned());
            }
            row["ranking_value"] = if comparable {
                metric["value"].clone()
            } else {
                Value::Null
            };
            row["ranking_state"] = json!(if comparable {
                "MEASURED_IN_SELECTED_SCOPE"
            } else {
                "UNKNOWN_OR_OUTSIDE_SELECTED_SCOPE"
            });
        }
        if units.len() > 1 {
            return Err("metric ranking has incompatible units".into());
        }
    }
    rows.sort_by(|a, b| {
        let rank = if opts.sort == "reach" {
            b["declared_reach"]
                .as_u64()
                .cmp(&a["declared_reach"].as_u64())
        } else if METRICS.contains(&opts.sort.as_str()) {
            match (a["ranking_value"].as_f64(), b["ranking_value"].as_f64()) {
                (Some(a), Some(b)) => b.total_cmp(&a),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                _ => Ordering::Equal,
            }
        } else {
            Ordering::Equal
        };
        rank.then_with(|| {
            a["id"]
                .as_str()
                .or(a["debt_id"].as_str())
                .cmp(&b["id"].as_str().or(b["debt_id"].as_str()))
        })
        .then_with(|| a["consumer"].to_string().cmp(&b["consumer"].to_string()))
    });
    let total = rows.len();
    if opts.offset > total {
        return Err("offset exceeds matching rows".into());
    }
    let end = opts.offset.saturating_add(opts.limit).min(total);
    let output = json!({"schema_version":1,"command":opts.command,"selector":opts.selector,"inventory":{"scope":"validated records in the explicitly selected registry directory","coverage":"DECLARATIONS_ONLY; global consumer coverage UNKNOWN","records":records.len(),"production_manifest_integration":"UNAVAILABLE_PENDING_ISSUE_47","runtime_inspection":"UNSUPPORTED","authority":"NONE"},"ranking":{"dimension":opts.sort,"filters":opts.filters,"measurement_scope":opts.measurement_scope,"denominator":opts.denominator,"meaning":"Descending declared reach or measured dimension value in the exact selected scope. Throughput is a rate, not a cost. No combined score, security/difficulty ranking, performance superiority or NATIVE inference."},"pagination":{"offset":opts.offset,"limit":opts.limit,"matching_rows":total,"next_offset":if end<total{Some(end)}else{None}},"rows":rows[opts.offset..end]});
    Ok((output, opts.json))
}

pub fn cli(root: &Path, args: &[String]) -> CheckResult<()> {
    let (output, machine) = query(root, args)?;
    let text = if machine {
        serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?
    } else {
        let mut text = "COST-L offline declarations; global coverage UNKNOWN. Runtime inspection and production-manifest integration unavailable.\n".to_owned();
        for row in output["rows"].as_array().unwrap() {
            if let Some(id) = row["id"].as_str() {
                text.push_str(&format!(
                    "{} | {} | {} | declared consumers: {}\n",
                    id,
                    row["status"].as_str().unwrap(),
                    row["title"].as_str().unwrap(),
                    row["declared_reach"]
                ));
                if output["command"] == "show" {
                    text.push_str(&serde_json::to_string_pretty(row).map_err(|e| e.to_string())?);
                    text.push('\n');
                } else {
                    text.push_str(&format!("  CPU: {} {}; memory: {} {}; security: {}; maintenance: {}; migration: {}\n",row["runtime_cost"]["cpu"]["state"].as_str().unwrap(),row["runtime_cost"]["cpu"]["value"],row["runtime_cost"]["memory"]["state"].as_str().unwrap(),row["runtime_cost"]["memory"]["value"],row["security_impact"].as_str().unwrap(),row["complexity_impact"].as_str().unwrap(),row["support"]["migration_target"].as_str().unwrap()));
                }
                if let Some(state) = row["ranking_state"].as_str() {
                    text.push_str(&format!(
                        "  Selected dimension {}: {} {}\n",
                        output["ranking"]["dimension"].as_str().unwrap(),
                        state,
                        row["ranking_value"]
                    ));
                }
            } else {
                let c = &row["consumer"];
                text.push_str(&format!(
                    "{} <- {} | {} | {} | scope: {} | modules: {}\n",
                    row["debt_id"].as_str().unwrap(),
                    c["id"].as_str().unwrap(),
                    c["kind"].as_str().unwrap(),
                    c["dependency"].as_str().unwrap(),
                    c["scope"].as_str().unwrap(),
                    c["module_refs"]
                ));
            }
        }
        text.push_str(&format!(
            "Matching rows: {}; next offset: {}\n",
            output["pagination"]["matching_rows"], output["pagination"]["next_offset"]
        ));
        text
    };
    if text.len().saturating_add(1) > MAX_OUTPUT_BYTES {
        return Err("output exceeds 1 MiB; reduce --limit (no partial stdout emitted)".into());
    }
    println!("{text}");
    Ok(())
}
