//! Structural exception ownership gate, not a detector of undeclared behavior.
use crate::{CheckResult, read, read_json};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path, time::SystemTime};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    records: Vec<Record>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    id: String,
    kind: String,
    owner: String,
    consumers: Vec<String>,
    rationale: String,
    scope: String,
    created: String,
    reviewed: String,
    review_due: String,
    removal_condition: String,
    replacement: String,
    support_until: Option<String>,
    boundaries: Vec<String>,
    disposition: Option<Disposition>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Disposition {
    reviewed: String,
    rationale: String,
    next_review: String,
    support_until: Option<String>,
}
fn date(s: &str) -> CheckResult<u64> {
    let parts: Vec<_> = s.split('-').collect();
    if parts.len() != 3
        || parts[0].len() != 4
        || parts[1].len() != 2
        || parts[2].len() != 2
        || parts.iter().any(|p| !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(format!("invalid UTC date: {s}"));
    }
    let numbers: Vec<u64> = parts
        .iter()
        .map(|p| p.parse())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("invalid UTC date: {s}"))?;
    let (y, m, d) = (numbers[0], numbers[1], numbers[2]);
    let leap = |y: u64| y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400));
    let months = [
        31,
        if leap(y) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1970..=9999).contains(&y)
        || !(1..=12).contains(&m)
        || d == 0
        || d > months[(m - 1) as usize]
    {
        return Err(format!("invalid UTC date: {s}"));
    }
    Ok((1970..y)
        .map(|year| if leap(year) { 366 } else { 365 })
        .sum::<u64>()
        + months[..(m - 1) as usize].iter().sum::<u64>()
        + d
        - 1)
}

/// Every declared boundary reference must match in both directions.
pub fn validate(value: Value, today: u64, sources: &[(String, String)]) -> CheckResult<()> {
    let registry: Registry = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if registry.records.len() > 128 {
        return Err("exception registry exceeds 128 records".into());
    }
    let id_pattern = Regex::new(r"^EXC-[0-9]{4}$").unwrap();
    let references = Regex::new(r"\bEXC-[0-9]{4}\b").unwrap();
    let mut declared = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for r in registry.records {
        if !id_pattern.is_match(&r.id) || !ids.insert(r.id.clone()) {
            return Err("invalid or duplicate exception ID".into());
        }
        for field in [
            &r.owner,
            &r.rationale,
            &r.scope,
            &r.removal_condition,
            &r.replacement,
        ] {
            if field.trim().is_empty() {
                return Err(format!("{}: empty required field", r.id));
            }
        }
        if r.consumers.is_empty()
            || r.consumers.iter().any(|s| s.trim().is_empty())
            || r.boundaries.is_empty()
        {
            return Err(format!(
                "{}: named consumers/targets and boundaries required",
                r.id
            ));
        }
        let created = date(&r.created)?;
        let reviewed = date(&r.reviewed)?;
        let due = date(&r.review_due)?;
        if created > reviewed || reviewed > today || due <= reviewed {
            return Err("invalid review chronology".into());
        }
        let support = match r.kind.as_str() {
            "software" => Some(date(
                r.support_until
                    .as_deref()
                    .ok_or("software support deadline required")?,
            )?),
            "hardware" if r.support_until.is_none() => None,
            _ => {
                return Err(
                    "hardware retires with named target support; unknown exception kind".into(),
                );
            }
        };
        if support.is_some_and(|s| s < reviewed) {
            return Err("support deadline precedes review".into());
        }
        if let Some(d) = &r.disposition {
            let review = date(&d.reviewed)?;
            if review < reviewed
                || review > today
                || d.rationale.trim().is_empty()
                || date(&d.next_review)? <= today
            {
                return Err("invalid reviewed disposition".into());
            }
            if today >= due && review < due {
                return Err("disposition predates due review".into());
            }
            if support.is_some_and(|s| today >= s)
                && (review < support.unwrap()
                    || date(
                        d.support_until
                            .as_deref()
                            .ok_or("reviewed support extension required")?,
                    )? <= today)
            {
                return Err("expired support needs reviewed extension".into());
            }
        } else if today >= due || support.is_some_and(|s| today >= s) {
            return Err(format!("{}: overdue without reviewed disposition", r.id));
        }
        for boundary in r.boundaries {
            declared.insert((boundary, r.id.clone()));
        }
    }
    let mut actual = BTreeSet::new();
    for (path, text) in sources {
        for token in references.find_iter(text) {
            actual.insert((path.clone(), token.as_str().to_owned()));
        }
    }
    if actual != declared {
        return Err("dangling or unregistered exception boundary reference".into());
    }
    Ok(())
}

