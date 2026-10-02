use repository_checks::{database, documents, exceptions, read_json, report, security_artifacts};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

struct Temp(PathBuf);

impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "kolvrt-repository-host-contracts-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn translation_manifest(root: &Path, pairs: &[(&str, &str, &str)]) {
    let mut entries = serde_json::Map::new();
    for (relative, source, translated) in pairs {
        let source_path = root.join(relative);
        let translated_path = root.join("translations/ru").join(relative);
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::create_dir_all(translated_path.parent().unwrap()).unwrap();
        fs::write(&source_path, source).unwrap();
        fs::write(&translated_path, translated).unwrap();
        entries.insert(
            (*relative).into(),
            json!({
                "source_sha256": repository_checks::hash(source),
                "translation_sha256": repository_checks::hash(translated)
            }),
        );
    }
    let path = root.join("translations/manifest.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        serde_json::to_vec(&json!({
            "source_language":"en",
            "locales":{"ru":entries}
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn research_corpus_schema_ledger_and_semantic_checks_accept_the_retained_records() {
    let repository = root();
    let cases_path = repository.join("research/cases");
    let cases = database::validate(
        &repository,
        &cases_path,
        repository_checks::MINIMUM_RESEARCH_CASES,
    )
    .unwrap();
    assert!(cases.len() >= repository_checks::MINIMUM_RESEARCH_CASES);
    database::check_ledger(&repository).unwrap();

    let validator = database::schema(&repository).unwrap();
    let valid = cases
        .iter()
        .find(|case| {
            case["sources"]
                .as_array()
                .is_some_and(|sources| sources.len() >= 2)
        })
        .unwrap();
    assert!(
        database::case_errors(
            &validator,
            valid,
            &format!("{}.json", valid["id"].as_str().unwrap())
        )
        .is_empty()
    );

    let mut wrong_url = valid.clone();
    wrong_url["sources"][0]["url"] = json!("ftp://example.test/source");
    assert!(
        !database::case_errors(
            &validator,
            &wrong_url,
            &format!("{}.json", valid["id"].as_str().unwrap())
        )
        .is_empty()
    );

    let mut wrong_date = valid.clone();
    wrong_date["sources"][0]["research_date"] = json!("2026-10-01");
    assert!(
        database::case_errors(
            &validator,
            &wrong_date,
            &format!("{}.json", valid["id"].as_str().unwrap())
        )
        .iter()
        .any(|e| e.contains("research date differs"))
    );

    let mut bad_filename = valid.clone();
    assert!(
        database::case_errors(&validator, &bad_filename, "wrong-name.json")
            .iter()
            .any(|e| e.contains("filename must match ID"))
    );
    bad_filename["sources"][0]["id"] = bad_filename["sources"][1]["id"].clone();
    assert!(
        database::case_errors(&validator, &bad_filename, "wrong-name.json")
            .iter()
            .any(|e| e.contains("duplicate source ID"))
    );

    let mut missing_evidence = valid.clone();
    missing_evidence["evidence"]["historical_context"] = json!(["absent"]);
    assert!(
        database::case_errors(
            &validator,
            &missing_evidence,
            &format!("{}.json", valid["id"].as_str().unwrap())
        )
        .iter()
        .any(|e| e.contains("invalid or missing evidence"))
    );

    let mut wrong_compatibility = valid.clone();
    wrong_compatibility["compatibility_required"] = json!("NO");
    wrong_compatibility["compatibility_scope"] = json!("synthetic.required.scope");
    assert!(
        database::case_errors(
            &validator,
            &wrong_compatibility,
            &format!("{}.json", valid["id"].as_str().unwrap())
        )
        .iter()
        .any(|e| e.contains("NO compatibility requires NONE scope"))
    );

    let mut duplicate_source = valid.clone();
    let source = duplicate_source["sources"][0].clone();
    duplicate_source["sources"]
        .as_array_mut()
        .unwrap()
        .push(source);
    assert!(
        database::case_errors(
            &validator,
            &duplicate_source,
            &format!("{}.json", valid["id"].as_str().unwrap())
        )
        .iter()
        .any(|e| e.contains("duplicate source ID"))
    );
}

#[test]
fn markdown_walker_sorts_and_can_skip_translations() {
    let temp = Temp::new();
    temp.write("z.md", "z");
    temp.write("a/inside.md", "inside");
    temp.write("translations/ru.md", "ru");
    temp.write("notes.txt", "ignored");

    let all = documents::markdown(&temp.0, false).unwrap();
    assert_eq!(
        all.iter()
            .map(|p| p
                .strip_prefix(&temp.0)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"))
            .collect::<Vec<_>>(),
        ["a/inside.md", "translations/ru.md", "z.md"]
    );
    let public = documents::markdown(&temp.0, true).unwrap();
    assert_eq!(
        public
            .iter()
            .map(|p| p
                .strip_prefix(&temp.0)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"))
            .collect::<Vec<_>>(),
        ["a/inside.md", "z.md"]
    );
    assert!(documents::markdown(&temp.0.join("missing"), false).is_err());
}

#[test]
fn markdown_links_resolve_escapes_fragments_external_urls_and_reject_bad_targets() {
    let temp = Temp::new();
    let source = temp.write("docs/page.md", "source");
    temp.write("docs/space name.md", "target");
    temp.write("outside.md", "outside");
    let root = temp.0.join("docs");
    let targets = documents::links(
        &root,
        &source,
        "[encoded](space%20name.md#part) [self](#local) [remote](https://example.test/a) [mail](mailto:a@example.test)",
    )
    .unwrap();
    assert_eq!(
        targets,
        [
            "space name.md#part",
            "page.md#local",
            "https://example.test/a",
            "mailto:a@example.test"
        ]
    );

    assert!(documents::links(&root, &source, "[bad](missing.md)").is_err());
    assert!(documents::links(&root, &source, "[bad](bad%Q0.md)").is_err());
    assert!(documents::links(&root, &source, "[bad](bad%FF.md)").is_err());
    assert!(documents::links(&root, &source, "[outside](../outside.md)").is_err());
}

#[test]
fn law_checks_require_unique_complete_language_neutral_evidence() {
    let repository = root();
    let current =
        repository_checks::read(&repository.join("docs/architecture/kernel-laws.md")).unwrap();
    let ids = documents::check_laws(&current).unwrap();
    assert!(!ids.is_empty());

    assert!(documents::check_laws("").is_err());
    let mut duplicate = current.clone();
    let heading = duplicate
        .lines()
        .find(|line| line.starts_with("## LAW-"))
        .unwrap()
        .to_owned();
    duplicate.push_str("\n");
    duplicate.push_str(&heading);
    assert!(documents::check_laws(&duplicate).is_err());

    let missing_field = "## LAW-001 — Example\n**Rule:** A rule.\n";
    assert!(documents::check_laws(missing_field).is_err());
    let language_specific = "## LAW-001 — Example\n**Rule:** A rule.\n**Rationale:** A reason.\n**Historical evidence:** research/cases/KOL-PATH-0001.json\n**Prevents:** A failure.\n**Allowed exceptions:** None.\n**Enforcement:** Structural check.\n**Testing:** Cargo test.\n";
    assert!(documents::check_laws(language_specific).is_err());
}

#[test]
fn research_indexes_render_both_locales_and_retain_check_mode() {
    let repository = root();
    let cases = database::validate(
        &repository,
        &repository.join("research/cases"),
        repository_checks::MINIMUM_RESEARCH_CASES,
    )
    .unwrap();
    let catalog: Value =
        read_json(&repository.join("research/sources/case-index-text.json")).unwrap();
    let english = report::render(&repository, &cases, &catalog, "en").unwrap();
    let russian = report::render(&repository, &cases, &catalog, "ru").unwrap();
    assert!(english.starts_with("# Pathology cases"));
    assert!(russian.starts_with("# Случаи исследования"));
    assert!(report::render(&repository, &cases, &catalog, "fr").is_err());
    assert!(report::render(&repository, &cases, &json!({}), "en").is_err());

    report::report(&repository, true).unwrap();
}

#[test]
fn repository_exception_registry_matches_source_boundaries() {
    exceptions::check(&root()).unwrap();
}

#[test]
fn security_research_scenarios_reject_wrong_scope_missing_fields_and_unbounded_sets() {
    let repository = root();
    let mut scenarios =
        read_json(&repository.join("research/fixtures/security-boundaries.json")).unwrap();
    assert!(security_artifacts::scenarios(&scenarios).is_ok());

    scenarios["artifact_kind"] = json!("execution-evidence");
    assert!(security_artifacts::scenarios(&scenarios).is_err());
    scenarios["artifact_kind"] = json!("future-scenario-specifications");
    scenarios["execution_status"] = json!("executed");
    assert!(security_artifacts::scenarios(&scenarios).is_err());
    scenarios["execution_status"] = json!("not-executed");

    scenarios["scenarios"] = json!([]);
    assert!(security_artifacts::scenarios(&scenarios).is_err());
    scenarios["scenarios"] = json!(vec![
        json!({"id":"x","trigger":"t","expected":"e","execution_status":"not-executed"});
        129
    ]);
    assert!(security_artifacts::scenarios(&scenarios).is_err());
    scenarios["scenarios"] =
        json!([{"id":" ","trigger":"t","expected":"e","execution_status":"not-executed"}]);
    assert!(security_artifacts::scenarios(&scenarios).is_err());
    scenarios["scenarios"] =
        json!([{"id":"x","trigger":"t","expected":"e","execution_status":"passed"}]);
    assert!(security_artifacts::scenarios(&scenarios).is_err());
}

#[test]
fn security_artifact_check_uses_schema_and_keeps_examples_proposed() {
    let repository = root();
    assert!(security_artifacts::check(&repository).is_ok());

    let temp = Temp::new();
    for relative in [
        "schemas/security-domain.schema.json",
        "research/fixtures/security-domain-review.json",
        "research/fixtures/security-boundaries.json",
    ] {
        let source = repository.join(relative);
        let target = temp.0.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(source, target).unwrap();
    }
    assert!(security_artifacts::check(&temp.0).is_ok());

    let review = temp.0.join("research/fixtures/security-domain-review.json");
    let mut value: Value = read_json(&review).unwrap();
    value["evidence_state"] = json!("accepted");
    fs::write(review, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(security_artifacts::check(&temp.0).is_err());
}

#[test]
fn translation_gate_matches_hashes_structure_links_laws_and_adr_metadata() {
    let temp = Temp::new();
    let english_adr = "# Decision\n\nDocument status: HISTORICAL\nEvidence scope: design record\nCurrent reference: [topic](../topic.md)\nSupersedes: none\n";
    let russian_adr = "# Решение\n\nDocument status: HISTORICAL\nEvidence scope: проектная запись\nCurrent reference: [тема](../../../../docs/topic.md)\nSupersedes: none\n";
    translation_manifest(
        &temp.0,
        &[
            (
                "docs/architecture-decisions/0001-sample.md",
                english_adr,
                russian_adr,
            ),
            (
                "docs/topic.md",
                "# Topic\n\nSee [law](law.md).\n",
                "# Тема\n\nСм. [правило](law.md).\n",
            ),
            (
                "docs/law.md",
                "## LAW-001 — Rule\n",
                "## LAW-001 — Правило\n",
            ),
        ],
    );
    assert!(documents::check_translations(&temp.0).is_ok());

    fs::write(
        temp.0
            .join("translations/ru/docs/architecture-decisions/0001-sample.md"),
        russian_adr.replace("HISTORICAL", "CURRENT"),
    )
    .unwrap();
    assert!(documents::check_translations(&temp.0).is_err());
}

#[test]
fn translation_recording_validates_scope_and_records_exact_normalized_hashes() {
    let temp = Temp::new();
    let source = temp.write("docs/new.md", "# Source\r\n");
    let translated = temp.write("translations/ru/docs/new.md", "# Текст\n");
    temp.write(
        "translations/manifest.json",
        r#"{"source_language":"en","locales":{"ru":{}}}"#,
    );
    assert!(documents::record_translation(&temp.0, "x", "docs/new.md").is_err());
    assert!(documents::record_translation(&temp.0, "ru", "outside.md").is_err());
    documents::record_translation(&temp.0, "ru", "docs/new.md").unwrap();

    let manifest = read_json(&temp.0.join("translations/manifest.json")).unwrap();
    assert_eq!(
        manifest["locales"]["ru"]["docs/new.md"]["source_sha256"],
        repository_checks::hash(&fs::read_to_string(source).unwrap())
    );
    assert_eq!(
        manifest["locales"]["ru"]["docs/new.md"]["translation_sha256"],
        repository_checks::hash(&fs::read_to_string(translated).unwrap())
    );
}
