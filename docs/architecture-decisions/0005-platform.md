# ADR-0005 — ARM64-first platform contracts

Status: **Accepted**. Date: 2026-10-02.

Document status: HISTORICAL MILESTONE
Evidence scope: initial ARM64 platform decision; Phase 0.1 and 0.2 statements describe that planning snapshot.
Current reference: [Current scheduler milestone](0016-scheduler-ownership.md)

## Context

QEMU virt is a useful starting point. Device descriptions, DMA and silicon defects are distinct concerns (cases 10–13, 27).

## Decision

Start with AArch64 EL1, MMU, SMP, GICv3, timer, device tree, UART and VirtIO. Keep core contracts architecture-independent; add PCIe as transport needs justify it.

## Alternatives

Start with x86; hard-code a QEMU board layout; implement every firmware interface immediately.

## Why rejected

These conflict with the target, impede machine-version evolution, or expand scope without evidence.

## Consequences

Pin the kernel toolchain and machine in Phase 0.2. Phase 0.1 contains no boot code. The host documentation toolchain is pinned separately.

## Compatibility impact

An external compatibility environment does not promise AArch32 or x86 execution. Hardware adapters have separate scopes.

## Performance impact

QEMU results are not silicon performance measurements.

## Security impact

Interrupts, MMU and foreign descriptors require audited boundaries. DMA isolation policy remains open.

## Testing

Check device-tree fixtures and platform models, then emulator and hardware integration.

## Reversibility

A second port implements the contracts. If it requires a core rewrite, review the abstraction.

## Evidence

[Cases and sources](../research/case-index.md); [other systems](../research/reference-systems.md).

[Russian translation](../../translations/ru/docs/architecture-decisions/0005-platform.md)
