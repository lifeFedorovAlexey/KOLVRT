# KOLVRT Arena — Replaceable Component Arena / Performance & Reliability Passports

Document status: DESIGN BASELINE
Evidence scope: architectural requirements and staged roadmap; no Arena runner, passport schema, leaderboard or component comparison has been implemented or measured by this document.
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

Passports describe one implementation in a particular contract, suite, workload, profile and platform cohort. Every metric carries units, scope, collection method and evidence references. Publish all three passports together.

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

These commands illustrate the intended interface; they are not available commands:

```bash
kolvrt arena compare scheduler/reference scheduler/ivan-v3
kolvrt arena compare ipc/native ipc/lockfree-x
kolvrt arena leaderboard allocator
```

Compare only matching contract/suite versions, architecture, platform, execution profile, workload and budgets. Display the cohort, units, uncertainty, eligibility and receipt links. Cross-cohort data may be browsed separately but must not produce a speed ratio or rank.

There is no universal SCORE or automatic crown. Show multidimensional trade-offs and, where uncertainty permits, a Pareto frontier among eligible candidates. A user may sort by a named metric or apply declared workload budgets; preserve all dimensions, failures and workload coverage in the view. A scheduler with a better median but a failed soak remains ineligible; a scheduler with worse p99 must show that regression even when it passes a permissive tail budget. Insufficient evidence yields an inconclusive comparison.

<a name="kolvrt-arena-cost-l"></a>

## COST-L migration evidence

Link an adapter and its native replacement through their actual implementation identities and allocated [COST-L records](compatibility-debt.md). A COST-L ID identifies a recurring compatibility cause, not the module or benchmark. One replacement may address several causes; avoid double-counting shared costs.

Report paired absolute values and deltas for CPU, median/p95/p99, memory, allocations, copies per useful operation, context switches, binary size and authority surface, with uncertainty and preserved guarantees. Distinguish adapter-only, shared service and end-to-end costs. Show regressions and unknowns as well as gains. A capability removed from a manifest counts as an authority reduction only if the effective dependency closure no longer requires that authority.

Arena evidence supports a migration decision; it does not authorize deployment or prove that dormant consumers permit retirement. Existing COST-L support, lifecycle and removal obligations still apply. Do not allocate an example COST-L ID or invent migration savings for presentation.

<a name="kolvrt-arena-roadmap"></a>

## Roadmap direction and acceptance gates

Track **Replaceable Component Arena / Performance & Reliability Passports** as a distinct roadmap direction alongside the measurement and reliability work identified as #32–#36. This document does not assign a new issue number or claim those issues are complete. Sequence follows current architectural decisions, not whichever prototype was built first.

1. Define contract and suite ownership/versioning for the first real component family. Review inputs, outputs, authority, lifetime and failures; provide shared correctness and negative controls before admitting candidates.
2. Design versioned passport and receipt schemas with explicit unknown/status handling. Validate evidence links, units, cohort compatibility and rejected/missing measurements; retain raw artifacts and reproducible recipes.
3. Build a bounded runner and paired comparison only when two real comparable implementations exist, as required by the benchmark methodology. Demonstrate a rerun and rejection of weakened checks, failed soak and incompatible cohorts.
4. Publish a multidimensional report and scoped leaderboard. Keep failures visible, reject universal scoring, show uncertainty and test that a good median cannot bypass admission gates.
5. Connect actual adapter/native comparisons to COST-L records with attribution and preserved semantics. Expand to other component families and physical ARM64 only with their own reviewed contracts and evidence.

Initial delivery is host tooling and reviewed build-time substitution. Production switching, a public plugin ABI and an online ranking service require separate architectural decisions and evidence. Current kernel tests and measurement receipts remain scoped to their original milestones; they do not become Arena passports automatically.

[Russian translation](../../translations/ru/docs/architecture/component-arena.md)

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
      "summary": "Planned replaceable-component Arena and its architecture boundary.",
      "depends_on": [
        "kolvrt.arena.contract",
        "kolvrt.arena.admission",
        "kolvrt.arena.passports",
        "kolvrt.arena.receipts",
        "kolvrt.arena.comparison",
        "kolvrt.arena.cost-l",
        "kolvrt.arena.roadmap",
        "adr.0006",
        "doc.kolvrt.architecture.benchmarking"
      ],
      "tags": ["arena", "passports", "components"],
      "read_when": ["compare replaceable components Arena"],
      "gaps": [
        "Arena runner, passport schemas and leaderboard are not implemented or measured."
      ],
      "feature": {
        "implementation": "PLANNED",
        "implementation_scope": "Design baseline for replaceable component contracts, passports and reproducible comparisons; no Arena runtime tooling exists.",
        "sources": [],
        "acceptance": [],
        "issues": [],
        "adrs": ["adr.0006"],
        "limitations": [
          "No implemented runner, versioned passport schema, eligible leaderboard, measured Arena comparisons or physical ARM64 acceptance."
        ],
        "next_gate": "Review contract and common suite ownership/versioning for the first real component family before designing schemas and a bounded runner.",
        "verification": [
          {
            "environment": "host-process",
            "state": "UNKNOWN",
            "reason": "No Arena implementation or execution receipt exists."
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
