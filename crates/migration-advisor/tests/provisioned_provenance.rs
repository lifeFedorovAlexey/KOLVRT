use ed25519_dalek::{Signer, SigningKey};
use migration_advisor::provenance::*;
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn temp(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "kolvrt-issue19-{name}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}
fn producer_key(
    id: &str,
    key: &SigningKey,
    role: Role,
    producer_id: &str,
    custody: &str,
    enforcement: &str,
) -> TrustedKey {
    TrustedKey {
        key_id: id.into(),
        public_key: hex(&key.verifying_key().to_bytes()),
        roles: vec![role],
        revoked: false,
        compromised: false,
        producer: Some(ProducerIdentity {
            producer_id: producer_id.into(),
            custody_domain: custody.into(),
            enforcement_domain: enforcement.into(),
        }),
        valid_from_unix: Some(100),
        valid_until_unix: Some(10_000),
    }
}
fn policy(issuer: &SigningKey, a: &SigningKey, b: &SigningKey) -> TrustPolicy {
    TrustPolicy {
        schema: 2,
        keys: vec![
            producer_key(
                "issuer",
                issuer,
                Role::Session,
                "challenge-authority",
                "issuer-vault",
                "issuer-service",
            ),
            producer_key(
                "producer-a",
                a,
                Role::ContractTest,
                "producer-a",
                "custody-a",
                "runner-a",
            ),
            producer_key(
                "producer-b",
                b,
                Role::ContractTest,
                "producer-b",
                "custody-b",
                "runner-b",
            ),
        ],
        session: Some(SessionPolicy {
            max_lifetime_secs: 120,
            max_future_skew_secs: 2,
            allow_offline: false,
        }),
        required_session_id: None,
        max_attestation_age_seconds: None,
        max_clock_skew_seconds: None,
        independence: vec![IndependencePolicy {
            role: Role::ContractTest,
            min_producers: 2,
            min_custody_domains: 2,
            min_enforcement_domains: 2,
        }],
    }
}
fn make_session(context: &[u8], id: u8, issued: u64, expires: u64) -> SessionBinding {
    SessionBinding {
        id: format!("{id:02x}").repeat(16),
        challenge_digest: subject_digest(format!("challenge-{id}").as_bytes()),
        context_digest: subject_digest(context),
        issued_at_unix: issued,
        expires_at_unix: expires,
    }
}
fn attestation(
    key: &SigningKey,
    key_id: &str,
    role: Role,
    payload: &[u8],
    session: Option<&SessionBinding>,
) -> Attestation {
    let digest = subject_digest(payload);
    let message = session.map_or_else(
        || signing_message(role, &digest),
        |session| signing_message_bound(role, &digest, session),
    );
    Attestation {
        role,
        subject_digest: digest,
        key_id: key_id.into(),
        signature: hex(&key.sign(&message).to_bytes()),
        session: session.cloned(),
        issued_at_unix_seconds: None,
        session_id: None,
    }
}
fn session_attestation(key: &SigningKey, session: &SessionBinding) -> Attestation {
    attestation(
        key,
        "issuer",
        Role::Session,
        &serde_json::to_vec(session).unwrap(),
        None,
    )
}
fn store(
    root: &Path,
    id: u8,
    now: u64,
) -> (SignedArtifactStore, SigningKey, SigningKey, SigningKey) {
    let issuer = SigningKey::from_bytes(&[41; 32]);
    let a = SigningKey::from_bytes(&[42; 32]);
    let b = SigningKey::from_bytes(&[43; 32]);
    let artifacts = root.join("artifacts");
    let ledger = root.join("sessions");
    std::fs::create_dir_all(&artifacts).unwrap();
    let verifier =
        SignedArtifactStore::provisioned(policy(&issuer, &a, &b), artifacts, ledger, now).unwrap();
    let _ = id;
    (verifier, issuer, a, b)
}

#[test]
fn provisioned_sessions_bind_context_enforce_roles_and_consume_replay_state() {
    let root = temp("chain");
    let context = br#"{"machine":"host-a","workload":"opaque"}"#;
    let (verifier, issuer, a, b) = store(&root, 1, 1_020);
    let session = make_session(context, 1, 1_000, 1_060);
    let payload = br#"{"passed":true,"suite":"consumer-contract"}"#;
    let raw = b"exact artifact bytes";
    let artifact = subject_digest(raw);
    std::fs::write(root.join("artifacts").join(&artifact), raw).unwrap();
    let mut attestations = vec![session_attestation(&issuer, &session)];
    attestations.push(attestation(
        &a,
        "producer-a",
        Role::ContractTest,
        payload,
        Some(&session),
    ));
    attestations.push(attestation(
        &b,
        "producer-b",
        Role::ContractTest,
        payload,
        Some(&session),
    ));

    verifier
        .begin_session(Some(&session), context, &attestations)
        .unwrap();
    verifier
        .authenticate(
            Role::ContractTest,
            payload,
            std::slice::from_ref(&artifact),
            &attestations,
        )
        .unwrap();
    let assurance = verifier.assurance();
    assert!(assurance.provisioned_policy && assurance.session_bound && assurance.replay_checked);
    assert!(
        verifier
            .authenticate(Role::Runtime, payload, &[], &attestations)
            .is_err()
    );
    assert!(
        verifier
            .begin_session(Some(&session), context, &attestations)
            .is_err(),
        "replayed session accepted"
    );

    let outsider = SigningKey::from_bytes(&[44; 32]);
    let forged = attestation(
        &outsider,
        "self-trusted",
        Role::ContractTest,
        payload,
        Some(&session),
    );
    assert!(
        verifier
            .authenticate(Role::ContractTest, payload, &[], &[forged])
            .is_err()
    );

    std::fs::write(root.join("artifacts").join(&artifact), b"substituted bytes").unwrap();
    assert!(
        verifier
            .authenticate(Role::ContractTest, payload, &[artifact], &attestations)
            .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn one_producer_cannot_satisfy_configured_independence_and_revoked_keys_fail() {
    let root = temp("independence");
    let context = b"request context";
    let (verifier, issuer, a, b) = store(&root, 2, 1_020);
    let session = make_session(context, 2, 1_000, 1_060);
    let payload = b"contract result";
    let one_producer = vec![
        session_attestation(&issuer, &session),
        attestation(
            &a,
            "producer-a",
            Role::ContractTest,
            payload,
            Some(&session),
        ),
    ];
    verifier
        .begin_session(Some(&session), context, &one_producer)
        .unwrap();
    assert!(
        verifier
            .authenticate(Role::ContractTest, payload, &[], &one_producer)
            .unwrap_err()
            .contains("independence")
    );

    let context2 = b"request context 2";
    let session2 = make_session(context2, 3, 1_000, 1_060);
    let mut revoked_policy = policy(&issuer, &a, &b);
    revoked_policy.keys[1].revoked = true;
    let verifier2 = SignedArtifactStore::provisioned(
        revoked_policy,
        root.join("artifacts"),
        root.join("sessions-revoked"),
        1_020,
    )
    .unwrap();
    let attestations = vec![
        session_attestation(&issuer, &session2),
        attestation(
            &a,
            "producer-a",
            Role::ContractTest,
            payload,
            Some(&session2),
        ),
        attestation(
            &b,
            "producer-b",
            Role::ContractTest,
            payload,
            Some(&session2),
        ),
    ];
    verifier2
        .begin_session(Some(&session2), context2, &attestations)
        .unwrap();
    assert!(
        verifier2
            .authenticate(Role::ContractTest, payload, &[], &attestations)
            .is_err()
    );
    let context3 = b"request context 3";
    let session3 = make_session(context3, 6, 1_000, 1_060);
    let mut compromised_policy = policy(&issuer, &a, &b);
    compromised_policy.keys[1].compromised = true;
    let verifier3 = SignedArtifactStore::provisioned(
        compromised_policy,
        root.join("artifacts"),
        root.join("sessions-compromised"),
        1_020,
    )
    .unwrap();
    let compromised = vec![
        session_attestation(&issuer, &session3),
        attestation(
            &a,
            "producer-a",
            Role::ContractTest,
            payload,
            Some(&session3),
        ),
        attestation(
            &b,
            "producer-b",
            Role::ContractTest,
            payload,
            Some(&session3),
        ),
    ];
    verifier3
        .begin_session(Some(&session3), context3, &compromised)
        .unwrap();
    assert!(
        verifier3
            .authenticate(Role::ContractTest, payload, &[], &compromised)
            .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn expired_keys_and_untrusted_offline_clock_fail_closed() {
    let root = temp("clock-policy");
    let issuer = SigningKey::from_bytes(&[51; 32]);
    let a = SigningKey::from_bytes(&[52; 32]);
    let b = SigningKey::from_bytes(&[53; 32]);
    std::fs::create_dir_all(root.join("artifacts")).unwrap();
    let mut expired = policy(&issuer, &a, &b);
    expired.keys[1].valid_until_unix = Some(1_010);
    let verifier = SignedArtifactStore::provisioned(
        expired,
        root.join("artifacts"),
        root.join("expired"),
        1_020,
    )
    .unwrap();
    let context = b"clock context";
    let session = make_session(context, 7, 1_000, 1_060);
    let payload = b"contract result";
    let attestations = vec![
        session_attestation(&issuer, &session),
        attestation(
            &a,
            "producer-a",
            Role::ContractTest,
            payload,
            Some(&session),
        ),
        attestation(
            &b,
            "producer-b",
            Role::ContractTest,
            payload,
            Some(&session),
        ),
    ];
    verifier
        .begin_session(Some(&session), context, &attestations)
        .unwrap();
    assert!(
        verifier
            .authenticate(Role::ContractTest, payload, &[], &attestations)
            .is_err()
    );

    let mut offline = policy(&issuer, &a, &b);
    offline.session.as_mut().unwrap().allow_offline = true;
    let result = SignedArtifactStore::provisioned(
        offline,
        root.join("artifacts"),
        root.join("offline"),
        1_020,
    );
    assert!(matches!(result, Err(error) if error.contains("offline")));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stale_context_tampered_and_incomplete_sessions_fail_closed() {
    let root = temp("freshness");
    let context = b"expected context";
    let (verifier, issuer, a, b) = store(&root, 4, 1_020);
    let stale = make_session(context, 4, 900, 1_000);
    let stale_attestations = vec![session_attestation(&issuer, &stale)];
    assert!(
        verifier
            .begin_session(Some(&stale), context, &stale_attestations)
            .unwrap_err()
            .contains("stale")
    );

    let valid = make_session(context, 5, 1_000, 1_060);
    let mut attestations = vec![session_attestation(&issuer, &valid)];
    let payload = b"one signed contract";
    attestations.push(attestation(
        &a,
        "producer-a",
        Role::ContractTest,
        payload,
        Some(&valid),
    ));
    attestations.push(attestation(
        &b,
        "producer-b",
        Role::ContractTest,
        payload,
        Some(&valid),
    ));
    assert!(
        verifier
            .begin_session(Some(&valid), b"tampered context", &attestations)
            .unwrap_err()
            .contains("context")
    );
    verifier
        .begin_session(Some(&valid), context, &attestations)
        .unwrap();
    verifier
        .authenticate(Role::ContractTest, payload, &[], &attestations)
        .unwrap();
    // A different required role with no signature makes a chain incomplete.
    assert!(
        verifier
            .authenticate(Role::Runtime, payload, &[], &attestations)
            .is_err()
    );
    let assurance = verifier.assurance();
    assert!(
        assurance.provisioned_policy && assurance.freshness_checked && assurance.replay_checked
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn provisioned_policy_rejects_deployment_signer_aliases_and_legacy_freshness() {
    let root = temp("policy-domains");
    let issuer = SigningKey::from_bytes(&[61; 32]);
    let a = SigningKey::from_bytes(&[62; 32]);
    let b = SigningKey::from_bytes(&[63; 32]);
    let mut aliased = policy(&issuer, &a, &b);
    aliased.keys.push(producer_key(
        "deployer-alias",
        &a,
        Role::Deployment,
        "deployer",
        "vault",
        "service",
    ));
    assert!(matches!(
        SignedArtifactStore::provisioned(aliased, root.join("artifacts"), root.join("alias"), 1_020),
        Err(error) if error.contains("dedicated role")
    ));
    let mut ambiguous = policy(&issuer, &a, &b);
    ambiguous.required_session_id = Some("a".repeat(64));
    ambiguous.max_attestation_age_seconds = Some(60);
    assert!(matches!(
        SignedArtifactStore::provisioned(ambiguous, root.join("artifacts"), root.join("legacy"), 1_020),
        Err(error) if error.contains("legacy freshness")
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn signing_key_aliases_cannot_supply_independent_producer_evidence() {
    let root = temp("signer-alias");
    std::fs::create_dir_all(root.join("artifacts")).unwrap();
    let issuer = SigningKey::from_bytes(&[71; 32]);
    let a = SigningKey::from_bytes(&[72; 32]);
    let b = SigningKey::from_bytes(&[73; 32]);
    let mut aliased = policy(&issuer, &a, &b);
    aliased.keys[2].public_key = aliased.keys[1].public_key.clone();
    let verifier = SignedArtifactStore::provisioned(
        aliased,
        root.join("artifacts"),
        root.join("sessions"),
        1_020,
    )
    .unwrap();
    let context = b"independence context";
    let session = make_session(context, 8, 1_000, 1_060);
    let payload = b"contract result";
    let attestations = vec![
        session_attestation(&issuer, &session),
        attestation(
            &a,
            "producer-a",
            Role::ContractTest,
            payload,
            Some(&session),
        ),
        attestation(
            &a,
            "producer-b",
            Role::ContractTest,
            payload,
            Some(&session),
        ),
    ];
    verifier
        .begin_session(Some(&session), context, &attestations)
        .unwrap();
    assert!(
        verifier
            .authenticate(Role::ContractTest, payload, &[], &attestations)
            .unwrap_err()
            .contains("independence")
    );
    std::fs::remove_dir_all(root).unwrap();
}
