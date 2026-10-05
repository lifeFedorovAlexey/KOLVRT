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
    let verifier = SignedArtifactStore::provisioned(
        policy(&issuer, &a, &b),
        artifacts,
        ledger,
        fixed_clock(now),
    )
    .unwrap();
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
        fixed_clock(1_020),
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
        fixed_clock(1_020),
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
        fixed_clock(1_020),
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
        fixed_clock(1_020),
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
        SignedArtifactStore::provisioned(aliased, root.join("artifacts"), root.join("alias"), fixed_clock(1_020)),
        Err(error) if error.contains("dedicated role")
    ));
    let mut ambiguous = policy(&issuer, &a, &b);
    ambiguous.required_session_id = Some("a".repeat(64));
    ambiguous.max_attestation_age_seconds = Some(60);
    assert!(matches!(
        SignedArtifactStore::provisioned(ambiguous, root.join("artifacts"), root.join("legacy"), fixed_clock(1_020)),
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
        fixed_clock(1_020),
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

// Static time exists only in deterministic fixtures, not the public verifier API.
struct FixtureClock(u64);
impl migration_advisor::provenance::TrustedUtcClock for FixtureClock {
    fn now_unix(&self) -> Result<u64, String> {
        Ok(self.0)
    }
}
fn fixed_clock(now: u64) -> std::sync::Arc<dyn migration_advisor::provenance::TrustedUtcClock> {
    std::sync::Arc::new(FixtureClock(now))
}

#[derive(Default)]
struct MutableClock {
    now: std::sync::atomic::AtomicU64,
    unavailable: std::sync::atomic::AtomicBool,
}
impl TrustedUtcClock for MutableClock {
    fn now_unix(&self) -> Result<u64, String> {
        use std::sync::atomic::Ordering;
        if self.unavailable.load(Ordering::SeqCst) {
            return Err("fixture unavailable".into());
        }
        Ok(self.now.load(Ordering::SeqCst))
    }
}
impl MutableClock {
    fn at(now: u64) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            now: now.into(),
            unavailable: false.into(),
        })
    }
    fn set(&self, now: u64) {
        self.now.store(now, std::sync::atomic::Ordering::SeqCst);
    }
}

struct SequenceClock(std::sync::Mutex<std::collections::VecDeque<u64>>);
impl TrustedUtcClock for SequenceClock {
    fn now_unix(&self) -> Result<u64, String> {
        self.0
            .lock()
            .unwrap()
            .pop_front()
            .ok_or("fixture clock exhausted".into())
    }
}

