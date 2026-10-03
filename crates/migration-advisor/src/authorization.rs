//! Separate, signed deployment authorization. This validates a gate artifact only;
//! it has no installation or routing-mutation API.
use crate::{
    advisor::{
        CandidateStatus, MigrationPlan, Request, RollbackStrategy, RuntimeEvidence,
        advise_with_verifier,
    },
    digest_valid,
    provenance::{Attestation, EvidenceVerifier, Role, subject_digest},
    solver::Solver,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct RouteGeneration {
    pub route: String,
    pub generation: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StateRetirementConsent {
    /// Exact old-state snapshot retained for rollback.
    pub previous_state_digest: String,
    /// Retirement is permitted only after the separately operated candidate health check.
    pub only_after_health_check: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentAuthorization {
    pub schema: u32,
    pub candidate: String,
    pub plan_digest: String,
    pub context: crate::advisor::Context,
    /// Exact active runtime route generations expected at the authorization boundary.
    pub route_generations: Vec<RouteGeneration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_retirement: Option<StateRetirementConsent>,
    /// Explicit acknowledgement of every recorded rollback precondition.
    pub satisfied_rollback_preconditions: Vec<String>,
    /// Explicit authorization of every recorded irreversible change.
    pub authorized_irreversible_changes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedDeploymentAuthorization {
    pub authorization: DeploymentAuthorization,
    pub attestations: Vec<Attestation>,
}

#[derive(Debug, Serialize)]
pub struct AuthorizationReceipt {
    pub schema: u32,
    pub authorization_digest: String,
    pub candidate: String,
    pub plan_digest: String,
    pub route_generations: Vec<RouteGeneration>,
    pub state_retirement: Option<StateRetirementConsent>,
    pub satisfied_rollback_preconditions: Vec<String>,
    pub authorized_irreversible_changes: Vec<String>,
    /// This receipt is an authorization decision, not a deployment operation.
    pub authorization_verified: bool,
    pub deployment_executed: bool,
}

fn exact_strings(actual: &[String], expected: &[String]) -> bool {
    let actual_set: BTreeSet<_> = actual.iter().collect();
    let expected_set: BTreeSet<_> = expected.iter().collect();
    actual.len() == actual_set.len()
        && expected_set.len() == expected.len()
        && actual_set == expected_set
}

fn active_route_generations(runtime: &RuntimeEvidence) -> Result<Vec<RouteGeneration>, String> {
    let mut seen = BTreeSet::new();
    let mut routes = Vec::with_capacity(runtime.routes.len());
    for route in &runtime.routes {
        if route.route.trim().is_empty() || route.generation == 0 || !seen.insert(&route.route) {
            return Err("runtime evidence has ambiguous active route generations".into());
        }
        routes.push(RouteGeneration {
            route: route.route.clone(),
            generation: route.generation,
        });
    }
    routes.sort();
    Ok(routes)
}

fn verify_authorization_bindings(
    authorization: &DeploymentAuthorization,
    runtime: &RuntimeEvidence,
    migration: &MigrationPlan,
) -> Result<Vec<RouteGeneration>, String> {
    if authorization.schema != 1
        || !digest_valid(&authorization.candidate)
        || !digest_valid(&authorization.plan_digest)
        || authorization.candidate != migration.candidate
        || authorization.plan_digest != migration.plan_digest
        || authorization.context != migration.context
        || authorization.context != runtime.context
        || !runtime.complete
        || runtime.lost_events != 0
    {
        return Err(
            "authorization does not bind a complete matching candidate/runtime plan".into(),
        );
    }
    let expected_routes = active_route_generations(runtime)?;
    let mut authorized_routes = authorization.route_generations.clone();
    authorized_routes.sort();
    if authorized_routes != expected_routes {
        return Err("authorization route generations differ from runtime evidence".into());
    }
    if !exact_strings(
        &authorization.satisfied_rollback_preconditions,
        &migration.rollback_preconditions,
    ) || !exact_strings(
        &authorization.authorized_irreversible_changes,
        &migration.irreversible_changes,
    ) {
        return Err(
            "authorization must acknowledge the exact rollback and irreversible-change lists"
                .into(),
        );
    }
    match (
        &migration.rollback_strategy,
        migration.persistent_data_change,
    ) {
        (RollbackStrategy::RestoreSnapshot { snapshot_digest }, true) => {
            let consent = authorization
                .state_retirement
                .as_ref()
                .ok_or("persistent-state retirement needs separate authorization")?;
            if consent.previous_state_digest != snapshot_digest.as_str()
                || !consent.only_after_health_check
            {
                return Err(
                    "state retirement must bind the rollback snapshot and wait for health checks"
                        .into(),
                );
            }
        }
        (_, false) if authorization.state_retirement.is_none() => {}
        _ => return Err("state retirement authorization does not match the migration plan".into()),
    }
    Ok(expected_routes)
}

/// Recompute the advisor result, require a fully verified candidate, bind all
/// authorization choices to its exact plan and current runtime generation set,
/// then authenticate the independent deployment-role signature.
pub fn authorize(
    request: &Request,
    signed: &SignedDeploymentAuthorization,
    solver: &impl Solver,
    verifier: &impl EvidenceVerifier,
) -> Result<AuthorizationReceipt, String> {
    let authorization = &signed.authorization;
    let report = advise_with_verifier(request, solver, verifier)?;
    if report.automatic_replacement {
        return Err("advisor must not perform automatic replacement".into());
    }
    let candidate = report
        .candidates
        .iter()
        .find(|candidate| candidate.package == authorization.candidate)
        .ok_or("authorization candidate is absent from the recomputed report")?;
    if candidate.status != CandidateStatus::VerifiedCandidate
        || candidate.plan_digest.as_deref() != Some(authorization.plan_digest.as_str())
        || !candidate.assurance.missing_checks.is_empty()
        || !candidate.assurance.contracts_passed
        || !candidate.assurance.runtime_complete
        || !candidate.assurance.provenance_verified
        || !candidate.assurance.statistical_gain_supported
        || !candidate.assurance.rollback_documented
        || candidate.proposal.is_none()
    {
        return Err("candidate does not satisfy every advisor authorization gate".into());
    }
    let runtime = request
        .runtime
        .as_ref()
        .filter(|_| request.context.source == crate::advisor::EvidenceSource::OsRuntime)
        .ok_or("deployment authorization requires OS runtime evidence")?;
    let migration = request
        .migration_plans
        .iter()
        .find(|plan| {
            plan.candidate == authorization.candidate
                && plan.plan_digest == authorization.plan_digest
                && plan.context == authorization.context
        })
        .ok_or("authorization has no exact migration plan")?;
    let route_generations = verify_authorization_bindings(authorization, runtime, migration)?;

    let payload = serde_json::to_vec(authorization).map_err(|error| error.to_string())?;
    let mut references = vec![
        authorization.candidate.clone(),
        authorization.plan_digest.clone(),
        runtime.artifact.clone(),
        authorization.context.kernel.clone(),
        authorization.context.workload.clone(),
        authorization.context.protocol.clone(),
    ];
    if let RollbackStrategy::RestoreSnapshot { snapshot_digest } = &migration.rollback_strategy {
        references.push(snapshot_digest.clone());
    }
    references.extend(migration.rollback_execution_receipt.clone());
    verifier.authenticate(
        Role::Deployment,
        &payload,
        &references,
        &signed.attestations,
    )?;
    Ok(AuthorizationReceipt {
        schema: 1,
        authorization_digest: subject_digest(&payload),
        candidate: authorization.candidate.clone(),
        plan_digest: authorization.plan_digest.clone(),
        route_generations,
        state_retirement: authorization.state_retirement.clone(),
        satisfied_rollback_preconditions: authorization.satisfied_rollback_preconditions.clone(),
        authorized_irreversible_changes: authorization.authorized_irreversible_changes.clone(),
        authorization_verified: true,
        deployment_executed: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisor::{Context, EvidenceSource, RouteEvidence};

    fn context() -> Context {
        Context {
            machine: "fixture".into(),
            workload: "a".repeat(64),
            kernel: "b".repeat(64),
            protocol: "c".repeat(64),
            source: EvidenceSource::OsRuntime,
        }
    }

    fn runtime() -> RuntimeEvidence {
        RuntimeEvidence {
            artifact: "d".repeat(64),
            package: "e".repeat(64),
            context: context(),
            window_start_ns: 1,
            window_end_ns: 2,
            complete: true,
            lost_events: 0,
            routes: vec![RouteEvidence {
                route: "svc/window".into(),
                adapter: None,
                generation: 7,
                native_admissions: 1,
                compat_admissions: 0,
                failed: 0,
                adapter_cpu_ns: Some(1),
                copied_bytes: Some(0),
            }],
        }
    }

    fn migration() -> MigrationPlan {
        MigrationPlan {
            candidate: "f".repeat(64),
            plan_digest: "1".repeat(64),
            context: context(),
            required_changes: vec!["switch candidate".into()],
            persistent_data_change: true,
            rollback_strategy: RollbackStrategy::RestoreSnapshot {
                snapshot_digest: "2".repeat(64),
            },
            rollback_execution_receipt: Some("3".repeat(64)),
            rollback_preconditions: vec!["candidate restart passed".into()],
            irreversible_changes: vec!["retire obsolete data".into()],
        }
    }

    fn authorization() -> DeploymentAuthorization {
        DeploymentAuthorization {
            schema: 1,
            candidate: "f".repeat(64),
            plan_digest: "1".repeat(64),
            context: context(),
            route_generations: vec![RouteGeneration {
                route: "svc/window".into(),
                generation: 7,
            }],
            state_retirement: Some(StateRetirementConsent {
                previous_state_digest: "2".repeat(64),
                only_after_health_check: true,
            }),
            satisfied_rollback_preconditions: vec!["candidate restart passed".into()],
            authorized_irreversible_changes: vec!["retire obsolete data".into()],
        }
    }

    #[test]
    fn binds_exact_route_generations_rollback_and_retirement() {
        let runtime = runtime();
        let migration = migration();
        let auth = authorization();
        assert_eq!(
            verify_authorization_bindings(&auth, &runtime, &migration).unwrap(),
            auth.route_generations
        );

        let mut changed = auth.clone();
        changed.route_generations[0].generation += 1;
        assert!(verify_authorization_bindings(&changed, &runtime, &migration).is_err());

        let mut changed = auth.clone();
        changed
            .state_retirement
            .as_mut()
            .unwrap()
            .only_after_health_check = false;
        assert!(verify_authorization_bindings(&changed, &runtime, &migration).is_err());

        let mut changed = auth.clone();
        changed.satisfied_rollback_preconditions.clear();
        assert!(verify_authorization_bindings(&changed, &runtime, &migration).is_err());

        let mut changed = auth.clone();
        changed.authorized_irreversible_changes.clear();
        assert!(verify_authorization_bindings(&changed, &runtime, &migration).is_err());

        let mut changed = auth;
        changed.state_retirement = None;
        assert!(verify_authorization_bindings(&changed, &runtime, &migration).is_err());

        let mut changed_runtime = runtime.clone();
        let duplicate_route = changed_runtime.routes[0].clone();
        changed_runtime.routes.push(duplicate_route);
        assert!(active_route_generations(&changed_runtime).is_err());
    }
}
