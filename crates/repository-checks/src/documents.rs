use crate::{CheckResult, database, finish, hash, read, read_json, write_json};
use regex::Regex;
use serde_json::json;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const MANIFEST: &str = "translations/manifest.json";
const IGNORED: [&str; 4] = [".git", ".toolchains", "target", "node_modules"];

fn markdown_walk(
    directory: &Path,
    skip_translations: bool,
    output: &mut Vec<PathBuf>,
) -> CheckResult<()> {
    for entry in fs::read_dir(directory).map_err(|e| format!("{}: {e}", directory.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if IGNORED.contains(&name.as_ref()) || (skip_translations && name == "translations") {
            continue;
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err(format!(
                "documentation traversal refuses symlink: {}",
                entry.path().display()
            ));
        }
        if kind.is_dir() {
            markdown_walk(&entry.path(), skip_translations, output)?;
        } else if entry.path().extension().is_some_and(|e| e == "md") {
            output.push(entry.path());
        }
    }
    Ok(())
}
pub fn markdown(root: &Path, skip_translations: bool) -> CheckResult<Vec<PathBuf>> {
    let mut paths = Vec::new();
    markdown_walk(root, skip_translations, &mut paths)?;
    paths.sort();
    Ok(paths)
}
fn relative(root: &Path, path: &Path) -> CheckResult<String> {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .map_err(|_| format!("path outside repository: {}", path.display()))
}
pub fn structure(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut fenced = false;
    let item = Regex::new(r"^\s*(?:[-*] |\d+\. )").unwrap();
    for line in text.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
            result.push("fence".into());
        } else if !fenced {
            let level = line.chars().take_while(|c| *c == '#').count();
            if (1..=6).contains(&level) && line.as_bytes().get(level) == Some(&b' ') {
                result.push(format!("h{level}"));
            } else if line.starts_with('|') {
                result.push(format!("table:{}", line.matches('|').count()));
            } else if item.is_match(line) {
                result.push("list".into());
            }
        }
    }
    result
}

fn decode_url_path(text: &str) -> CheckResult<String> {
    let bytes = text.as_bytes();
    let mut output = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3).ok_or("incomplete URL escape")?;
            let hex = std::str::from_utf8(hex).map_err(|e| e.to_string())?;
            output.push(u8::from_str_radix(hex, 16).map_err(|e| e.to_string())?);
            i += 3;
        } else {
            output.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(output).map_err(|e| e.to_string())
}

pub fn links(root: &Path, path: &Path, text: &str) -> CheckResult<Vec<String>> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let link = Regex::new(r"\[[^\]]*\]\(([^)]+)\)").unwrap();
    let scheme = Regex::new(r"^[a-zA-Z][a-zA-Z0-9+.-]*:").unwrap();
    let mut result = Vec::new();
    for capture in link.captures_iter(text) {
        let raw = &capture[1];
        if scheme.is_match(raw) {
            result.push(raw.to_owned());
            continue;
        }
        let (local, fragment) = raw.split_once('#').unwrap_or((raw, ""));
        let local = decode_url_path(local.trim_matches(['<', '>']))?;
        let target = if local.is_empty() {
            path.to_path_buf()
        } else {
            path.parent().unwrap().join(local)
        };
        let resolved = target
            .canonicalize()
            .map_err(|e| format!("{}: missing link {raw}: {e}", path.display()))?;
        let mut rel = relative(&root, &resolved)?;
        if rel.starts_with("translations/") {
            rel = rel.splitn(3, '/').nth(2).unwrap_or("").to_owned();
        }
        if !fragment.is_empty() {
            rel.push('#');
            rel.push_str(fragment);
        }
        result.push(rel);
    }
    Ok(result)
}

