# ADR-0009 — Native slice design baseline

Status: **Accepted**. Date: 2026-10-02.

Document status: HISTORICAL MILESTONE
Evidence scope: Phase 0 design baseline; the Phase 1 authorization statement records the decision state at acceptance.
Current reference: [Current handle milestone](0020-handle-transfer-and-retention.md)

## Context

Phase 0 needs a concrete stopping condition. Broad subsystem promises and a fixed law count cannot establish readiness.

## Decision

Accept the [first native slice](../architecture/first-native-slice.md) as the design baseline, with its threat model, versioned protocol, platform constraints and implementation gates. Accept LAW-041, LAW-042 and LAW-043 for the independent gaps documented by the [law audit](../architecture/law-audit.md). Scope future work through the [decision register](../research/open-questions.md).

## Alternatives

Declare the initial 17 laws universally sufficient; design every future subsystem now; implement the kernel before resolving authority and failure outcomes.

## Why rejected

A number is not a coverage argument. Unused subsystems create speculative contracts, while premature implementation embeds unresolved ownership choices.

## Consequences

Phase 0 closes at the verified design baseline described in the [review](../research/phase-0-review.md). Phase 1 still needs explicit authorization. Research debt remains visible and cannot justify stronger claims.

## Compatibility impact

The first slice is native-only. No foreign ABI or compatibility performance promise is added.

## Performance impact

Fixed small capacities make exhaustion observable. Their values are experiment parameters, not universal kernel laws; no latency result has been measured.

## Security impact

Isolated service intent does not prove hardware isolation. Privileged panic is fatal; ambiguous effects are not automatically retried.

## Testing

Run formatters, linters, workspace tests, finite models and the host process experiment; cross-compile the no_std protocol candidate. Preserve negative controls and explicit hardware limitations.

## Reversibility

Revise the decision, paired specifications and model evidence when the workload or a counterexample changes a premise. Never silently expand the meaning of a passed finite model.

[Russian translation](../../translations/ru/docs/architecture-decisions/0009-native-slice-baseline.md)
