//! Trust policy is supplied outside the request. Signatures authenticate assertions,
//! not the honesty of a measurement producer or deployment authority.
use crate::digest_valid;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::Read,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;
const HASH_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Role {
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
    pub issued_at_unix_seconds: Option<u64>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedKey {
    pub key_id: String,
    pub public_key: String,
    pub roles: Vec<Role>,
    pub revoked: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrustPolicy {
    pub schema: u32,
    pub keys: Vec<TrustedKey>,
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
    format!("{:x}", Sha256::digest(payload))
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

pub struct SignedArtifactStore {
    policy: TrustPolicy,
    root: PathBuf,
}
impl SignedArtifactStore {
    pub fn new(policy: TrustPolicy, root: PathBuf) -> Result<Self, String> {
        let mut ids = BTreeSet::new();
        if policy.schema != 1 {
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
        Ok(Self { policy, root })
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
        if format!("{:x}", hash.finalize()) != digest {
            return Err("artifact bytes do not match digest".into());
        }
        Ok(())
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
        let now = if self.policy.required_session_id.is_some()
            || self.policy.max_attestation_age_seconds.is_some()
            || self.policy.max_clock_skew_seconds.is_some()
        {
            Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| "system clock predates Unix epoch")?
                    .as_secs(),
            )
        } else {
            None
        };
        let authenticated = attestations
            .iter()
            .filter(|a| a.role == role && a.subject_digest == digest)
            .any(|a| {
                let Some(message) = attestation_message(a, &self.policy, now) else {
                    return false;
                };
                self.policy
                    .keys
                    .iter()
                    .filter(|key| {
                        key.key_id == a.key_id && !key.revoked && key.roles.contains(&role)
                    })
                    .any(|key| {
                        let Ok(public) = hex_bytes(&key.public_key) else {
                            return false;
                        };
                        let Ok(signature) = hex_bytes(&a.signature) else {
                            return false;
                        };
                        VerifyingKey::from_bytes(&public).is_ok_and(|key| {
                            key.verify_strict(&message, &Signature::from_bytes(&signature))
                                .is_ok()
                        })
                    })
            });
        if !authenticated {
            return Err("missing, revoked, wrong-role or invalid Ed25519 signature".into());
        }
        for reference in references.iter().collect::<BTreeSet<_>>() {
            self.verify_bytes(reference)?;
        }
        Ok(())
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
