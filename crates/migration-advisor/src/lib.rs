//! Host-side, read-only migration planning. No installation or routing mutation.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub mod advisor;
pub mod provenance;
pub mod routing_evidence;
pub mod solver;
pub mod statistics;

pub const SCHEMA: u32 = 2;
/// Exact enumeration is deliberately limited; large universes need another Solver.
pub const MAX_PACKAGES: usize = 20;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub capability_id: String,
    pub semantic_version: SemanticVersion,
    /// Digest of the semantic contract, not a security authority or package name.
    pub contract_digest: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub capability_id: String,
    pub min_version: SemanticVersion,
    pub max_version: SemanticVersion,
    /// Exact consumer specification to test; not the dependency matching key.
    pub contract_digest: String,
}
impl Requirement {
    pub fn matches(&self, provided: &Capability) -> bool {
        self.capability_id == provided.capability_id
            && self.min_version.major == provided.semantic_version.major
            && (self.min_version..=self.max_version).contains(&provided.semantic_version)
    }
}

/// Outer list is AND, inner list is OR. Empty alternatives are invalid.
pub type Requirements = Vec<Vec<Requirement>>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub version: u32,
    /// SHA-256 of the exact executable/package artifact.
    pub identity: String,
    pub provides: Vec<Capability>,
    pub requires: Requirements,
    pub conflicts: Vec<Requirement>,
}

pub fn covers(packages: &[&Package], requirements: &Requirements) -> bool {
    requirements.iter().all(|alternatives| {
        alternatives.iter().any(|r| {
            packages
                .iter()
                .any(|p| p.provides.iter().any(|c| r.matches(c)))
        })
    })
}

pub fn digest_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn requirement_valid(r: &Requirement) -> bool {
    !r.capability_id.trim().is_empty()
        && r.min_version.major > 0
        && r.min_version.major == r.max_version.major
        && r.min_version <= r.max_version
        && digest_valid(&r.contract_digest)
}

pub fn validate_requirements(requirements: &Requirements) -> Result<(), String> {
    if requirements
        .iter()
        .any(|clause| clause.is_empty() || clause.iter().any(|r| !requirement_valid(r)))
    {
        return Err("invalid requirement or empty alternative clause".into());
    }
    Ok(())
}

pub fn validate_catalog(catalog: &[Package]) -> Result<(), String> {
    if catalog.is_empty() || catalog.len() > MAX_PACKAGES {
        return Err(format!("catalog must contain 1..={MAX_PACKAGES} packages"));
    }
    let mut identities = BTreeSet::new();
    let mut versions = BTreeSet::new();
    for p in catalog {
        if p.name.trim().is_empty()
            || p.version == 0
            || !digest_valid(&p.identity)
            || !identities.insert(&p.identity)
            || !versions.insert((&p.name, p.version))
            || p.provides.iter().any(|c| {
                c.capability_id.trim().is_empty()
                    || c.semantic_version.major == 0
                    || !digest_valid(&c.contract_digest)
            })
            || p.conflicts.iter().any(|r| !requirement_valid(r))
        {
            return Err("invalid or duplicate package identity/version/capability".into());
        }
        let unique: BTreeSet<_> = p
            .provides
            .iter()
            .map(|c| (&c.capability_id, c.semantic_version))
            .collect();
        if unique.len() != p.provides.len() {
            return Err("duplicate provided capability/version".into());
        }
        validate_requirements(&p.requires)?;
    }
    Ok(())
}

pub fn plan_digest(plan: &solver::Plan) -> String {
    // Plan verification requires sorted identities; serialization is canonical for this type.
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(plan).expect("string-only plan"))
    )
}
