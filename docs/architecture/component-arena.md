# KOLVRT Arena — Replaceable Component Arena / Performance & Reliability Passports

Document status: CURRENT
Evidence scope: experimental general Arena validation and the accepted bounded QEMU CLOCK pilot; the source-bound logical map has terminal and HTML views; records/history and general attribution remain unimplemented.
Current reference: [Benchmark methodology](benchmarking.md); [ADR-0006](../architecture-decisions/0006-metrics.md)

<a name="kolvrt-arena-scope"></a>

## Principle and architectural boundary

**KOLVRT should make architectural improvement observable, reproducible and competitive.**

Do not say your module is better. Show the numbers.

Arena applies to replaceable allocators, schedulers, IPC queues, drivers and filesystems. Replacement means a reviewed implementation behind an explicit contract, not similar code. It does not require a universal plugin ABI, dynamic loading or hot swapping. Static build selection is sufficient for initial comparisons; placement, authority and lifetime follow the native architecture.

Earlier implementation does not establish architectural authority. Before extending a component for a later milestone, re-derive its architecture from current invariants and accepted decisions. Refactor or remove an early implementation that constrains, contradicts or prematurely freezes that architecture. Existing code, effort spent, compatibility with tests and avoiding rework do not justify retaining a design. Arena contracts and suites must evolve through reviewed decisions rather than freeze an accidental early design; historical receipts retain their original contract version.

<a name="kolvrt-arena-contract"></a>

## Replaceable component contract

Each replaceable component must declare a versioned contract and a benchmark suite. The contract defines:

- Inputs: types, preconditions, bounds, concurrency and supported workloads.
- Outputs: useful results, postconditions, ordering and observable side effects.
- Authority: caller and component rights, required capabilities, privileged operations and trust boundaries; substitution cannot silently expand authority.
- Lifetime: ownership, borrowing, initialization, shutdown, cancellation, retirement and reclamation, including in-flight work.
- Failure semantics: errors, timeouts, partial effects, resource exhaustion, recovery and invariant preservation.

Record component identity, implementation digest, owner, contract version and suite digest. An adapter needed to satisfy the contract belongs in the measured dependency closure. A changed guarantee creates a different comparison cohort or requires an explicit contract revision; it cannot be hidden as an optimization. Benchmark coverage must describe omitted workloads and unsupported operations.

<a name="kolvrt-arena-admission"></a>

## Common test contract and admission

All candidates in a comparison run the same versioned correctness oracle, invariant checks, negative controls, fault campaigns and security obligations under equivalent resource limits. A candidate cannot qualify by deleting tests, disabling checks, weakening isolation or skipping required capabilities. Review changes to the shared suite independently of the candidate and rerun affected implementations after a contract or suite revision.

Predeclare workload-specific correctness, security, soak and tail-latency gates before seeing results. A fast median cannot compensate for failed reliability or a breached p99 budget. Keep failed candidates and their measurements visible with reasons, but exclude them from the eligible set. Use PASS, FAIL, INCONCLUSIVE, NOT RUN and UNSUPPORTED with evidence and scope; missing evidence never means PASS or zero. Qualification proves only the recorded campaign and platform, not general safety.

<a name="kolvrt-arena-passports"></a>

## Implementation passports

Passports describe one implementation in a particular contract, suite, workload, profile and platform cohort. Every metric carries units, scope, collection method and evidence references. Publish all PERF/SEC/REL/RES dimensions together.

| Passport             | Required dimensions                                                                                                                                                                                                                      |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Performance Passport | Latency median/p95/p99, CPU time and utilization with denominator, memory and peak use, allocations/count/bytes, copies/count/bytes, voluntary/involuntary context switches, binary size with artifact and dependency scope              |
| Reliability Passport | Soak duration, attempted and successfully completed operations, injected failure types/counts, fuzz runs/seeds/corpus/tool versions, crashes, leaks, invariant failures, recovery outcomes                                               |
| Security Passport    | Lexical `unsafe` sites and assembly boundaries with inventory scope, privileged entry points/operations, required capabilities and effective authority, exposed interfaces/attack surface, trusted computing base and dependency closure |

