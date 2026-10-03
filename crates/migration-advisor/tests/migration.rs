use migration_advisor::{
    Capability, Package, Requirement, Requirements, SemanticVersion, advisor::*, plan_digest,
    solver::*,
};

fn hash(n: u8) -> String {
    format!("{n:064x}")
}
fn version(major: u32) -> SemanticVersion {
    SemanticVersion {
        major,
        minor: 0,
        patch: 0,
    }
}
fn policy() -> migration_advisor::statistics::StatisticalPolicy {
    let pilot = power_pilot();
    migration_advisor::statistics::StatisticalPolicy {
        min_resampling_blocks: 100,
        resampling_block_length: 1,
        bootstrap_resamples: 2000,
        confidence_basis_points: 9500,
        seed: 7,
        improvement_margin_ns: 5,
        p95_regression_budget_ns: 10,
        p99_regression_budget_ns: None,
        p95_memory_regression_budget_bytes: None,
        p95_copied_bytes_regression_budget: None,
        p95_energy_regression_budget_uj: None,
        consumer_resource_budgets: Vec::new(),
        prospective_power: Some(migration_advisor::statistics::ProspectivePowerPlan {
            pilot_digest: migration_advisor::provenance::subject_digest(
                &serde_json::to_vec(&pilot).unwrap(),
            ),
            expected_mean_gain_ns: 50,
            target_power_basis_points: 9000,
            planned_effective_blocks: 100,
        }),
    }
}
// Only this explicitly injected fixture verifier trusts synthetic assertions. The CLI never does.
struct FixtureVerifier;
impl migration_advisor::provenance::EvidenceVerifier for FixtureVerifier {
    fn authenticate(
        &self,
        _: migration_advisor::provenance::Role,
        _: &[u8],
        _: &[String],
        _: &[migration_advisor::provenance::Attestation],
    ) -> Result<(), String> {
        Ok(())
    }
}
fn trusted_fixture(r: &Request, solver: &impl Solver) -> Result<Report, String> {
    advise_with_verifier(r, solver, &FixtureVerifier)
}
fn requirement(name: &str) -> Requirement {
    Requirement {
        capability_id: name.into(),
        min_version: version(1),
        max_version: version(1),
        contract_digest: hash(99),
    }
}
fn requirement_major(name: &str, major: u32) -> Requirement {
    Requirement {
        capability_id: name.into(),
        min_version: version(major),
        max_version: version(major),
        contract_digest: hash(99),
    }
}
fn package(name: &str, id: u8, capability: &str) -> Package {
    Package {
        name: name.into(),
        version: 1,
        identity: hash(id),
        variant: None,
        provides: vec![Capability {
            capability_id: capability.into(),
            semantic_version: version(1),
            contract_digest: hash(99),
        }],
        requires: vec![],
        conflicts: vec![],
    }
}
fn variant_package(
    name: &str,
    id: u8,
    capability: &str,
    capability_major: u32,
    architecture: &str,
    abi: &str,
    build: &str,
) -> Package {
    let mut result = package(name, id, capability);
    result.provides[0].semantic_version = version(capability_major);
    result.variant = Some(migration_advisor::PackageVariant {
        architecture: architecture.into(),
        abi: abi.into(),
        build: build.into(),
    });
    result
}
fn solver() -> ClosureSolver {
    ClosureSolver { max_states: 1024 }
}
fn context() -> Context {
    Context {
        machine: "test-machine".into(),
        workload: hash(90),
        kernel: hash(91),
        protocol: hash(92),
        source: EvidenceSource::OsRuntime,
    }
}
fn power_pilot() -> PowerPilot {
    PowerPilot {
        artifact: hash(82),
        context: context(),
        paired_gains_ns: (0..100)
            .map(|i| if i % 2 == 0 { -50 } else { 50 })
            .collect(),
    }
}
fn request() -> Request {
    let current = package("old", 1, "window");
    let candidate = package("replacement", 2, "window");
    let installed = Plan {
        packages: vec![hash(1)],
    };
    let candidate_plan = Plan {
        packages: vec![hash(2)],
    };
    Request {
        schema: migration_advisor::SCHEMA,
        catalog: vec![current, candidate],
        current: hash(1),
        installed: installed.clone(),
        requirements: vec![vec![requirement("window")]],
        retained: vec![],
        context: context(),
        runtime: Some(RuntimeEvidence {
            artifact: hash(70),
            package: hash(1),
            context: context(),
            window_start_ns: 0,
            window_end_ns: 1000,
            complete: true,
            lost_events: 0,
            routes: vec![RouteEvidence {
                route: "compat.window".into(),
                adapter: Some(hash(71)),
                generation: 1,
                native_admissions: 0,
                compat_admissions: 10,
                failed: 0,
                adapter_cpu_ns: Some(100),
                copied_bytes: None,
            }],
        }),
        tests: [1, 2]
            .into_iter()
            .map(|id| ContractTest {
                package: hash(id),
                contract: hash(99),
                suite: hash(80),
                context: context(),
                passed: true,
            })
            .collect(),
        benchmarks: vec![Benchmark {
            artifact: hash(81),
            baseline_plan: plan_digest(&installed),
            candidate_plan: plan_digest(&candidate_plan),
            context: context(),
            statistics: policy(),
            stabilized: true,
            failures: 0,
            timeouts: 0,
            unfinished: 0,
            pairs: (0..100)
                .map(|i| Pair {
                    consumer_id: None,
                    baseline_ns: 100,
                    candidate_ns: 50,
                    request_latency: None,
                    baseline_memory_bytes: None,
                    candidate_memory_bytes: None,
                    baseline_copied_bytes: None,
                    candidate_copied_bytes: None,
                    baseline_energy_uj: None,
                    candidate_energy_uj: None,
                    baseline_compat_admissions: 10,
                    candidate_compat_admissions: 0,
                    useful_units: 10,
                    baseline_first: i % 2 == 0,
                    oracle_passed: true,
                })
                .collect(),
        }],
        statistics: policy(),
        power_pilot: Some(power_pilot()),
        attestations: vec![],
        migration_plans: vec![MigrationPlan {
            candidate: hash(2),
            plan_digest: plan_digest(&candidate_plan),
            context: context(),
            required_changes: vec!["replace route binding after explicit authorization".into()],
            persistent_data_change: false,
            rollback_strategy: RollbackStrategy::ReinstallPrevious {
                previous_plan: plan_digest(&installed),
            },
            rollback_preconditions: vec!["old artifact and compatible state retained".into()],
            irreversible_changes: vec![],
        }],
    }
}

