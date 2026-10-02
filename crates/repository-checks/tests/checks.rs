use repository_checks::{
    database, documents, hash, parse_json, read, read_json, report, write_json,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}
fn valid() -> Value {
    read_json(&root().join("research/cases/KOL-PATH-0001.json")).unwrap()
}
fn validator() -> &'static jsonschema::Validator {
    static SCHEMA: OnceLock<jsonschema::Validator> = OnceLock::new();
    SCHEMA.get_or_init(|| database::schema(&root()).unwrap())
}
fn rejects(case: &Value) {
    assert!(
        !database::case_errors(validator(), case, "KOL-PATH-0001.json").is_empty(),
        "accepted malformed case: {case}"
    );
}

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let parent = std::env::temp_dir().join(format!(
            "kolvrt-checks-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&parent).unwrap();
        Self(parent)
    }
    fn put(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn case(&self, case: &Value, name: &str) {
        write_json(&self.0.join(name), case).unwrap();
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn repository_data_and_documents_pass() {
    let root = root();
    database::validate(&root, &root.join("research/cases"), 30).unwrap();
    documents::check_docs(&root).unwrap();
    documents::check_translations(&root).unwrap();
    report::report(&root, true).unwrap();
}
#[test]
fn valid_record_passes() {
    assert!(database::case_errors(validator(), &valid(), "KOL-PATH-0001.json").is_empty());
}
#[test]
fn every_required_field_is_enforced() {
    let schema = read_json(&root().join("schemas/research-case.schema.json")).unwrap();
    for field in schema["required"].as_array().unwrap() {
        let mut case = valid();
        case.as_object_mut()
            .unwrap()
            .remove(field.as_str().unwrap());
        rejects(&case);
    }
}
#[test]
fn empty_whitespace_and_wrong_types_fail() {
    for (field, values) in [
        (
            "kolvrt_native_semantics",
            vec![json!(""), json!("   "), json!(17), Value::Null],
        ),
        ("sources", vec![json!([]), json!({}), Value::Null]),
        (
            "tests_required",
            vec![json!([]), json!([""]), json!(["   "]), json!("test")],
        ),
        (
            "benchmarks_required",
            vec![json!([]), json!([""]), json!(["   "]), json!(false)],
        ),
    ] {
        for value in values {
            let mut case = valid();
            case[field] = value;
            rejects(&case);
        }
    }
}
#[test]
fn unknown_properties_and_enums_fail() {
    for (field, value) in [
        ("kolvrt_decision", json!("LOOKS_GOOD")),
        ("category", json!(["BUG"])),
        ("compatibility_required", json!(true)),
        ("confidence", json!(0.9)),
        ("typo", json!("value")),
    ] {
        let mut case = valid();
        case[field] = value;
        rejects(&case);
    }
}
#[test]
fn duplicate_ids_and_filenames_fail() {
    let temp = Temp::new();
    temp.case(&valid(), "KOL-PATH-0001.json");
    temp.case(&valid(), "KOL-PATH-0002.json");
    let error = database::validate(&root(), &temp.0, 1).unwrap_err();
    assert!(error.contains("duplicate ID") && error.contains("filename"));
}
#[test]
fn duplicate_keys_fail_at_every_depth() {
    for text in [
        r#"{"id":1,"id":2}"#,
        r#"{"nested":{"x":true,"x":false}}"#,
        r#"[{"x":1,"x":2}]"#,
    ] {
        assert!(parse_json(text).unwrap_err().contains("duplicate JSON key"));
    }
    assert_eq!(
        parse_json(r#"{"left":{"x":1},"right":{"x":2}}"#).unwrap()["right"]["x"],
        2
    );
}
#[test]
fn malformed_and_nonfinite_json_fail() {
    for text in [
        "{broken",
        r#"{"value":NaN}"#,
        r#"{"value":Infinity}"#,
        "1e9999",
        "{} trailing",
    ] {
        assert!(parse_json(text).is_err());
    }
}
#[test]
fn evidence_references_must_resolve() {
    let mut case = valid();
    case["evidence"]["root_cause"] = json!(["S99999"]);
    rejects(&case);
}
#[test]
fn duplicate_sources_fail() {
    let mut case = valid();
    let duplicate = case["sources"][0].clone();
    case["sources"].as_array_mut().unwrap().push(duplicate);
    rejects(&case);
}
#[test]
fn primary_source_is_required() {
    let mut case = valid();
    case["sources"][0]["kind"] = json!("SECONDARY_ANALYSIS");
    rejects(&case);
}
#[test]
fn formats_and_chronology_are_enforced() {
    for (field, value) in [
        ("url", "file:///tmp/paper"),
        ("url", "https:///"),
        ("date", "2026-02-30"),
        ("date", "2099-01-01"),
        ("commit", "not-a-commit"),
        ("research_date", "2026-09-30"),
    ] {
        let mut case = valid();
        case["sources"][0][field] = json!(value);
        rejects(&case);
    }
}
#[test]
fn compatibility_must_match_decision_and_scope() {
    for (decision, compat, scope) in [
        ("COMPAT_ONLY", "NO", "NONE"),
        ("NATIVE_FIX", "NO", "module"),
        ("COMPAT_ONLY", "YES", "NONE"),
        ("NATIVE_FIX", "CONDITIONAL", "NONE"),
    ] {
        let mut case = valid();
        case["kolvrt_decision"] = json!(decision);
        case["compatibility_required"] = json!(compat);
        case["compatibility_scope"] = json!(scope);
        rejects(&case);
    }
}
#[test]
fn unresolved_history_and_decisions_need_questions() {
    for (field, value) in [
        ("first_known_version", Value::Null),
        ("kolvrt_decision", json!("RESEARCH_REQUIRED")),
        ("status", json!("UNRESOLVED")),
    ] {
        let mut case = valid();
        case["first_known_version"] = json!("Known fixture version");
        case["open_questions"] = json!([]);
        case[field] = value;
        rejects(&case);
    }
}
#[test]
fn unknown_provenance_needs_explanation() {
    let mut case = valid();
    case["sources"][0]["provenance_limitations"] = json!("");
    rejects(&case);
}
#[test]
fn empty_missing_and_underfilled_databases_fail() {
    let temp = Temp::new();
    assert!(database::validate(&root(), &temp.0, 1).is_err());
    assert!(database::validate(&root(), &temp.0.join("absent"), 1).is_err());
    temp.case(&valid(), "KOL-PATH-0001.json");
    assert!(database::validate(&root(), &temp.0, 30).is_err());
    assert!(database::validate(&root(), &temp.0, 0).is_err());
}
#[test]
fn command_failure_is_nonzero() {
    let temp = Temp::new();
    let output = Command::new(env!("CARGO_BIN_EXE_repository-checks"))
        .current_dir(root())
        .args(["validate", "--directory"])
        .arg(&temp.0)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("found 0"));
}

fn law(id: usize) -> String {
    let mut text = format!("## LAW-{id:03} — Independent obligation\n\n");
    for field in [
        "Rule",
        "Rationale",
        "Historical evidence",
        "Prevents",
        "Allowed exceptions",
        "Enforcement",
        "Testing",
    ] {
        text.push_str(&format!("**{field}:** A complete obligation.\n\n"));
    }
    text.push_str("[Evidence](../../research/cases/KOL-PATH-0001.json)\n\n");
    text
}
#[test]
fn law_sets_have_no_numeric_quota() {
    for size in [1, 17, 40, 51, 75] {
        let text = (1..=size).map(law).collect::<String>();
        assert_eq!(documents::check_laws(&text).unwrap().len(), size);
    }
    assert!(documents::check_laws("").is_err());
    assert!(documents::check_laws(&(law(1) + &law(1))).is_err());
    assert!(documents::check_laws(&law(1).replace("**Testing:**", "**Missing:**")).is_err());
    assert!(documents::check_laws(&(law(1) + "Requires Rust crates.")).is_err());
}

fn translated_fixture() -> Temp {
    let temp = Temp::new();
    temp.put(
        "README.md",
        "# Project\n\nEnglish prose.\n\n[Russian](translations/ru/README.md)\n",
    );
    temp.put(
        "translations/ru/README.md",
        "# Проект\n\nРусский текст.\n\n[Оригинал](../../README.md)\n",
    );
    temp.put(
        "translations/manifest.json",
        r#"{"source_language":"en","locales":{"ru":{}}}"#,
    );
    documents::record_translation(&temp.0, "ru", "README.md").unwrap();
    documents::check_translations(&temp.0).unwrap();
    temp
}
#[test]
fn changed_original_or_translation_requires_review() {
    for rel in ["README.md", "translations/ru/README.md"] {
        let temp = translated_fixture();
        let path = temp.0.join(rel);
        let before = read(&temp.0.join("translations/manifest.json")).unwrap();
        fs::write(&path, read(&path).unwrap() + "\nAdditional text.\n").unwrap();
        assert!(
            documents::check_translations(&temp.0)
                .unwrap_err()
                .contains("changed since")
        );
        assert_eq!(
            read(&temp.0.join("translations/manifest.json")).unwrap(),
            before
        );
        documents::record_translation(&temp.0, "ru", "README.md").unwrap();
        documents::check_translations(&temp.0).unwrap();
    }
}
#[test]
fn missing_and_orphan_translations_fail() {
    let temp = translated_fixture();
    fs::remove_file(temp.0.join("translations/ru/README.md")).unwrap();
    assert!(
        documents::check_translations(&temp.0)
            .unwrap_err()
            .contains("missing translation")
    );
    let temp = translated_fixture();
    temp.put("translations/ru/orphan.md", "# Лишний\n");
    assert!(
        documents::check_translations(&temp.0)
            .unwrap_err()
            .contains("orphan translation")
    );
    let temp = translated_fixture();
    temp.put("new.md", "# New\n");
    assert!(
        documents::check_translations(&temp.0)
            .unwrap_err()
            .contains("missing manifest entry")
    );
}
#[test]
fn translation_structure_links_and_identifiers_are_checked() {
    for addition in [
        "\n## Лишний раздел\n",
        "\nLAW-001\n",
        "\n[Неверная ссылка](missing.md)\n",
    ] {
        let temp = translated_fixture();
        let path = temp.0.join("translations/ru/README.md");
        fs::write(&path, read(&path).unwrap() + addition).unwrap();
        documents::record_translation(&temp.0, "ru", "README.md").unwrap();
        assert!(documents::check_translations(&temp.0).is_err());
    }
}
#[test]
fn canonical_cyrillic_and_unregistered_languages_fail() {
    let temp = translated_fixture();
    temp.put("README.md", "# Кириллица\n");
    assert!(
        documents::check_translations(&temp.0)
            .unwrap_err()
            .contains("Cyrillic")
    );
    let temp = translated_fixture();
    temp.put("translations/de/README.md", "# Projekt\n");
    assert!(
        documents::check_translations(&temp.0)
            .unwrap_err()
            .contains("unregistered")
    );
}
#[test]
fn explicit_recording_rejects_traversal_and_unknown_locale() {
    let temp = translated_fixture();
    for (locale, path) in [
        ("../ru", "README.md"),
        ("de", "README.md"),
        ("ru", "../README.md"),
        ("ru", "translations/ru/README.md"),
    ] {
        assert!(documents::record_translation(&temp.0, locale, path).is_err());
    }
}
#[test]
fn hashes_normalize_line_endings() {
    assert_eq!(hash("a\r\nb\r\n"), hash("a\nb\n"));
    assert_ne!(hash("a\n"), hash("b\n"));
}
#[test]
fn changed_case_invalidates_index_translation() {
    let root = root();
    let cases = database::validate(&root, &root.join("research/cases"), 30).unwrap();
    let mut catalog = read_json(&root.join("research/sources/case-index-text.json")).unwrap();
    catalog["KOL-PATH-0001"]["source_sha256"] = json!("outdated");
    assert!(
        report::render(&root, &cases, &catalog, "en")
            .unwrap_err()
            .contains("review bilingual")
    );
}