#[test]
fn unavailable_constructor_clock_never_creates_an_accepted_verifier() {
    let root = temp("constructor-clock-error");
    std::fs::create_dir_all(root.join("artifacts")).unwrap();
    let issuer = SigningKey::from_bytes(&[41; 32]);
    let a = SigningKey::from_bytes(&[42; 32]);
    let b = SigningKey::from_bytes(&[43; 32]);
    let clock = MutableClock::at(1020);
    clock
        .unavailable
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let result = SignedArtifactStore::provisioned(
        policy(&issuer, &a, &b),
        root.join("artifacts"),
        root.join("sessions"),
        clock,
    );
    assert!(matches!(result,Err(error) if error.contains("unavailable")));
    assert!(
        std::fs::read_dir(root.join("sessions"))
            .unwrap()
            .next()
            .is_none()
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn moving_store(
    root: &Path,
    clock: std::sync::Arc<dyn TrustedUtcClock>,
    mutate: impl FnOnce(&mut TrustPolicy),
) -> (SignedArtifactStore, SigningKey, SigningKey, SigningKey) {
    let issuer = SigningKey::from_bytes(&[41; 32]);
    let a = SigningKey::from_bytes(&[42; 32]);
    let b = SigningKey::from_bytes(&[43; 32]);
    let mut configured = policy(&issuer, &a, &b);
    mutate(&mut configured);
    std::fs::create_dir_all(root.join("artifacts")).unwrap();
    let store = SignedArtifactStore::provisioned(
        configured,
        root.join("artifacts"),
        root.join("sessions"),
        clock,
    )
    .unwrap();
    (store, issuer, a, b)
}
fn bound_evidence(
    issuer: &SigningKey,
    a: &SigningKey,
    b: &SigningKey,
    session: &SessionBinding,
    payload: &[u8],
) -> Vec<Attestation> {
    vec![
        session_attestation(issuer, session),
        attestation(a, "producer-a", Role::ContractTest, payload, Some(session)),
        attestation(b, "producer-b", Role::ContractTest, payload, Some(session)),
    ]
}

#[test]
fn admission_and_later_authentication_use_current_clock_not_constructor_snapshot() {
    let root = temp("moving-expiry");
    let clock = MutableClock::at(1020);
    let (store, issuer, a, b) = moving_store(&root, clock.clone(), |_| {});
    let context = b"moving context";
    let payload = b"signed payload";
    let session = make_session(context, 1, 1000, 1060);
    let evidence = bound_evidence(&issuer, &a, &b, &session, payload);
    store
        .begin_session(Some(&session), context, &evidence)
        .unwrap();
    store
        .authenticate(Role::ContractTest, payload, &[], &evidence)
        .unwrap();
    clock.set(1060);
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &[], &evidence)
            .unwrap_err()
            .contains("stale")
    );
    let root2 = temp("expired-before-admission");
    let clock2 = MutableClock::at(1020);
    let (store2, issuer2, a2, b2) = moving_store(&root2, clock2.clone(), |_| {});
    let evidence2 = bound_evidence(&issuer2, &a2, &b2, &session, payload);
    clock2.set(1060);
    assert!(
        store2
            .begin_session(Some(&session), context, &evidence2)
            .unwrap_err()
            .contains("stale")
    );
    assert!(
        std::fs::read_dir(root2.join("sessions"))
            .unwrap()
            .next()
            .is_none()
    );
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(root2).unwrap();
}

