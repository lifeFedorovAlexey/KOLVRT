fn main() {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-arg=-T{root}/linker.ld");
    println!("cargo:rerun-if-changed=linker.ld");
}
