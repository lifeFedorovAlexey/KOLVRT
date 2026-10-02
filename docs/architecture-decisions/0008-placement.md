# ADR-0008 — Service protection domains

Status: **Accepted for the first native slice**. Date: 2026-10-02.

## Context

A library boundary cannot contain memory corruption. The [threat model](../architecture/threat-model.md) treats application and service failures as separate from privileged mechanism failures. The [host experiment](../../research/results/native-state-models.json) compares identical request and response frames inside one process and across a child-process pipe.

## Decision

Place the initial bounded echo service in a separate EL0 address space. Keep only irreducible privileged enforcement for authority, scheduling and memory protection in EL1; service arbitration and policy default to EL0 when existing narrow primitives suffice. The supervisor observes failure and may create a fresh service; it never silently retries an operation with unknown effects. This accepts a protection boundary, not a generic driver-placement rule.

Issue #5 requires an admission record for each new privileged responsibility under the
[kernel admission policy](../architecture/kernel-admission-policy.md). Name the exact
privilege/MMU/IRQ/protection invariant, demonstrate why an EL0 service plus existing
narrow primitives cannot enforce it, and admit only the minimum mechanism. Global
ownership, authoritative state, coordination, convenience and speed alone are insufficient.
The worked mapping-retirement record reviews an existing boundary, not a new admission.
This refines LAW-042 and ADR-0013 without adding a law or moving existing code.

## Alternatives

An in-process service; moving all device drivers and adapters out of the kernel immediately.

## Why rejected

The former cannot enforce the required failure boundary. The latter adds DMA, device-reset and compatibility requirements absent from the first workload. Neither follows from a popularity comparison between operating systems.

## Consequences

The first slice must implement genuine address-space isolation and bounded messages. Device and compatibility placement remains deferred until a concrete consumer and threat analysis exist.

## Compatibility impact

No foreign ABI is part of the first slice. Later adapters preserve whole state-domain identity.

## Performance impact

The host experiment establishes frame equivalence and failure observation only. It does not rank latency, copies or throughput. Measure these on the eventual implementation before optimizing the boundary.

Retain equivalent-workload latency, throughput and resource evidence with equal authority,
isolation, accounting and outcome guarantees. Evaluate applicable IPC improvements,
owned/shared-memory protocols and batching, recording unsuitable alternatives. Shared
memory still requires lifetime, permissions, ordering and cancellation analysis. Even
expensive IPC after optimization cannot admit service policy into EL1. Any protection-boundary
revision requires a new ADR and updated threat/failure model.

## Security impact

The host child uses the same account and is not a security sandbox. EL0 isolation, user-copy and access revocation remain implementation acceptance tests.

## Testing

Both paths return identical bytes; a child crash is observed, the supervisor survives, a fresh service responds, and an invalid response ID is rejected. Finite models separately cover cancellation, lifetime and transfer. No combined kernel execution has occurred.

## Reversibility

Moving a service into a privileged domain requires a new decision that revises the threat model and proves preservation of authority and outcome contracts.

[Russian translation](../../translations/ru/docs/architecture-decisions/0008-placement.md)
