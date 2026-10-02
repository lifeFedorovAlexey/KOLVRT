use repository_checks::documents;
use std::path::Path;

#[test]
fn current_document_tree_passes_the_documentation_gate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let source = root.join("docs/kernel/testing.md");
    let translation = root.join("translations/ru/docs/kernel/testing.md");
    let source_text = std::fs::read_to_string(&source).unwrap();
    let translation_text = std::fs::read_to_string(&translation).unwrap();
    assert_eq!(
        documents::links(root, &source, &source_text).unwrap(),
        documents::links(root, &translation, &translation_text).unwrap(),
        "kernel testing translation must retain the same link targets",
    );

    documents::check_docs(root).unwrap();
}