fn refresh_power_pilot_binding(request: &mut Request) {
    if let (Some(pilot), Some(plan)) = (
        request.power_pilot.as_mut(),
        request.statistics.prospective_power.as_mut(),
    ) {
        pilot.context = request.context.clone();
        plan.pilot_digest =
            migration_advisor::provenance::subject_digest(&serde_json::to_vec(pilot).unwrap());
        for benchmark in &mut request.benchmarks {
            benchmark.statistics = request.statistics.clone();
        }
    }
}

#[test]
fn closure_alternatives_conflicts_and_independent_verifier() {
    let mut root = package("root", 1, "window");
    root.requires = vec![vec![requirement("unavailable"), requirement("storage")]];
    let mut bad = package("bad", 2, "storage");
    bad.conflicts = vec![requirement("window")];
    let good = package("good", 3, "storage");
    let catalog = vec![root, bad, good];
    let req = vec![vec![requirement("window")]];
    let retained: Requirements = vec![];
    let root = hash(1);
    let problem = Problem {
        catalog: &catalog,
        requirements: &req,
        root: &root,
        retained: &retained,
    };
    let result = solver().solve(&problem).unwrap();
    assert_eq!(result.packages, vec![hash(1), hash(3)]);
    assert_eq!(
        result,
        ExhaustiveSolver { max_states: 1000 }
            .solve(&problem)
            .unwrap()
    );
    assert!(
        verify(
            &problem,
            &Plan {
                packages: vec![hash(1)]
            }
        )
        .is_err()
    );
    assert!(
        verify(
            &problem,
            &Plan {
                packages: vec![hash(1), hash(2)]
            }
        )
        .is_err()
    );
    assert!(
        verify(
            &problem,
            &Plan {
                packages: vec![hash(1), hash(1)]
            }
        )
        .is_err()
    );
    assert!(
        verify(
            &problem,
            &Plan {
                packages: vec![hash(1), hash(44)]
            }
        )
        .is_err()
    );
    assert_eq!(
        ExhaustiveSolver { max_states: 1 }.solve(&problem),
        Err(SolveError::BudgetExceeded)
    );
}

#[test]
fn scoped_solver_selects_side_by_side_abi_and_build_variants() {
    let mut old_root = variant_package("app-old", 10, "window", 1, "aarch64", "abi-v1", "old");
    old_root.requires = vec![vec![requirement("runtime-v1")]];
    let mut new_root = variant_package("app-new", 11, "window", 1, "aarch64", "abi-v2", "new");
    new_root.requires = vec![vec![requirement_major("runtime-v2", 2)]];
    let mut runtime_v1 = variant_package(
        "runtime",
        12,
        "runtime-v1",
        1,
        "aarch64",
        "abi-v1",
        "build-101",
    );
    runtime_v1.conflicts = vec![requirement_major("runtime-v2", 2)];
    let mut runtime_v2 = variant_package(
        "runtime",
        13,
        "runtime-v2",
        2,
        "aarch64",
        "abi-v2",
        "build-202",
    );
    runtime_v2.conflicts = vec![requirement_major("runtime-v1", 1)];
    let wrong_arch = variant_package(
        "runtime",
        14,
        "runtime-v1",
        1,
        "x86_64",
        "abi-v1",
        "build-303",
    );
    let catalog = vec![old_root, new_root, runtime_v1, runtime_v2, wrong_arch];
    let app_requirement = vec![vec![requirement("window")]];
    let retained = vec![];
    let old_root_identity = hash(10);
    let new_root_identity = hash(11);
    let namespaces = vec![
        ScopedNamespace {
            id: "legacy-client",
            root: &old_root_identity,
            architecture: "aarch64",
            abi: "abi-v1",
            requirements: &app_requirement,
            retained: &retained,
        },
        ScopedNamespace {
            id: "next-client",
            root: &new_root_identity,
            architecture: "aarch64",
            abi: "abi-v2",
            requirements: &app_requirement,
            retained: &retained,
        },
    ];
    let problem = ScopedProblem {
        catalog: &catalog,
        namespaces: &namespaces,
    };
    let plan = ScopedClosureSolver { max_states: 1024 }
        .solve(&problem)
        .unwrap();
    verify_scoped(&problem, &plan).unwrap();
    assert_eq!(plan.selections.len(), 4);
    assert!(plan.selections.contains(&ScopedSelection {
        namespace: "legacy-client".into(),
        identity: hash(12),
    }));
    assert!(plan.selections.contains(&ScopedSelection {
        namespace: "next-client".into(),
        identity: hash(13),
    }));

    let mut invalid = plan;
    let selection = invalid
        .selections
        .iter_mut()
        .find(|item| item.namespace == "legacy-client" && item.identity == hash(12))
        .unwrap();
    selection.identity = hash(13);
    invalid.selections.sort();
    assert!(verify_scoped(&problem, &invalid).is_err());
}

#[test]
fn schema_three_requests_remain_readable_without_changing_legacy_package_payloads() {
    let mut request = request();
    request.schema = migration_advisor::COMPATIBLE_SCHEMA;
    let report = trusted_fixture(&request, &solver()).unwrap();
    assert_eq!(report.schema, migration_advisor::SCHEMA);
    let package: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&request.catalog[0]).unwrap()).unwrap();
    assert!(package.get("variant").is_none());
}

