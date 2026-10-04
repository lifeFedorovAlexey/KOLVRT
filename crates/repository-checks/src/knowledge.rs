//! Deterministic navigation over authored Markdown and existing research records.
//! A valid catalog is not architectural acceptance or behavioral evidence.
use crate::{CheckResult, documents, hash, parse_json, read, read_json, write_json};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
    process::Command,
};
mod impact;

const MARKER: &str = "<!-- knowledge -->";
const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
const MAX_NODES: usize = 8192;
const MAX_CONTEXT_BYTES: usize = 4 * 1024 * 1024;
const EDGES: &[&str] = &[
    "depends_on",
    "related_to",
    "supersedes",
    "implemented_by",
    "validated_by",
    "motivates",
    "contrasts_with",
    "blocked_by",
    "cost_l",
    "source_of_truth",
];

// CommonMark fences close only with the same character and at least the
// opening length. Short or mixed fences inside examples remain content.
fn fence_line(line: &str, fence: &mut Option<(char, usize)>) -> bool {
    let trimmed = line.trim_start();
    if line.len() - trimmed.len() > 3 {
        return false;
    }
    let Some(ch) = trimmed.chars().next() else {
        return false;
    };
    if ch != '`' && ch != '~' {
        return false;
    }
    let length = trimmed.chars().take_while(|c| *c == ch).count();
    if length < 3 {
        return false;
    }
    if let Some((opened, minimum)) = *fence {
        if ch == opened && length >= minimum && trimmed[length..].trim().is_empty() {
            *fence = None;
        }
    } else if ch != '`' || !trimmed[length..].contains('`') {
        *fence = Some((ch, length));
    }
    true
}

/// Parse one designated block, ignoring markers inside code examples.
pub fn metadata(text: &str) -> CheckResult<Option<Value>> {
    if text.len() > MAX_DOCUMENT_BYTES {
        return Err("knowledge document exceeds 1 MiB".into());
    }
    let mut fenced = None;
    let mut markers = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim() == MARKER && fenced.is_none() {
            markers.push(i);
        }
        fence_line(line, &mut fenced);
    }
    if markers.is_empty() {
        return Ok(None);
    }
    if markers.len() != 1 {
        return Err("duplicate knowledge blocks".into());
    }
    let lines: Vec<_> = text.lines().collect();
    let start = (markers[0] + 1..lines.len())
        .find(|i| !lines[*i].trim().is_empty())
        .ok_or("knowledge marker has no JSON block")?;
    if lines.get(start) != Some(&"```json") {
        return Err("knowledge marker requires a JSON fence".into());
    }
    let end = (start + 1..lines.len())
        .find(|i| lines[*i] == "```")
        .ok_or("unclosed knowledge fence")?;
    parse_json(&lines[start + 1..end].join("\n")).map(Some)
}

/// Heading ranges exclude fenced examples; explicit anchors bind to the next heading.
pub fn headings(text: &str) -> Vec<(usize, usize, String)> {
    let mut result = Vec::new();
    let mut fence = None;
    for (i, line) in text.lines().enumerate() {
        if fence_line(line, &mut fence) {
            continue;
        }
        if fence.is_some() {
            continue;
        }
        let level = line.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&level) && line.as_bytes().get(level) == Some(&b' ') {
            result.push((i + 1, level, line[level + 1..].trim().to_owned()));
        }
    }
    result
}

pub fn section(text: &str, anchor: &str) -> CheckResult<(usize, usize, String)> {
    let literal = format!("<a name=\"{anchor}\"></a>");
    let hs = headings(text);
    let mut fence = None;
    let mut positions = Vec::new();
    for (i, line) in text.lines().enumerate() {
        fence_line(line, &mut fence);
        if fence.is_none() && line.trim() == literal {
            positions.push(i + 1);
        }
    }
    if positions.len() != 1 {
        return Err(format!(
            "anchor {anchor} must occur exactly once outside fences"
        ));
    }
    let pos = positions[0];
    let (start, level, title) = hs
        .iter()
        .find(|h| h.0 > pos)
        .ok_or_else(|| format!("anchor {anchor} has no heading"))?;
    if text
        .lines()
        .skip(pos)
        .take(start - pos - 1)
        .any(|l| !l.trim().is_empty())
    {
        return Err(format!(
            "anchor {anchor} must immediately precede its heading"
        ));
    }
    let end = hs
        .iter()
        .find(|h| h.0 > *start && h.1 <= *level)
        .map_or(text.lines().count(), |h| h.0 - 1);
    Ok((pos, end, title.clone()))
}

fn safe_path(root: &Path, name: &str) -> CheckResult<()> {
    if name.contains('\\')
        || name.contains(':')
        || Path::new(name)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("unsafe knowledge path: {name}"));
    }
    let base = root.canonicalize().map_err(|e| e.to_string())?;
    let target = root
        .join(name)
        .canonicalize()
        .map_err(|e| format!("missing source {name}: {e}"))?;
    if !target.starts_with(base) || !target.is_file() {
        return Err(format!("knowledge source outside repository: {name}"));
    }
    Ok(())
}
fn strings(value: &Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
fn field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        l.strip_prefix(&format!("{key}: "))
            .map(|s| s.trim().to_owned())
    })
}
fn schema(root: &Path) -> CheckResult<jsonschema::Validator> {
    let value = read_json(&root.join("schemas/knowledge.schema.json"))?;
    jsonschema::meta::validate(&value).map_err(|e| e.to_string())?;
    jsonschema::draft202012::options()
        .build(&value)
        .map_err(|e| e.to_string())
}

