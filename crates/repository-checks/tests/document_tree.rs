use repository_checks::documents;
use std::path::Path;

#[test]
fn current_document_tree_passes_the_documentation_gate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    documents::check_docs(root).unwrap();
}