Follow the [benchmark methodology](benchmarking.md) for sampling, censoring, attribution and observer cost. Quantiles need adequate tail coverage and uncertainty; count failed and unfinished operations. Distinguish kernel image, component text/data and loaded memory when reporting size. State leak detection methods and their limits. Security counts are audit evidence, not proof: fewer `unsafe` blocks or capabilities do not by themselves establish smaller risk. Do not reward moving unsafe code or privileged work into uncounted dependencies.

<a name="kolvrt-arena-receipts"></a>

## Reproducible receipts

Retain commit and source-tree digest, dirty-tree patch when applicable, component/dependency/artifact digests, compiler and linker versions, flags/features, DEV/PROD profile, contract/suite/oracle/analysis versions, workload and dataset digests, seeds, build/run commands, raw samples, logs and exclusions. Hardware records include CPU/board/stepping, firmware, RAM, CPU count, affinity, frequency/thermal state and device setup. QEMU records include version, machine, CPU, accelerator and arguments. Keep hardware and emulation cohorts separate.

Use paired runs and recorded ordering under equivalent quotas, devices and instrumentation. Preserve failed and invalid runs. Each report must resolve to its receipts and raw artifacts; unavailable data is explicitly unavailable. A reproducible recipe permits rerunning an experiment; it does not promise identical timing. CI checks receipt integrity and completeness, while independent reruns and review establish confidence.

<a name="kolvrt-arena-comparison"></a>

## Comparison and leaderboard

The implemented terminal view is `cargo xtask arena report [DIR|RUN]`. With no path it opens the retained CLOCK campaign `research/arena/runs/clock-query-2978ad9`; this is a historical default, not automatic discovery of the latest source. It reads original run passports through the existing assessor and shows operation, environment, repetitions, metric/statistic, units, sample counts, losses and admission. Invalid or incomplete evidence stays visible. Viewing results does not execute a kernel campaign.

Use `cargo xtask arena diff BASE_RUN CANDIDATE_RUN` for a readable descriptive comparison. Both runs must pass the existing admission and exact comparison-class checks; DEV and PROD are not interchangeable. The view shows absolute and percentage differences with the declared metric direction. A favorable observed value is not proof of improvement: repeated measurements of one implementation, observer OFF/ON pairs and insufficient uncertainty evidence cannot establish a revision regression or superiority. Existing `assess` and `compare` commands retain machine-readable JSON output.

These commands illustrate the intended interface; they are not available commands:

```bash
kolvrt arena compare scheduler/reference scheduler/ivan-v3
kolvrt arena compare ipc/native ipc/lockfree-x
kolvrt arena leaderboard allocator
```

Compare only matching contract/suite versions, architecture, platform, execution profile, workload and budgets. Display the cohort, units, uncertainty, eligibility and receipt links. Cross-cohort data may be browsed separately but must not produce a speed ratio or rank.

There is no universal SCORE or automatic crown. Show multidimensional trade-offs and, where uncertainty permits, a Pareto frontier among eligible candidates. A user may sort by a named metric or apply declared workload budgets; preserve all dimensions, failures and workload coverage in the view. A scheduler with a better median but a failed soak remains ineligible; a scheduler with worse p99 must show that regression even when it passes a permissive tail budget. Insufficient evidence yields an inconclusive comparison.

## Current logical architecture map

`cargo xtask arena map` renders the [canonical component map](kernel-component-map.md) in the terminal. `cargo xtask arena map --html [OUTPUT]` writes an interactive standalone HTML view; the default output is `target/arena-map.html`. Both views use the same version 1 model: 13 functional EL1 groups, two production EL0 services and 28 selected typed interactions. Elements name logical mechanisms; source paths provide evidence, not the counting boundary. Shared sources do not imply exclusive ownership, physical isolation, a new ABI or additive code size or cost.