fn valid_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("PLANNED", "RESEARCH")
            | ("RESEARCH", "PLANNED")
            | ("PLANNED" | "RESEARCH", "EXPERIMENTAL")
            | ("EXPERIMENTAL", "BOUNDED_IMPLEMENTED" | "IMPLEMENTED")
            | ("BOUNDED_IMPLEMENTED", "IMPLEMENTED" | "EXPERIMENTAL")
            | ("IMPLEMENTED", "BOUNDED_IMPLEMENTED" | "EXPERIMENTAL")
    ) || (!matches!(from, "REMOVED" | "SUPERSEDED") && matches!(to, "REMOVED" | "SUPERSEDED"))
}
pub fn validate_feature(root: &Path, feature: &Value) -> CheckResult<()> {
    let state = feature["implementation"]
        .as_str()
        .ok_or("missing implementation state")?;
    let implemented = matches!(state, "BOUNDED_IMPLEMENTED" | "IMPLEMENTED");
    let sources = strings(feature, "sources");
    if (implemented || state == "EXPERIMENTAL") && sources.is_empty() {
        return Err("implemented/experimental feature needs source paths".into());
    }
    if !matches!(state, "REMOVED" | "SUPERSEDED") {
        for path in &sources {
            safe_path(root, path)?;
        }
    }
    if implemented && strings(feature, "acceptance").is_empty() {
        return Err("implemented feature needs acceptance evidence".into());
    }
    for path in strings(feature, "acceptance") {
        safe_path(root, &path)?;
    }
    let mut previous: Option<&str> = None;
    for event in feature["transitions"]
        .as_array()
        .ok_or("feature requires transition ledger")?
    {
        let from = event["from"].as_str().unwrap_or("");
        let to = event["to"].as_str().unwrap_or("");
        if previous.is_none() {
            if from != "UNRECORDED" {
                return Err("initial feature event must adopt UNRECORDED state".into());
            }
        } else if previous != Some(from) || !valid_transition(from, to) {
            return Err(format!("illegal feature transition {from} -> {to}"));
        }
        if matches!(to, "BOUNDED_IMPLEMENTED" | "IMPLEMENTED")
            && strings(event, "acceptance").is_empty()
        {
            return Err("implemented transition needs acceptance".into());
        }
        for receipt in strings(event, "acceptance") {
            safe_path(root, &receipt)?;
        }
        previous = Some(to);
    }
    if previous != Some(state) {
        return Err("feature state differs from last transition".into());
    }
    let mut environments = BTreeSet::new();
    for v in feature["verification"]
        .as_array()
        .ok_or("feature verification required")?
    {
        let environment = v["environment"].as_str().unwrap_or("");
        if !environments.insert(environment) {
            return Err("duplicate verification environment".into());
        }
        // Retained historical evidence remains integrity checked when stale.
        if let Some(receipt) = v["receipt"].as_str() {
            safe_path(root, receipt)?;
            if v["receipt_sha256"] != hash(&read(&root.join(receipt))?) {
                return Err(format!("verification receipt digest mismatch: {receipt}"));
            }
        }
        if v["state"] == "VERIFIED" {
            if strings(feature, "sources").is_empty() {
                return Err("VERIFIED requires declared feature sources".into());
            }
            let receipt = v["receipt"].as_str().ok_or("VERIFIED needs receipt")?;
            safe_path(root, receipt)?;
            if v["receipt_sha256"] != hash(&read(&root.join(receipt))?) {
                return Err(format!("verification receipt digest mismatch: {receipt}"));
            }
            if v["scope"].as_str().is_none_or(|s| s.trim().is_empty()) {
                return Err("VERIFIED needs explicit revision/profile/platform scope".into());
            }
            let record = read_json(&root.join(receipt))?;
            let sources = record["source_files"]
                .as_array()
                .ok_or("VERIFIED requires receipt source_files")?;
            let mut recorded = BTreeMap::new();
            for source in sources {
                let path = source["path"]
                    .as_str()
                    .ok_or("receipt source path required")?;
                let digest = source["sha256_lf"].as_str();
                if digest.is_some_and(|d| {
                    d.len() != 64
                        || !d
                            .bytes()
                            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                }) || recorded.insert(path, digest).is_some()
                {
                    return Err("duplicate or malformed receipt source digest".into());
                }
            }
            for source in strings(feature, "sources") {
                let current = hash(&read(&root.join(&source))?);
                if recorded.get(source.as_str()).copied().flatten() != Some(current.as_str()) {
                    return Err(format!(
                        "VERIFIED source mismatch: {source}; record STALE or provide a new exact-source receipt"
                    ));
                }
            }
            if environment == "qemu-arm64"
                && (record["correctness"]["matrix"] != "passed"
                    || record["source_files"].as_array().is_none_or(Vec::is_empty)
                    || record["profiles"].as_object().is_none_or(|p| {
                        p.is_empty()
                            || p.values()
                                .any(|p| p["run"]["qemu_version"].as_str().is_none())
                    }))
            {
                return Err("QEMU VERIFIED requires a passing exact-source QEMU receipt".into());
            }
            if environment.starts_with("physical-arm64")
                && record
                    .get("profiles")
                    .and_then(Value::as_object)
                    .is_some_and(|p| p.values().any(|p| p["run"].get("qemu_version").is_some()))
            {
                return Err("QEMU receipt cannot verify physical ARM64".into());
            }
        }
    }
    if feature["readiness"] == "READY" && strings(feature, "readiness_acceptance").is_empty() {
        return Err("READY needs separate readiness acceptance".into());
    }
    for path in strings(feature, "readiness_acceptance") {
        safe_path(root, &path)?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Knowledge {
    pub catalog: Value,
    pub graph: Value,
}
impl Knowledge {
    pub fn build(root: &Path) -> CheckResult<Self> {
        let validator = schema(root)?;
        let enrollment = read_json(&root.join("docs/knowledge-enrollment.json"))?;
        let enrolled = strings(&enrollment, "documents");
        let mut found = BTreeSet::new();
        let mut nodes = BTreeMap::<String, Value>::new();
        let mut aliases = BTreeMap::<String, String>::new();
        let mut inventory = Vec::new();
        let mut inputs = BTreeMap::<String, String>::new();
        let mut current_references = Vec::new();
        for path in documents::markdown(root, true)? {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let text = read(&path)?;
            let Some(meta) = metadata(&text).map_err(|e| format!("{rel}: {e}"))? else {
                inventory.push(rel);
                continue;
            };
            found.insert(rel.clone());
            validator
                .validate(&meta)
                .map_err(|e| format!("{rel}: {e}"))?;
            documents::check_document_status(&text).map_err(|e| format!("{rel}: {e}"))?;
            let references =
                documents::links(root, &path, &field(&text, "Current reference").unwrap())?;
            current_references.push((
                rel.clone(),
                field(&text, "Document status").unwrap(),
                references,
            ));
            let mut units = vec![meta.clone()];
            units.extend(meta["units"].as_array().cloned().unwrap_or_default());
            let doc_id = meta["id"].as_str().unwrap();
            let hs = headings(&text);
            let title = hs
                .iter()
                .find(|h| h.1 == 1)
                .ok_or("knowledge document needs H1")?
                .2
                .clone();
            inputs.insert(rel.clone(), hash(&text));
            for (i, unit) in units.iter().enumerate() {
                let id = unit["id"].as_str().unwrap().to_owned();
                let (start, end, title) = if i == 0 {
                    (1, text.lines().count(), title.clone())
                } else {
                    section(&text, unit["anchor"].as_str().unwrap())?
                };
                if (unit["kind"] == "feature") != unit.get("feature").is_some()
                    || (i == 0 && unit.get("feature").is_some())
                {
                    return Err(format!(
                        "{id}: canonical feature must be a feature-kind section unit"
                    ));
                }
                if let Some(feature) = unit.get("feature") {
                    validate_feature(root, feature).map_err(|e| format!("{id}: {e}"))?;
                    if !matches!(
                        feature["implementation"].as_str(),
                        Some("REMOVED" | "SUPERSEDED")
                    ) {
                        for source in strings(feature, "sources") {
                            inputs.insert(source.clone(), hash(&read(&root.join(source))?));
                        }
                    }
                }
                let mut locations = BTreeMap::new();
                let mut localized = BTreeMap::new();
                locations.insert(
                    "en".to_owned(),
                    json!({"path":rel,"start":start,"end":end,"sha256":hash(&text)}),
                );
                for locale in read_json(&root.join("translations/manifest.json"))?["locales"]
                    .as_object()
                    .ok_or("translation locales required")?
                    .keys()
                {
                    let translated = format!("translations/{locale}/{rel}");
                    let local = read(&root.join(&translated))?;
                    let lm = metadata(&local)?
                        .ok_or_else(|| format!("{translated}: missing knowledge metadata"))?;
                    validator
                        .validate(&lm)
                        .map_err(|e| format!("{translated}: {e}"))?;
                    if machine_metadata(&lm) != machine_metadata(&meta)
                        || field(&local, "Document status") != field(&text, "Document status")
                    {
                        return Err(format!("{rel}: EN/{locale} knowledge metadata differs"));
                    }
                    let (ls, le, local_title) = if i == 0 {
                        (
                            1,
                            local.lines().count(),
                            headings(&local)
                                .into_iter()
                                .find(|h| h.1 == 1)
                                .ok_or("translation needs H1")?
                                .2,
                        )
                    } else {
                        section(&local, unit["anchor"].as_str().unwrap())?
                    };
                    let local_unit = if i == 0 { &lm } else { &lm["units"][i - 1] };
                    localized.insert(locale.clone(), json!({"title":local_title,"summary":local_unit["summary"],"tags":local_unit["tags"],"read_when":local_unit["read_when"]}));
                    inputs.insert(translated.clone(), hash(&local));
                    locations.insert(
                        locale.clone(),
                        json!({"path":translated,"start":ls,"end":le,"sha256":hash(&local)}),
                    );
                }
                let mut node = unit.clone();
                node.as_object_mut().unwrap().remove("units");
                node["title"] = json!(title);
                node["path"] = json!(rel);
                node["document"] = json!(doc_id);
                node["document_status"] = json!(field(&text, "Document status").unwrap());
                node["evidence_scope"] = json!(field(&text, "Evidence scope").unwrap());
                node["locations"] = json!(locations);
                node["localized"] = json!(localized);
                let bytes = text
                    .lines()
                    .skip(start - 1)
                    .take(end - start + 1)
                    .map(|l| l.len() + 1)
                    .sum::<usize>();
                node["estimated_bytes"] = json!(bytes);
                for alias in strings(unit, "aliases") {
                    if aliases.insert(alias.clone(), id.clone()).is_some() {
                        return Err(format!("duplicate alias {alias}"));
                    }
                }
                if nodes.insert(id.clone(), node).is_some() {
                    return Err(format!("duplicate knowledge ID {id}"));
                }
            }
        }
        if enrolled.iter().collect::<BTreeSet<_>>().len() != enrolled.len() {
            return Err("duplicate enrollment path".into());
        }
        for path in enrolled {
            if !found.contains(&path) {
                return Err(format!("enrolled document missing knowledge ID: {path}"));
            }
        }
        // Projections preserve record-owned status; no new architecture is inferred.
        for (directory, prefix, kind) in [
            ("research/cost-l", "cost-l", "cost-l"),
            ("research/cases", "case.kol-path", "research-case"),
        ] {
            for path in crate::database::json_files(&root.join(directory))? {
                if path.file_name().is_some_and(|n| n == "registry.json") {
                    continue;
                }
                let record = read_json(&path)?;
                let original = record["id"].as_str().ok_or("research record requires ID")?;
                let number = original.rsplit('-').next().unwrap();
                let id = format!("{prefix}.{number}");
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                inputs.insert(rel.clone(), hash(&read(&path)?));
                let projected = json!({"id":id,"path":rel,"kind":kind,"title":record["title"],"summary":record.get("summary").cloned().unwrap_or_else(|| json!("Source-backed research; read record scope and uncertainty.")),"entity_status":record["status"],"aliases":[original],"sources":record["sources"],"related_cost_l":record["related_cost_l"],"locations":{"en":{"path":rel,"sha256":hash(&read(&path)?)}}});
                if nodes.insert(id.clone(), projected).is_some()
                    || aliases.insert(original.into(), id).is_some()
                {
                    return Err("duplicate projected ID/alias".into());
                }
            }
        }
        for path in [
            "docs/knowledge-enrollment.json",
            "schemas/knowledge.schema.json",
            "translations/manifest.json",
        ] {
            inputs.insert(path.into(), hash(&read(&root.join(path))?));
        }
        if nodes.len() > MAX_NODES {
            return Err("knowledge node budget exceeded".into());
        }
        for alias in aliases.keys() {
            if nodes.contains_key(alias) {
                return Err(format!("alias shadows canonical ID {alias}"));
            }
        }
        for (path, status, targets) in current_references {
            if status == "CURRENT"
                && !targets.is_empty()
                && targets.iter().all(|t| {
                    let path = t.split('#').next().unwrap();
                    nodes
                        .values()
                        .any(|n| n["path"] == path && n["document_status"] == "SUPERSEDED")
                })
            {
                return Err(format!(
                    "{path}: Current reference points only to superseded authority"
                ));
            }
        }
        let mut edges = Vec::new();
        for (id, node) in &nodes {
            for target in strings(node, "depends_on") {
                edges.push(json!({"from":id,"type":"depends_on","to":target}));
            }
            for edge in node["relationships"].as_array().into_iter().flatten() {
                let kind = edge["type"].as_str().unwrap_or("");
                if !EDGES.contains(&kind) {
                    return Err(format!("unknown edge type {kind}"));
                }
                if kind == "supersedes"
                    && edge["scope"].as_str().is_none_or(|s| s.trim().is_empty())
                {
                    return Err("supersedes needs scope".into());
                }
                let mut e = edge.clone();
                e["from"] = json!(id);
                edges.push(e);
            }
            if let Some(feature) = node.get("feature") {
                for path in strings(feature, "sources") {
                    if !matches!(
                        feature["implementation"].as_str(),
                        Some("REMOVED" | "SUPERSEDED")
                    ) {
                        edges.push(json!({"from":id,"type":"implemented_by","to":path}));
                    }
                }
                for path in strings(feature, "acceptance") {
                    edges.push(json!({"from":id,"type":"validated_by","to":path}));
                }
                for adr in strings(feature, "adrs") {
                    edges.push(json!({"from":id,"type":"depends_on","to":adr}));
                }
            }
        }
        edges.sort_by_key(Value::to_string);
        edges.dedup();
        for edge in &edges {
            let target = edge["to"].as_str().ok_or("edge target required")?;
            let kind = edge["type"].as_str().unwrap();
            if matches!(kind, "implemented_by" | "validated_by") {
                safe_path(root, target)?;
            } else {
                let resolved = aliases.get(target).map_or(target, String::as_str);
                let node = nodes
                    .get(resolved)
                    .ok_or_else(|| format!("dangling {kind} relationship to {target}"))?;
                if kind == "cost_l" && node["kind"] != "cost-l" {
                    return Err("cost_l must target a debt record".into());
                }
                let source = &nodes[edge["from"].as_str().unwrap()];
                if kind == "depends_on"
                    && source["document_status"] == "CURRENT"
                    && (node["document_status"] == "SUPERSEDED"
                        || matches!(
                            node["feature"]["implementation"].as_str(),
                            Some("REMOVED" | "SUPERSEDED")
                        ))
                {
                    return Err(format!(
                        "current authority depends on superseded {resolved}"
                    ));
                }
            }
        }
        for kind in ["depends_on", "supersedes"] {
            acyclic(&nodes, &aliases, &edges, kind)?;
        }
        let entries: Vec<_> = nodes
            .values()
            .map(|node| {
                let mut small = node.clone();
                for key in [
                    "locations",
                    "feature",
                    "sources",
                    "evidence_scope",
                    "schema_version",
                    "localized",
                ] {
                    small.as_object_mut().unwrap().remove(key);
                }
                if let Some(f) = node.get("feature") {
                    small["implementation"] = f["implementation"].clone();
                }
                small
            })
            .collect();
        Ok(Self {
            catalog: json!({"schema_version":1,"entries":entries,"aliases":aliases,"unenrolled_documents":inventory}),
            graph: json!({"schema_version":1,"inputs":inputs,"nodes":nodes,"edges":edges}),
        })
    }
    fn resolve<'a>(&'a self, id: &'a str) -> CheckResult<&'a str> {
        let resolved = self.catalog["aliases"][id].as_str().unwrap_or(id);
        if self.graph["nodes"].get(resolved).is_none() {
            return Err(format!("unknown knowledge ID {id}"));
        }
        Ok(resolved)
    }
    pub fn closure(&self, ids: &[String]) -> CheckResult<Vec<String>> {
        let mut visited = BTreeSet::new();
        let mut todo = ids.to_vec();
        let mut prerequisites = BTreeMap::<String, Vec<String>>::new();
        for e in self.graph["edges"].as_array().unwrap() {
            if e["type"] == "depends_on" {
                prerequisites
                    .entry(e["from"].as_str().unwrap().into())
                    .or_default()
                    .push(e["to"].as_str().unwrap().into());
            }
        }
        while let Some(id) = todo.pop() {
            let id = self.resolve(&id)?.to_owned();
            if !visited.insert(id.clone()) {
                continue;
            }
            if visited.len() > MAX_NODES {
                return Err("dependency budget exceeded".into());
            }
            todo.extend(prerequisites.get(&id).into_iter().flatten().cloned());
        }
        Ok(visited.into_iter().collect())
    }
    pub fn impact(&self, input: &str) -> CheckResult<Value> {
        let mut seen = BTreeSet::new();
        if let Ok(id) = self.resolve(input) {
            seen.insert(id.to_owned());
        }
        for (id, node) in self.graph["nodes"].as_object().unwrap() {
            if node["path"] == input {
                seen.insert(id.clone());
            }
        }
        let edges = self.graph["edges"].as_array().unwrap();
        if edges.iter().any(|e| e["to"] == input) {
            seen.insert(input.to_owned());
        }
        if seen.is_empty() {
            return Err(format!("unknown knowledge ID or declared path {input}"));
        }
        loop {
            let before = seen.len();
            for e in edges {
                if seen.contains(e["to"].as_str().unwrap()) {
                    seen.insert(e["from"].as_str().unwrap().to_owned());
                }
            }
            if seen.len() == before {
                break;
            }
        }
        Ok(
            json!({"input":input,"affected":seen,"scope":"Declared documentation relationships; review canonical docs, locale pairs, summaries and evidence applicability. Not exhaustive code impact."}),
        )
    }
    fn check_locale(&self, root: &Path, locale: &str) -> CheckResult<()> {
        if locale != "en"
            && read_json(&root.join("translations/manifest.json"))?["locales"]
                .get(locale)
                .is_none()
        {
            return Err(format!("unsupported locale {locale}"));
        }
        Ok(())
    }
    pub fn render(&self, root: &Path, ids: &[String], locale: &str) -> CheckResult<String> {
        self.check_locale(root, locale)?;
        let mut ranges = BTreeMap::<String, BTreeSet<usize>>::new();
        let mut headers = String::new();
        for id in ids {
            let id = self.resolve(id)?;
            let node = &self.graph["nodes"][id];
            let loc = node["locations"]
                .get(locale)
                .or_else(|| node["locations"].get("en"))
                .ok_or("missing locale location")?;
            let path = loc["path"].as_str().unwrap();
            safe_path(root, path)?;
            let text = read(&root.join(path))?;
            if loc["sha256"] != hash(&text) {
                return Err(format!("stale knowledge range: {path}"));
            }
            headers.push_str(&format!(
                "{id}: {} [{}; {}; digest {}]\n",
                node["localized"][locale]["title"]
                    .as_str()
                    .or_else(|| node["title"].as_str())
                    .unwrap_or(id),
                node["document_status"]
                    .as_str()
                    .unwrap_or("record-owned state"),
                node["evidence_scope"]
                    .as_str()
                    .unwrap_or("see original research record"),
                loc["sha256"].as_str().unwrap()
            ));
            if let Some(feature) = node.get("feature") {
                headers.push_str(&format!(
                    "Implementation: {}; scope: {}; next gate: {}\n",
                    feature["implementation"].as_str().unwrap(),
                    feature["implementation_scope"].as_str().unwrap(),
                    feature["next_gate"].as_str().unwrap()
                ));
                headers.push_str(&format!(
                    "Readiness: {}; limitations: {}; verification: {}; acceptance paths: {}\n",
                    feature["readiness"],
                    feature["limitations"],
                    feature["verification"],
                    feature["acceptance"]
                ));
            }
            let selected = ranges.entry(path.into()).or_default();
            if let (Some(start), Some(end)) = (loc["start"].as_u64(), loc["end"].as_u64()) {
                selected.extend(start as usize..=end as usize);
                if let Some((first, _, _)) = headings(&text).iter().find(|h| h.1 == 2) {
                    selected.extend(1..*first);
                }
                // Include enclosing heading introduction: mandatory scoped warnings precede children.
                for (i, (start, level, _)) in headings(&text).iter().enumerate() {
                    if *start >= loc["start"].as_u64().unwrap() as usize {
                        break;
                    }
                    let hs = headings(&text);
                    let end = hs
                        .iter()
                        .skip(i + 1)
                        .find(|h| h.1 <= *level)
                        .map_or(text.lines().count(), |h| h.0 - 1);
                    if end >= loc["end"].as_u64().unwrap() as usize {
                        let intro_end = hs.get(i + 1).map_or(end, |h| h.0 - 1);
                        selected.extend(*start..=intro_end);
                    }
                }
            } else {
                selected.extend(1..=text.lines().count());
            }
        }
        let mut output = headers;
        for (path, selected) in ranges {
            output.push_str(&format!("\nSource: {path}\n"));
            let text = read(&root.join(&path))?;
            let mut hide = false;
            let mut fence = None;
            for (i, line) in text.lines().enumerate() {
                if line.trim() == MARKER && fence.is_none() {
                    hide = true;
                    continue;
                }
                if hide {
                    if line == "```" {
                        hide = false;
                    }
                    continue;
                }
                fence_line(line, &mut fence);
                if selected.contains(&(i + 1)) {
                    output.push_str(line);
                    output.push('\n');
                }
            }
        }
        if output.len() > MAX_CONTEXT_BYTES {
            return Err("context exceeds 4 MiB; choose narrower units".into());
        }
        Ok(output)
    }
    pub fn context(
        &self,
        root: &Path,
        query: &str,
        budget: usize,
        locale: &str,
    ) -> CheckResult<Value> {
        if query.len() > 4096 || budget > MAX_CONTEXT_BYTES {
            return Err("query/budget limit exceeded".into());
        }
        self.check_locale(root, locale)?;
        let words: BTreeSet<_> = query
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '.')
            .filter(|s| s.chars().count() > 2)
            .map(str::to_lowercase)
            .collect();
        let mut ranked = Vec::new();
        for node in self.catalog["entries"].as_array().unwrap() {
            if (matches!(
                node["document_status"].as_str(),
                Some("HISTORICAL" | "HISTORICAL MILESTONE" | "SUPERSEDED")
            ) || matches!(
                node["implementation"].as_str(),
                Some("REMOVED" | "SUPERSEDED")
            )) && !words.iter().any(|w| {
                matches!(
                    w.as_str(),
                    "historical"
                        | "history"
                        | "removed"
                        | "superseded"
                        | "история"
                        | "исторический"
                        | "удалённый"
                )
            }) {
                continue;
            }
            let mut score = 0;
            for (field, weight) in [
                ("id", 10),
                ("tags", 6),
                ("read_when", 5),
                ("title", 3),
                ("summary", 1),
            ] {
                let value = format!(
                    "{} {}",
                    node[field],
                    self.graph["nodes"][node["id"].as_str().unwrap()]["localized"][locale][field]
                )
                .to_lowercase();
                score += words.iter().filter(|w| value.contains(w.as_str())).count() * weight;
            }
            if node["kind"] == "feature" {
                score *= 2;
            }
            if score > 0 {
                ranked.push((score, node["id"].as_str().unwrap().to_owned()));
            }
        }
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        // Discovery is bounded; closures are atomic and never truncated to fit.
        let top = ranked.first().map_or(0, |r| r.0);
        let seeds: Vec<_> = ranked
            .iter()
            .filter(|r| r.0 * 5 >= top * 3)
            .take(3)
            .map(|(_, id)| id.clone())
            .collect();
        let ids = self.closure(&seeds)?;
        let text = self.render(root, &ids, locale)?;
        let mut missing = Vec::new();
        if ids.is_empty() {
            missing.push(json!({"missing":"UNKNOWN: no matching enrolled context; refine the query or inspect unenrolled inventory."}));
        }
        for id in &ids {
            for gap in strings(&self.graph["nodes"][id], "gaps") {
                missing.push(json!({"id":id,"missing":gap}));
            }
        }
        Ok(
            json!({"query":query,"locale":locale,"seeds":seeds,"reason":"Up to three deterministic ID/tag/read-condition/title/summary matches scoring at least 60% of the best candidate, plus mandatory prerequisite closure; related edges are optional.","selected":ids,"missing":missing,"bytes":text.len(),"estimated_tokens":text.len().div_ceil(4),"estimate_method":"ceil UTF-8 bytes/4; approximate, not a tokenizer","budget_bytes":budget,"budget_exceeded":text.len()>budget,"context":text}),
        )
    }
}
fn machine_metadata(value: &Value) -> Value {
    let mut result = value.clone();
    fn strip(v: &mut Value) {
        if let Some(o) = v.as_object_mut() {
            for key in ["summary", "tags", "read_when"] {
                o.remove(key);
            }
            if let Some(units) = o.get_mut("units").and_then(Value::as_array_mut) {
                for u in units {
                    strip(u);
                }
            }
        }
    }
    strip(&mut result);
    result
}
fn acyclic(
    nodes: &BTreeMap<String, Value>,
    aliases: &BTreeMap<String, String>,
    edges: &[Value],
    kind: &str,
) -> CheckResult<()> {
    let mut degree: BTreeMap<String, usize> = nodes.keys().map(|id| (id.clone(), 0)).collect();
    let mut outgoing = BTreeMap::<String, Vec<String>>::new();
    for e in edges.iter().filter(|e| e["type"] == kind) {
        let to = e["to"].as_str().unwrap();
        let to = aliases.get(to).map_or(to, String::as_str).to_owned();
        *degree.get_mut(&to).unwrap() += 1;
        outgoing
            .entry(e["from"].as_str().unwrap().into())
            .or_default()
            .push(to);
    }
    let mut ready: Vec<_> = degree
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut visited = 0;
    while let Some(id) = ready.pop() {
        visited += 1;
        for target in outgoing.get(&id).into_iter().flatten() {
            let n = degree.get_mut(target).unwrap();
            *n -= 1;
            if *n == 0 {
                ready.push(target.clone());
            }
        }
    }
    if visited != nodes.len() {
        Err(format!("{kind} cycle"))
    } else {
        Ok(())
    }
}

pub fn generate(root: &Path, check: bool) -> CheckResult<()> {
    let knowledge = Knowledge::build(root)?;
    for (path, value) in [
        ("docs/catalog.json", &knowledge.catalog),
        ("docs/knowledge-graph.json", &knowledge.graph),
    ] {
        if check {
            if read_json(&root.join(path))? != *value {
                return Err(format!(
                    "stale generated knowledge output: {path}; run docs generate"
                ));
            }
        } else {
            write_json(&root.join(path), value)?;
        }
    }
    summaries(root, &knowledge, check)
}

/// Reproducible executed retrieval receipt; no LLM quality or kernel claim.
pub fn pilot(root: &Path, check: bool) -> CheckResult<()> {
    let knowledge = Knowledge::build(root)?;
    let query = "review Phase 3.4 capability revocation";
    let result = knowledge.context(root, query, 131072, "en")?;
    let selected = result["selected"].as_array().unwrap();
    for required in [
        "kolvrt.security.capability-revocation",
        "kolvrt.handles.identity",
        "kolvrt.handles.lifetime",
        "kolvrt.process.reclamation",
        "kolvrt.memory.user-copy",
        "law.009",
        "law.013",
        "law.018",
        "adr.0013",
        "adr.0017",
        "adr.0018",
        "adr.0019",
        "adr.0020",
        "adr.0022",
        "kolvrt.security.event-revocation",
        "kolvrt.security.domains",
        "adr.0023",
    ] {
        if !selected.contains(&json!(required)) {
            return Err(format!("pilot misses mandatory context {required}"));
        }
    }
    if result["missing"].as_array().unwrap().is_empty() || result["budget_exceeded"] == true {
        return Err("pilot must preserve remaining gaps within budget".into());
    }
    let catalog_bytes = serde_json::to_vec(&knowledge.catalog)
        .map_err(|e| e.to_string())?
        .len();
    let all = documents::markdown(&root.join("docs"), true)?;
    let baseline_bytes = all
        .iter()
        .try_fold(0, |sum, p| read(p).map(|s| sum + s.len()))?;
    let mut source_paths = BTreeSet::new();
    let mut evidence = BTreeMap::new();
    for id in selected {
        let node = &knowledge.graph["nodes"][id.as_str().unwrap()];
        source_paths.insert(node["path"].as_str().unwrap().to_owned());
        if let Some(feature) = node.get("feature") {
            for path in strings(feature, "acceptance") {
                evidence.insert(path.clone(), read(&root.join(path))?.len());
            }
        }
    }
    let full_bytes = source_paths
        .iter()
        .try_fold(0, |sum, p| read(&root.join(p)).map(|s| sum + s.len()))?;
    let mut source_files = Vec::new();
    for path in [
        "crates/repository-checks/src/knowledge.rs",
        "crates/repository-checks/src/knowledge/impact.rs",
        "crates/repository-checks/tests/knowledge.rs",
        "schemas/implementation-impact.schema.json",
    ] {
        source_files.push(json!({"path":path,"sha256_lf":hash(&read(&root.join(path))?)}));
    }
    let selected_locations:Vec<_>=selected.iter().map(|id|json!({"id":id,"location":knowledge.graph["nodes"][id.as_str().unwrap()]["locations"]["en"]})).collect();
    let context_bytes = result["bytes"].as_u64().unwrap() as usize;
    let report = json!({"schema_version":1,"claim":"Executed offline deterministic navigation and metadata validation only; no LLM correctness, kernel or physical-hardware evidence.","correctness":{"retrieval":"passed","mandatory_ids":"present","planned_gap":"explicit","budget":"within 131072 bytes"},"query":query,"source_files":source_files,"catalog_sha256":hash(&knowledge.catalog.to_string()),"catalog_serialization":"compact standard JSON, whitespace has no meaning","catalog_bytes":catalog_bytes,"catalog_nodes":knowledge.catalog["entries"].as_array().unwrap().len(),"docs_baseline":{"documents":all.len(),"utf8_bytes":baseline_bytes},"selected_full_documents":{"count":source_paths.len(),"utf8_bytes":full_bytes},"rendered_context_bytes":context_bytes,"context_plus_catalog_bytes":context_bytes+catalog_bytes,"approximate_tokens":(context_bytes+catalog_bytes).div_ceil(4),"estimate":"ceil UTF-8 bytes/4, not a tokenizer","selected":selected_locations,"seeds":result["seeds"],"missing":result["missing"],"optional_raw_acceptance_bytes":evidence,"evidence_loading":"Receipts are linked, not loaded by context. Inspect their exact-source scope when the task audits execution; add their bytes to any end-to-end claim.","external_issue_body_bytes_loaded":0,"issue_body_scope":"Planned issue #24 requirements are authored in the canonical feature contract; live issue body is not loaded. Online existence is a separate optional check.","coverage_review":"The actual test asserts required authority/lifetime IDs; a reviewer still judges semantic sufficiency. Unenrolled historical/docs domains remain staged."});
    let mut report = report;
    let ru = knowledge.context(root, "проверь отзыв полномочий", 262144, "ru")?;
    for required in selected {
        if !ru["selected"].as_array().unwrap().contains(required) {
            return Err(format!("Russian pilot misses required context {required}"));
        }
    }
    let localized_navigation: BTreeMap<_, _> = knowledge.graph["nodes"]
        .as_object()
        .unwrap()
        .iter()
        .filter_map(|(id, node)| node["localized"].get("ru").map(|v| (id, v)))
        .collect();
    let localized_bytes = serde_json::to_vec(&localized_navigation)
        .map_err(|e| e.to_string())?
        .len();
    let ru_bytes = ru["bytes"].as_u64().unwrap() as usize;
    report["russian_pilot"] = json!({"query":ru["query"],"locale":"ru","seeds":ru["seeds"],"selected":ru["selected"],"missing":ru["missing"],"rendered_context_bytes":ru_bytes,"catalog_bytes":catalog_bytes,"localized_navigation_bytes":localized_bytes,"context_plus_navigation_bytes":ru_bytes+catalog_bytes+localized_bytes,"approximate_tokens":(ru_bytes+catalog_bytes+localized_bytes).div_ceil(4),"correctness":"English pilot prerequisites preserved; Unicode query and Russian locations executed; no semantic completeness proof."});
    let impact = knowledge.impact("kolvrt.handles.identity")?;
    for id in [
        "kolvrt.security.domains",
        "kolvrt.security.capability-revocation",
    ] {
        if !impact["affected"].as_array().unwrap().contains(&json!(id)) {
            return Err(format!("pilot impact misses {id}"));
        }
    }
    report["impact"] = impact;
    report["issue_body_scope"] = json!(
        "Current requirements and remaining gates are authored in canonical contracts; live issue bodies are not loaded. Online existence is a separate optional check."
    );
    let path = root.join("research/results/documentation-knowledge-pilot.json");
    if check {
        if read_json(&path)? != report {
            return Err("stale documentation pilot receipt; run docs pilot".into());
        }
    } else {
        write_json(&path, &report)?;
    }
    Ok(())
}
fn summaries(root: &Path, knowledge: &Knowledge, check: bool) -> CheckResult<()> {
    for locale in ["en", "ru"] {
        let path = if locale == "en" {
            "README.md".to_owned()
        } else {
            format!("translations/{locale}/README.md")
        };
        let text = read(&root.join(&path))?;
        let begin = "<!-- feature-summary:start -->";
        let end = "<!-- feature-summary:end -->";
        let start = text
            .find(begin)
            .ok_or("README missing generated feature summary start")?
            + begin.len();
        let finish = text
            .find(end)
            .ok_or("README missing generated feature summary end")?;
        let mut table = if locale == "en" {
            "\n\n| Canonical feature | Implementation | Evidence limit |\n| --- | --- | --- |\n"
                .to_owned()
        } else {
            "\n\n| РљР°РЅРѕРЅРёС‡РµСЃРєР°СЏ С„СѓРЅРєС†РёСЏ | Р РµР°Р»РёР·Р°С†РёСЏ | Р“СЂР°РЅРёС†Р° РґРѕРєР°Р·Р°С‚РµР»СЊСЃС‚РІ |\n| --- | --- | --- |\n".to_owned()
        };
        for (id, node) in knowledge.graph["nodes"].as_object().unwrap() {
            if node.get("feature").is_none() {
                continue;
            }
            let loc = &node["locations"][locale];
            let prefix = if locale == "ru" {
                "translations/ru/"
            } else {
                ""
            };
            let p = loc["path"].as_str().unwrap().strip_prefix(prefix).unwrap();
            let p = format!("{p}#{}", node["anchor"].as_str().unwrap());
            let limit = node["feature"]["verification"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| {
                    format!(
                        "{}: {}",
                        v["environment"].as_str().unwrap(),
                        v["state"].as_str().unwrap()
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            table.push_str(&format!(
                "| [{id}]({p}) | {} | {limit} |\n",
                node["feature"]["implementation"].as_str().unwrap()
            ));
        }
        table.push('\n');
        let normalize = |s: &str| {
            s.lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| {
                    l.split('|')
                        .map(str::trim)
                        .map(|c| {
                            if !c.is_empty() && c.chars().all(|v| v == '-') {
                                "---"
                            } else {
                                c
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("|")
                })
                .collect::<Vec<_>>()
        };
        if check {
            if normalize(&text[start..finish]) != normalize(&table) {
                return Err(format!("stale feature summary in {path}"));
            }
        } else if normalize(&text[start..finish]) != normalize(&table) {
            fs::write(
                root.join(path),
                format!("{}{}{}", &text[..start], table, &text[finish..]),
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// History-aware CI checks declared semantic/evidence impact, not code patterns.
pub fn check_change(root: &Path, base: &str) -> CheckResult<()> {
    if base.starts_with('-') {
        return Err("invalid base revision".into());
    }
    let run = |args: &[&str]| -> CheckResult<String> {
        let out = Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).into());
        }
        String::from_utf8(out.stdout).map_err(|e| e.to_string())
    };
    let changed = format!(
        "{}{}",
        run(&["diff", "--name-only", base, "--"])?,
        run(&["ls-files", "--others", "--exclude-standard"])?
    );
    let changed: BTreeSet<_> = changed.lines().collect();
    let knowledge = Knowledge::build(root)?;
    let mut old_features = BTreeMap::new();
    if let Ok(old_enrollment) = run(&["show", &format!("{base}:docs/knowledge-enrollment.json")]) {
        for path in strings(&parse_json(&old_enrollment)?, "documents") {
            if let Ok(old) = run(&["show", &format!("{base}:{path}")])
                && let Some(meta) = metadata(&old)?
            {
                let mut ids = vec![meta["id"].as_str().unwrap().to_owned()];
                ids.extend(
                    meta["units"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|u| u["id"].as_str().unwrap().to_owned()),
                );
                for id in ids {
                    knowledge.resolve(&id).map_err(|_| {
                        format!(
                            "retired/moved knowledge ID {id} disappeared; retain alias/tombstone"
                        )
                    })?;
                }
                for unit in meta["units"].as_array().into_iter().flatten() {
                    if let Some(feature) = unit.get("feature") {
                        let id = knowledge.resolve(unit["id"].as_str().unwrap())?.to_owned();
                        old_features.insert(id, feature.clone());
                    }
                }
            }
        }
    }
    for id in old_features.keys() {
        if knowledge.graph["nodes"][id].get("feature").is_none() {
            return Err(format!(
                "{id}: canonical feature history must be retained, including terminal states"
            ));
        }
    }
    impact::check(root, base, &changed, &knowledge, &old_features, run)?;
    for (id, node) in knowledge.graph["nodes"].as_object().unwrap() {
        let Some(feature) = node.get("feature") else {
            continue;
        };
        if let Some(old_feature) = old_features.get(id) {
            let old_events = &old_feature["transitions"];
            let new_events = &feature["transitions"];
            if let (Some(a), Some(b)) = (old_events.as_array(), new_events.as_array())
                && (b.len() < a.len() || b[..a.len()] != a[..])
            {
                return Err(format!("{id}: feature transition history was rewritten"));
            }
        }
    }
    Ok(())
}

pub fn cli(root: &Path, args: &[String]) -> CheckResult<()> {
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "generate" if args.len() == 1 || (args.len() == 2 && args[1] == "--check") => {
            return generate(root, args.len() == 2);
        }
        "check-change" if args.len() == 2 => return check_change(root, &args[1]),
        "pilot" if args.len() == 1 || (args.len() == 2 && args[1] == "--check") => {
            return pilot(root, args.len() == 2);
        }
        _ => {}
    }
    let k = Knowledge::build(root)?;
    match command {
        "check-issues" if args.len()==1 => {
            let numbers:BTreeSet<_>=k.graph["nodes"].as_object().unwrap().values().flat_map(|n|n["feature"]["issues"].as_array().into_iter().flatten().filter_map(Value::as_u64)).collect();
            for number in numbers {
                let result=Command::new("gh").args(["issue","view",&number.to_string(),"--repo","lifeFedorovAlexey/KOLVRT","--json","number,state,url"]).output();
                let value=match result {
                    Ok(o) if o.status.success()=>json!({"issue":number,"availability":"VALID","snapshot":parse_json(&String::from_utf8_lossy(&o.stdout))?,"meaning":"Issue existence/state does not establish feature acceptance."}),
                    _=>json!({"issue":number,"availability":"UNKNOWN","reason":"GitHub lookup unavailable or inaccessible; offline validation makes no existence claim."})
                };
                println!("{value}");
            }
        }
        "find" if args.len()==2 => { let q=args[1].to_lowercase();for n in k.catalog["entries"].as_array().unwrap(){if n.to_string().to_lowercase().contains(&q){println!("{}",n);}} },
        "show" if args.len()==2 || (args.len()==4 && args[2]=="--locale") => {let locale=if args.len()==4 {&args[3]}else{"en"};println!("{}",k.render(root,&[args[1].clone()],locale)?);},
        "deps" if args.len()==2 => println!("{}",json!(k.closure(&[args[1].clone()])?)),
        "impact" if args.len()==2 => println!("{}",k.impact(&args[1])?),
        "related" if args.len()==2 => {let id=k.resolve(&args[1])?;for e in k.graph["edges"].as_array().unwrap(){if e["from"]==id || e["to"]==id{println!("{e}");}}},
        "context" if args.len() >= 2 => {
            let mut budget = 65536;
            let mut locale = "en";
            let mut seen = BTreeSet::new();
            for option in args[2..].chunks(2) {
                if option.len() != 2 || !seen.insert(option[0].as_str()) { return Err("missing or duplicate context option".into()); }
                match option[0].as_str() {
                    "--locale" => locale = &option[1],
                    "--budget-bytes" => budget = option[1].parse().map_err(|_| "invalid byte budget")?,
                    _ => return Err("unknown context option".into()),
                }
            }
            let result = k.context(root, &args[1], budget, locale)?;
            println!("{}", serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?);
            if result["budget_exceeded"] == true { return Err("required context exceeds budget; choose narrower units, no dependency was silently dropped".into()); }
        },
        _=>return Err("docs: generate [--check] | pilot [--check] | check-issues | find TEXT | show ID [--locale ru] | deps ID | related ID | impact ID | context TASK [--locale ru] [--budget-bytes N] | check-change BASE".into())
    }
    Ok(())
}
