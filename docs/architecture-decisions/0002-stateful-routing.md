# ADR-0002 — Routing by state domain

Status: **Accepted**. Date: 2026-10-02.

## Context

Descriptors, open-file descriptions, locks, credentials and waits couple API families (cases 02, 03, 17, 26).

## Decision

Select by consumer/state-domain binding, API family and semantic route. The route names its protocol and exact semantic version. The binding separately pins digest, dependency closure, native operations, adapter chain, rights, device scope, quotas, profile and generation. Consumer and domain identities remain distinct. Shared-state closure constrains splitting and switching.

Admission intersects package, process, driver, device and administrator constraints; denial cannot be overridden by defaults or precedence. Dispatch uses the pinned route and current authorization checks. Every new independent selector requires an ADR, a named workload, evidence that existing selectors are insufficient, shared-state analysis and rejection tests. The [routing model](../architecture/routing-model.md) specifies the finite window example and rejection cases.

This clarification addresses issue #2 under LAW-003, LAW-004, LAW-005 and LAW-009. It adds no law or general resolver implementation.

Issue #9 strengthens LAW-004: deny fallback unless trusted scoped policy authorizes a
finite alternate transition. Preserve authority, quotas and shared state; differing
semantics require consumer opt-in and a new binding. PROD needs explicit production
authorization and a mandatory visible result independent of optional diagnostics.
Committed or unknown effects cannot authorize blind replay. Supported drain/commit or
restart contracts govern rebinding; production gains no experimental live-switch permission.
The routing model states synthetic-host checks and remaining runtime gaps.

## Alternatives

Arbitrary per-call routing; one process-wide flag; routing by application name; a flat key containing admission constraints; a generic routing graph.

## Why rejected

Per-call routing breaks state; one flag is too coarse; an application name is not a contract. A flat key obscures semantics versus authority. No demonstrated workload justifies a generic graph's policy and state complexity.

## Consequences

Define a domain graph and explicit handle export/import. Start with a finite family table.

## Compatibility impact

Mixed consumers are possible; some compatibility environments cannot be split without a gateway.

## Performance impact

Resolve bindings outside the hot path; measure dispatch and gateway costs separately.

## Security impact

Intersect policies, reject conflicts and downgrades, and never grant additional authority.

## Testing

Test resolver conflicts and model-check rebinding and unloading after quiescence. Reject denied rights, out-of-scope devices, quota exhaustion, empty policy intersections and incompatible handle/state import. Current finite window and host domain checks cover only documented subsets; general admission and gateway execution remain verification gaps.

## Reversibility

Resolver implementation may change while pinned semantic bindings remain compatible.

## Evidence

[Cases and sources](../research/case-index.md); [other systems](../research/reference-systems.md).

[Russian translation](../../translations/ru/docs/architecture-decisions/0002-stateful-routing.md)
