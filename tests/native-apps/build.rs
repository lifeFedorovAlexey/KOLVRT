fn main() {
    let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let linker = manifest
        .join("../../apps/native-apps/linker.ld")
        .canonicalize()
        .unwrap();
    println!("cargo:rerun-if-changed={}", linker.display());
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("aarch64") {
        println!("cargo:rustc-link-arg=-T{}", linker.display());
        println!("cargo:rustc-link-arg=-zmax-page-size=4096");
    }
}