pub fn check(root: &Path) -> CheckResult<()> {
    fn walk(root: &Path, directory: &Path, sources: &mut Vec<(String, String)>) -> CheckResult<()> {
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                return Err("exception traversal refuses symlinks".into());
            }
            if kind.is_dir() {
                walk(root, &path, sources)?;
            } else if path.extension().is_some_and(|e| e == "rs" || e == "toml") {
                sources.push((
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    read(&path)?,
                ));
            }
        }
        Ok(())
    }
    let mut sources = Vec::new();
    walk(root, &root.join("crates"), &mut sources)?;
    let today = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs()
        / 86400;
    validate(
        read_json(&root.join("policy/exceptions.json"))?,
        today,
        &sources,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn record() -> Value {
        json!({"id":format!("EXC-{:04}", 1), "kind":"software", "owner":"routing maintainers",
            "consumers":["synthetic legacy window workload"], "rationale":"test compatibility",
            "scope":"fixture only", "created":"2026-10-02", "reviewed":"2026-10-02",
            "review_due":"2027-01-01", "support_until":"2027-04-01", "replacement":"native window",
            "removal_condition":"no fixture consumers remain", "boundaries":["crates/fixture.rs"]})
    }
    fn run(r: Value, now: &str, referenced: bool) -> CheckResult<()> {
        // Construct reference dynamically so test fixtures do not declare a real boundary.
        let token = if referenced {
            format!("EXC-{:04}", 1)
        } else {
            String::new()
        };
        validate(
            json!({"records":[r]}),
            date(now)?,
            &[("crates/fixture.rs".into(), token)],
        )
    }
    #[test]
    fn missing_fields_and_dangling_references_fail() {
        assert!(run(record(), "2026-10-02", true).is_ok());
        for field in [
            "owner",
            "consumers",
            "rationale",
            "scope",
            "created",
            "reviewed",
            "review_due",
            "replacement",
            "removal_condition",
            "support_until",
            "boundaries",
        ] {
            let mut r = record();
            r.as_object_mut().unwrap().remove(field);
            assert!(run(r, "2026-10-02", true).is_err(), "{field}");
        }
        assert!(run(record(), "2026-10-02", false).is_err());
        assert!(
            validate(
                json!({"records":[]}),
                date("2026-10-02").unwrap(),
                &[("x".into(), format!("EXC-{:04}", 2))]
            )
            .is_err()
        );
    }
    #[test]
    fn overdue_requires_current_reviewed_disposition() {
        assert!(run(record(), "2027-04-01", true).is_err());
        let mut r = record();
        r["disposition"] = json!({"reviewed":"2027-04-01", "rationale":"named consumer still needs adapter", "next_review":"2027-05-01", "support_until":"2027-06-01"});
        assert!(run(r.clone(), "2027-04-01", true).is_ok());
        for (field, value) in [
            ("reviewed", "2026-12-31"),
            ("reviewed", "2027-04-02"),
            ("next_review", "2027-04-01"),
            ("support_until", "2027-04-01"),
            ("rationale", ""),
        ] {
            let mut invalid = r.clone();
            invalid["disposition"][field] = json!(value);
            assert!(run(invalid, "2027-04-01", true).is_err(), "{field}");
        }
        assert!(run(r, "2027-06-01", true).is_err());
    }
    #[test]
    fn hardware_has_review_but_no_clock_based_disable() {
        let mut r = record();
        r["kind"] = json!("hardware");
        r.as_object_mut().unwrap().remove("support_until");
        r["removal_condition"] = json!("support for named affected target ends");
        assert!(run(r.clone(), "2026-10-02", true).is_ok());
        assert!(run(r, "2027-01-01", true).is_err());
    }
    #[test]
    fn calendar_dates_are_validated() {
        assert_eq!(date("1970-01-01").unwrap(), 0);
        assert!(date("2028-02-29").is_ok());
        assert!(date("2027-02-29").is_err());
        assert!(date("2026-13-01").is_err());
        assert!(date("2026-+1-01").is_err());
    }
}
