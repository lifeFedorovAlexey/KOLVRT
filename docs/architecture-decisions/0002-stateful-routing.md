# ADR-0002 — Routing by state domain

Status: **Accepted**. Date: 2026-10-02.

## Context

Descriptors, open-file descriptions, locks, credentials and waits couple API families (cases 02, 03, 17, 26).

## Decision

Bind consumer, family and protocol to an exact version and digest. Shared-state closure constrains splitting and switching.

## Alternatives

Arbitrary per-call routing; one process-wide flag; routing by application name.

## Why rejected

Per-call routing breaks state; one flag is too coarse; an application name is not a contract.

## Consequences

Define a domain graph and explicit handle export/import. Start with a finite family table.

## Compatibility impact

Mixed consumers are possible; some compatibility environments cannot be split without a gateway.

## Performance impact

Resolve bindings outside the hot path; measure dispatch and gateway costs separately.

## Security impact

Intersect policies, reject conflicts and downgrades, and never grant additional authority.

## Testing

Test resolver conflicts and model-check rebinding and unloading after quiescence.

## Reversibility

Resolver implementation may change while pinned semantic bindings remain compatible.

## Evidence

[Cases and sources](../research/CASE_INDEX.md); [other systems](../../research/other-systems/COMPARISON.md).

[Russian translation](../../translations/ru/docs/architecture-decisions/0002-stateful-routing.md)