#[test]
fn unsatisfiable_cycles_versions_and_retained_consumers() {
    let mut a = package("a", 1, "window");
    let mut b = package("b", 2, "storage");
    a.requires = vec![vec![requirement("storage")]];
    b.requires = vec![vec![requirement("window")]];
    let mut catalog = vec![a, b];
    let req = vec![vec![requirement("window")]];
    let retained = vec![vec![requirement("storage")]];
    let root = hash(1);
    let solve = |catalog: &[Package]| {
        solver().solve(&Problem {
            catalog,
            requirements: &req,
            root: &root,
            retained: &retained,
        })
    };
    let solve_reference = |catalog: &[Package]| {
        ExhaustiveSolver { max_states: 1000 }.solve(&Problem {
            catalog,
            requirements: &req,
            root: &root,
            retained: &retained,
        })
    };
    assert_eq!(solve(&catalog).unwrap().packages.len(), 2);
    assert_eq!(solve(&catalog), solve_reference(&catalog));
    catalog[1].provides[0].semantic_version = version(2);
    assert_eq!(solve(&catalog), Err(SolveError::Unsatisfiable));
    assert_eq!(solve(&catalog), solve_reference(&catalog));
    catalog[1].provides[0].semantic_version = version(1);
    catalog[1].provides[0].contract_digest = hash(98);
    assert!(solve(&catalog).is_ok());
    assert_eq!(solve(&catalog), solve_reference(&catalog));
}

#[test]
fn closure_solver_scales_with_dependency_alternatives_not_all_subsets() {
    let chain_length = 120u8;
    let mut root = package("root", 1, "window");
    root.requires = vec![vec![requirement("dep-0")]];
    let mut catalog = vec![root];
    for index in 0..chain_length {
        let mut dependency = package(
            &format!("dependency-{index:03}"),
            index + 2,
            &format!("dep-{index}"),
        );
        if index + 1 < chain_length {
            dependency.requires = vec![vec![requirement(&format!("dep-{}", index + 1))]];
        }
        catalog.push(dependency);
    }
    // The direct provider forms a second, much smaller closure. The solver must explore both
    // branches and return the minimum plan after comparing their complete dependency closures.
    catalog.push(package("direct-provider", 200, "dep-0"));
    let requirements = vec![vec![requirement("window")]];
    let retained = vec![];
    let problem = Problem {
        catalog: &catalog,
        requirements: &requirements,
        root: &hash(1),
        retained: &retained,
    };

    let plan = ClosureSolver { max_states: 122 }.solve(&problem).unwrap();
    assert_eq!(plan.packages, vec![hash(1), hash(200)]);
    verify(&problem, &plan).unwrap();
    assert_eq!(
        ClosureSolver { max_states: 121 }.solve(&problem),
        Err(SolveError::BudgetExceeded)
    );
    assert!(matches!(
        ExhaustiveSolver { max_states: 1000 }.solve(&problem),
        Err(SolveError::Invalid(_))
    ));
}

#[test]
fn qualified_observation_does_not_authorize_replacement() {
    let report = trusted_fixture(&request(), &solver()).unwrap();
    assert_eq!(report.preferred_observed, Some(hash(2)));
    assert!(!report.automatic_replacement);
    assert_eq!(report.debt.unwrap().compat_share, Some(1.0));
    assert_eq!(report.candidates[0].candidate_median_ns, Some(50));
}

#[test]
fn missing_failed_stale_or_synthetic_evidence_never_recommends() {
    let mutations: Vec<fn(&mut Request)> = vec![
        |r| r.tests[1].passed = false,
        |r| r.tests[1].package = hash(33),
        |r| r.tests[1].contract = hash(33),
        |r| r.tests[1].context.workload = hash(33),
        |r| r.benchmarks[0].candidate_plan = hash(33),
        |r| r.benchmarks[0].context.machine = "another machine".into(),
        |r| r.benchmarks[0].failures = 1,
        |r| r.benchmarks[0].timeouts = 1,
        |r| r.benchmarks[0].unfinished = 1,
        |r| r.benchmarks[0].stabilized = false,
        |r| r.benchmarks[0].pairs[0].oracle_passed = false,
        |r| r.benchmarks[0].pairs[0].candidate_ns = 20000,
        |r| {
            for p in &mut r.benchmarks[0].pairs {
                p.candidate_compat_admissions = 11;
            }
        },
        |r| r.benchmarks[0].pairs[0].useful_units = 9,
        |r| r.benchmarks[0].pairs[1].baseline_first = true,
        |r| {
            r.benchmarks[0].pairs.pop();
        },
        |r| r.runtime.as_mut().unwrap().lost_events = 1,
        |r| r.runtime.as_mut().unwrap().complete = false,
        |r| r.runtime = None,
        |r| {
            r.context.source = EvidenceSource::HostModel;
            r.runtime.as_mut().unwrap().context = r.context.clone();
            for t in &mut r.tests {
                t.context = r.context.clone();
            }
            r.benchmarks[0].context = r.context.clone();
            refresh_power_pilot_binding(r);
        },
    ];
    for (i, mutate) in mutations.into_iter().enumerate() {
        let mut r = request();
        mutate(&mut r);
        assert!(
            trusted_fixture(&r, &solver())
                .unwrap()
                .preferred_observed
                .is_none(),
            "mutation {i}"
        );
    }
}

#[test]
fn malformed_evidence_and_manifests_are_rejected() {
    let mutations: Vec<fn(&mut Request)> = vec![
        |r| r.schema = 99,
        |r| r.requirements[0].clear(),
        |r| r.catalog[1].identity = r.catalog[0].identity.clone(),
        |r| {
            let c = r.catalog[1].provides[0].clone();
            r.catalog[1].provides.push(c);
        },
        |r| r.tests.push(r.tests[0].clone()),
        |r| r.benchmarks.push(r.benchmarks[0].clone()),
        |r| r.runtime.as_mut().unwrap().context.workload = hash(33),
        |r| r.runtime.as_mut().unwrap().window_end_ns = 0,
        |r| r.installed.packages = vec![hash(44)],
    ];
    for mutate in mutations {
        let mut r = request();
        mutate(&mut r);
        assert!(trusted_fixture(&r, &solver()).is_err());
    }
}

