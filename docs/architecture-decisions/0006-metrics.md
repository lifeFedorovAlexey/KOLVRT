# ADR-0006 — Dependency measurements and fair comparisons

Document status: CURRENT
Evidence scope: Accepted intended obligations; implementation and verification remain separately scoped.
Current reference: [Documentation policy](../documentation-policy.md)

Status: **Accepted**. Date: 2026-10-02.

## Context

Call counts, CPU time and rare mandatory dependencies are not equivalent (cases 15, 21, 25).

## Decision

Publish a multidimensional vector, textual states, coverage and preregistered paired benchmarks; no universal score.

## Alternatives

A weighted 0–100 score; declaring native paths successful without evidence; comparing only means.

## Why rejected

Weights are arbitrary, declarations do not establish execution paths, and means hide tails and failures.

## Consequences

Store raw observations and attribution. MOSTLY_NATIVE requires consumer-specific migration budgets.

## Compatibility impact

Compatibility may be faster. Separate software dependency from hardware translation.

## Performance impact

Measure observer cost with instrumentation on and off. Unsupported counters are unavailable.

## Security impact

Limit tracing permissions, memory and payload collection.

## Testing

Test zero denominators, nested-call double counting, event loss, censored latency and missing counters.

## Reversibility

Add dimensions through versioned schemas without rewriting old observations.

## Evidence

[Cases and sources](../research/case-index.md); [other systems](../research/reference-systems.md).

[Russian translation](../../translations/ru/docs/architecture-decisions/0006-metrics.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0006",
  "kind": "adr",
  "aliases": ["ADR-0006"],
  "summary": "Scoped architecture decision; acceptance is intended authority, not proof of all implementation.",
  "tags": ["architecture", "decision"]
}
```
