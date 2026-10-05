use repository_checks::{
    knowledge::{self, Knowledge},
    parse_json,
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn verified_requires_current_sources_and_preserves_historical_receipt_integrity() {
    use repository_checks::hash;
    let f = Fixture::new();
    fs::write(f.0.join("source.rs"), "original\r\n").unwrap();
    let record = json!({"source_files":[{"path":"source.rs","sha256_lf":hash("original\n")}],"correctness":{"matrix":"passed"},"profiles":{"dev":{"run":{"qemu_version":"fixture"}}}});
    fs::write(f.0.join("receipt.json"), record.to_string()).unwrap();
    let mut feature = json!({"implementation":"EXPERIMENTAL","sources":["source.rs"],"acceptance":[],"transitions":[{"from":"UNRECORDED","to":"EXPERIMENTAL"}],"verification":[{"environment":"qemu-arm64","state":"VERIFIED","reason":"fixture","scope":"fixture only","receipt":"receipt.json","receipt_sha256":hash(&record.to_string())}],"readiness":"NOT_READY"});
    knowledge::validate_feature(&f.0, &feature).unwrap();
    fs::write(f.0.join("source.rs"), "changed\n").unwrap();
    assert!(
        knowledge::validate_feature(&f.0, &feature)
            .unwrap_err()
            .contains("record STALE")
    );
    feature["verification"][0]["state"] = json!("STALE");
    knowledge::validate_feature(&f.0, &feature).unwrap();
    fs::write(f.0.join("receipt.json"), "{}").unwrap();
    assert!(
        knowledge::validate_feature(&f.0, &feature)
            .unwrap_err()
            .contains("receipt digest mismatch")
    );
    fs::write(f.0.join("receipt.json"), record.to_string()).unwrap();
    feature["verification"][0]["state"] = json!("VERIFIED");
    feature["sources"] = json!(["receipt.json"]);
    assert!(knowledge::validate_feature(&f.0, &feature).is_err());
    let mut duplicate = record.clone();
    duplicate["source_files"]
        .as_array_mut()
        .unwrap()
        .push(record["source_files"][0].clone());
    fs::write(f.0.join("receipt.json"), duplicate.to_string()).unwrap();
    feature["verification"][0]["receipt_sha256"] = json!(hash(&duplicate.to_string()));
    assert!(
        knowledge::validate_feature(&f.0, &feature)
            .unwrap_err()
            .contains("duplicate")
    );
}

#[test]
fn unicode_queries_match_localized_navigation_and_neutral_ids() {
    let f = Fixture::new();
    f.doc("doc.test", "test-section", json!({}));
    fs::create_dir_all(f.0.join("translations/ru/docs")).unwrap();
    let text = fs::read_to_string(f.0.join("docs/test.md"))
        .unwrap()
        .replace("Useful section", "Отзыв полномочий");
    fs::write(f.0.join("translations/ru/docs/test.md"), text).unwrap();
    fs::write(
        f.0.join("translations/manifest.json"),
        "{\"locales\":{\"ru\":{}}}",
    )
    .unwrap();
    let k = Knowledge::build(&f.0).unwrap();
    let result = k
        .context(&f.0, "ПРОВЕРЬ ОТЗЫВ ПОЛНОМОЧИЙ", 65536, "ru")
        .unwrap();
    assert!(
        result["selected"]
            .as_array()
            .unwrap()
            .contains(&json!("kolvrt.test.section"))
    );
    assert!(
        result["context"]
            .as_str()
            .unwrap()
            .contains("Отзыв полномочий")
    );
    assert!(
        !k.context(&f.0, "kolvrt.test.section", 65536, "ru").unwrap()["selected"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(k.context(&f.0, "отзыв", 65536, "xx").is_err());
    knowledge::cli(
        &f.0,
        &[
            "context".into(),
            "отзыв полномочий".into(),
            "--locale".into(),
            "ru".into(),
            "--budget-bytes".into(),
            "65536".into(),
        ],
    )
    .unwrap();
    assert!(knowledge::cli(&f.0, &["context".into(), "отзыв".into(), "--locale".into()]).is_err());
}

#[test]
fn implementation_declarations_cover_new_files_and_explicit_review_dispositions() {
    use repository_checks::hash;
    use std::process::Command;
    let f = Fixture::new();
    fs::create_dir_all(f.0.join("crates/example/src")).unwrap();
    let path = "crates/example/src/lib.rs";
    fs::write(f.0.join(path), "before\n").unwrap();
    let feature = json!({"implementation":"EXPERIMENTAL","implementation_scope":"Fixture implementation","sources":[path],"acceptance":[],"issues":[],"adrs":[],"limitations":["fixture only"],"next_gate":"acceptance","verification":[{"environment":"host","state":"UNKNOWN","reason":"fixture only"}],"readiness":"NOT_READY","transitions":[{"from":"UNRECORDED","to":"EXPERIMENTAL","reason":"initial fixture","acceptance":[]}]});
    f.doc("doc.test","test-section",json!({"units":[{"id":"kolvrt.test.section","kind":"feature","anchor":"test-section","summary":"Fixture","feature":feature}]}));
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .current_dir(&f.0)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "--quiet"]);
    git(&["add", "."]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.invalid",
        "commit",
        "--quiet",
        "-m",
        "Fixture",
    ]);
    let base = git(&["rev-parse", "HEAD"]);
    fs::write(f.0.join(path), "after\n").unwrap();
    assert!(
        knowledge::check_change(&f.0, "HEAD")
            .unwrap_err()
            .contains("require docs/implementation-impact")
    );
    let mut declaration = json!({"schema_version":1,"base_commit":base,"files":[{"path":path,"before_sha256_lf":hash("before\n"),"after_sha256_lf":hash("after\n"),"disposition":"existing-feature","reason":"Refactor unrelated helper; fixture feature behavior is unchanged."}],"features":[{"id":"kolvrt.test.section","disposition":"no-impact","reason":"Reviewed helper-only change; no fixture contract or evidence claim changes."}]});
    let write = |d: &serde_json::Value| {
        fs::write(f.0.join("docs/implementation-impact.json"), d.to_string()).unwrap()
    };
    write(&declaration);
    knowledge::check_change(&f.0, "HEAD").unwrap();
    declaration["features"][0]["disposition"] = json!("semantic-change");
    write(&declaration);
    let doc = fs::read_to_string(f.0.join("docs/test.md")).unwrap();
    fs::write(f.0.join("docs/test.md"), format!("{doc}\n\n")).unwrap();
    assert!(
        knowledge::check_change(&f.0, "HEAD")
            .unwrap_err()
            .contains("substantive")
    );
    declaration["features"][0]["disposition"] = json!("evidence-change");
    write(&declaration);
    assert!(
        knowledge::check_change(&f.0, "HEAD")
            .unwrap_err()
            .contains("changed evidence")
    );
    declaration["features"][0]["disposition"] = json!("no-impact");
    write(&declaration);
    fs::write(f.0.join(path), "later\n").unwrap();
    assert!(
        knowledge::check_change(&f.0, "HEAD")
            .unwrap_err()
            .contains("source digest mismatch")
    );
    fs::write(f.0.join(path), "after\n").unwrap();
    fs::write(f.0.join("crates/example/src/new.rs"), "new mechanism\n").unwrap();
    assert!(
        knowledge::check_change(&f.0, "HEAD")
            .unwrap_err()
            .contains("every changed")
    );
    declaration["files"].as_array_mut().unwrap().push(json!({"path":"crates/example/src/new.rs","before_sha256_lf":null,"after_sha256_lf":hash("new mechanism\n"),"disposition":"new-feature","reason":"This added mechanism must enroll a new canonical feature before merge."}));
    write(&declaration);
    assert!(
        knowledge::check_change(&f.0, "HEAD")
            .unwrap_err()
            .contains("newly enrolled")
    );
    declaration["files"][1]["disposition"] = json!("no-feature-impact");
    declaration["files"][1]["reason"] =
        json!("Fixture-only helper addition; reviewer must confirm no new product mechanism.");
    write(&declaration);
    knowledge::check_change(&f.0, "HEAD").unwrap();
    declaration["base_commit"] = json!("0000000000000000000000000000000000000000");
    write(&declaration);
    assert!(
        knowledge::check_change(&f.0, "HEAD")
            .unwrap_err()
            .contains("base_commit")
    );
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "kolvrt-knowledge-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::copy(
            source.join("schemas/knowledge.schema.json"),
            root.join("schemas/knowledge.schema.json"),
        )
        .unwrap();
        fs::copy(
            source.join("schemas/implementation-impact.schema.json"),
            root.join("schemas/implementation-impact.schema.json"),
        )
        .unwrap();
        fs::create_dir_all(root.join("research/cases")).unwrap();
        fs::create_dir_all(root.join("research/cost-l")).unwrap();
        fs::create_dir_all(root.join("translations")).unwrap();
        fs::write(root.join("translations/manifest.json"), "{\"locales\":{}}").unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(
            root.join("docs/knowledge-enrollment.json"),
            "{\"documents\":[\"docs/test.md\"]}",
        )
        .unwrap();
        Self(root)
    }
    fn doc(&self, id: &str, anchor: &str, extra: serde_json::Value) {
        let mut m = json!({"schema_version":1,"id":id,"kind":"subsystem-contract","summary":"A fixture","units":[{"id":"kolvrt.test.section","kind":"contract-section","summary":"Useful section","anchor":anchor}]});
        for (k, v) in extra.as_object().unwrap() {
            m[k] = v.clone();
        }
        fs::write(self.0.join("docs/test.md"),format!("# Test\n\nDocument status: CURRENT\nEvidence scope: unit fixture only\nCurrent reference: [test](test.md)\n\n<a name=\"{anchor}\"></a>\n\n## Useful section\n\nScoped content.\n\n<!-- knowledge -->\n```json\n{m}\n```\n")).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn parser_rejects_duplicate_keys_blocks_and_unclosed_metadata() {
    assert!(
        knowledge::metadata("<!-- knowledge -->\n```json\n{\"id\":1,\"id\":2}\n```\n").is_err()
    );
    assert!(
        knowledge::metadata(
            "<!-- knowledge -->\n```json\n{}\n```\n<!-- knowledge -->\n```json\n{}\n```\n"
        )
        .is_err()
    );
    assert!(knowledge::metadata("<!-- knowledge -->\n```json\n{}").is_err());
    assert!(
        knowledge::metadata("```md\n<!-- knowledge -->\n```\n")
            .unwrap()
            .is_none()
    );
    assert!(parse_json("{\"a\":1,\"a\":2}").is_err());
    assert!(
        knowledge::metadata("````md\n```\n~~~\n<!-- knowledge -->\n```json\n{}\n```\n````\n")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        knowledge::headings("````md\n```\n## Example\n~~~\n## Still example\n````\n## Real\n")
            .len(),
        1
    );
}
#[test]
fn section_subtrees_ignore_fenced_headings_and_preserve_nested_content() {
    let t = "# Doc\n\n<a name=\"test-section\"></a>\n\n## Parent\nWarning\n### Child\nDetails\n```md\n## Example\n```\n## Next\n";
    let (start, end, title) = knowledge::section(t, "test-section").unwrap();
    assert_eq!((start, end, title.as_str()), (3, 11, "Parent"));
    assert!(knowledge::section(t, "missing").is_err());
    assert!(
        knowledge::section(
            &(t.to_owned() + "<a name=\"test-section\"></a>\n"),
            "test-section"
        )
        .is_err()
    );
}
#[test]
fn enrolled_ids_alias_collisions_unknown_edges_cycles_and_superseded_authority_fail() {
    let f = Fixture::new();
    f.doc("doc.test", "test-section", json!({}));
    assert!(Knowledge::build(&f.0).is_ok());
    for invalid in [
        json!({"unknown":true}),
        json!({"id":"BAD"}),
        json!({"aliases":["kolvrt.test.section"]}),
        json!({"depends_on":["unknown.target"]}),
        json!({"depends_on":["doc.test"]}),
        json!({"units":[{"id":"doc.test","kind":"contract-section","summary":"duplicate","anchor":"test-section"}]}),
        json!({"relationships":[{"type":"arbitrary","to":"doc.test"}]}),
        json!({"relationships":[{"type":"supersedes","to":"kolvrt.test.section"}]}),
        json!({"relationships":[{"type":"cost_l","to":"kolvrt.test.section"}]}),
        json!({"relationships":[{"type":"implemented_by","to":"../outside"}]}),
    ] {
        f.doc("doc.test", "test-section", invalid.clone());
        assert!(Knowledge::build(&f.0).is_err(), "{invalid}");
    }
    f.doc("doc.test", "test-section", json!({}));
    let text = fs::read_to_string(f.0.join("docs/test.md"))
        .unwrap()
        .replace("<a name=\"test-section\"></a>", "");
    fs::write(f.0.join("docs/test.md"), text).unwrap();
    assert!(Knowledge::build(&f.0).is_err());
    fs::remove_file(f.0.join("docs/test.md")).unwrap();
    assert!(Knowledge::build(&f.0).is_err());
}
#[test]
fn movement_keeps_identity_and_extraction_refuses_stale_ranges() {
    let f = Fixture::new();
    f.doc("doc.test", "test-section", json!({"aliases":["old.test"]}));
    let k = Knowledge::build(&f.0).unwrap();
    assert!(
        k.render(&f.0, &["old.test".into()], "en")
            .unwrap()
            .contains("Scoped content.")
    );
    assert!(
        k.render(
            &f.0,
            &["doc.test".into(), "kolvrt.test.section".into()],
            "en"
        )
        .unwrap()
        .matches("Scoped content.")
        .count()
            == 1
    );
    fs::rename(f.0.join("docs/test.md"), f.0.join("docs/moved.md")).unwrap();
    let moved_text = fs::read_to_string(f.0.join("docs/moved.md"))
        .unwrap()
        .replace("(test.md)", "(moved.md)");
    fs::write(f.0.join("docs/moved.md"), moved_text).unwrap();
    fs::write(
        f.0.join("docs/knowledge-enrollment.json"),
        "{\"documents\":[\"docs/moved.md\"]}",
    )
    .unwrap();
    let moved = Knowledge::build(&f.0).unwrap();
    assert_eq!(
        moved.closure(&["old.test".into()]).unwrap(),
        vec!["doc.test"]
    );
    fs::write(f.0.join("docs/moved.md"), "changed").unwrap();
    assert!(moved.render(&f.0, &["old.test".into()], "en").is_err());
    assert!(k.render(&f.0, &["old.test".into()], "en").is_err());
}
#[test]
fn feature_transitions_sources_acceptance_and_receipt_binding_fail_closed() {
    let f = Fixture::new();
    let planned = json!({"implementation":"PLANNED","sources":[],"acceptance":[],"verification":[{"environment":"qemu","state":"UNKNOWN"}],"transitions":[{"from":"UNRECORDED","to":"PLANNED","reason":"adoption"}]});
    assert!(knowledge::validate_feature(&f.0, &planned).is_ok());
    let mut bad = planned.clone();
    bad["implementation"] = json!("IMPLEMENTED");
    assert!(knowledge::validate_feature(&f.0, &bad).is_err());
    let mut bad = planned.clone();
    bad["transitions"] =
        json!([{"from":"UNRECORDED","to":"PLANNED"},{"from":"PLANNED","to":"IMPLEMENTED"}]);
    assert!(knowledge::validate_feature(&f.0, &bad).is_err());
    let mut bad = planned.clone();
    bad["verification"] = json!([{"environment":"qemu","state":"VERIFIED","receipt":"missing.json","scope":"fixture"}]);
    assert!(knowledge::validate_feature(&f.0, &bad).is_err());
    fs::write(f.0.join("receipt.json"), "{}").unwrap();
    let mut bad = planned.clone();
    bad["verification"] = json!([{"environment":"qemu","state":"VERIFIED","receipt":"receipt.json","scope":"fixture","receipt_sha256":"wrong"}]);
    assert!(knowledge::validate_feature(&f.0, &bad).is_err());
    let mut bad = planned.clone();
    bad["readiness"] = json!("READY");
    assert!(knowledge::validate_feature(&f.0, &bad).is_err());
}
#[test]
fn actual_pilot_closure_includes_authority_and_reports_planned_gaps() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let k = Knowledge::build(&root).unwrap();
    let result = k
        .context(
            &root,
            "review Phase 3.4 capability revocation",
            131072,
            "en",
        )
        .unwrap();
    let ids = result["selected"].as_array().unwrap();
    for id in [
        "kolvrt.security.capability-revocation",
        "kolvrt.handles.identity",
        "kolvrt.handles.lifetime",
        "kolvrt.process.reclamation",
        "law.009",
        "law.013",
        "adr.0013",
        "adr.0022",
        "kolvrt.security.event-revocation",
        "kolvrt.security.domains",
        "adr.0023",
    ] {
        assert!(ids.contains(&json!(id)), "missing {id}");
    }
    assert!(!result["missing"].as_array().unwrap().is_empty());
    assert_eq!(
        k.graph["nodes"]["kolvrt.security.capability-revocation"]["feature"]["implementation"],
        "BOUNDED_IMPLEMENTED"
    );
    assert_eq!(result["budget_exceeded"], false);
    for input in [
        "kolvrt.handles.identity",
        "crates/kernel-core/src/handles.rs",
        "docs/kernel/handles.md",
    ] {
        let impact = k.impact(input).unwrap();
        assert!(
            impact["affected"]
                .as_array()
                .unwrap()
                .contains(&json!("kolvrt.security.capability-revocation"))
        );
        assert!(
            impact["affected"]
                .as_array()
                .unwrap()
                .contains(&json!("kolvrt.security.domains"))
        );
    }
    assert!(k.impact("unknown/not-declared.rs").is_err());
    assert_eq!(
        k.graph["nodes"]["kolvrt.security.event-revocation"]["feature"]["implementation"],
        "BOUNDED_IMPLEMENTED"
    );
    assert_eq!(
        k.context(&root, "review Phase 3.4 capability revocation", 1, "en")
            .unwrap()["budget_exceeded"],
        true
    );
    let ru = k
        .render(&root, &["kolvrt.handles.identity".into()], "ru")
        .unwrap();
    assert!(ru.chars().any(|c| ('А'..='я').contains(&c)));
}