#[test]
fn zero_observation_and_unknown_cost_do_not_erase_requirements() {
    let mut r = request();
    let route = &mut r.runtime.as_mut().unwrap().routes[0];
    route.compat_admissions = 0;
    route.adapter_cpu_ns = None;
    let report = trusted_fixture(&r, &solver()).unwrap();
    assert!(report.preferred_observed.is_none());
    let debt = report.debt.unwrap();
    assert_eq!(debt.compat_share, None);
    assert_eq!(debt.unknown_cost_routes.len(), 1);
    r.catalog[1].provides[0].contract_digest = hash(44);
    assert_eq!(
        trusted_fixture(&r, &solver()).unwrap().candidates[0].status,
        CandidateStatus::PartialEvidenceCandidate
    );
}

#[test]
fn untrusted_solver_cannot_bypass_closure_check() {
    struct Liar;
    impl Solver for Liar {
        fn solve(&self, _: &Problem<'_>) -> Result<Plan, SolveError> {
            Ok(Plan {
                packages: vec![hash(44)],
            })
        }
    }
    assert!(trusted_fixture(&request(), &Liar).is_err());
}

#[test]
fn cli_emits_read_only_report_and_rejects_unknown_fields() {
    let mut r = request();
    r.context.source = EvidenceSource::HostModel;
    r.runtime = None;
    r.tests.clear();
    r.benchmarks.clear();
    refresh_power_pilot_binding(&mut r);
    let path = std::env::temp_dir().join(format!("kolvrt-advisor-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_vec(&r).unwrap()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_migration-advisor"))
        .arg("advise")
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["automatic_replacement"], false);
    assert!(report["preferred_observed"].is_null());
    let mut json = serde_json::to_value(r).unwrap();
    json["auto_install"] = true.into();
    std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_migration-advisor"))
        .arg("advise")
        .arg(&path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn tied_or_unequal_work_candidates_have_no_preference() {
    let mut r = request();
    r.catalog.push(package("third", 3, "window"));
    let mut test = r.tests[1].clone();
    test.package = hash(3);
    r.tests.push(test);
    let mut benchmark = r.benchmarks[0].clone();
    benchmark.artifact = hash(83);
    benchmark.candidate_plan = plan_digest(&Plan {
        packages: vec![hash(3)],
    });
    r.benchmarks.push(benchmark);
    let mut m = r.migration_plans[0].clone();
    m.candidate = hash(3);
    m.plan_digest = r.benchmarks[1].candidate_plan.clone();
    r.migration_plans.push(m);
    assert!(
        trusted_fixture(&r, &solver())
            .unwrap()
            .preferred_observed
            .is_none()
    );
    let report = trusted_fixture(&r, &solver()).unwrap();
    assert_eq!(
        report.candidates[0]
            .proposal
            .as_ref()
            .unwrap()
            .expected_gain
            .as_ref()
            .unwrap()
            .comparison_family_size,
        2
    );
    for p in &mut r.benchmarks[1].pairs {
        p.candidate_ns = 40;
    }
    assert_eq!(
        trusted_fixture(&r, &solver()).unwrap().preferred_observed,
        Some(hash(3))
    );
    for p in &mut r.benchmarks[1].pairs {
        p.useful_units = 20;
    }
    assert!(
        trusted_fixture(&r, &solver())
            .unwrap()
            .preferred_observed
            .is_none()
    );
}

#[test]
fn newer_versions_are_candidates_and_dependency_contracts_are_required() {
    let mut r = request();
    r.catalog[1].name = "old".into();
    r.catalog[0].version = 2;
    assert!(
        trusted_fixture(&r, &solver())
            .unwrap()
            .candidates
            .is_empty()
    );
    r.catalog[1].version = 3;
    assert_eq!(trusted_fixture(&r, &solver()).unwrap().candidates.len(), 1);
    r.catalog[1].requires = vec![vec![requirement("storage")]];
    r.catalog.push(package("storage", 3, "storage"));
    r.benchmarks[0].candidate_plan = plan_digest(&Plan {
        packages: vec![hash(2), hash(3)],
    });
    r.migration_plans[0].plan_digest = r.benchmarks[0].candidate_plan.clone();
    assert!(
        trusted_fixture(&r, &solver())
            .unwrap()
            .preferred_observed
            .is_none()
    );
    r.tests.push(ContractTest {
        package: hash(3),
        contract: hash(99),
        suite: hash(80),
        context: context(),
        passed: true,
    });
    assert_eq!(
        trusted_fixture(&r, &solver()).unwrap().preferred_observed,
        Some(hash(2))
    );
}

#[test]
fn unsigned_claims_and_partial_telemetry_remain_visible_without_strong_preference() {
    let r = request();
    let report = migration_advisor::advisor::advise(&r, &solver()).unwrap();
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::PartialEvidenceCandidate
    );
    assert!(!report.candidates[0].assurance.provenance_verified);
    assert!(report.preferred_observed.is_none());
    assert!(
        report.candidates[0]
            .proposal
            .as_ref()
            .unwrap()
            .expected_gain
            .is_some()
    );
    let mut r = r;
    r.runtime.as_mut().unwrap().lost_events = 1;
    let report = trusted_fixture(&r, &solver()).unwrap();
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::PartialEvidenceCandidate
    );
    assert!(report.candidates[0].assurance.contracts_passed);
    assert!(report.candidates[0].assurance.statistical_gain_supported);
    assert!(!report.candidates[0].assurance.runtime_complete);
    assert!(report.preferred_observed.is_none());
}

