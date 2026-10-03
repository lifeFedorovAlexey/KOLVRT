# Capability-based migration advisor

KOLVRT has a separate, read-only host subsystem that turns compatibility debt into a migration proposal. The [implementation](../../crates/migration-advisor/src/lib.rs) uses schema v2, a planning interface under development rather than a frozen project ABI. Version 1 requests are rejected explicitly. Kernel runtime collection and package-manager integration are not implemented.

```text
runtime evidence -> required capabilities -> dependency solutions
                 -> exact-artifact contract tests -> paired A/B evidence
                 -> migration proposal with rollback -> authorized deployment
```

## Analysis and placement

[Debian Provides](https://www.debian.org/doc/debian-policy/ch-relationships.html#virtual-packages-provides), [Spack virtual dependencies](https://spack.readthedocs.io/en/latest/packaging_guide_creation.html#virtual-dependencies) and [CUDF/Mancoosi](https://www.mancoosi.org/cudf/) motivate interface-based dependencies and a solver separate from package-manager execution. These are architectural references, not format-compatibility claims. A satisfiable graph does not prove behavior, data compatibility, durability, permissions, startup correctness or performance.

Package capabilities describe facilities, not kernel security authority. Catalog ingestion, solving, contract execution, benchmarking, advice and deployment stay separate and outside EL1. Similar names, descriptions or embeddings cannot satisfy a requirement. Mandatory paths remain requirements even when telemetry never observes them.

The subsystem rule is **Observation may recommend. Only authorization may deploy.** It enforces an advisor boundary, not a new numbered kernel law. Changing the kernel law set requires its architecture-decision process. The advisor contains no installation or routing-mutation API; `automatic_replacement` and each proposal's `deployment_authorized` are always false.

## Versioned semantic contracts

The [model](../../crates/migration-advisor/src/lib.rs) separates `capability_id`, `semantic_version` (`major`, `minor`, `patch`) and `contract_digest`. The digest identifies exact specification bytes; it is not the compatibility identity. Dependency matching uses capability ID and an inclusive semantic-version interval within one positive major version. Unstable major-zero contracts and intervals crossing major versions are unsupported in this profile. Advertised compatible minor/patch extensions may have different digests. Semantic compatibility remains an explicit declaration, never a fact inferred from version numbers alone.

A requirement retains the consumer's specification digest for testing. The candidate must pass both its own advertised specification and the consumer's required specification. Passing only the new provider specification cannot prove old consumer behavior. An incompatible major version requires an explicit consumer migration rather than a matching shortcut. Package release versions remain separate positive integers in this initial catalog format.

Requirements are AND lists of OR alternatives. Packages declare `provides`, `requires`, `conflicts`, name, release version and artifact digest. Duplicate/ambiguous manifests and empty alternatives are rejected. The request supplies the actual installed closure, replacement surface and retained consumers' requirements. The candidate root itself must provide the replacement surface. Rare startup paths, state formats and lifecycle obligations must be included explicitly; runtime cannot erase them.

## Solver profile and verification

The [solver and checker](../../crates/migration-advisor/src/solver.rs) are independent of deployment. The reference solver enumerates at most 20 packages, minimizes package count and breaks ties by sorted artifact identity. The CLI bounds search to 524288 root-containing states per candidate. Budget exhaustion is distinct from UNSAT. Cycles are permitted if requirements are satisfied. The checker verifies identities, transitive closure, retained requirements and conflicts, and verifies the actual installed baseline rather than constructing a fictional one.

`reference_single_version` is reported as the resolution model. Its one-version-per-package-name restriction belongs to this initial solver/checker profile, **not to KOLVRT architecture**. Side-by-side runtimes, ABI variants, old application closures and sandbox namespaces require an explicit scoped model and checker. They must not be prohibited by a future package manager merely because this reference implementation omits them. The profile returns one canonical closure per candidate; alternative-closure exploration and a scalable SAT/CUDF backend remain open work.

## Evidence levels and proposals

The [advisor](../../crates/migration-advisor/src/advisor.rs) records context: machine identity, workload/kernel/protocol digests and source (`os_runtime`, `host_model` or `host_process`). Runtime records bind package identity, window, route generations, adapter digests, native/compat admissions, failures, exclusive adapter CPU nanoseconds and copied bytes. Missing cost remains unknown. Known expensive routes and unknown-cost routes are separate. Compatibility share is unknown with lost events, incomplete classification or no admissions; observations never prove module removability. Follow the [diagnostic contract](diagnostics.md).

Results have explicit assurance dimensions and missing checks, not a fabricated numerical confidence score:

- `VERIFIED_CANDIDATE`: dependency solution, provider/consumer contract tests, authenticated evidence, complete loss-free OS compat observation, supported statistical gain and documented rollback satisfy this profile's gates.
- `PARTIAL_EVIDENCE_CANDIDATE`: a dependency solution exists, but some checks are absent, insufficient, unsigned or incomplete. Show available tests, measurements, rollback and precise gaps. Lost telemetry prevents a strong preference, not discovery or presentation.
- `REJECTED_CANDIDATE`: a supplied contract/oracle/experimental validity check failed, or a measured tail regression exceeded budget. Missing evidence is not a failed test.
- `BLOCKED`: solving is invalid, unsatisfiable or exceeds its budget; report the distinct reason.

Every solved candidate includes a proposal with `candidate`, `expected_gain`, `required_changes`, `rollback_strategy`, `rollback_preconditions` and `irreversible_changes`. Unprovided fields are explicitly null, not assumed safe. Migration-plan metadata binds the candidate plan and experiment context and needs its own trusted signature for verified status. Gain and confidence interval are computed from authenticated samples rather than accepted as asserted summaries.

Rollback supports restoring a digest-bound snapshot, reinstalling the exact previous plan, or explicitly declaring rollback unavailable. Reinstallation is rejected as a rollback for persistent-data changes. A stale baseline, missing rollback or declared irreversible effects keeps the candidate partial. A snapshot digest and signed plan authenticate a documented strategy; they do not prove a successful restoration drill or restore external side effects. Deployment must separately validate rollback preconditions, test restoration and authorize irreversible changes.

## Statistical A/B policy

The [statistics module](../../crates/migration-advisor/src/statistics.rs) replaces the requirement to win every pair with a paired percentile-bootstrap interval for **mean absolute wall-latency gain**. Whole independent paired workload runs are resampled together; requests within one run are not independent observations. A single noisy pair is retained and may be tolerated. No outliers are discarded. Seed, sample minimum, confidence level, bootstrap repetitions, minimum useful gain and p95/p99 regression budgets are predeclared and included in the signed benchmark payload; changing the policy invalidates comparability.

The policy accepts 30..4096 pairs, 2000..10000 bootstrap repetitions and 90%..99% nominal confidence. The example uses 100 pairs, 2000 repetitions and 95%. A pair count is an input bound, not a guarantee of adequate power or coverage. The gate requires the interval's lower bound above the declared gain margin and the p95 regression within budget. Optional p99 budgeting requires available p99 evidence. Report absolute effect, relative mean effect, interval, median, p95 and p99. Nearest-rank quantiles use `sorted[ceil(p*n)-1]`. p95 stays unavailable below 100 independent runs and p99 below 1000; these are conservative availability limits, not universal tail-accuracy guarantees. They describe **run-level workload duration**, not pooled request latency.

Candidate roots satisfying the replacement surface in the supplied catalog form one comparison family. Every benchmark interval uses a Bonferroni-adjusted confidence level for that family, including roots with no matched benchmark. The report records family size, adjusted confidence and draws available per tail. If the configured resample count leaves fewer than 20 draws in either adjusted tail, the result is `inadequate_comparison_resolution` and cannot support a gain. This prevents a missing benchmark from shrinking the family after candidate discovery. It does not prove that the catalog lists every real alternative, and the minimum tail count is not a power calculation.

The bootstrap assumes independent, representative paired runs and approximates coverage. It cannot authenticate independence, repair censoring or handle autocorrelation automatically. Dependence-aware blocks, power analysis and broader resource budgets remain required for workloads that need them. Follow the [benchmark methodology](benchmarking.md): equivalent useful results, equal safety and quotas, reset fixtures, independent snapshots for side effects, retained warmup/raw samples, and predeclared stopping/exclusion rules.

The advisor also checks alternating A/B order, stable useful-unit counts, successful oracles/stabilization and zero failures/timeouts/unfinished requests. Total measured compatibility admissions must decrease; individual noisy pairs need not all improve. Native labeling earns no bonus. Failed or incomplete measurements cannot be hidden by a fast median.

`preferred_observed` selects the lowest observed median among verified candidates with equal useful units; tied medians or unequal work produce no preference. It is an observed ordering, not proof of statistical superiority between all candidates. Dependence-aware block resampling, prospective power analysis, joint resource budgets and benchmarks of every alternative closure remain future work.

## Provenance and trust boundary

The [provenance module](../../crates/migration-advisor/src/provenance.rs) implements Ed25519 verification using pinned `ed25519-dalek` and its [strict verification API](https://docs.rs/ed25519-dalek/2.2.0/ed25519_dalek/struct.VerifyingKey.html#method.verify_strict). Trust policy and artifact root are supplied separately from the request. Input cannot install its own trusted key or assert `verified: true`. With no external verifier the CLI never grants verified status, even when every imported `passed` flag is true.

Trusted keys have explicit roles (`catalog`, `runtime`, `contract_test`, `benchmark`, `proposal`) and revocation flags. Weak keys and duplicate key IDs are rejected. Each attestation signs a domain-separated role and SHA-256 subject digest. The subject is `serde_json::to_vec` of the exact schema-v2 typed payload, excluding the separate attestation list; producers use the exported `subject_digest` and `signing_message` helpers. Field order and serialization belong to this versioned protocol, not arbitrary raw-JSON whitespace or key order. The domain is `KOLVRT-MIGRATION-ATTESTATION/2`.

The signed package manifest binds executable bytes and specification digests. Signed test results bind package, consumer/provider specification, suite and context. Signed benchmark results bind both exact plans, context, statistical policy and raw-artifact digest. Runtime and proposal metadata are separately signed. The verifier streams and hashes referenced package/specification/suite/kernel/workload/protocol/adapter/raw-result/snapshot artifacts. Files are addressed by lowercase SHA-256 filenames under the configured store, limited to 512 MiB each; missing, substituted or escaping artifacts fail authentication. The store must be an owned, stable snapshot during verification. Results do not pin files for later deployment.

A valid signature authenticates the configured producer's assertion. It cannot prove that a producer executed tests honestly, collected on the named physical machine, retained independent samples, or encoded raw data correctly. The producer must generate the typed summary from raw evidence and enforce those obligations. Key ownership, trusted signer provisioning, freshness/session policy, compromise recovery, secure acquisition and hardware-rooted attestation remain production integration work. Manual revocation in the externally loaded policy is implemented; no online revocation service or trusted timestamp is claimed. Package/manifest/test/benchmark signing authority is distinct from deployment authority.

## Running and verification

```text
cargo run --locked -p migration-advisor -- advise crates/migration-advisor/examples/request.json
cargo run --locked -p migration-advisor -- advise request.json --trust-policy trust.json --artifact-root artifacts
cargo test --locked -p migration-advisor
```

The [example](../../crates/migration-advisor/examples/request.json) is a host-model catalog with no live evidence, attestations or rollback plan. It reports a visible partial candidate with explicit gaps. The CLI writes a versioned JSON report to stdout, errors to stderr, bounds request and trust-policy input to 4 MiB and rejects unknown fields. Trust-policy schema 1 is separate from request schema 2. Plan digests bind sorted plan JSON; signed manifests additionally bind metadata.

[Planning/integration tests](../../crates/migration-advisor/tests/migration.rs) cover dependency alternatives/cycles/conflicts, retained consumers, semantic extensions with both specifications, search exhaustion, forged solver output, unsigned and partial evidence, noisy runs, uncertain effects, tail budgets, rollback gaps, CLI behavior and an end-to-end signed chain. [Cryptographic tests](../../crates/migration-advisor/tests/provenance.rs) reject changed payloads/bytes, bad signatures, wrong roles/keys and revoked keys. All measurements and producer keys in tests are synthetic fixtures, including the authenticated CLI fixture. They test verification mechanics, not real package behavior or hardware speedup.

## Remaining integration

The [migration workbench](migration-workbench.md) executes host-process contracts, 100 paired A/B runs, local signing and isolated immutable-workload rollback. Its QEMU exporter joins kernel-owned process generations, CPU ownership and resident frames to the exact loaded image digest and EL0 route-generation observations. This bounded fixture bridge is not a live production OS telemetry service: exclusive adapter/native CPU attribution, complete installed-state/catalog ownership, real application contract execution and physical ARM64 KOLVRT A/B remain unimplemented. No installed KOLVRT package migration or physical ARM64 speedup is measured. A benchmark runner waits for two real comparable paths, as required by the existing methodology. A production exporter and trusted catalog must retain causal attribution, loss reporting and actual consumer/installed-state identity.

Before production add trusted producer provisioning and freshness, actual contract/rollback execution, stable artifact capture, sample-dependence validation, scalable scoped solving, memory/copy/energy and consumer-specific regression budgets. Expose proposals in userspace; authorize deployment separately with route-generation, state-retirement and restoration checks. **Compatibility cannot be established by words; contracts must test it.** Signatures preserve the evidence chain but cannot replace its execution.

[Russian translation](../../translations/ru/docs/architecture/migration-advisor.md)