#[test]
fn stale_outputs_and_machine_locale_drift_are_rejected() {
    let f = Fixture::new();
    f.doc("doc.test", "test-section", json!({}));
    fs::create_dir_all(f.0.join("translations/ru/docs")).unwrap();
    let text = fs::read_to_string(f.0.join("docs/test.md")).unwrap();
    fs::write(f.0.join("translations/ru/docs/test.md"), text.clone()).unwrap();
    fs::write(
        f.0.join("translations/manifest.json"),
        "{\"locales\":{\"ru\":{}}}",
    )
    .unwrap();
    for path in ["README.md", "translations/ru/README.md"] {
        fs::write(
            f.0.join(path),
            "<!-- feature-summary:start -->\n<!-- feature-summary:end -->\n",
        )
        .unwrap();
    }
    knowledge::generate(&f.0, false).unwrap();
    let russian_summary = fs::read_to_string(f.0.join("translations/ru/README.md")).unwrap();
    assert!(russian_summary.contains("Каноническая функция"));
    assert!(russian_summary.contains("Реализация"));
    assert!(russian_summary.contains("Граница доказательств"));
    knowledge::generate(&f.0, true).unwrap();
    fs::write(f.0.join("docs/catalog.json"), "{}").unwrap();
    assert!(knowledge::generate(&f.0, true).is_err());
    fs::write(
        f.0.join("translations/ru/docs/test.md"),
        text.replace("doc.test", "doc.other"),
    )
    .unwrap();
    assert!(Knowledge::build(&f.0).is_err());
}