#[test]
fn semantic_extension_needs_both_provider_and_consumer_spec_tests() {
    let mut r = request();
    r.requirements[0][0].max_version.minor = 9;
    r.catalog[1].provides[0].semantic_version.minor = 1;
    r.catalog[1].provides[0].contract_digest = hash(98);
    assert_eq!(
        trusted_fixture(&r, &solver()).unwrap().candidates[0].status,
        CandidateStatus::PartialEvidenceCandidate
    );
    let mut provider_spec = r.tests[1].clone();
    provider_spec.contract = hash(98);
    r.tests.push(provider_spec);
    assert_eq!(
        trusted_fixture(&r, &solver()).unwrap().candidates[0].status,
        CandidateStatus::VerifiedCandidate
    );
    r.tests.remove(1); // New provider spec alone cannot prove the consumer's old spec.
    assert!(
        !trusted_fixture(&r, &solver()).unwrap().candidates[0]
            .assurance
            .contracts_passed
    );
    r.catalog[1].provides[0].semantic_version.major = 2;
    assert!(
        trusted_fixture(&r, &solver())
            .unwrap()
            .candidates
            .is_empty()
    );
}

#[test]
fn request_latency_quantiles_are_descriptive_and_not_run_independence_units() {
    use migration_advisor::statistics::analyze_family;

    let mut r = request();
    for (run, pair) in r.benchmarks[0].pairs.iter_mut().enumerate() {
        pair.request_latency = Some(RequestLatencySamples {
            sample_stride: 241,
            sample_width: 16,
            baseline_ns: vec![1000 + run as u64; 128],
            candidate_ns: vec![900 + run as u64; 128],
        });
    }
    let statistics = analyze_family(&r.benchmarks[0].pairs, &policy(), 1).unwrap();
    let requests = statistics.request_latency.as_ref().unwrap();
    assert_eq!(statistics.paired_runs, 100);
    assert_eq!(statistics.resampling_blocks, 100);
    assert_eq!(requests.sample_stride, 241);
    assert_eq!(requests.sample_width, 16);
    assert_eq!(requests.sampled_groups_per_path, 12_800);
    assert_eq!(requests.baseline.p95_ns, Some(1094));
    assert_eq!(requests.candidate.p95_ns, Some(994));

    r.benchmarks[0].pairs[0].request_latency = None;
    assert!(analyze_family(&r.benchmarks[0].pairs, &policy(), 1).is_err());
}

#[test]
fn one_noisy_pair_is_tolerated_but_uncertain_effect_and_tail_regression_are_not() {
    let mut r = request();
    r.benchmarks[0].pairs[0].candidate_ns = 110;
    assert_eq!(
        trusted_fixture(&r, &solver()).unwrap().preferred_observed,
        Some(hash(2))
    );
    for (i, p) in r.benchmarks[0].pairs.iter_mut().enumerate() {
        p.candidate_ns = if i % 2 == 0 { 50 } else { 150 };
    }
    assert!(
        trusted_fixture(&r, &solver())
            .unwrap()
            .preferred_observed
            .is_none()
    );
    for (i, p) in r.benchmarks[0].pairs.iter_mut().enumerate() {
        p.candidate_ns = if i < 10 { 120 } else { 50 };
    }
    let report = trusted_fixture(&r, &solver()).unwrap();
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::RejectedCandidate
    );
    assert_eq!(
        report.candidates[0]
            .proposal
            .as_ref()
            .unwrap()
            .expected_gain
            .as_ref()
            .unwrap()
            .decision,
        "tail_regression"
    );
    r.statistics.p99_regression_budget_ns = Some(10);
    r.benchmarks[0].statistics = r.statistics.clone();
    for p in &mut r.benchmarks[0].pairs {
        p.candidate_ns = 50;
    }
    assert_eq!(
        trusted_fixture(&r, &solver()).unwrap().candidates[0].status,
        CandidateStatus::PartialEvidenceCandidate
    );
}

#[test]
fn configured_multidimensional_budgets_require_evidence_and_reject_regressions() {
    use migration_advisor::statistics::analyze_family;

    let mut resource_policy = policy();
    resource_policy.p95_memory_regression_budget_bytes = Some(0);
    resource_policy.p95_copied_bytes_regression_budget = Some(8);
    resource_policy.p95_energy_regression_budget_uj = Some(0);
    let make_pairs = |memory: Option<u64>, copied: Option<u64>, energy: Option<u64>| {
        (0..100)
            .map(|i| Pair {
                consumer_id: None,
                baseline_ns: 100,
                candidate_ns: 50,
                request_latency: None,
                baseline_memory_bytes: Some(1024),
                candidate_memory_bytes: memory,
                baseline_copied_bytes: Some(16),
                candidate_copied_bytes: copied,
                baseline_energy_uj: Some(100),
                candidate_energy_uj: energy,
                baseline_compat_admissions: 1,
                candidate_compat_admissions: 0,
                useful_units: 1,
                baseline_first: i % 2 == 0,
                oracle_passed: true,
            })
            .collect::<Vec<_>>()
    };

    let missing =
        analyze_family(&make_pairs(Some(1000), Some(16), None), &resource_policy, 1).unwrap();
    assert_eq!(missing.decision, "insufficient_resource_evidence");

    let within = analyze_family(
        &make_pairs(Some(1024), Some(16), Some(100)),
        &resource_policy,
        1,
    )
    .unwrap();
    assert_eq!(within.decision, "supported_latency_gain");
    assert_eq!(within.memory_bytes.unwrap().candidate.p95, Some(1024));

    let regression = analyze_family(
        &make_pairs(Some(1024), Some(25), Some(100)),
        &resource_policy,
        1,
    )
    .unwrap();
    assert_eq!(regression.decision, "resource_regression");
}

#[test]
fn resource_budget_regression_rejects_the_advisor_candidate() {
    let mut request = request();
    request.statistics.p95_copied_bytes_regression_budget = Some(0);
    request.benchmarks[0].statistics = request.statistics.clone();
    for pair in &mut request.benchmarks[0].pairs {
        pair.baseline_copied_bytes = Some(8);
        pair.candidate_copied_bytes = Some(16);
    }

    let report = trusted_fixture(&request, &solver()).unwrap();
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::RejectedCandidate
    );
    assert!(!report.candidates[0].assurance.statistical_gain_supported);
}