pub fn law_ids(text: &str) -> Vec<String> {
    Regex::new(r"(?m)^## (LAW-\d{3}) — ")
        .unwrap()
        .captures_iter(text)
        .map(|c| c[1].to_owned())
        .collect()
}
pub fn check_laws(text: &str) -> CheckResult<Vec<String>> {
    let headings = Regex::new(r"(?m)^## (LAW-\d{3}) — ").unwrap();
    let matches: Vec<_> = headings.captures_iter(text).collect();
    let ids = law_ids(text);
    let unique: BTreeSet<_> = ids.iter().collect();
    let mut errors = Vec::new();
    if ids.is_empty() || unique.len() != ids.len() {
        errors.push("laws must form a nonempty set of unique IDs; no numeric quota applies".into());
    }
    let language = Regex::new(r"\b(?:Rust|Cargo|crate|crates|Miri)\b").unwrap();
    for (i, entry) in matches.iter().enumerate() {
        let start = entry.get(0).unwrap().end();
        let end = matches
            .get(i + 1)
            .map(|m| m.get(0).unwrap().start())
            .unwrap_or(text.len());
        let body = &text[start..end];
        for field in [
            "Rule",
            "Rationale",
            "Historical evidence",
            "Prevents",
            "Allowed exceptions",
            "Enforcement",
            "Testing",
        ] {
            let pattern = Regex::new(&format!(r"\*\*{}:\*\*\s+\S", regex::escape(field))).unwrap();
            if !pattern.is_match(body) {
                errors.push(format!("{}: missing {field}", &entry[1]));
            }
        }
        if !body.contains("research/cases/KOL-PATH-") {
            errors.push(format!("{}: missing case evidence", &entry[1]));
        }
        if language.is_match(body) {
            errors.push(format!(
                "{}: move language-specific mechanisms to implementation policy",
                &entry[1]
            ));
        }
    }
    finish(errors)?;
    Ok(ids)
}

/// Structural publication gate; acceptance of conformance evidence remains a review.
pub fn check_abi_publication(
    text: &str,
    load_decision: impl Fn(&str) -> CheckResult<String>,
) -> CheckResult<()> {
    fn field<'a>(text: &'a str, name: &str) -> CheckResult<&'a str> {
        let prefix = format!("{name}: ");
        let values: Vec<_> = text
            .lines()
            .filter_map(|s| s.strip_prefix(&prefix))
            .collect();
        if values.len() != 1 || values[0].trim().is_empty() {
            return Err(format!("ABI publication requires exactly one {name}"));
        }
        Ok(values[0].trim())
    }
    let contract = field(text, "ABI contract")?;
    let stage = field(text, "Publication stage")?;
    let freeze = field(text, "ABI-FREEZE")?;
    if !matches!(stage, "EXPERIMENTAL" | "CANDIDATE" | "PUBLIC" | "STABLE") {
        return Err("unknown ABI publication stage".into());
    }
    if freeze == "none" {
        return if stage == "STABLE" {
            Err("STABLE requires an accepted ABI-FREEZE".into())
        } else {
            Ok(())
        };
    }
    let reference =
        Regex::new(r"^\[[^\]]+\]\(\.\./architecture-decisions/([0-9]{4}-[a-z0-9-]+\.md)\)$")
            .unwrap();
    let captures = reference
        .captures(freeze)
        .ok_or("ABI-FREEZE requires a local ADR link")?;
    let decision = load_decision(&captures[1])?;
    if !decision
        .lines()
        .any(|s| s.starts_with("Status: **Accepted**."))
        || field(&decision, "Decision kind")? != "ABI-FREEZE"
        || field(&decision, "ABI contract")? != contract
    {
        return Err("ABI-FREEZE must be accepted and cover the exact contract/version".into());
    }
    Ok(())
}

