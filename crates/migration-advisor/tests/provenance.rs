use ed25519_dalek::{Signer, SigningKey};
use migration_advisor::provenance::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn signatures_bind_payload_role_key_and_artifact_bytes() {
    let root = std::env::temp_dir().join(format!("kolvrt-provenance-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let raw = b"fixture package bytes";
    let digest = subject_digest(raw);
    std::fs::write(root.join(&digest), raw).unwrap();
    let key = SigningKey::from_bytes(&[42; 32]); // Public test fixture only.
    let public = hex(&key.verifying_key().to_bytes());
    let payload = br#"{"passed":true,"package":"fixture"}"#;
    let subject = subject_digest(payload);
    let signature = hex(&key
        .sign(&signing_message(Role::ContractTest, &subject))
        .to_bytes());
    let attestation = Attestation {
        role: Role::ContractTest,
        subject_digest: subject,
        key_id: "test".into(),
        signature,
        session: None,
    };
    let policy = TrustPolicy {
        schema: 1,
        keys: vec![TrustedKey {
            key_id: "test".into(),
            public_key: public,
            roles: vec![Role::ContractTest],
            revoked: false,
            compromised: false,
            producer: None,
            valid_from_unix: None,
            valid_until_unix: None,
        }],
        session: None,
        independence: vec![],
    };
    let store = SignedArtifactStore::new(policy.clone(), root.clone()).unwrap();
    let attestations = vec![attestation];
    let refs = vec![digest.clone()];
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &refs, &attestations)
            .is_ok()
    );
    assert!(
        store
            .authenticate(
                Role::ContractTest,
                b"changed passed flag",
                &refs,
                &attestations
            )
            .is_err()
    );
    assert!(
        store
            .authenticate(Role::Catalog, payload, &refs, &attestations)
            .is_err()
    );
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &refs, &[])
            .is_err()
    );
    let mut revoked = policy.clone();
    revoked.keys[0].revoked = true;
    assert!(
        SignedArtifactStore::new(revoked, root.clone())
            .unwrap()
            .authenticate(Role::ContractTest, payload, &refs, &attestations)
            .is_err()
    );
    let mut wrong_key = policy.clone();
    wrong_key.keys[0].public_key =
        hex(&SigningKey::from_bytes(&[43; 32]).verifying_key().to_bytes());
    assert!(
        SignedArtifactStore::new(wrong_key, root.clone())
            .unwrap()
            .authenticate(Role::ContractTest, payload, &refs, &attestations)
            .is_err()
    );
    let mut malformed = attestations.clone();
    malformed[0].signature = "00".repeat(64);
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &refs, &malformed)
            .is_err()
    );
    std::fs::write(root.join(&digest), b"substituted bytes").unwrap();
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &refs, &attestations)
            .is_err()
    );
    assert!(
        store
            .authenticate(
                Role::ContractTest,
                payload,
                &["../foreign".into()],
                &attestations
            )
            .is_err()
    );
    std::fs::remove_file(root.join(&digest)).unwrap();
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &refs, &attestations)
            .is_err()
    );
    std::fs::remove_dir(root).unwrap();
}