#[test]
fn consumer_specific_resource_budgets_fail_closed_and_report_each_consumer() {
    use migration_advisor::statistics::{ConsumerResourceBudget, analyze_family};

    let mut policy = policy();
    policy.consumer_resource_budgets = ["client-a", "client-b"]
        .into_iter()
        .map(|consumer_id| ConsumerResourceBudget {
            consumer_id: consumer_id.into(),
            p95_memory_regression_budget_bytes: Some(0),
            p95_copied_bytes_regression_budget: Some(0),
            p95_energy_regression_budget_uj: None,
        })
        .collect();
    let pairs = (0..200)
        .map(|i| {
            let consumer_id = if i < 100 { "client-a" } else { "client-b" };
            Pair {
                consumer_id: Some(consumer_id.into()),
                baseline_ns: 100,
                candidate_ns: 50,
                request_latency: None,
                baseline_memory_bytes: Some(1024),
                candidate_memory_bytes: Some(if consumer_id == "client-a" {
                    1024
                } else {
                    1032
                }),
                baseline_copied_bytes: Some(16),
                candidate_copied_bytes: Some(0),
                baseline_energy_uj: None,
                candidate_energy_uj: None,
                baseline_compat_admissions: 1,
                candidate_compat_admissions: 0,
                useful_units: 1,
                baseline_first: i % 2 == 0,
                oracle_passed: true,
            }
        })
        .collect::<Vec<_>>();
    let assessment = analyze_family(&pairs, &policy, 1).unwrap();
    assert_eq!(assessment.decision, "resource_regression");
    assert_eq!(assessment.consumer_resources.len(), 2);
    assert_eq!(assessment.consumer_resources[0].consumer_id, "client-a");
    assert_eq!(
        assessment.consumer_resources[0].memory_within_budget,
        Some(true)
    );
    assert_eq!(assessment.consumer_resources[1].consumer_id, "client-b");
    assert_eq!(
        assessment.consumer_resources[1].memory_within_budget,
        Some(false)
    );

    let mut split_block_policy = policy.clone();
    split_block_policy.resampling_block_length = 2;
    let mut split_block = pairs.clone();
    split_block[1].consumer_id = Some("client-b".into());
    assert_eq!(
        analyze_family(&split_block, &split_block_policy, 1)
            .unwrap()
            .decision,
        "insufficient_resource_evidence"
    );

    let mut duplicate_budget = policy.clone();
    duplicate_budget
        .consumer_resource_budgets
        .push(duplicate_budget.consumer_resource_budgets[0].clone());
    assert!(analyze_family(&pairs, &duplicate_budget, 1).is_err());

    let mut missing_identity = pairs;
    missing_identity[0].consumer_id = None;
    assert_eq!(
        analyze_family(&missing_identity, &policy, 1)
            .unwrap()
            .decision,
        "insufficient_resource_evidence"
    );
}

#[test]
fn authenticated_consumer_resource_regression_rejects_the_candidate() {
    use migration_advisor::statistics::ConsumerResourceBudget;

    let mut request = request();
    request.statistics.consumer_resource_budgets = vec![ConsumerResourceBudget {
        consumer_id: "window-client/1".into(),
        p95_memory_regression_budget_bytes: Some(0),
        p95_copied_bytes_regression_budget: None,
        p95_energy_regression_budget_uj: None,
    }];
    request.benchmarks[0].statistics = request.statistics.clone();
    for pair in &mut request.benchmarks[0].pairs {
        pair.consumer_id = Some("window-client/1".into());
        pair.baseline_memory_bytes = Some(1024);
        pair.candidate_memory_bytes = Some(1032);
    }
    let report = trusted_fixture(&request, &solver()).unwrap();
    let candidate = &report.candidates[0];
    assert_eq!(candidate.status, CandidateStatus::RejectedCandidate);
    assert_eq!(
        candidate
            .proposal
            .as_ref()
            .unwrap()
            .expected_gain
            .as_ref()
            .unwrap()
            .consumer_resources[0]
            .memory_within_budget,
        Some(false)
    );
}

#[test]
fn small_samples_cannot_supply_tails_and_bootstrap_is_reproducible() {
    use migration_advisor::statistics::analyze;
    let mut r = request();
    r.benchmarks[0].pairs.truncate(30);
    let mut policy = policy();
    policy.min_resampling_blocks = 30;
    let result = analyze(&r.benchmarks[0].pairs, &policy).unwrap();
    assert!(result.baseline.p95_ns.is_none());
    assert!(result.baseline.p99_ns.is_none());
    assert_eq!(result.decision, "insufficient_tail_evidence");
    let repeated = analyze(&r.benchmarks[0].pairs, &policy).unwrap();
    assert_eq!(result.gain_interval_ns, repeated.gain_interval_ns);
    r.benchmarks[0].pairs.truncate(2);
    assert!(analyze(&r.benchmarks[0].pairs, &policy).is_err());
    let r = request();
    let pairs: Vec<_> = r.benchmarks[0]
        .pairs
        .iter()
        .cloned()
        .cycle()
        .take(1000)
        .collect();
    policy.min_resampling_blocks = 1000;
    policy.p99_regression_budget_ns = Some(10);
    let result = analyze(&pairs, &policy).unwrap();
    assert_eq!(result.candidate.p99_ns, Some(50));
    assert_eq!(result.decision, "supported_latency_gain");
}

