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
        subject_digest: subject.clone(),
        key_id: "test".into(),
        signature,
        issued_at_unix_seconds: None,
        session_id: None,
    };
    let policy = TrustPolicy {
        schema: 1,
        keys: vec![TrustedKey {
            key_id: "test".into(),
            public_key: public,
            roles: vec![Role::ContractTest, Role::Deployment],
            revoked: false,
        }],
        required_session_id: None,
        max_attestation_age_seconds: None,
        max_clock_skew_seconds: None,
    };
    let store = SignedArtifactStore::new(policy.clone(), root.clone()).unwrap();
    let attestations = vec![attestation];
    let refs = vec![digest.clone()];
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &refs, &attestations)
            .is_ok()
    );
    let deployment_signature = key.sign(&signing_message(Role::Deployment, &subject));
    let deployment_attestation = Attestation {
        role: Role::Deployment,
        subject_digest: subject.clone(),
        key_id: "test".into(),
        signature: hex(&deployment_signature.to_bytes()),
        issued_at_unix_seconds: None,
        session_id: None,
    };
    assert!(store
        .authenticate(
            Role::Deployment,
            payload,
            &refs,
            &[deployment_attestation.clone()]
        )
        .is_ok());
    assert!(store
        .authenticate(
            Role::ContractTest,
            payload,
            &refs,
            &[deployment_attestation]
        )
        .is_err());
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

#[test]
fn session_attestations_require_fresh_timestamp_and_external_challenge() {
    let root = std::env::temp_dir().join(format!("kolvrt-freshness-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let key = SigningKey::from_bytes(&[47; 32]);
    let payload = br#"{"passed":true}"#;
    let digest = subject_digest(payload);
    let session = "a".repeat(64);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let make = |issued_at, session_id: &str| {
        let signature = key.sign(&session_signing_message(
            Role::ContractTest,
            &digest,
            issued_at,
            session_id,
        ));
        Attestation {
            role: Role::ContractTest,
            subject_digest: digest.clone(),
            key_id: "fresh-test".into(),
            signature: hex(&signature.to_bytes()),
            issued_at_unix_seconds: Some(issued_at),
            session_id: Some(session_id.into()),
        }
    };
    let policy = TrustPolicy {
        schema: 1,
        keys: vec![TrustedKey {
            key_id: "fresh-test".into(),
            public_key: hex(&key.verifying_key().to_bytes()),
            roles: vec![Role::ContractTest],
            revoked: false,
        }],
        required_session_id: Some(session.clone()),
        max_attestation_age_seconds: Some(60),
        max_clock_skew_seconds: Some(5),
    };
    let store = SignedArtifactStore::new(policy, root.clone()).unwrap();
    let valid = make(now, &session);
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &[], &[valid])
            .is_ok()
    );
    let stale = make(now.saturating_sub(120), &session);
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &[], &[stale])
            .is_err()
    );
    let wrong_session = make(now, &"b".repeat(64));
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &[], &[wrong_session])
            .is_err()
    );
    let future = make(now.saturating_add(10), &session);
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &[], &[future])
            .is_err()
    );
    std::fs::remove_dir(root).unwrap();
}