pub fn check_docs(root: &Path) -> CheckResult<()> {
    for document in [
        "docs/architecture/first-native-slice.md",
        "docs/architecture/compatibility-model.md",
        "docs/architecture/unsafe-policy.md",
        "docs/architecture-decisions/0004-unsafe.md",
        "docs/architecture-decisions/0010-kernel-foundation.md",
    ] {
        check_document_status(&read(&root.join(document))?)?;
    }
    check_abi_publication(
        &read(&root.join("docs/architecture/native-abi.md"))?,
        |name| read(&root.join("docs/architecture-decisions").join(name)),
    )?;
    for path in markdown(root, false)? {
        links(root, &path, &read(&path)?)?;
    }
    let ids = check_laws(&read(&root.join("docs/architecture/kernel-laws.md"))?)?;
    let review = read_json(&root.join("research/results/law-review.json"))?;
    let old = review["previous_ids"]
        .as_array()
        .ok_or("law review requires previous IDs")?;
    let old: BTreeSet<_> = old
        .iter()
        .map(|x| x.as_str().ok_or("invalid previous law ID"))
        .collect::<Result<_, _>>()?;
    let rows = review["dispositions"]
        .as_array()
        .ok_or("law review requires dispositions")?;
    let mut previous = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for row in rows {
        let prev = row["previous"].as_str().ok_or("missing previous law")?;
        if !previous.insert(prev) {
            return Err(format!("duplicate previous law: {prev}"));
        }
        targets.insert(row["target"].as_str().ok_or("missing law target")?);
        for key in ["reason", "reason_ru"] {
            if row[key].as_str().is_none_or(|s| s.trim().is_empty()) {
                return Err(format!("{prev}: missing {key}"));
            }
        }
    }
    for addition in review["additions"]
        .as_array()
        .ok_or("law review requires an additions list")?
    {
        let id = addition["id"].as_str().ok_or("missing added law ID")?;
        if !targets.insert(id)
            || addition["reason"]
                .as_str()
                .is_none_or(|s| s.trim().is_empty())
        {
            return Err("each added law needs a unique ID and review reason".into());
        }
    }
    if old != previous || targets != ids.iter().map(String::as_str).collect() {
        return Err(
            "law review must account for all previous IDs and exactly the active laws".into(),
        );
    }
    let mut count = 0;
    for path in markdown(&root.join("docs/architecture-decisions"), false)? {
        if path.file_name().is_some_and(|x| x == "README.md") {
            continue;
        }
        count += 1;
        let text = read(&path)?;
        for heading in [
            "Context",
            "Decision",
            "Alternatives",
            "Why rejected",
            "Consequences",
            "Compatibility impact",
            "Performance impact",
            "Security impact",
            "Testing",
            "Reversibility",
        ] {
            if !text.lines().any(|line| line == format!("## {heading}")) {
                return Err(format!("{}: missing {heading}", path.display()));
            }
        }
    }
    if count == 0 {
        return Err("no architecture decisions found".into());
    }
    database::check_ledger(root)
}

/// Bounded metadata validation, not proof that claims match execution evidence.
pub fn check_document_status(text: &str) -> CheckResult<()> {
    let mut values = Vec::new();
    for name in ["Document status", "Document scope", "Status reference"] {
        let prefix = format!("{name}: ");
        let fields: Vec<_> = text
            .lines()
            .filter_map(|s| s.strip_prefix(&prefix))
            .collect();
        if fields.len() != 1 || fields[0].trim().is_empty() {
            return Err(format!("document requires exactly one nonempty {name}"));
        }
        values.push(fields[0].trim());
    }
    if !matches!(
        values[0],
        "CURRENT" | "DESIGN BASELINE" | "HISTORICAL" | "SUPERSEDED"
    ) {
        return Err("unknown document status".into());
    }
    let link = Regex::new(r"^\[[^\]]+\]\(([^)]+)\)$").unwrap();
    let capture = link
        .captures(values[2])
        .ok_or("status reference must be a local Markdown link")?;
    let target = &capture[1];
    if target.starts_with(['/', '#']) || target.contains(':') || !target.ends_with(".md") {
        return Err("status reference must name a local document".into());
    }
    Ok(())
}

fn locale_valid(locale: &str) -> bool {
    locale != "en"
        && Regex::new(r"^[a-z]{2,3}(?:-[A-Za-z0-9]+)*$")
            .unwrap()
            .is_match(locale)
}

