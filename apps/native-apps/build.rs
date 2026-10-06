fn main() {
    println!("cargo:rerun-if-changed=linker.ld");
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("aarch64") {
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        println!("cargo:rustc-link-arg=-T{root}/linker.ld");
        println!("cargo:rustc-link-arg=-zmax-page-size=4096");
    }
}