#[test]
fn current_authority_cannot_depend_on_superseded_and_ordinary_cycles_are_legal() {
    let f = Fixture::new();
    f.doc(
        "doc.test",
        "test-section",
        json!({"depends_on":["doc.old"]}),
    );
    let old = "# Old\nDocument status: SUPERSEDED\nEvidence scope: replaced scope\nCurrent reference: [new](test.md)\n\n<!-- knowledge -->\n```json\n{\"schema_version\":1,\"id\":\"doc.old\",\"kind\":\"policy\",\"summary\":\"Old\"}\n```\n";
    fs::write(f.0.join("docs/old.md"), old).unwrap();
    assert!(Knowledge::build(&f.0).is_err());
    f.doc(
        "doc.test",
        "test-section",
        json!({"relationships":[{"type":"related_to","to":"doc.old"}]}),
    );
    fs::write(
        f.0.join("docs/old.md"),
        old.replace(
            "\"summary\":\"Old\"",
            "\"summary\":\"Old\",\"relationships\":[{\"type\":\"related_to\",\"to\":\"doc.test\"}]",
        ),
    )
    .unwrap();
    assert!(Knowledge::build(&f.0).is_ok());
}

#[test]
fn git_base_checks_preserve_feature_history_across_moves() {
    use std::process::Command;
    let f = Fixture::new();
    let feature = json!({"implementation":"PLANNED","implementation_scope":"fixture planned behavior","sources":[],"acceptance":[],"issues":[24],"adrs":[],"limitations":["not implemented"],"next_gate":"acceptance","verification":[{"environment":"qemu-arm64","state":"UNKNOWN","reason":"missing implementation"}],"readiness":"NOT_READY","transitions":[{"from":"UNRECORDED","to":"PLANNED","reason":"adopt existing intent","acceptance":[]}]});
    f.doc("doc.test","test-section",json!({"units":[{"id":"kolvrt.test.section","kind":"feature","anchor":"test-section","summary":"Test feature","feature":feature}]}));
    let git = |args: &[&str]| {
        let o = Command::new("git")
            .current_dir(&f.0)
            .args(args)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    };
    git(&["init", "--quiet"]);
    git(&["add", "."]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.invalid",
        "commit",
        "--quiet",
        "-m",
        "Fixture",
    ]);
    knowledge::check_change(&f.0, "HEAD").unwrap();
    let text = fs::read_to_string(f.0.join("docs/test.md"))
        .unwrap()
        .replace("(test.md)", "(moved.md)");
    fs::rename(f.0.join("docs/test.md"), f.0.join("docs/moved.md")).unwrap();
    fs::write(f.0.join("docs/moved.md"), &text).unwrap();
    fs::write(
        f.0.join("docs/knowledge-enrollment.json"),
        "{\"documents\":[\"docs/moved.md\"]}",
    )
    .unwrap();
    knowledge::check_change(&f.0, "HEAD").unwrap();
    fs::write(
        f.0.join("docs/moved.md"),
        text.replace("adopt existing intent", "silently rewrite history"),
    )
    .unwrap();
    assert!(knowledge::check_change(&f.0, "HEAD").is_err());
}