pub fn check_translations(root: &Path) -> CheckResult<()> {
    let manifest = read_json(&root.join(MANIFEST))?;
    if manifest["source_language"] != "en" {
        return Err("canonical source language must be en".into());
    }
    let locales = manifest["locales"]
        .as_object()
        .ok_or("manifest requires locales")?;
    if locales.is_empty() {
        return Err("at least one translation language is required".into());
    }
    let originals: BTreeSet<_> = markdown(root, true)?
        .iter()
        .map(|p| relative(root, p))
        .collect::<CheckResult<_>>()?;
    let cyrillic = Regex::new(r"[\u0400-\u04ff]").unwrap();
    let identifiers = Regex::new(r"\bLAW-\d{3}\b").unwrap();
    let mut errors = Vec::new();
    for rel in &originals {
        if cyrillic.is_match(&read(&root.join(rel))?) {
            errors.push(format!("{rel}: Cyrillic in English Markdown"));
        }
    }
    // Unregistered directories must not silently escape translation checks.
    for entry in fs::read_dir(root.join("translations")).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir()
            && !locales.contains_key(entry.file_name().to_string_lossy().as_ref())
        {
            errors.push(format!(
                "unregistered translation directory: {}",
                entry.path().display()
            ));
        }
    }
    for (locale, entries) in locales {
        if !locale_valid(locale) {
            return Err(format!("invalid locale: {locale}"));
        }
        let entries = entries
            .as_object()
            .ok_or("locale requires a document map")?;
        let listed: BTreeSet<_> = entries.keys().cloned().collect();
        let base = root.join("translations").join(locale);
        let actual: BTreeSet<_> = markdown(&base, false)?
            .iter()
            .map(|p| relative(&base, p))
            .collect::<CheckResult<_>>()?;
        for path in originals.difference(&listed) {
            errors.push(format!("{locale}: missing manifest entry {path}"));
        }
        for path in listed.difference(&originals) {
            errors.push(format!("{locale}: orphan manifest entry {path}"));
        }
        for path in actual.difference(&originals) {
            errors.push(format!("{locale}: orphan translation {path}"));
        }
        for rel in originals.intersection(&listed) {
            let source = root.join(rel);
            let translated = base.join(rel);
            if !translated.is_file() {
                errors.push(format!("{locale}: missing translation {rel}"));
                continue;
            }
            let en = read(&source)?;
            let local = read(&translated)?;
            let entry = &entries[rel];
            if entry["source_sha256"] != hash(&en) {
                errors.push(format!(
                    "{locale}/{rel}: source changed since translation review"
                ));
            }
            if entry["translation_sha256"] != hash(&local) {
                errors.push(format!("{locale}/{rel}: translation changed since review"));
            }
            if structure(&en) != structure(&local) {
                errors.push(format!(
                    "{locale}/{rel}: heading/table/list structure differs"
                ));
            }
            match (links(root, &source, &en), links(root, &translated, &local)) {
                (Ok(a), Ok(b)) if a != b => {
                    errors.push(format!("{locale}/{rel}: corresponding link targets differ"))
                }
                (Err(e), _) | (_, Err(e)) => errors.push(e),
                _ => {}
            }
            let en_ids: Vec<_> = identifiers.find_iter(&en).map(|m| m.as_str()).collect();
            let local_ids: Vec<_> = identifiers.find_iter(&local).map(|m| m.as_str()).collect();
            if en_ids != local_ids {
                errors.push(format!("{locale}/{rel}: law identifiers differ"));
            }
            if locale == "ru" && !cyrillic.is_match(&local) {
                errors.push(format!("{rel}: missing Russian prose"));
            }
        }
    }
    finish(errors)
}

pub fn record_translation(root: &Path, locale: &str, rel: &str) -> CheckResult<()> {
    if !locale_valid(locale) {
        return Err("invalid translation locale".into());
    }
    let originals: BTreeSet<_> = markdown(root, true)?
        .iter()
        .map(|p| relative(root, p))
        .collect::<CheckResult<_>>()?;
    if !originals.contains(rel) {
        return Err("expected a canonical repository Markdown path".into());
    }
    let path = root.join(MANIFEST);
    let mut manifest = read_json(&path)?;
    let entries = manifest["locales"][locale]
        .as_object_mut()
        .ok_or("register the locale before recording translations")?;
    let source = read(&root.join(rel))?;
    let translation = read(&root.join("translations").join(locale).join(rel))?;
    entries.insert(
        rel.into(),
        json!({"source_sha256":hash(&source), "translation_sha256":hash(&translation)}),
    );
    write_json(&path, &manifest)
}
