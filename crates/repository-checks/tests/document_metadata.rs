use repository_checks::documents::{check_abi_publication, check_document_status, structure};

fn status(status: &str, reference: &str) -> String {
    format!(
        "Document status: {status}\nEvidence scope: focused test fixture\nCurrent reference: {reference}\n"
    )
}

#[test]
fn document_status_accepts_declared_lifecycle_values_and_local_references() {
    for value in [
        "CURRENT",
        "DESIGN BASELINE",
        "HISTORICAL",
        "HISTORICAL MILESTONE",
        "SUPERSEDED",
    ] {
        assert!(
            check_document_status(&status(
                value,
                "See [the governing record](../architecture-decisions/0001-test.md)."
            ))
            .is_ok(),
            "{value}"
        );
    }
}

#[test]
fn document_status_rejects_missing_duplicate_and_invalid_metadata() {
    assert!(check_document_status("# no metadata\n").is_err());
    assert!(check_document_status(&status("UNKNOWN", "[record](record.md)")).is_err());
    assert!(check_document_status(&status("CURRENT", "plain text only")).is_err());
    assert!(
        check_document_status(&status("CURRENT", "[remote](https://example.test/a.md)")).is_err()
    );
    assert!(check_document_status(&status("CURRENT", "[absolute](/a.md)")).is_err());
    assert!(check_document_status(&status("CURRENT", "[anchor](#section)")).is_err());
    assert!(check_document_status(&status("CURRENT", "[local artifact](record.txt)")).is_ok());

    let missing_scope = status("CURRENT", "[record](record.md)")
        .replace("Evidence scope: focused test fixture\n", "");
    assert!(check_document_status(&missing_scope).is_err());
    let duplicate = format!(
        "{}Document status: CURRENT\n",
        status("CURRENT", "[record](record.md)")
    );
    assert!(check_document_status(&duplicate).is_err());
}

#[test]
fn abi_publication_requires_a_matching_accepted_freeze_decision() {
    let candidate = "ABI contract: request/0\nPublication stage: CANDIDATE\nABI-FREEZE: none\n";
    assert!(check_abi_publication(candidate, |_| panic!("no decision lookup for none")).is_ok());
    let experimental =
        "ABI contract: request/0\nPublication stage: EXPERIMENTAL\nABI-FREEZE: none\n";
    assert!(check_abi_publication(experimental, |_| panic!("no decision lookup for none")).is_ok());

    let freeze = "ABI contract: request/0\nPublication stage: PUBLIC\nABI-FREEZE: [decision](../architecture-decisions/0001-test.md)\n";
    assert!(
        check_abi_publication(freeze, |_| Ok(
            "Status: **Accepted**.\nDecision kind: ABI-FREEZE\nABI contract: request/0\n".into()
        ))
        .is_ok()
    );
    assert!(check_abi_publication(freeze, |_| Err("missing decision".into())).is_err());
    assert!(
        check_abi_publication(freeze, |_| Ok(
            "Status: **Proposed**.\nDecision kind: ABI-FREEZE\nABI contract: request/0\n".into()
        ))
        .is_err()
    );
    assert!(
        check_abi_publication(freeze, |_| Ok(
            "Status: **Accepted**.\nDecision kind: OTHER\nABI contract: request/0\n".into()
        ))
        .is_err()
    );
    assert!(
        check_abi_publication(freeze, |_| Ok(
            "Status: **Accepted**.\nDecision kind: ABI-FREEZE\nABI contract: request/1\n".into()
        ))
        .is_err()
    );

    assert!(
        check_abi_publication(
            "ABI contract: x\nPublication stage: UNKNOWN\nABI-FREEZE: none\n",
            |_| Ok(String::new())
        )
        .is_err()
    );
    assert!(
        check_abi_publication(
            "ABI contract: x\nPublication stage: STABLE\nABI-FREEZE: none\n",
            |_| Ok(String::new())
        )
        .is_err()
    );
    assert!(
        check_abi_publication(
            "Publication stage: EXPERIMENTAL\nABI-FREEZE: none\n",
            |_| Ok(String::new())
        )
        .is_err()
    );
    assert!(
        check_abi_publication(
            "ABI contract: x\nPublication stage: EXPERIMENTAL\n",
            |_| Ok(String::new())
        )
        .is_err()
    );
    assert!(check_abi_publication("ABI contract: x\nPublication stage: EXPERIMENTAL\nABI-FREEZE: none\nABI-FREEZE: none\n", |_| Ok(String::new())).is_err());
    assert!(check_abi_publication("ABI contract: x\nPublication stage: PUBLIC\nABI-FREEZE: https://example.test/freeze.md\n", |_| Ok(String::new())).is_err());
}

#[test]
fn markdown_structure_ignores_markup_inside_fences() {
    assert_eq!(
        structure(
            "# Heading\n- item\n1. ordered\n|a|b|\n```md\n## ignored\n- ignored\n|x|\n```\n### Subheading\n"
        ),
        ["h1", "list", "list", "table:3", "fence", "fence", "h3"]
    );
    assert!(structure("#not-heading\n####### invalid\nordinary\n").is_empty());
}

#[test]
fn legacy_metadata_fields_are_not_accepted_as_aliases() {
    let legacy = "Document status: CURRENT\nDocument scope: bounded\nStatus reference: [record](record.md)\n";
    assert!(check_document_status(legacy).is_err());
    let current = status("CURRENT", "[record](record.md)");
    assert!(check_document_status(&current).is_ok());
    for obsolete in [
        "Document scope: bounded\n",
        "Status reference: [record](record.md)\n",
    ] {
        assert!(check_document_status(&format!("{current}{obsolete}")).is_err());
    }
}
