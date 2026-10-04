# ADR-0011 — Performance and reliability method review

Status: **Accepted**. Date: 2026-10-02.

Document status: HISTORICAL MILESTONE
Evidence scope: compatibility-route creation state at ADR acceptance; compatibility routing is later accepted in Phase 2.
Current reference: [Versioned EL0 routing](0015-el0-versioned-routing.md)

## Context

The user requires performance-first method selection balanced against reliability, with historical OS errors checked before adoption. Benchmarks alone cannot establish this requirement.

## Decision

Require the [implementation decision gate](../architecture/implementation-review.md) before accepting methods. Review alternatives, costs, failure dependencies and relevant cases before implementation acceptance. Prefer the fastest reviewed option within correctness constraints; distinguish complexity arguments from measured comparative superiority. This implements LAW-040, LAW-036 and LAW-041 without adding a quota-driven law.

## Alternatives

Select by convention, copy a known OS mechanism, optimize only happy paths, or defer correctness repairs as temporary architecture.

## Why rejected

Popularity does not demonstrate workload fit. Known failure mechanisms remain failures even if copied code is fast. Unmeasured ranking cannot satisfy honest observation.

## Consequences

Every material change needs a reviewable choice, invariants, bounded failure behavior and evidence. A failed law check stops the decision and requires an ADR alternative. Unavailable comparative evidence is recorded, not invented.

## Compatibility impact

The same review later applies to compatibility routes; none are created now.

## Performance impact

Review hot-path operation count, memory traffic, metadata, contention, tail latency and DEV cost in PROD. Measurements need workload, units, warm-up, iterations and raw samples. TCG observations do not establish hardware throughput.

## Security impact

Authority, initialization, alias lifetime, memory ordering and failure containment are acceptance constraints, not tunable benchmark shortcuts.

## Testing

Real subsystem tests, negative controls and implementation inspection complement measurements. Host-only models do not replace kernel faults or IRQ validation. Unsafe inventory does not substitute for invariant review.

## Reversibility

Reopen a choice when workload or platform changes; preserve evidence and compare the replacement. No automatic scheduler, userspace or compatibility expansion.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0011",
  "kind": "adr",
  "aliases": ["ADR-0011"],
  "summary": "Scoped architecture decision; acceptance is intended authority, not proof of all implementation.",
  "tags": ["architecture", "decision"]
}
```