#[test]
fn unavailable_or_rollback_clock_invalidates_even_after_recovery() {
    for unavailable in [true, false] {
        let root = temp("clock-invalidated");
        let clock = MutableClock::at(1020);
        let (store, issuer, a, b) = moving_store(&root, clock.clone(), |_| {});
        let context = b"clock context";
        let payload = b"signed payload";
        let session = make_session(context, 1, 1000, 1100);
        let evidence = bound_evidence(&issuer, &a, &b, &session, payload);
        store
            .begin_session(Some(&session), context, &evidence)
            .unwrap();
        if unavailable {
            clock
                .unavailable
                .store(true, std::sync::atomic::Ordering::SeqCst);
        } else {
            clock.set(1019);
        }
        assert!(
            store
                .authenticate(Role::ContractTest, payload, &[], &evidence)
                .unwrap_err()
                .contains("invalidated")
        );
        clock
            .unavailable
            .store(false, std::sync::atomic::Ordering::SeqCst);
        clock.set(1030);
        assert!(
            store
                .authenticate(Role::ContractTest, payload, &[], &evidence)
                .unwrap_err()
                .contains("invalidated")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn current_key_rotation_and_session_issuer_expiry_are_not_hidden_by_snapshot() {
    for issuer_expires in [false, true] {
        let root = temp("moving-key-expiry");
        let clock = MutableClock::at(1020);
        let (store, issuer, a, b) = moving_store(&root, clock.clone(), |policy| {
            policy.keys[if issuer_expires { 0 } else { 1 }].valid_until_unix = Some(1030);
        });
        let context = b"key context";
        let payload = b"signed payload";
        let session = make_session(context, 1, 1000, 1100);
        let evidence = bound_evidence(&issuer, &a, &b, &session, payload);
        store
            .begin_session(Some(&session), context, &evidence)
            .unwrap();
        store
            .authenticate(Role::ContractTest, payload, &[], &evidence)
            .unwrap();
        clock.set(1030);
        assert!(
            store
                .authenticate(Role::ContractTest, payload, &[], &evidence)
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn expiration_during_authentication_is_rejected_at_final_acceptance() {
    let root = temp("mid-auth-expiry");
    let clock = std::sync::Arc::new(SequenceClock(std::sync::Mutex::new(
        [1020, 1020, 1020, 1020, 1060].into(),
    )));
    let (store, issuer, a, b) = moving_store(&root, clock, |_| {});
    let context = b"mid auth";
    let payload = b"signed payload";
    let session = make_session(context, 1, 1000, 1060);
    let evidence = bound_evidence(&issuer, &a, &b, &session, payload);
    store
        .begin_session(Some(&session), context, &evidence)
        .unwrap();
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &[], &evidence)
            .unwrap_err()
            .contains("stale")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn final_acceptance_rechecks_producer_rotation_and_clock_availability() {
    for unavailable in [false, true] {
        let root = temp("final-check");
        let samples = if unavailable {
            vec![1020, 1020, 1020, 1020]
        } else {
            vec![1020, 1020, 1020, 1020, 1030]
        };
        let clock = std::sync::Arc::new(SequenceClock(std::sync::Mutex::new(samples.into())));
        let (store, issuer, a, b) = moving_store(&root, clock, |policy| {
            policy.keys[1].valid_until_unix = Some(1030);
        });
        let context = b"final context";
        let payload = b"signed payload";
        let session = make_session(context, 1, 1000, 1100);
        let evidence = bound_evidence(&issuer, &a, &b, &session, payload);
        store
            .begin_session(Some(&session), context, &evidence)
            .unwrap();
        assert!(
            store
                .authenticate(Role::ContractTest, payload, &[], &evidence)
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn failed_publication_keeps_challenge_consumed_and_never_activates_session() {
    let root = temp("mid-admission-expiry");
    let clock = std::sync::Arc::new(SequenceClock(std::sync::Mutex::new(
        [1020, 1020, 1060].into(),
    )));
    let (store, issuer, a, b) = moving_store(&root, clock, |_| {});
    let context = b"mid admission";
    let payload = b"signed payload";
    let session = make_session(context, 1, 1000, 1060);
    let evidence = bound_evidence(&issuer, &a, &b, &session, payload);
    assert!(
        store
            .begin_session(Some(&session), context, &evidence)
            .unwrap_err()
            .contains("stale")
    );
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &[], &evidence)
            .unwrap_err()
            .contains("no active")
    );
    let (new_store, _, _, _) = moving_store(&root, fixed_clock(1020), |_| {});
    assert!(
        new_store
            .begin_session(Some(&session), context, &evidence)
            .unwrap_err()
            .contains("replayed")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn future_sessions_are_rejected_without_consuming_challenge() {
    let root = temp("future-clock");
    let (store, issuer, a, b) = moving_store(&root, fixed_clock(1020), |_| {});
    let context = b"future context";
    let payload = b"signed payload";
    let session = make_session(context, 1, 1040, 1100);
    let evidence = bound_evidence(&issuer, &a, &b, &session, payload);
    assert!(
        store
            .begin_session(Some(&session), context, &evidence)
            .unwrap_err()
            .contains("not-yet-valid")
    );
    assert!(
        std::fs::read_dir(root.join("sessions"))
            .unwrap()
            .next()
            .is_none()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_distinct_session_admission_cannot_replace_active_binding() {
    let root = temp("parallel-sessions");
    let (store, issuer, a, b) = moving_store(&root, fixed_clock(1020), |_| {});
    let store = std::sync::Arc::new(store);
    let context = b"parallel context";
    let payload = b"signed payload";
    let sessions = [
        make_session(context, 1, 1000, 1100),
        make_session(context, 2, 1000, 1100),
    ];
    let proofs: Vec<_> = sessions
        .iter()
        .map(|session| bound_evidence(&issuer, &a, &b, session, payload))
        .collect();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|index| {
            let store = store.clone();
            let barrier = barrier.clone();
            let session = sessions[index].clone();
            let proof = proofs[index].clone();
            std::thread::spawn(move || {
                barrier.wait();
                store.begin_session(Some(&session), context, &proof).is_ok()
            })
        })
        .collect();
    let accepted: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(accepted.iter().filter(|ok| **ok).count(), 1);
    let winner = usize::from(!accepted[0]);
    store
        .authenticate(Role::ContractTest, payload, &[], &proofs[winner])
        .unwrap();
    assert!(
        store
            .authenticate(Role::ContractTest, payload, &[], &proofs[1 - winner])
            .is_err()
    );
    assert_eq!(std::fs::read_dir(root.join("sessions")).unwrap().count(), 1);
    std::fs::remove_dir_all(root).unwrap();
}