#[test]
fn multiple_candidate_intervals_are_adjusted_and_need_enough_tail_resamples() {
    use migration_advisor::statistics::{analyze, analyze_family};
    let mut r = request();
    for (i, pair) in r.benchmarks[0].pairs.iter_mut().enumerate() {
        pair.baseline_ns = 200 + i as u64;
        pair.candidate_ns = 100 + (i % 7) as u64 * 10;
    }
    let pairs = &r.benchmarks[0].pairs;
    let single = analyze(pairs, &policy()).unwrap();
    let family = analyze_family(pairs, &policy(), 2).unwrap();
    assert!(family.gain_interval_ns[0] <= single.gain_interval_ns[0]);
    assert!(family.gain_interval_ns[1] >= single.gain_interval_ns[1]);
    assert_eq!(family.comparison_family_size, 2);
    assert_eq!(family.comparison_confidence_basis_points, 9750.0);
    assert!(family.comparison_tail_resamples >= 20);
    let broad_family = analyze_family(pairs, &policy(), 100).unwrap();
    assert_eq!(broad_family.decision, "inadequate_comparison_resolution");
    assert!(!broad_family.decision.eq("supported_latency_gain"));
}

#[test]
fn an_unmeasured_catalog_candidate_still_counts_in_the_comparison_family() {
    let mut r = request();
    r.catalog.push(package("another-replacement", 3, "window"));
    let report = trusted_fixture(&r, &solver()).unwrap();
    let gain = report.candidates[0]
        .proposal
        .as_ref()
        .unwrap()
        .expected_gain
        .as_ref()
        .unwrap();
    assert_eq!(gain.comparison_family_size, 2);
    assert_eq!(gain.decision, "supported_latency_gain");
    assert!(report.candidates.iter().any(|candidate| {
        candidate.package == hash(3)
            && candidate
                .proposal
                .as_ref()
                .is_some_and(|proposal| proposal.expected_gain.is_none())
    }));
}

#[test]
fn moving_block_bootstrap_uses_ordered_pairs_and_effective_tail_count() {
    use migration_advisor::statistics::analyze_family;
    let r = request();
    let mut pairs = r.benchmarks[0].pairs[..60].to_vec();
    for (i, pair) in pairs.iter_mut().enumerate() {
        let gain = if (i / 10) % 2 == 0 {
            80 + (i % 10) as u64
        } else {
            20 + (i % 10) as u64
        };
        pair.baseline_ns = 200 + gain;
        pair.candidate_ns = 200;
    }
    let mut policy = policy();
    policy.min_resampling_blocks = 30;
    policy.resampling_block_length = 2;
    let result = analyze_family(&pairs, &policy, 1).unwrap();
    assert_eq!(result.paired_runs, 60);
    assert_eq!(result.resampling_block_length, 2);
    assert_eq!(result.resampling_blocks, 30);
    assert!(result.dependence.lag_one_autocorrelation.unwrap() > 0.5);
    assert!(result.dependence.estimated_effective_pairs.unwrap() < 60.0);
    assert!(result.baseline.p95_ns.is_none());
    assert_eq!(
        result.gain_interval_ns,
        analyze_family(&pairs, &policy, 1).unwrap().gain_interval_ns
    );
    policy.resampling_block_length = 3;
    assert!(analyze_family(&pairs, &policy, 1).is_err());
}

#[test]
fn prospective_power_uses_separate_pilot_and_blocks_unplanned_gain_claims() {
    use migration_advisor::statistics::analyze_family_with_power;
    let request = request();
    let benchmark = &request.benchmarks[0];
    let pilot = &request.power_pilot.as_ref().unwrap().paired_gains_ns;
    let result =
        analyze_family_with_power(&benchmark.pairs, &benchmark.statistics, 1, Some(pilot)).unwrap();
    assert_eq!(result.decision, "supported_latency_gain");
    let assessment = result.prospective_power.unwrap();
    assert!(assessment.adequate);
    assert_eq!(assessment.pilot_pairs, 100);
    assert_eq!(assessment.planned_effective_blocks, 100);
    assert!(assessment.required_effective_blocks.unwrap() <= 100);

    let result =
        analyze_family_with_power(&benchmark.pairs, &benchmark.statistics, 1, None).unwrap();
    assert_eq!(result.decision, "missing_power_pilot");

    let mut underplanned = benchmark.statistics.clone();
    underplanned
        .prospective_power
        .as_mut()
        .unwrap()
        .planned_effective_blocks = 101;
    let result =
        analyze_family_with_power(&benchmark.pairs, &underplanned, 1, Some(pilot)).unwrap();
    assert_eq!(result.decision, "insufficient_prospective_power");
    assert!(!result.prospective_power.unwrap().adequate);

    let mut overrun = benchmark.statistics.clone();
    overrun
        .prospective_power
        .as_mut()
        .unwrap()
        .planned_effective_blocks = 99;
    let result = analyze_family_with_power(&benchmark.pairs, &overrun, 1, Some(pilot)).unwrap();
    assert_eq!(result.decision, "insufficient_prospective_power");
    assert!(!result.prospective_power.unwrap().adequate);

    let mut undeclared = benchmark.statistics.clone();
    undeclared.prospective_power = None;
    let result = analyze_family_with_power(&benchmark.pairs, &undeclared, 1, None).unwrap();
    assert_eq!(result.decision, "missing_prospective_power");
}

#[test]
fn rollback_and_irreversible_changes_are_part_of_every_solved_proposal() {
    let mut r = request();
    r.migration_plans.clear();
    let report = trusted_fixture(&r, &solver()).unwrap();
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::PartialEvidenceCandidate
    );
    assert!(
        report.candidates[0]
            .proposal
            .as_ref()
            .unwrap()
            .rollback_strategy
            .is_none()
    );
    let mut r = request();
    r.migration_plans[0].persistent_data_change = true;
    assert!(trusted_fixture(&r, &solver()).is_err()); // Reinstall cannot restore persistent data.
    r.migration_plans[0].rollback_strategy = RollbackStrategy::RestoreSnapshot {
        snapshot_digest: hash(55),
    };
    r.migration_plans[0]
        .irreversible_changes
        .push("external durable state cannot be restored".into());
    let report = trusted_fixture(&r, &solver()).unwrap();
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::PartialEvidenceCandidate
    );
    let proposal = report.candidates[0].proposal.as_ref().unwrap();
    assert_eq!(proposal.irreversible_changes.as_ref().unwrap().len(), 1);
    assert!(!proposal.deployment_authorized);
}

