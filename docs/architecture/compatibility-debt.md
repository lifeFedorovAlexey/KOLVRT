# COST-L compatibility-debt registry

Document status: CURRENT
Evidence scope: implemented bounded offline registry and research candidates; no Linux driver support or runtime accounting.
Current reference: [Registry implementation](../../crates/repository-checks/src/cost_l.rs)

## Identity and boundaries

COST-L-#### identifies a concrete recurring reason for incompatibility, not an individual driver. COST means measurable compatibility/technical debt cost. L intentionally means Linux and Legacy: Cost of Linux / Legacy; the Russian rendering means Price of Linux legacy. The double meaning and relevant Russian proverbs are project culture; neither replaces evidence.

A COST-L ID is not a kernel API, security capability, package facility, adapter identity, EXC exception or KOL-PATH case. Those concepts may reference each other. Native authority, semantic versions and implementation digests retain their existing meanings under the [native model](native-model.md), [compatibility model](compatibility-model.md) and [routing model](routing-model.md).

The native kernel imports no registry metadata or Linux semantics. Translation cannot expand effective authority (LAW-009); a missing right requires separate native authorization. Required hardware protection is a separate dimension under LAW-031. A compatibility route cannot be NATIVE, but a hardware-only workaround does not automatically make native software COMPAT.

## Format and allocation

[Record schema v1](../../schemas/cost-l.schema.json) and [allocation schema v1](../../schemas/cost-l-registry.schema.json) use closed JSON Schema Draft 2020-12 plus semantic checks. The format follows existing repository JSON/duplicate-key/unknown-provenance conventions; related information is grouped rather than duplicating flat counts or implementing a universal graph.

[Allocation ledger](../../research/cost-l/registry.json) lists each assigned stable ID exactly once. Every allocation needs a matching COST-L-####.json record; every record needs an allocation. Never recycle an ID or remove its historical record. Adding records does not require reaching a numerical quota. Zero is reserved. Ledger consistency detects accidental removal, not a malicious edit deleting both ledger and history; reviewed version control and future CI history checks remain necessary.

Record groups contain identity/status/dates, source-backed history, native decision, module/consumer relations, costs, support, lifecycle, confidence and open questions. Source URLs carry pinned commits or versions and locators. KOL-PATH and EXC references remain external identities. Category/subsystem are bounded extensible labels; origin classification is a reviewed enum including historical constraints and legitimate hardware/performance/security trade-offs.

Null with an explanation represents unknown origin/owner/observation; unknown cost uses state UNKNOWN, null value/provenance, unknown coverage and a reason. Unknown does not mean zero, false or not applicable. Empty module/consumer arrays in research-only records mean no declared implementation, not a claim that Linux has no users. Invented driver counts and metrics are forbidden.

Files/receipts are limited to 64 KiB, the registry to 4096 records, strings to 4096 characters and collections to schema-specific bounds. These are host input limits, not kernel budgets or a target record count. Reconsider bounds explicitly when real data requires it. Duplicate keys, unknown fields, malformed dates/IDs, unresolved references, inconsistent module artifacts and contradictory lifecycle/support fail.

## Lifecycle and support

| Transition                       | Review obligation                                                                                      |
| -------------------------------- | ------------------------------------------------------------------------------------------------------ |
| CANDIDATE → CONFIRMED            | Recurring cause established against an accepted native difference; preserve sources and uncertainty    |
| CANDIDATE or CONFIRMED → RETIRED | Rejected or unnecessary research case; retain the historical identity                                  |
| CONFIRMED → ACTIVE               | Named consumers, actual module declarations, owner, finite support and native migration/removal target |
| ACTIVE → DEPRECATED              | Reviewed migration and support decision; no premature cancellation of promises                         |
| DEPRECATED → ACTIVE              | Explicit recorded renewal; a successor artifact alone is insufficient                                  |
| DEPRECATED → RETIRING            | End unsupported admission and resolve obligations                                                      |
| RETIRING → DEPRECATED            | Record blocked retirement and revised support disposition                                              |
| RETIRING → RETIRED               | No remaining supported obligations; retain historical implementation/evidence records                  |

Each event has a date and decision. Events are ordered, start with the original candidate and end in the declared current state; unsupported jumps and resurrection of RETIRED fail. Confirmed/supporting states require an accepted native difference. Research-only entries cannot pretend to implement adapters.

Keep debt lifecycle, semantic-version support expiry and actual runtime quiescence separate. Zero calls does not cancel named offline/recovery/archival obligations; hypothetical consumers cannot renew support. A retired formerly active record retains owner/artifact history and completed finite support obligations. The checker does not prove drained callbacks, IRQ/DMA, reset or grace periods and never deletes code, closes admission or unloads a module. Apply [ADR-0007](../architecture-decisions/0007-bug-compat.md) and [security recovery rules](../security/threat-model.md) before real removal.

## Relations and measurements

One debt may relate to many drivers/modules; one module may appear in several debt records. Consumers pin each declared module's ID, semantic version and scope, with direct/transitive kind and support deadline. Different versions of the same module can coexist; shared module ID/version/scope must identify the same artifact. Consumer counts are derived within a named inventory; no mutable consumer_count is stored. Support start is inclusive and until is the first unsupported calendar day in the declared UTC date domain; expiry does not prove runtime quiescence.

Cost dimensions are CPU time/latency in ns, memory in bytes, copies/allocations/context switches as counts and throughput in operations/s. MEASURED needs value, denominator, scope, known coverage, observation date and a bounded local receipt with exact SHA-256. Partial coverage needs its limitation. A measured zero differs from UNKNOWN. Hash validation establishes retained bytes, not producer honesty, causal attribution, payload correctness, equal work or a statistical gain; those are future accounting/benchmark/advisor obligations. Source history and runtime provenance remain separate.

The registry does not include live drivers, authenticated inspectors, universal scores, per-call parsing or kernel dispatch. Existing LEGACY/COMPAT/MIXED/MOSTLY_NATIVE/NATIVE text states and supplementary colors remain unchanged. Preserve genuinely faster compatibility results; never add artificial penalties.

## Commands and current evidence

```text
cargo run --locked -p repository-checks -- check-cost-l
cargo run --locked -p repository-checks -- check-cost-l --directory research/cost-l
cargo run --locked -p repository-checks -- validate
cargo test --locked -p repository-checks --test cost_l
```

The normal check/validate commands include this registry. [Behavioral tests](../../crates/repository-checks/tests/cost_l.rs) exercise malformed records, duplicate JSON, allocation loss, missing evidence, lifecycle/support failures, offline obligations, UNKNOWN versus measured zero, digest tampering and actual CLI exit codes. Synthetic active/retired/measurement fixtures are tests, not supported deployments.

Three source-backed records are CANDIDATE: [allocation context](../../research/cost-l/COST-L-0001.json), [sysfs text ABI](../../research/cost-l/COST-L-0002.json) and [ioctl layout](../../research/cost-l/COST-L-0003.json). Each pins Linux v6.12 documentation, leaves original introduction and actual consumers/cost unknown, and claims no implemented module. The [seed review](../../research/compatibility/taxonomy.md) also records deferred/non-debt cases instead of forcing every API into a record.

Foundation is implemented locally for issue #46; final incompatibility confirmation still needs #45 evidence and native decisions. Production module manifests/CI, offline queries, runtime accounting, benchmarks, migration and authorized inspection remain issues #47–#52. No Linux Driver Host, hardware survey or publication of personal inventory occurs here.

[Russian translation](../../translations/ru/docs/architecture/compatibility-debt.md)
