fn main() {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-arg=-T{root}/linker.ld");
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-env-changed=KOLVRT_BOOT_PAYLOAD");
    if let Ok(path) = std::env::var("KOLVRT_BOOT_PAYLOAD") {
        println!("cargo:rerun-if-changed={path}");
        println!("cargo:rustc-env=KOLVRT_BOOT_PAYLOAD={path}");
    }
}
