//! Trust policy is supplied outside the request. Signatures authenticate assertions,
//! not the honesty of a measurement producer or deployment authority.
use crate::digest_valid;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Read,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;
const HASH_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Session,
    Catalog,
    Runtime,
    ContractTest,
    Benchmark,
    Proposal,
    Deployment,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Attestation {
    pub role: Role,
    pub subject_digest: String,
    pub key_id: String,
    pub signature: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionBinding>,
    #[serde(default)]
    pub issued_at_unix_seconds: Option<u64>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SessionBinding {
    /// Random lowercase-hex challenge id allocated by an external session issuer.
    pub id: String,
    pub challenge_digest: String,
    pub context_digest: String,
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerIdentity {
    pub producer_id: String,
    pub custody_domain: String,
    pub enforcement_domain: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedKey {
    pub key_id: String,
    pub public_key: String,
    pub roles: Vec<Role>,
    pub revoked: bool,
    #[serde(default)]
    pub compromised: bool,
    #[serde(default)]
    pub producer: Option<ProducerIdentity>,
    #[serde(default)]
    pub valid_from_unix: Option<u64>,
    #[serde(default)]
    pub valid_until_unix: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionPolicy {
    pub max_lifetime_secs: u64,
    pub max_future_skew_secs: u64,
    /// Offline sessions are rejected until a trusted offline-time source is configured.
    pub allow_offline: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndependencePolicy {
    pub role: Role,
    pub min_producers: usize,
    pub min_custody_domains: usize,
    pub min_enforcement_domains: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrustPolicy {
    pub schema: u32,
    pub keys: Vec<TrustedKey>,
    #[serde(default)]
    pub session: Option<SessionPolicy>,
    #[serde(default)]
    pub independence: Vec<IndependencePolicy>,
    /// Fresh attestations must match this externally supplied session challenge.
    #[serde(default)]
    pub required_session_id: Option<String>,
    /// Maximum age of a signed timestamp. None keeps legacy timeless policy behavior.
    #[serde(default)]
    pub max_attestation_age_seconds: Option<u64>,
    #[serde(default)]
    pub max_clock_skew_seconds: Option<u64>,
}

pub fn subject_digest(payload: &[u8]) -> String {
    (Sha256::digest(payload))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

/// Stable domain separation. Sign the serialized typed payload's digest and its role.
pub fn signing_message(role: Role, digest: &str) -> Vec<u8> {
    format!(
        "KOLVRT-MIGRATION-ATTESTATION/3\n{}\n{digest}\n",
        serde_json::to_string(&role).unwrap()
    )
    .into_bytes()
}

/// Session-aware signature domain. Timestamp and externally issued session challenge are signed.
pub fn session_signing_message(
    role: Role,
    digest: &str,
    issued_at_unix_seconds: u64,
    session_id: &str,
) -> Vec<u8> {
    format!(
        "KOLVRT-MIGRATION-ATTESTATION/4\n{}\n{digest}\n{issued_at_unix_seconds}\n{session_id}\n",
        serde_json::to_string(&role).unwrap()
    )
    .into_bytes()
}

pub fn signing_message_bound(role: Role, digest: &str, session: &SessionBinding) -> Vec<u8> {
    let session_digest = subject_digest(&serde_json::to_vec(session).expect("session encoding"));
    format!(
        "{}SESSION\n{session_digest}\n",
        String::from_utf8(signing_message(role, digest)).unwrap()
    )
    .into_bytes()
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TrustAssurance {
    pub experimental_fixture_mode: bool,
    pub provisioned_policy: bool,
    pub role_scoped_signatures: bool,
    pub revocation_checked: bool,
    pub session_bound: bool,
    pub session_verified: bool,
    pub freshness_checked: bool,
    pub replay_checked: bool,
    pub producer_independence_checked: bool,
    pub producer_independence_satisfied: bool,
    pub evidence_chain_complete: bool,
}

pub trait EvidenceVerifier {
    /// Must authenticate the exact payload, signer role and all referenced artifact bytes.
    /// Implementations are trusted application configuration, never request-supplied code.
    fn authenticate(
        &self,
        role: Role,
        payload: &[u8],
        references: &[String],
        attestations: &[Attestation],
    ) -> Result<(), String>;
    fn begin_session(
        &self,
        session: Option<&SessionBinding>,
        _context: &[u8],
        _attestations: &[Attestation],
    ) -> Result<(), String> {
        if session.is_none() {
            Ok(())
        } else {
            Err("no provisioned session verifier configured".into())
        }
    }
    fn assurance(&self) -> TrustAssurance {
        TrustAssurance::default()
    }
}

pub struct NoTrust;
impl EvidenceVerifier for NoTrust {
    fn authenticate(
        &self,
        _: Role,
        _: &[u8],
        _: &[String],
        _: &[Attestation],
    ) -> Result<(), String> {
        Err("no external trust policy and artifact verifier configured".into())
    }
}

fn hex_bytes<const N: usize>(text: &str) -> Result<[u8; N], String> {
    if text.len() != N * 2
        || !text
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("invalid lowercase hexadecimal encoding".into());
    }
    let mut bytes = [0; N];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
    }
    Ok(bytes)
}

/// Trusted application configuration, never deserialized from a request.
/// The provider must return current UTC seconds or fail; monotonicity is checked
/// by each verifier, but authenticity and a progressing clock remain provider obligations.
pub trait TrustedUtcClock: Send + Sync {
    fn now_unix(&self) -> Result<u64, String>;
}

/// Host CLI clock adapter. OS UTC is an explicit deployment trust assumption,
/// not an authenticated production time service.
pub struct SystemUtcClock;
impl TrustedUtcClock for SystemUtcClock {
    fn now_unix(&self) -> Result<u64, String> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .map_err(|_| "system UTC clock unavailable or before UNIX epoch".into())
    }
}

#[derive(Default)]
struct ClockState {
    last_unix: Option<u64>,
    failed: bool,
}

#[derive(Clone)]
struct ActiveSession {
    binding: SessionBinding,
    issuer_key_id: String,
}

pub struct SignedArtifactStore {
    policy: TrustPolicy,
    root: PathBuf,
    ledger: Option<PathBuf>,
    clock: Option<Arc<dyn TrustedUtcClock>>,
    clock_state: Mutex<ClockState>,
    session_attempted: AtomicBool,
    active_session: Mutex<Option<ActiveSession>>,
}
impl SignedArtifactStore {
    pub fn new(policy: TrustPolicy, root: PathBuf) -> Result<Self, String> {
        let mut ids = BTreeSet::new();
        if policy.schema != 1 || policy.session.is_some() || !policy.independence.is_empty() {
            return Err("unsupported trust-policy schema".into());
        }
        if policy.max_attestation_age_seconds == Some(0)
            || (policy.max_attestation_age_seconds.is_some()
                && policy.required_session_id.is_none())
            || policy
                .required_session_id
                .as_ref()
                .is_some_and(|session| !digest_valid(session))
        {
            return Err("invalid freshness age or externally supplied session challenge".into());
        }
        for key in &policy.keys {
            if key.key_id.trim().is_empty() || key.roles.is_empty() || !ids.insert(&key.key_id) {
                return Err("empty or duplicate trusted key/roles".into());
            }
            if key.roles.contains(&Role::Deployment)
                && (key.roles.len() != 1
                    || policy.keys.iter().any(|other| {
                        other.public_key == key.public_key
                            && other.roles.iter().any(|role| *role != Role::Deployment)
                    }))
            {
                return Err("deployment authorization keys must have a dedicated role".into());
            }
            let key = VerifyingKey::from_bytes(&hex_bytes(&key.public_key)?)
                .map_err(|e| e.to_string())?;
            if key.is_weak() {
                return Err("weak trusted key".into());
            }
        }
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        if !root.is_dir() {
            return Err("artifact root is not a directory".into());
        }
        Ok(Self {
            policy,
            root,
            ledger: None,
            clock: None,
            clock_state: Mutex::new(ClockState::default()),
            session_attempted: AtomicBool::new(false),
            active_session: Mutex::new(None),
        })
    }

    /// Provisioned verifier. Policy, ledger location and clock come from trusted
    /// application configuration; none are selected by the migration request.
    pub fn provisioned(
        policy: TrustPolicy,
        root: PathBuf,
        ledger: PathBuf,
        clock: Arc<dyn TrustedUtcClock>,
    ) -> Result<Self, String> {
        if policy.schema != 2 {
            return Err("provisioned verification requires trust-policy schema 2".into());
        }
        if policy.required_session_id.is_some()
            || policy.max_attestation_age_seconds.is_some()
            || policy.max_clock_skew_seconds.is_some()
        {
            return Err("provisioned policy rejects legacy freshness configuration".into());
        }
        let session = policy
            .session
            .as_ref()
            .ok_or("provisioned policy requires explicit session clock contract")?;
        if session.max_lifetime_secs == 0
            || session.max_future_skew_secs > 300
            || session.allow_offline
        {
            return Err("unsupported session lifetime, skew or offline clock contract".into());
        }
        if policy.independence.is_empty() {
            return Err("provisioned policy requires explicit producer-independence rules".into());
        }
        let mut ids = BTreeSet::new();
        for key in &policy.keys {
            if key.key_id.trim().is_empty()
                || key.roles.is_empty()
                || !ids.insert(&key.key_id)
                || key
                    .valid_from_unix
                    .zip(key.valid_until_unix)
                    .is_some_and(|(a, b)| a >= b)
            {
                return Err("empty, duplicate or invalid-lifetime trusted key".into());
            }
            if key.roles.contains(&Role::Deployment)
                && (key.roles.len() != 1
                    || policy.keys.iter().any(|other| {
                        other.public_key == key.public_key
                            && other.roles.iter().any(|role| *role != Role::Deployment)
                    }))
            {
                return Err("deployment authorization keys must have a dedicated role".into());
            }
            let producer = key
                .producer
                .as_ref()
                .ok_or("provisioned key missing producer identity")?;
            if producer.producer_id.trim().is_empty()
                || producer.custody_domain.trim().is_empty()
                || producer.enforcement_domain.trim().is_empty()
            {
                return Err("empty producer/custody/enforcement identity".into());
            }
            let public = VerifyingKey::from_bytes(&hex_bytes(&key.public_key)?)
                .map_err(|e| e.to_string())?;
            if public.is_weak() {
                return Err("weak trusted key".into());
            }
        }
        if !policy
            .keys
            .iter()
            .any(|key| key.roles.contains(&Role::Session) && !key.revoked && !key.compromised)
        {
            return Err("provisioned policy has no active session issuer".into());
        }
        for independence in &policy.independence {
            if independence.role == Role::Session
                || independence.min_producers < 2
                || independence.min_custody_domains < 2
                || independence.min_enforcement_domains < 2
            {
                return Err("independence requirements must require at least two independent producers/domains".into());
            }
        }
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        if !root.is_dir() {
            return Err("artifact root is not a directory".into());
        }
        fs::create_dir_all(&ledger).map_err(|e| e.to_string())?;
        let ledger = ledger.canonicalize().map_err(|e| e.to_string())?;
        if !ledger.is_dir() {
            return Err("session ledger is not a directory".into());
        }
        let verifier = Self {
            policy,
            root,
            ledger: Some(ledger),
            clock: Some(clock),
            clock_state: Mutex::new(ClockState::default()),
            session_attempted: AtomicBool::new(false),
            active_session: Mutex::new(None),
        };
        verifier.checked_now()?;
        Ok(verifier)
    }

    fn checked_now(&self) -> Result<u64, String> {
        // The external clock callback runs outside every verifier mutex.
        let sample = self
            .clock
            .as_ref()
            .ok_or("trusted UTC clock unavailable")?
            .now_unix();
        let mut state = self
            .clock_state
            .lock()
            .map_err(|_| "clock verifier poisoned")?;
        if state.failed {
            return Err("trusted clock verifier invalidated".into());
        }
        let Ok(now) = sample else {
            state.failed = true;
            return Err("trusted UTC clock unavailable; verifier invalidated".into());
        };
        if state.last_unix.is_some_and(|last| now < last) {
            state.failed = true;
            return Err("trusted UTC clock moved backwards; verifier invalidated".into());
        }
        state.last_unix = Some(now);
        Ok(now)
    }

    fn verification_time(&self) -> Result<Option<u64>, String> {
        if self.policy.schema == 2 {
            return self.checked_now().map(Some);
        }
        if self.policy.required_session_id.is_some()
            || self.policy.max_attestation_age_seconds.is_some()
            || self.policy.max_clock_skew_seconds.is_some()
        {
            return SystemUtcClock.now_unix().map(Some);
        }
        Ok(None)
    }

    fn session_current(&self, session: &SessionBinding, now: u64) -> Result<(), String> {
        let policy = self
            .policy
            .session
            .as_ref()
            .ok_or("missing session policy")?;
        if session.issued_at_unix > now.saturating_add(policy.max_future_skew_secs)
            || now >= session.expires_at_unix
        {
            return Err("stale or not-yet-valid evidence session".into());
        }
        Ok(())
    }

    fn active_current(&self, active: &ActiveSession, now: u64) -> Result<(), String> {
        self.session_current(&active.binding, now)?;
        if !self.policy.keys.iter().any(|key| {
            key.key_id == active.issuer_key_id && self.key_usable(key, Role::Session, Some(now))
        }) {
            return Err("session issuer no longer valid in policy snapshot".into());
        }
        Ok(())
    }

    fn verify_bytes(&self, digest: &str) -> Result<(), String> {
        if !digest_valid(digest) {
            return Err("invalid referenced digest".into());
        }
        let path = self
            .root
            .join(digest)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !path.starts_with(&self.root) {
            return Err("artifact escapes trusted store".into());
        }
        let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        if file.metadata().map_err(|e| e.to_string())?.len() > MAX_ARTIFACT_BYTES {
            return Err("artifact exceeds 512 MiB".into());
        }
        let mut hash = Sha256::new();
        let mut buffer = [0; HASH_BUFFER_BYTES];
        let mut total = 0u64;
        loop {
            let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > MAX_ARTIFACT_BYTES {
                return Err("artifact grew beyond limit".into());
            }
            hash.update(&buffer[..n]);
        }
        if (hash.finalize())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            != digest
        {
            return Err("artifact bytes do not match digest".into());
        }
        Ok(())
    }

    fn key_usable(&self, key: &TrustedKey, role: Role, at: Option<u64>) -> bool {
        key.roles.contains(&role)
            && !key.revoked
            && !key.compromised
            && at.is_none_or(|at| {
                key.valid_from_unix.is_none_or(|start| at >= start)
                    && key.valid_until_unix.is_none_or(|end| at < end)
            })
    }

    fn verify_signature(
        &self,
        attestation: &Attestation,
        role: Role,
        digest: &str,
        at: Option<u64>,
        active: Option<&ActiveSession>,
    ) -> bool {
        if self.policy.schema == 2 && at.is_none() {
            return false;
        }
        let Some(key) = self
            .policy
            .keys
            .iter()
            .find(|key| key.key_id == attestation.key_id && self.key_usable(key, role, at))
        else {
            return false;
        };
        let Ok(public) = hex_bytes(&key.public_key) else {
            return false;
        };
        let Ok(public) = VerifyingKey::from_bytes(&public) else {
            return false;
        };
        let Ok(signature) = hex_bytes(&attestation.signature) else {
            return false;
        };
        let message = if self.policy.schema == 2 {
            if attestation.issued_at_unix_seconds.is_some() || attestation.session_id.is_some() {
                return false;
            }
            let Some(session) = attestation.session.as_ref() else {
                return false;
            };
            if active.map(|active| &active.binding) != Some(session) {
                return false;
            }
            signing_message_bound(role, digest, session)
        } else {
            if attestation.session.is_some() {
                return false;
            }
            let Some(message) = attestation_message(attestation, &self.policy, at) else {
                return false;
            };
            message
        };
        public
            .verify_strict(&message, &Signature::from_bytes(&signature))
            .is_ok()
    }
}
impl EvidenceVerifier for SignedArtifactStore {
    fn authenticate(
        &self,
        role: Role,
        payload: &[u8],
        references: &[String],
        attestations: &[Attestation],
    ) -> Result<(), String> {
        let digest = subject_digest(payload);
        let active = self
            .active_session
            .lock()
            .map_err(|_| "session verifier poisoned")?
            .clone();
        if self.policy.schema == 2 && active.is_none() {
            return Err("no active provisioned session".into());
        }
        // Revalidate after artifact I/O, before returning an accepted result.
        // The policy/session are immutable snapshots during both passes.
        let passes = if self.policy.schema == 2 { 2 } else { 1 };
        let mut verified_matching: Option<Vec<&Attestation>> = None;
        for pass in 0..passes {
            let at = self.verification_time()?;
            if let (Some(active), Some(now)) = (active.as_ref(), at) {
                self.active_current(active, now)?;
            }
            let matching: Vec<_> = if let Some(verified) = verified_matching.take() {
                // Bytes, signatures, policy and pinned session are unchanged.
                // Recheck temporal key eligibility, not Ed25519 or artifact I/O.
                verified
                    .into_iter()
                    .filter(|attestation| {
                        self.policy
                            .keys
                            .iter()
                            .find(|key| key.key_id == attestation.key_id)
                            .is_some_and(|key| self.key_usable(key, role, at))
                    })
                    .collect()
            } else {
                attestations
                    .iter()
                    .filter(|a| a.role == role && a.subject_digest == digest)
                    .filter(|a| self.verify_signature(a, role, &digest, at, active.as_ref()))
                    .collect()
            };
            let authenticated = !matching.is_empty();
            if !authenticated {
                return Err("missing, revoked, wrong-role or invalid Ed25519 signature".into());
            }
            for rule in self
                .policy
                .independence
                .iter()
                .filter(|rule| rule.role == role)
            {
                let eligible: Vec<_> = matching
                    .iter()
                    .filter_map(|attestation| {
                        self.policy
                            .keys
                            .iter()
                            .find(|key| key.key_id == attestation.key_id)
                            .and_then(|key| key.producer.as_ref())
                    })
                    .collect();
                // Different key IDs and declared domains do not make aliases of one
                // signing key independent evidence producers.
                let signers: BTreeSet<_> = matching
                    .iter()
                    .filter_map(|attestation| {
                        self.policy
                            .keys
                            .iter()
                            .find(|key| key.key_id == attestation.key_id)
                            .map(|key| &key.public_key)
                    })
                    .collect();
                let producer_ids: BTreeSet<_> = eligible.iter().map(|p| &p.producer_id).collect();
                let custody: BTreeSet<_> = eligible.iter().map(|p| &p.custody_domain).collect();
                let enforcement: BTreeSet<_> =
                    eligible.iter().map(|p| &p.enforcement_domain).collect();
                if signers.len() < rule.min_producers
                    || producer_ids.len() < rule.min_producers
                    || custody.len() < rule.min_custody_domains
                    || enforcement.len() < rule.min_enforcement_domains
                {
                    return Err(format!(
                        "{role:?}: producer independence policy not satisfied"
                    ));
                }
            }
            if pass == 0 {
                for reference in references.iter().collect::<BTreeSet<_>>() {
                    self.verify_bytes(reference)?;
                }
                verified_matching = Some(matching);
            }
        }
        Ok(())
    }

    fn begin_session(
        &self,
        session: Option<&SessionBinding>,
        context: &[u8],
        attestations: &[Attestation],
    ) -> Result<(), String> {
        if self.policy.schema == 1 {
            return if session.is_none() {
                Ok(())
            } else {
                Err("experimental fixture policy rejects provisioned sessions".into())
            };
        }
        let session = session.ok_or("missing provisioned evidence session")?;
        let policy = self
            .policy
            .session
            .as_ref()
            .ok_or("missing session policy")?;
        if session.id.len() != 32
            || !session
                .id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            || !digest_valid(&session.challenge_digest)
            || !digest_valid(&session.context_digest)
            || session.context_digest != subject_digest(context)
            || session.expires_at_unix <= session.issued_at_unix
            || session.expires_at_unix - session.issued_at_unix > policy.max_lifetime_secs
        {
            return Err("invalid session id, challenge, context or lifetime".into());
        }
        if policy.allow_offline {
            return Err(
                "offline evidence is disabled without a trusted offline clock source".into(),
            );
        }
        let now = self.checked_now()?;
        self.session_current(session, now)?;
        let session_bytes = serde_json::to_vec(session).map_err(|error| error.to_string())?;
        let digest = subject_digest(&session_bytes);
        let issuer_attestation = attestations
            .iter()
            .find(|attestation| {
                attestation.role == Role::Session
                    && attestation.subject_digest == digest
                    && attestation.session.is_none()
                    && attestation.issued_at_unix_seconds.is_none()
                    && attestation.session_id.is_none()
            })
            .ok_or("missing trusted session challenge signature")?;
        let issuer_key = self
            .policy
            .keys
            .iter()
            .find(|key| {
                key.key_id == issuer_attestation.key_id
                    && self.key_usable(key, Role::Session, Some(now))
            })
            .ok_or("untrusted, wrong-role, revoked or out-of-rotation session issuer")?;
        let public = VerifyingKey::from_bytes(&hex_bytes(&issuer_key.public_key)?)
            .map_err(|error| error.to_string())?;
        let signature = Signature::from_bytes(&hex_bytes(&issuer_attestation.signature)?);
        public
            .verify_strict(&signing_message(Role::Session, &digest), &signature)
            .map_err(|_| "invalid session challenge signature")?;
        let ledger = self
            .ledger
            .as_ref()
            .ok_or("provisioned session ledger unavailable")?;
        if self.session_attempted.swap(true, Ordering::AcqRel) {
            return Err("provisioned verifier already used for session admission".into());
        }
        // Consume the issuer's challenge digest, so a retry cannot obtain a
        // second session id around the same signed challenge.
        let marker = ledger.join(format!("{}.used", session.challenge_digest));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(marker)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    "replayed evidence session".into()
                } else {
                    error.to_string()
                }
            })?;
        use std::io::Write;
        writeln!(
            file,
            "{} {}",
            session.issued_at_unix, session.context_digest
        )
        .map_err(|error| error.to_string())?;
        // A failed final check leaves the challenge consumed. Never roll back
        // a marker to make an interrupted or expired admission retryable.
        let final_now = self.checked_now()?;
        self.session_current(session, final_now)?;
        if !self.key_usable(issuer_key, Role::Session, Some(final_now)) {
            return Err("session issuer expired before publication".into());
        }
        *self
            .active_session
            .lock()
            .map_err(|_| "session verifier poisoned")? = Some(ActiveSession {
            binding: session.clone(),
            issuer_key_id: issuer_key.key_id.clone(),
        });
        Ok(())
    }

    fn assurance(&self) -> TrustAssurance {
        if self.policy.schema == 2 {
            TrustAssurance {
                provisioned_policy: true,
                role_scoped_signatures: true,
                revocation_checked: true,
                session_bound: true,
                session_verified: false,
                freshness_checked: true,
                replay_checked: true,
                producer_independence_checked: !self.policy.independence.is_empty(),
                ..TrustAssurance::default()
            }
        } else {
            TrustAssurance {
                experimental_fixture_mode: true,
                role_scoped_signatures: true,
                revocation_checked: true,
                ..TrustAssurance::default()
            }
        }
    }
}

fn attestation_message(
    attestation: &Attestation,
    policy: &TrustPolicy,
    now: Option<u64>,
) -> Option<Vec<u8>> {
    match (
        attestation.issued_at_unix_seconds,
        attestation.session_id.as_deref(),
    ) {
        (Some(issued_at), Some(session_id)) => {
            if !digest_valid(session_id)
                || policy
                    .required_session_id
                    .as_deref()
                    .is_some_and(|required| required != session_id)
            {
                return None;
            }
            if let Some(now) = now {
                let skew = policy.max_clock_skew_seconds.unwrap_or(0);
                if issued_at > now.saturating_add(skew)
                    || policy
                        .max_attestation_age_seconds
                        .is_some_and(|age| now.saturating_sub(issued_at) > age.saturating_add(skew))
                {
                    return None;
                }
            }
            Some(session_signing_message(
                attestation.role,
                &attestation.subject_digest,
                issued_at,
                session_id,
            ))
        }
        (None, None)
            if policy.required_session_id.is_none()
                && policy.max_attestation_age_seconds.is_none()
                && policy.max_clock_skew_seconds.is_none() =>
        {
            Some(signing_message(
                attestation.role,
                &attestation.subject_digest,
            ))
        }
        _ => None,
    }
}