The HTML view uses the full browser viewport for the graph. Drag the background to pan; use the wheel, +/− controls and Fit to navigate. A local RU/EN switch translates group names, mechanisms, responsibilities and interaction explanations. Compact node icons and member/incoming/outgoing counts describe topology, not throughput or performance. Node-local cost fields show an explicit unknown marker except for the separately labelled historical CLOCK path ranges. Hover or focus opens explanatory details; selecting a node pins them, and Escape closes them. These floating details replace a permanent inspector or large summary strip. CLOCK details retain the individual DEV/PROD repeat medians, sample counts and evidence links without claiming exclusive component cost.

The map validates canonical IDs, source paths and LF-normalized reviewed source hashes. Changed sources reject the snapshot as STALE instead of silently presenting an outdated topology as current. The historical CLOCK overlay reads retained admissible passports and labels DEV/PROD path measurements separately. It measures an envelope using three CLOCK calls, including partial transitions of the boundary calls; it does not isolate the cost of the CLOCK group or its neighbours. Other component costs, interaction frequencies and weights remain unknown; no missing measurement becomes zero. Viewing the map does not execute the kernel, establish a speedup or publish a record. Historical topology reconstruction, measured runtime edge weights and a general architecture graph remain outside this bounded view.

<a name="kolvrt-arena-cost-l"></a>

## COST-L migration evidence

Link an adapter and its native replacement through their actual implementation identities and allocated [COST-L records](compatibility-debt.md). A COST-L ID identifies a recurring compatibility cause, not the module or benchmark. One replacement may address several causes; avoid double-counting shared costs.

Report paired absolute values and deltas for CPU, median/p95/p99, memory, allocations, copies per useful operation, context switches, binary size and authority surface, with uncertainty and preserved guarantees. Distinguish adapter-only, shared service and end-to-end costs. Show regressions and unknowns as well as gains. A capability removed from a manifest counts as an authority reduction only if the effective dependency closure no longer requires that authority.

Arena evidence supports a migration decision; it does not authorize deployment or prove that dormant consumers permit retirement. Existing COST-L support, lifecycle and removal obligations still apply. Do not allocate an example COST-L ID or invent migration savings for presentation.

<a name="kolvrt-arena-roadmap"></a>

## Roadmap direction and acceptance gates

Track **Replaceable Component Arena / Performance & Reliability Passports** as a distinct roadmap direction alongside the measurement and reliability work identified as #32–#36. The shared contract is tracked by #93; The bounded CLOCK pipeline for #32 is accepted separately; #33–#36 retain their IPC, reliability, fuzz and publication scopes. Sequence follows current architectural decisions, not whichever prototype was built first.

1. Define contract and suite ownership/versioning for the first real component family. Review inputs, outputs, authority, lifetime and failures; provide shared correctness and negative controls before admitting candidates.
2. Design versioned passport and receipt schemas with explicit unknown/status handling. Validate evidence links, units, cohort compatibility and rejected/missing measurements; retain raw artifacts and reproducible recipes.
3. Build standalone useful mechanism measurement with an oracle; add paired comparison only when two real comparable implementations exist, as required by the benchmark methodology. Demonstrate a rerun and rejection of weakened checks, failed soak and incompatible cohorts.
4. Publish a multidimensional report and scoped leaderboard. Keep failures visible, reject universal scoring, show uncertainty and test that a good median cannot bypass admission gates.
5. Connect actual adapter/native comparisons to COST-L records with attribution and preserved semantics. Expand to other component families and physical ARM64 only with their own reviewed contracts and evidence.

Initial delivery is host tooling and reviewed build-time substitution. Production switching, a public plugin ABI and an online ranking service require separate architectural decisions and evidence. Current kernel tests and measurement receipts remain scoped to their original milestones; they do not become Arena passports automatically.

[Russian translation](../../translations/ru/docs/architecture/component-arena.md)

## Measurement contract foundation

