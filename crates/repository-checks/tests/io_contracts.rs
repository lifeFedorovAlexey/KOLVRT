use repository_checks::{finish, hash, read, read_json, write_json};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Temp(PathBuf);

impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "kolvrt-repository-io-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn repository_io_normalizes_reads_roundtrips_json_and_reports_path_errors() {
    let temp = Temp::new();
    let text_path = temp.0.join("text.md");
    fs::write(&text_path, "one\r\ntwo\r\n").unwrap();
    assert_eq!(read(&text_path).unwrap(), "one\ntwo\n");
    assert!(read(&temp.0.join("absent.md")).is_err());

    let json_path = temp.0.join("record.json");
    let expected = json!({"array": [true, false, null, -2, 3, 1.5], "text": "value"});
    write_json(&json_path, &expected).unwrap();
    assert_eq!(read_json(&json_path).unwrap(), expected);

    let invalid_json = temp.0.join("invalid.json");
    fs::write(&invalid_json, "{broken").unwrap();
    assert!(
        read_json(&invalid_json)
            .unwrap_err()
            .contains("invalid.json")
    );
    assert!(read_json(&temp.0.join("missing.json")).is_err());
    assert!(write_json(&temp.0.join("missing/record.json"), &expected).is_err());

    assert_eq!(hash("one\r\ntwo\r\n"), hash("one\ntwo\n"));
    assert!(finish(Vec::new()).is_ok());
    assert_eq!(
        finish(vec!["first".into(), "second".into()]).unwrap_err(),
        "first\nsecond"
    );
}
