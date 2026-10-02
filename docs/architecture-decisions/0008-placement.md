# ADR-0008 — Service protection domains

Status: **Accepted for the first native slice**. Date: 2026-10-02.

## Context

A library boundary cannot contain memory corruption. The [threat model](../architecture/threat-model.md) treats application and service failures as separate from privileged mechanism failures. The [host experiment](../../research/results/native-state-models.json) compares identical request and response frames inside one process and across a child-process pipe.

## Decision

Place the initial bounded echo service in a separate EL0 address space. Keep authority validation, request arbitration, scheduling and memory protection privileged. The supervisor observes failure and may create a fresh service; it never silently retries an operation with unknown effects. This accepts a protection boundary, not a generic driver-placement rule.

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

## Security impact

The host child uses the same account and is not a security sandbox. EL0 isolation, user-copy and access revocation remain implementation acceptance tests.

## Testing

Both paths return identical bytes; a child crash is observed, the supervisor survives, a fresh service responds, and an invalid response ID is rejected. Finite models separately cover cancellation, lifetime and transfer. No combined kernel execution has occurred.

## Reversibility

Moving a service into a privileged domain requires a new decision that revises the threat model and proves preservation of authority and outcome contracts.

[Russian translation](../../translations/ru/docs/architecture-decisions/0008-placement.md)
