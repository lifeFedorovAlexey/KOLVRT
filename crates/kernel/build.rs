fn main() {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-arg=-T{root}/linker.ld");
    println!("cargo:rerun-if-changed=linker.ld");
    for name in [
        "KOLVRT_NATIVE_ROOT_ELF",
        "KOLVRT_NATIVE_SERVICE_ELF",
        "KOLVRT_NATIVE_CLIENT_ELF",
        "KOLVRT_NATIVE_ARGUMENT",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
        if let Ok(value) = std::env::var(name) {
            if name.ends_with("_ELF") {
                println!("cargo:rerun-if-changed={value}");
            }
            println!("cargo:rustc-env={name}={value}");
        }
    }
    println!("cargo:rerun-if-env-changed=KOLVRT_BOOT_PAYLOAD");
    if let Ok(path) = std::env::var("KOLVRT_BOOT_PAYLOAD") {
        println!("cargo:rerun-if-changed={path}");
        println!("cargo:rustc-env=KOLVRT_BOOT_PAYLOAD={path}");
    }
}