Issue [#93](https://github.com/lifeFedorovAlexey/KOLVRT/issues/93) defines the standards-based Arena model A0–A5. The [measurement contract](arena-measurement-contract.md) now implements closed registry/profile/import schemas and offline comparison/admission through cargo xtask arena. The shared host foundation remains experimental. The separately accepted [CLOCK pilot](../research/clock-passport.md) supplies a real QEMU passport; no record service is implemented. PERF/SEC/REL/RES remain separate dimensions; SEC uses TOE/SPD/SFR/SAR, not a security score. #32–#36 and #50 own their real pipelines.

Fresh machine-mode evidence execution through the shared xtask runner requires exactly one validated [boot device observation](../kernel/devices.md). A missing inventory event fails the current execution gate even when other workload events pass. Historical receipt reading remains separate and accepts pre-descriptor streams; that compatibility cannot admit a new run without the observation. This changes evidence admission, not kernel performance, measurement methodology or Arena record eligibility.

## CLOCK pilot integration

The CLOCK pilot adds a real-kernel campaign producer through the existing xtask/native executor, with separate DEV/PROD profiles, frozen protocol, checked report oracle and matched recorder observations. Its [accepted bounded passport](../research/clock-passport.md) retains twelve successful DEV/PROD boots and six portable pair receipts; independent admission remains structural. Existing offline admission remains structural and record eligibility remains false; this producer does not implement records/history, hardware performance, IPC passports or REL/fuzz campaigns. Execution acceptance and current evidence belong to the pilot feature, not an inferred acceptance of the entire Arena.

## Issue 33 integration

Issue #33 adds an experimental external IPC producer through the shared CLOCK build/execution path. Its functional pilot and selected original production-service execution do not complete SEC applicability, main sampling or record publication. Existing Arena admission remains authoritative; incomplete or lossy populations cannot be relabeled eligible. [IPC pilot](../research/ipc-passport.md) owns its current scope and gates.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.architecture.arena",
  "kind": "subsystem-contract",
  "summary": "Arena design: replaceable components, passports and evidence-bound comparisons.",
  "units": [
    {
      "id": "kolvrt.arena",
      "anchor": "kolvrt-arena-scope",
      "kind": "feature",
      "summary": "Experimental general Arena validation with a separately accepted bounded CLOCK pilot; record publication gates remain open.",
      "depends_on": [
        "kolvrt.arena.contract",
        "kolvrt.arena.admission",
        "kolvrt.arena.passports",
        "kolvrt.arena.receipts",
        "kolvrt.arena.comparison",
        "kolvrt.arena.cost-l",
        "kolvrt.arena.roadmap",
        "adr.0006",
        "doc.kolvrt.architecture.benchmarking",
        "doc.kolvrt.arena.measurement-contract"
      ],
      "tags": ["arena", "passports", "components"],
      "read_when": ["compare replaceable components Arena"],
      "gaps": [
        "IPC, REL/fuzz and broader mechanism campaigns, trusted general evidence admission, records/history, historical topology reconstruction and measured runtime graph weights remain unimplemented; the bounded QEMU CLOCK passport is accepted separately."
      ],
      "feature": {
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "Offline A0/A1 registry and closed schemas, frozen profile/run validation, conservative comparison classes and correctness/SFR/SAR rejection gates, integrated with the separately accepted bounded CLOCK producer; a source-bound logical architecture map with terminal and HTML views; no general records publication.",
        "sources": [
          "crates/repository-checks/src/arena.rs",
          "crates/repository-checks/src/arena/report.rs",
          "crates/repository-checks/tests/arena.rs",
          "schemas/arena-standards.schema.json",
          "schemas/arena-profile.schema.json",
          "schemas/arena-run.schema.json",
          "research/arena/standards.json",
          "research/arena/profiles/user-copy-range.json",
          "crates/repository-checks/src/main.rs",
          "crates/repository-checks/src/lib.rs",
          "crates/xtask/src/main.rs",
          "scripts/run-arena.ps1",
          "crates/repository-checks/tests/arena_launcher.rs",
          "crates/repository-checks/src/arena/architecture.rs",
          "crates/repository-checks/src/arena/architecture.html"
        ],
        "acceptance": [
          "research/results/arena-foundation-host.json",
          "research/results/arena-foundation-host-final.json",
          "research/results/arena-launcher-host.json"
        ],
        "issues": [93, 32, 33, 34, 35, 36, 50],
        "adrs": ["adr.0006", "adr.0003", "adr.0024"],
        "limitations": [
          "Producer assertions are not attestation; record_eligible is always false. The proposed range profile and synthetic tests do not establish kernel PERF/SEC acceptance, DEV attribution, PROD external outcomes or physical ARM64 performance."
        ],
        "next_gate": "Extend reviewed pipelines to IPC #33 and REL/fuzz/SAR #34/#35; implement publication/regression #36 and independently verified contribution/custody before records. Preserve the accepted CLOCK pilot scope.",
        "verification": [
          {
            "environment": "host-process",
            "state": "STALE",
            "reason": "This feature retains historical receipts from before the Phase 3.7 source changes; its verification remains STALE pending a feature-scoped current-source review. The separately accepted bounded native-application fault/restart/shutdown evidence is recorded in docs/kernel/native-applications.md and research/results/native-phase37-43af402.json; it does not automatically renew this feature verification.",
            "scope": "Offline registry/profile/import and launch behavior, with synthetic assertions and no record eligibility.",
            "receipt": "research/results/arena-launcher-host.json",
            "receipt_sha256": "db0d6287a61386cddd8fc92673cfa95d2385a11381f83b9b827aeb51a51996be"
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical Arena campaign exists."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "PLANNED",
            "reason": "Enroll the existing Arena design baseline without claiming implementation or execution.",
            "acceptance": []
          },
          {
            "from": "PLANNED",
            "to": "EXPERIMENTAL",
            "reason": "Introduce the offline standards/profile/import contract foundation for #93; complete Arena runtime and records acceptance remains open.",
            "acceptance": []
          }
        ],
        "roadmap_gate": "Replaceable Component Arena / Performance & Reliability Passports"
      }
    },
    {
      "id": "kolvrt.arena.contract",
      "anchor": "kolvrt-arena-contract",
      "kind": "contract-section",
      "summary": "Versioned inputs, outputs, authority, lifetime and failure contract.",
      "depends_on": []
    },
    {
      "id": "kolvrt.arena.admission",
      "anchor": "kolvrt-arena-admission",
      "kind": "contract-section",
      "summary": "Shared correctness, security, reliability and tail-latency admission.",
      "depends_on": []
    },
    {
      "id": "kolvrt.arena.passports",
      "anchor": "kolvrt-arena-passports",
      "kind": "contract-section",
      "summary": "Performance, reliability and security passports with accounting scope.",
      "depends_on": []
    },
    {
      "id": "kolvrt.arena.receipts",
      "anchor": "kolvrt-arena-receipts",
      "kind": "contract-section",
      "summary": "Reproducible recipes, provenance, raw samples and platform cohorts.",
      "depends_on": []
    },
    {
      "id": "kolvrt.arena.comparison",
      "anchor": "kolvrt-arena-comparison",
      "kind": "contract-section",
      "summary": "Multidimensional eligible comparisons without universal scoring.",
      "depends_on": []
    },
    {
      "id": "kolvrt.arena.cost-l",
      "anchor": "kolvrt-arena-cost-l",
      "kind": "contract-section",
      "summary": "Actual adapter/native COST-L attribution and migration evidence.",
      "depends_on": []
    },
    {
      "id": "kolvrt.arena.roadmap",
      "anchor": "kolvrt-arena-roadmap",
      "kind": "contract-section",
      "summary": "Separate staged Arena acceptance gates.",
      "depends_on": []
    }
  ]
}
```
