use routing::{PROFILE_BYTES, Profile};
use sha2::{Digest, Sha256};
const SHA256_BYTES: usize = 32;
fn main() {
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    println!(
        "cargo:rustc-link-arg=-T{}",
        root.join("linker.ld").display()
    );
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-env-changed=KOLVRT_ROUTING_PROFILE");
    println!("cargo:rerun-if-env-changed=KOLVRT_ROUTING_DIGEST");
    let profile = if let Ok(path) = std::env::var("KOLVRT_ROUTING_PROFILE") {
        println!("cargo:rerun-if-changed={path}");
        let bytes = std::fs::read(path).expect("profile read");
        let expected = std::env::var("KOLVRT_ROUTING_DIGEST")
            .expect("trusted expected profile digest required");
        assert!(
            expected.len() == SHA256_BYTES * 2 && expected.bytes().all(|v| v.is_ascii_hexdigit())
        );
        let mut hash = [0u8; SHA256_BYTES];
        for (index, value) in hash.iter_mut().enumerate() {
            *value = u8::from_str_radix(&expected[index * 2..index * 2 + 2], 16).unwrap();
        }
        let profile: [u8; PROFILE_BYTES] = bytes.try_into().expect("profile length");
        let actual: [u8; SHA256_BYTES] =
            Sha256::digest(&profile[..PROFILE_BYTES - SHA256_BYTES]).into();
        assert_eq!(actual, hash, "trusted expected profile digest mismatch");
        assert_eq!(profile[PROFILE_BYTES - SHA256_BYTES..], hash);
        // Mandatory semantic/module/identity checks run in EL0 using exact target
        // features; host building must not require compiled compatibility adapters.
        profile
    } else {
        Profile::native().encode()
    };
    let digest = &profile[PROFILE_BYTES - SHA256_BYTES..];
    let output = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(output.join("profile.rs"), format!("const PROFILE: [u8; routing::PROFILE_BYTES] = {profile:?};\nconst EXPECTED: [u8; 32] = {digest:?};\n")).unwrap();
}