#[test]
fn signed_chain_and_cli_verify_bytes_but_tampering_downgrades_the_candidate() {
    use ed25519_dalek::{Signer, SigningKey};
    use migration_advisor::provenance::*;
    use std::collections::BTreeMap;
    let root = std::env::temp_dir().join(format!("kolvrt-advisor-chain-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    // Synthetic content with genuine hashes/signatures exercises provenance, not real performance.
    let mut replacements = BTreeMap::new();
    fn replace(
        value: &mut serde_json::Value,
        root: &std::path::Path,
        replacements: &mut BTreeMap<String, String>,
    ) {
        match value {
            serde_json::Value::String(s) if migration_advisor::digest_valid(s) => {
                let new = replacements.entry(s.clone()).or_insert_with(|| {
                    let raw = format!("synthetic artifact bytes for {s}");
                    let digest = subject_digest(raw.as_bytes());
                    std::fs::write(root.join(&digest), raw).unwrap();
                    digest
                });
                *s = new.clone();
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    replace(v, root, replacements);
                }
            }
            serde_json::Value::Object(o) => {
                for v in o.values_mut() {
                    replace(v, root, replacements);
                }
            }
            _ => {}
        }
    }
    let mut json = serde_json::to_value(request()).unwrap();
    replace(&mut json, &root, &mut replacements);
    let mut r: Request = serde_json::from_value(json).unwrap();
    r.benchmarks[0].baseline_plan = plan_digest(&r.installed);
    r.benchmarks[0].candidate_plan = plan_digest(&Plan {
        packages: vec![r.catalog[1].identity.clone()],
    });
    r.migration_plans[0].plan_digest = r.benchmarks[0].candidate_plan.clone();
    r.migration_plans[0].rollback_strategy = RollbackStrategy::ReinstallPrevious {
        previous_plan: r.benchmarks[0].baseline_plan.clone(),
    };
    if let (Some(power), Some(pilot)) = (
        r.statistics.prospective_power.as_mut(),
        r.power_pilot.as_ref(),
    ) {
        power.pilot_digest = subject_digest(&serde_json::to_vec(pilot).unwrap());
    }
    r.benchmarks[0].statistics = r.statistics.clone();
    let key = SigningKey::from_bytes(&[44; 32]);
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let sign = |role, payload: Vec<u8>| {
        let digest = subject_digest(&payload);
        Attestation {
            role,
            subject_digest: digest.clone(),
            key_id: "fixture-producer".into(),
            signature: hex(&key.sign(&signing_message(role, &digest)).to_bytes()),
            issued_at_unix_seconds: None,
            session_id: None,
        }
    };
    r.attestations.push(sign(
        Role::Benchmark,
        serde_json::to_vec(r.power_pilot.as_ref().unwrap()).unwrap(),
    ));
    for p in &r.catalog {
        r.attestations
            .push(sign(Role::Catalog, serde_json::to_vec(p).unwrap()));
    }
    for t in &r.tests {
        r.attestations
            .push(sign(Role::ContractTest, serde_json::to_vec(t).unwrap()));
    }
    r.attestations.push(sign(
        Role::Runtime,
        serde_json::to_vec(r.runtime.as_ref().unwrap()).unwrap(),
    ));
    r.attestations.push(sign(
        Role::Benchmark,
        serde_json::to_vec(&r.benchmarks[0]).unwrap(),
    ));
    r.attestations.push(sign(
        Role::Proposal,
        serde_json::to_vec(&r.migration_plans[0]).unwrap(),
    ));
    let policy = TrustPolicy {
        schema: 1,
        keys: vec![TrustedKey {
            key_id: "fixture-producer".into(),
            public_key: hex(&key.verifying_key().to_bytes()),
            roles: vec![
                Role::Catalog,
                Role::Runtime,
                Role::ContractTest,
                Role::Benchmark,
                Role::Proposal,
            ],
            revoked: false,
        }],
        required_session_id: None,
        max_attestation_age_seconds: None,
        max_clock_skew_seconds: None,
    };
    let store = SignedArtifactStore::new(policy.clone(), root.clone()).unwrap();
    let report = advise_with_verifier(&r, &solver(), &store).unwrap();
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::VerifiedCandidate
    );
    assert!(
        report.candidates[0]
            .proposal
            .as_ref()
            .unwrap()
            .expected_gain
            .as_ref()
            .unwrap()
            .prospective_power
            .as_ref()
            .unwrap()
            .adequate
    );
    assert!(
        !report.candidates[0]
            .proposal
            .as_ref()
            .unwrap()
            .deployment_authorized
    );
    let request_path = root.join("request.json");
    let trust_path = root.join("trust.json");
    std::fs::write(&request_path, serde_json::to_vec(&r).unwrap()).unwrap();
    std::fs::write(&trust_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_migration-advisor"))
        .arg("advise")
        .arg(&request_path)
        .arg("--trust-policy")
        .arg(&trust_path)
        .arg("--artifact-root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["candidates"][0]["status"], "VERIFIED_CANDIDATE");
    let pilot_attestation = r.attestations.remove(0);
    let report = advise_with_verifier(&r, &solver(), &store).unwrap();
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::PartialEvidenceCandidate
    );
    assert!(!report.candidates[0].assurance.provenance_verified);
    assert_eq!(
        report.candidates[0]
            .proposal
            .as_ref()
            .unwrap()
            .expected_gain
            .as_ref()
            .unwrap()
            .decision,
        "missing_power_pilot"
    );
    r.attestations.insert(0, pilot_attestation);
    r.benchmarks[0].pairs[0].candidate_ns = 1;
    let report = advise_with_verifier(&r, &solver(), &store).unwrap();
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::PartialEvidenceCandidate
    );
    assert!(!report.candidates[0].assurance.provenance_verified);
    assert!(report.preferred_observed.is_none());
    for file in std::fs::read_dir(&root).unwrap() {
        std::fs::remove_file(file.unwrap().path()).unwrap();
    }
    std::fs::remove_dir(root).unwrap();
}
