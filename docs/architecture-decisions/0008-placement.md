# ADR-0008 — Adapter and driver protection domains

Status: **Proposed — RESEARCH_REQUIRED**. Date: 2026-10-02.

## Context

Reference systems use different boundaries; their existence does not establish suitability for KOLVRT. A library boundary alone does not provide isolation.

## Decision

Specify interfaces now. Keep kernel versus user-space placement open until an IPC prototype and threat model exist. A privileged adapter needs separate justification.

## Alternatives

All adapters in the kernel; all in user space; arbitrary per-operation placement.

## Why rejected

These enlarge the trusted base, may need complex native primitives, or break ownership without a shared state model.

## Consequences

Phase 0.2 compares two real host implementations of one operation, then records the placement decision. Do not present fake kernel paths.

## Compatibility impact

Some external API families require one compatibility domain. Placement does not change the semantic contract.

## Performance impact

Measure copies, context switches, CPU time and p99 without choosing a winner in advance.

## Security impact

Model malicious adapters and devices. An in-process boundary is not a sandbox.

## Testing

Check fault containment, crash recovery, cancellation, shared-handle transfer and quotas.

## Reversibility

Keep the decision open and design public contracts to allow changing placement while preserving guarantees.

## Evidence

[Cases and sources](../research/CASE_INDEX.md); [other systems](../../research/other-systems/COMPARISON.md).

[Russian translation](../../translations/ru/docs/architecture-decisions/0008-placement.md)
