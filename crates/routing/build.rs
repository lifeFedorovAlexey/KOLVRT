use sha2::{Digest, Sha256};
const SHA256_BYTES: usize = 32;
fn main() {
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut generated = String::new();
    for (name, path) in [
        ("NATIVE_ID", "../kernel-core/src/window.rs"),
        ("ADAPTER_ID", "../window-compat/src/lib.rs"),
    ] {
        let path = root.join(path);
        println!("cargo:rerun-if-changed={}", path.display());
        let source = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
        let digest: [u8; SHA256_BYTES] = Sha256::digest(source.as_bytes()).into();
        generated.push_str(&format!(
            "pub const {name}: [u8; DIGEST_BYTES] = {digest:?};\n"
        ));
    }
    std::fs::write(
        std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("identity.rs"),
        generated,
    )
    .unwrap();
}
