use crate::solver::{Plan, Problem, Solver, verify};
use crate::{
    Package, Requirements, SCHEMA, digest_valid, plan_digest, validate_catalog,
    validate_requirements,
};
use crate::{
    provenance::{Attestation, EvidenceVerifier, NoTrust, Role, SessionBinding, TrustAssurance},
    statistics::{StatisticalPolicy, Statistics, analyze},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    OsRuntime,
    HostModel,
    /// Actual physical-host processes running the routing library, not the KOLVRT OS.
    HostProcess,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub machine: String,
    pub workload: String,
    pub kernel: String,
    /// Digest of the predeclared protocol: fixtures, limits, order, warmup, stopping rule,
    /// oracle, environment/affinity, probe policy and primary metric (wall nanoseconds).
    pub protocol: String,
    pub source: EvidenceSource,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteEvidence {
    pub route: String,
    pub adapter: Option<String>,
    pub generation: u64,
    pub native_admissions: u64,
    pub compat_admissions: u64,
    pub failed: u64,
    /// Exclusive adapter CPU cost; unknown is not zero.
    pub adapter_cpu_ns: Option<u64>,
    pub copied_bytes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEvidence {
    pub artifact: String,
    pub package: String,
    pub context: Context,
    pub window_start_ns: u64,
    pub window_end_ns: u64,
    pub complete: bool,
    pub lost_events: u64,
    pub routes: Vec<RouteEvidence>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContractTest {
    pub package: String,
    pub contract: String,
    pub suite: String,
    pub context: Context,
    pub passed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pair {
    pub baseline_ns: u64,
    pub candidate_ns: u64,
    pub baseline_compat_admissions: u64,
    pub candidate_compat_admissions: u64,
    pub useful_units: u64,
    pub baseline_first: bool,
    pub oracle_passed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Benchmark {
    pub artifact: String,
    pub baseline_plan: String,
    pub candidate_plan: String,
    pub context: Context,
    pub statistics: StatisticalPolicy,
    pub stabilized: bool,
    pub failures: u64,
    pub timeouts: u64,
    pub unfinished: u64,
    /// Ordered, independent, paired runs after marked warmup (retained in artifact).
    pub pairs: Vec<Pair>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: u32,
    pub catalog: Vec<Package>,
    pub current: String,
    /// Actual installed dependency closure, not a solver-invented baseline.
    pub installed: Plan,
    pub requirements: Requirements,
    pub retained: Requirements,
    pub context: Context,
    pub runtime: Option<RuntimeEvidence>,
    pub tests: Vec<ContractTest>,
    pub benchmarks: Vec<Benchmark>,
    pub statistics: StatisticalPolicy,
    pub attestations: Vec<Attestation>,
    #[serde(default)]
    pub session: Option<SessionBinding>,
    pub migration_plans: Vec<MigrationPlan>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RollbackStrategy {
    RestoreSnapshot { snapshot_digest: String },
    ReinstallPrevious { previous_plan: String },
    Unavailable { reason: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationPlan {
    pub candidate: String,
    pub plan_digest: String,
    pub context: Context,
    pub required_changes: Vec<String>,
    pub persistent_data_change: bool,
    pub rollback_strategy: RollbackStrategy,
    pub rollback_preconditions: Vec<String>,
    pub irreversible_changes: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CandidateStatus {
    Blocked,
    RejectedCandidate,
    PartialEvidenceCandidate,
    VerifiedCandidate,
}

#[derive(Debug, Serialize)]
pub struct Assurance {
    pub contracts_passed: bool,
    pub runtime_complete: bool,
    pub provenance_verified: bool,
    pub statistical_gain_supported: bool,
    pub rollback_documented: bool,
    pub missing_checks: Vec<String>,
    pub trust: TrustAssurance,
}

#[derive(Debug, Serialize)]
pub struct Proposal {
    pub candidate: String,
    pub expected_gain: Option<Statistics>,
    pub required_changes: Option<Vec<String>>,
    pub rollback_strategy: Option<RollbackStrategy>,
    pub rollback_preconditions: Option<Vec<String>>,
    pub irreversible_changes: Option<Vec<String>>,
    pub deployment_authorized: bool,
}

#[derive(Debug, Serialize)]
pub struct Debt {
    pub evidence: String,
    pub complete: bool,
    pub native_admissions: u128,
    pub compat_admissions: u128,
    /// None with incomplete classification or no admissions.
    pub compat_share: Option<f64>,
    /// Known exclusive CPU cost only; missing costs are listed separately.
    pub costly_routes: Vec<(String, u64)>,
    pub unknown_cost_routes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Candidate {
    pub package: String,
    pub name: String,
    pub version: u32,
    pub plan: Option<Plan>,
    pub plan_digest: Option<String>,
    pub status: CandidateStatus,
    pub reason: String,
    pub benchmark: Option<String>,
    pub baseline_median_ns: Option<u64>,
    pub candidate_median_ns: Option<u64>,
    pub assurance: Assurance,
    pub proposal: Option<Proposal>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: u32,
    pub resolution_model: crate::solver::ResolutionModel,
    pub baseline: Plan,
    pub baseline_digest: String,
    pub debt: Option<Debt>,
    pub candidates: Vec<Candidate>,
    /// Fastest observed qualified median; not statistical superiority or authorization.
    pub preferred_observed: Option<String>,
    pub automatic_replacement: bool,
}

fn context_valid(c: &Context) -> bool {
    !c.machine.trim().is_empty()
        && [&c.workload, &c.kernel, &c.protocol]
            .iter()
            .all(|d| digest_valid(d))
}

fn validate(request: &Request) -> Result<(), String> {
    if request.schema != SCHEMA || !context_valid(&request.context) {
        return Err("unsupported schema or invalid context".into());
    }
    request.statistics.validate()?;
    validate_catalog(&request.catalog)?;
    validate_requirements(&request.requirements)?;
    validate_requirements(&request.retained)?;
    if let Some(e) = &request.runtime {
        let mut routes = BTreeSet::new();
        if !digest_valid(&e.artifact)
            || e.package != request.current
            || e.context != request.context
            || e.window_start_ns >= e.window_end_ns
            || e.routes.iter().any(|r| {
                r.route.trim().is_empty()
                    || r.generation == 0
                    || !routes.insert((&r.route, r.generation))
                    || r.adapter.as_ref().is_some_and(|d| !digest_valid(d))
                    || (r.compat_admissions > 0 && r.adapter.is_none())
                    || u128::from(r.failed)
                        > u128::from(r.native_admissions) + u128::from(r.compat_admissions)
            })
        {
            return Err("invalid or mismatched runtime evidence".into());
        }
    }
    let mut tests = BTreeSet::new();
    for t in &request.tests {
        if !digest_valid(&t.package)
            || !digest_valid(&t.contract)
            || !digest_valid(&t.suite)
            || !context_valid(&t.context)
            || !tests.insert((
                &t.package,
                &t.contract,
                &t.context.machine,
                &t.context.workload,
                &t.context.kernel,
                &t.context.protocol,
            ))
        {
            return Err("invalid or ambiguous contract evidence".into());
        }
    }
    let mut benchmarks = BTreeSet::new();
    let mut artifacts = BTreeSet::new();
    for b in &request.benchmarks {
        if !digest_valid(&b.artifact)
            || !artifacts.insert(&b.artifact)
            || !digest_valid(&b.baseline_plan)
            || !digest_valid(&b.candidate_plan)
            || !context_valid(&b.context)
            || !benchmarks.insert((&b.baseline_plan, &b.candidate_plan))
        {
            return Err("invalid or ambiguous benchmark evidence".into());
        }
    }
    let mut migrations = BTreeSet::new();
    for m in &request.migration_plans {
        let rollback_valid = match &m.rollback_strategy {
            RollbackStrategy::RestoreSnapshot { snapshot_digest } => digest_valid(snapshot_digest),
            RollbackStrategy::ReinstallPrevious { previous_plan } => {
                digest_valid(previous_plan) && !m.persistent_data_change
            }
            RollbackStrategy::Unavailable { reason } => !reason.trim().is_empty(),
        };
        if !digest_valid(&m.candidate)
            || !digest_valid(&m.plan_digest)
            || !context_valid(&m.context)
            || !migrations.insert(&m.candidate)
            || !rollback_valid
            || m.required_changes.is_empty()
            || m.rollback_preconditions.is_empty()
            || m.required_changes
                .iter()
                .chain(&m.rollback_preconditions)
                .chain(&m.irreversible_changes)
                .any(|s| s.trim().is_empty())
        {
            return Err("invalid or ambiguous migration/rollback plan".into());
        }
    }
    Ok(())
}

// Collect exact provider-specification and consumer-specification test results.
fn contract_gate(request: &Request, plan: &Plan, root: &str) -> (bool, bool, BTreeSet<usize>) {
    let selected: Vec<_> = plan
        .packages
        .iter()
        .map(|id| request.catalog.iter().find(|p| &p.identity == id).unwrap())
        .collect();
    let mut results = BTreeSet::new();
    let mut missing = false;
    let mut failed = false;
    let mut check = |providers: &[&Package], contract: &str| {
        let found = request
            .tests
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                t.contract == contract
                    && t.context == request.context
                    && providers.iter().any(|p| p.identity == t.package)
            })
            .collect::<Vec<_>>();
        if let Some((i, _)) = found.iter().find(|(_, t)| t.passed) {
            results.insert(*i);
        } else {
            missing = true;
            failed |= found.iter().any(|(_, t)| !t.passed);
        }
    };
    for p in &selected {
        for c in &p.provides {
            check(&[*p], &c.contract_digest);
        }
    }
    let root = *selected.iter().find(|p| p.identity == root).unwrap();
    let obligations = std::iter::once((&request.requirements, vec![root]))
        .chain(std::iter::once((&request.retained, selected.clone())))
        .chain(selected.iter().map(|p| (&p.requires, selected.clone())));
    for (requirements, providers) in obligations {
        for clause in requirements {
            // Select an alternative with a passing consumer-spec test, or report the gap.
            let evaluated: Vec<_> = clause
                .iter()
                .map(|r| {
                    let matches: Vec<_> = providers
                        .iter()
                        .copied()
                        .filter(|p| p.provides.iter().any(|c| r.matches(c)))
                        .collect();
                    (r, matches)
                })
                .filter(|(_, p)| !p.is_empty())
                .collect();
            let chosen = evaluated
                .iter()
                .find(|(r, providers)| {
                    request.tests.iter().any(|t| {
                        t.contract == r.contract_digest
                            && t.context == request.context
                            && t.passed
                            && providers.iter().any(|p| p.identity == t.package)
                    })
                })
                .or_else(|| evaluated.first());
            if let Some((r, providers)) = chosen {
                check(providers, &r.contract_digest);
            } else {
                check(&[], "");
            }
        }
    }
    (!missing, failed, results)
}

fn authenticate<T: Serialize>(
    request: &Request,
    verifier: &impl EvidenceVerifier,
    role: Role,
    payload: &T,
    references: Vec<String>,
    gaps: &mut Vec<String>,
) {
    let bytes = serde_json::to_vec(payload).expect("typed evidence serialization");
    if let Err(error) = verifier.authenticate(role, &bytes, &references, &request.attestations) {
        gaps.push(format!("{role:?}: {error}"));
    }
}
fn context_references(context: &Context) -> Vec<String> {
    vec![
        context.kernel.clone(),
        context.workload.clone(),
        context.protocol.clone(),
    ]
}

pub fn advise(request: &Request, solver: &impl Solver) -> Result<Report, String> {
    advise_with_verifier(request, solver, &NoTrust)
}

pub fn advise_with_verifier(
    request: &Request,
    solver: &impl Solver,
    verifier: &impl EvidenceVerifier,
) -> Result<Report, String> {
    validate(request)?;
    let session_context =
        serde_json::to_vec(&request.context).expect("typed context serialization");
    let session_error = verifier
        .begin_session(
            request.session.as_ref(),
            &session_context,
            &request.attestations,
        )
        .err();
    let trust_assurance = verifier.assurance();
    let current = request
        .catalog
        .iter()
        .find(|p| p.identity == request.current)
        .ok_or("unknown current package")?;
    let problem = |root| Problem {
        catalog: &request.catalog,
        requirements: &request.requirements,
        retained: &request.retained,
        root,
    };
    let baseline = request.installed.clone();
    verify(&problem(&request.current), &baseline)?;
    let baseline_digest = plan_digest(&baseline);
    let debt = request.runtime.as_ref().map(|e| {
        let native: u128 = e
            .routes
            .iter()
            .map(|r| u128::from(r.native_admissions))
            .sum();
        let compat: u128 = e
            .routes
            .iter()
            .map(|r| u128::from(r.compat_admissions))
            .sum();
        let complete = e.complete && e.lost_events == 0 && !e.routes.is_empty();
        let mut costly_routes: Vec<_> = e
            .routes
            .iter()
            .filter_map(|r| {
                r.adapter_cpu_ns
                    .map(|ns| (format!("{}@{}", r.route, r.generation), ns))
            })
            .collect();
        costly_routes.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        Debt {
            evidence: e.artifact.clone(),
            complete,
            native_admissions: native,
            compat_admissions: compat,
            compat_share: (complete && native + compat > 0)
                .then(|| compat as f64 / (native + compat) as f64),
            costly_routes,
            unknown_cost_routes: e
                .routes
                .iter()
                .filter(|r| r.adapter_cpu_ns.is_none())
                .map(|r| format!("{}@{}", r.route, r.generation))
                .collect(),
        }
    });
    let runtime_ready = debt
        .as_ref()
        .is_some_and(|d| d.complete && d.compat_admissions > 0)
        && request.context.source == EvidenceSource::OsRuntime;

    let (baseline_tested, baseline_failed, baseline_results) =
        contract_gate(request, &baseline, &request.current);
    let mut candidates = Vec::new();
    for p in &request.catalog {
        if p.identity == current.identity
            || (p.name == current.name && p.version <= current.version)
            || !crate::covers(&[p], &request.requirements)
        {
            continue;
        }
        let mut candidate = Candidate {
            package: p.identity.clone(),
            name: p.name.clone(),
            version: p.version,
            plan: None,
            plan_digest: None,
            status: CandidateStatus::Blocked,
            reason: String::new(),
            benchmark: None,
            baseline_median_ns: None,
            candidate_median_ns: None,
            assurance: Assurance {
                contracts_passed: false,
                runtime_complete: runtime_ready,
                provenance_verified: false,
                statistical_gain_supported: false,
                rollback_documented: false,
                missing_checks: vec![],
                trust: trust_assurance.clone(),
            },
            proposal: None,
        };
        match solver.solve(&problem(&p.identity)) {
            Err(e) => candidate.reason = format!("solver: {e:?}"),
            Ok(plan) => {
                verify(&problem(&p.identity), &plan)?;
                let digest = plan_digest(&plan);
                candidate.plan_digest = Some(digest.clone());
                let (candidate_tested, candidate_failed, results) =
                    contract_gate(request, &plan, &p.identity);
                candidate.assurance.contracts_passed = baseline_tested && candidate_tested;
                let mut provenance_gaps: Vec<String> = session_error.clone().into_iter().collect();
                let ids: BTreeSet<_> = baseline.packages.iter().chain(&plan.packages).collect();
                for id in ids {
                    let package = request.catalog.iter().find(|p| &p.identity == id).unwrap();
                    let mut refs = vec![package.identity.clone()];
                    refs.extend(package.provides.iter().map(|c| c.contract_digest.clone()));
                    refs.extend(
                        package
                            .requires
                            .iter()
                            .flatten()
                            .chain(&package.conflicts)
                            .map(|r| r.contract_digest.clone()),
                    );
                    authenticate(
                        request,
                        verifier,
                        Role::Catalog,
                        package,
                        refs,
                        &mut provenance_gaps,
                    );
                }
                for i in baseline_results.union(&results) {
                    let t = &request.tests[*i];
                    let mut refs = context_references(&t.context);
                    refs.extend([t.package.clone(), t.contract.clone(), t.suite.clone()]);
                    authenticate(
                        request,
                        verifier,
                        Role::ContractTest,
                        t,
                        refs,
                        &mut provenance_gaps,
                    );
                }
                if let Some(e) = &request.runtime {
                    let mut refs = context_references(&e.context);
                    refs.extend([e.artifact.clone(), e.package.clone()]);
                    refs.extend(e.routes.iter().filter_map(|r| r.adapter.clone()));
                    authenticate(
                        request,
                        verifier,
                        Role::Runtime,
                        e,
                        refs,
                        &mut provenance_gaps,
                    );
                } else {
                    provenance_gaps.push("Runtime: missing evidence".into());
                }
                let metadata = request.migration_plans.iter().find(|m| {
                    m.candidate == p.identity
                        && m.plan_digest == digest
                        && m.context == request.context
                });
                let rollback_ready = metadata.is_some_and(|m| {
                    m.irreversible_changes.is_empty()
                        && match &m.rollback_strategy {
                            RollbackStrategy::RestoreSnapshot { .. } => true,
                            RollbackStrategy::ReinstallPrevious { previous_plan } => {
                                *previous_plan == baseline_digest
                            }
                            RollbackStrategy::Unavailable { .. } => false,
                        }
                });
                if let Some(m) = metadata {
                    let mut refs = context_references(&m.context);
                    if let RollbackStrategy::RestoreSnapshot { snapshot_digest } =
                        &m.rollback_strategy
                    {
                        refs.push(snapshot_digest.clone());
                    }
                    authenticate(
                        request,
                        verifier,
                        Role::Proposal,
                        m,
                        refs,
                        &mut provenance_gaps,
                    );
                } else {
                    provenance_gaps.push("Proposal: missing matched rollback/change plan".into());
                }
                candidate.assurance.rollback_documented = rollback_ready;
                let mut stats = None;
                let mut benchmark_failed = false;
                let mut debt_reduced = false;
                if let Some(b) = request.benchmarks.iter().find(|b| {
                    b.baseline_plan == baseline_digest
                        && b.candidate_plan == digest
                        && b.context == request.context
                }) {
                    candidate.benchmark = Some(b.artifact.clone());
                    let mut refs = context_references(&b.context);
                    refs.push(b.artifact.clone());
                    authenticate(
                        request,
                        verifier,
                        Role::Benchmark,
                        b,
                        refs,
                        &mut provenance_gaps,
                    );
                    let valid = b.statistics == request.statistics
                        && b.stabilized
                        && b.failures == 0
                        && b.timeouts == 0
                        && b.unfinished == 0
                        && !b.pairs.is_empty()
                        && b.pairs.len() <= crate::statistics::MAX_PAIRS
                        && b.pairs.iter().all(|p| {
                            p.oracle_passed
                                && p.useful_units > 0
                                && p.baseline_ns > 0
                                && p.candidate_ns > 0
                        })
                        && b.pairs
                            .iter()
                            .all(|p| p.useful_units == b.pairs[0].useful_units)
                        && b.pairs
                            .windows(2)
                            .all(|p| p[0].baseline_first != p[1].baseline_first);
                    benchmark_failed = !valid;
                    if valid {
                        let baseline_compat: u128 = b
                            .pairs
                            .iter()
                            .map(|p| u128::from(p.baseline_compat_admissions))
                            .sum();
                        let candidate_compat: u128 = b
                            .pairs
                            .iter()
                            .map(|p| u128::from(p.candidate_compat_admissions))
                            .sum();
                        debt_reduced = candidate_compat < baseline_compat;
                        match analyze(&b.pairs, &request.statistics) {
                            Ok(result) => {
                                candidate.baseline_median_ns = Some(result.baseline.median_ns);
                                candidate.candidate_median_ns = Some(result.candidate.median_ns);
                                candidate.assurance.statistical_gain_supported =
                                    result.decision == "supported_latency_gain";
                                benchmark_failed |= result.decision == "tail_regression";
                                stats = Some(result);
                            }
                            Err(error) => candidate.assurance.missing_checks.push(error),
                        }
                    }
                } else {
                    provenance_gaps.push("Benchmark: missing matched A/B evidence".into());
                }
                provenance_gaps.sort();
                provenance_gaps.dedup();
                candidate.assurance.provenance_verified = provenance_gaps.is_empty();
                candidate.assurance.trust.evidence_chain_complete = provenance_gaps.is_empty();
                candidate.assurance.trust.session_verified =
                    trust_assurance.session_bound && session_error.is_none();
                candidate.assurance.trust.freshness_checked &= session_error.is_none();
                candidate.assurance.trust.replay_checked &= session_error.is_none();
                candidate.assurance.trust.producer_independence_satisfied =
                    candidate.assurance.trust.producer_independence_checked
                        && provenance_gaps.is_empty();
                candidate.assurance.missing_checks.extend(provenance_gaps);
                for (ready, gap) in [
                    (
                        candidate.assurance.contracts_passed,
                        "missing or failed provider/consumer contract tests",
                    ),
                    (
                        runtime_ready,
                        "runtime source/coverage does not establish a complete KOLVRT OS workload observation",
                    ),
                    (
                        candidate.assurance.statistical_gain_supported,
                        "latency gain or tail budget is not statistically supported",
                    ),
                    (
                        rollback_ready,
                        "missing rollback, wrong baseline, or irreversible changes",
                    ),
                    (
                        debt_reduced,
                        "paired benchmark does not show lower total compatibility use",
                    ),
                ] {
                    if !ready {
                        candidate.assurance.missing_checks.push(gap.into());
                    }
                }
                let rejected = baseline_failed || candidate_failed || benchmark_failed;
                candidate.status = if rejected {
                    CandidateStatus::RejectedCandidate
                } else if candidate.assurance.missing_checks.is_empty() {
                    CandidateStatus::VerifiedCandidate
                } else {
                    CandidateStatus::PartialEvidenceCandidate
                };
                candidate.reason = if rejected { "a contract/oracle/experimental validity check failed or a tail regression exceeded budget" }
                    else if candidate.status == CandidateStatus::VerifiedCandidate { "authenticated scoped evidence supports a migration proposal; deployment needs separate authorization" }
                    else { "dependency solution exists; evidence gaps are explicit and no strong preference is issued" }.into();
                candidate.proposal = Some(Proposal {
                    candidate: p.identity.clone(),
                    expected_gain: stats,
                    required_changes: metadata.map(|m| m.required_changes.clone()),
                    rollback_strategy: metadata.map(|m| m.rollback_strategy.clone()),
                    rollback_preconditions: metadata.map(|m| m.rollback_preconditions.clone()),
                    irreversible_changes: metadata.map(|m| m.irreversible_changes.clone()),
                    deployment_authorized: false,
                });
                candidate.plan = Some(plan);
            }
        }
        candidates.push(candidate);
    }
    candidates.sort_by(|a, b| a.package.cmp(&b.package));
    let mut qualified: Vec<_> = candidates
        .iter()
        .filter(|c| c.status == CandidateStatus::VerifiedCandidate)
        .collect();
    qualified.sort_by_key(|c| c.candidate_median_ns);
    let useful_units: BTreeSet<_> = qualified
        .iter()
        .map(|c| {
            request
                .benchmarks
                .iter()
                .find(|b| Some(&b.artifact) == c.benchmark.as_ref())
                .unwrap()
                .pairs[0]
                .useful_units
        })
        .collect();
    let preferred_observed = qualified
        .first()
        .filter(|first| {
            useful_units.len() == 1
                && qualified
                    .get(1)
                    .is_none_or(|second| first.candidate_median_ns != second.candidate_median_ns)
        })
        .map(|c| c.package.clone());
    Ok(Report {
        schema: SCHEMA,
        resolution_model: crate::solver::ResolutionModel::ReferenceSingleVersion,
        baseline,
        baseline_digest,
        debt,
        candidates,
        preferred_observed,
        automatic_replacement: false,
    })
}
