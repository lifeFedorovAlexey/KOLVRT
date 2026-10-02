use crate::{CheckResult, database, hash, read, read_json};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

const DECISIONS: [&str; 6] = [
    "NATIVE_FIX",
    "COMPAT_ONLY",
    "HARDWARE_TRANSLATION",
    "ACCEPTED_TRADEOFF",
    "RESEARCH_REQUIRED",
    "NOT_APPLICABLE",
];
const INDEX: &str = "docs/research/CASE_INDEX.md";

fn text<'a>(entry: &'a Value, field: &str) -> CheckResult<&'a str> {
    let value = entry[field]
        .as_str()
        .ok_or_else(|| format!("index catalog: missing {field}"))?;
    if value.trim().is_empty() || value.contains(['|', '\n', '\r']) {
        return Err(format!("index catalog: invalid {field}"));
    }
    Ok(value)
}

pub fn render(root: &Path, cases: &[Value], catalog: &Value, locale: &str) -> CheckResult<String> {
    let en = locale == "en";
    if !en && locale != "ru" {
        return Err("case-index renderer supports en and ru".into());
    }
    let entries = catalog
        .as_object()
        .ok_or("case-index catalog must be an object")?;
    let listed: BTreeSet<_> = entries.keys().map(String::as_str).collect();
    let expected: BTreeSet<_> = cases.iter().map(|c| c["id"].as_str().unwrap()).collect();
    if listed != expected {
        return Err("case-index catalog must cover exactly the research cases".into());
    }
    let mut lines: Vec<String> = if en {
        vec!["# Pathology cases — Phase 0.1", "", "Research date: 2026-10-02. KOLVRT decisions are design conclusions, not implemented behavior.", "", "| ID | Case | Subsystem | Decision | Compatibility | Confidence |", "|---|---|---|---|---|---|"]
    } else {
        vec!["# Случаи исследования — этап 0.1", "", "Дата исследования: 2026-10-02. Решения KOLVRT — проектные выводы, а не реализованное поведение.", "", "| Идентификатор | Случай | Подсистема | Решение | Совместимость | Уверенность |", "|---|---|---|---|---|---|"]
    }.into_iter().map(str::to_owned).collect();
    let prefix = if en { "../.." } else { "../../../.." };
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let record = read(&root.join(format!("research/pathology/{id}.json")))?;
        if catalog[id]["source_sha256"] != hash(&record) {
            return Err(format!(
                "{id}: review bilingual index text after record change"
            ));
        }
        let translated = &catalog[id][locale];
        let title = text(translated, "title")?;
        let subsystem = text(translated, "subsystem")?;
        let questions = translated["open_questions"]
            .as_array()
            .ok_or("index catalog: missing questions")?;
        if questions.len() != case["open_questions"].as_array().unwrap().len()
            || questions.iter().any(|q| {
                q.as_str()
                    .is_none_or(|s| s.trim().is_empty() || s.contains(['\r', '\n']))
            })
        {
            return Err(format!(
                "{id}: index question count or text differs from record requirements"
            ));
        }
        lines.push(format!("| [{id}]({prefix}/research/pathology/{id}.json) | {title} | {subsystem} | {} | {} | {} |", case["kolvrt_decision"].as_str().unwrap(), case["compatibility_required"].as_str().unwrap(), case["confidence"].as_str().unwrap()));
    }
    lines.extend([
        "".into(),
        if en {
            "## Decision groups"
        } else {
            "## Группы решений"
        }
        .into(),
        "".into(),
    ]);
    for decision in DECISIONS {
        let ids: Vec<_> = cases
            .iter()
            .filter(|c| c["kolvrt_decision"] == decision)
            .map(|c| c["id"].as_str().unwrap())
            .collect();
        lines.push(format!(
            "- **{decision} ({})**: {}.",
            ids.len(),
            ids.join(", ")
        ));
    }
    lines.extend([
        "".into(),
        if en {
            "## Record questions"
        } else {
            "## Вопросы записей"
        }
        .into(),
        "".into(),
    ]);
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let questions: Vec<_> = catalog[id][locale]["open_questions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q.as_str().unwrap())
            .collect();
        if !questions.is_empty() {
            lines.push(format!("- **{id}**: {}", questions.join("; ")));
        }
    }
    lines.push("".into());
    lines.push(
        if en {
            "[Russian translation](../../translations/ru/docs/research/CASE_INDEX.md)"
        } else {
            "[Английский оригинал](../../../../docs/research/CASE_INDEX.md)"
        }
        .into(),
    );
    Ok(lines.join("\n") + "\n")
}

pub fn report(root: &Path, check: bool) -> CheckResult<()> {
    let cases = database::validate(root, &root.join("research/pathology"), 30)?;
    let catalog = read_json(&root.join("research/sources/case-index-text.json"))?;
    // Render both before writing either: malformed translations cannot cause a partial update.
    let en = render(root, &cases, &catalog, "en")?;
    let ru = render(root, &cases, &catalog, "ru")?;
    for (path, content) in [
        (root.join(INDEX), en),
        (root.join("translations/ru").join(INDEX), ru),
    ] {
        if check {
            if read(&path)? != content {
                return Err(format!("{} is stale", path.display()));
            }
        } else {
            fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}
