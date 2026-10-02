# ADR-0010 — Native EL1 foundation

Status: **Accepted**. Date: 2026-10-02.

## Context

The user authorized a real kernel before scheduler, userspace and compatibility. The earlier native EL0 slice remains a later milestone. A complete preceding level is allowed when SMP cannot yet be established correctly.

## Decision

Accept the working CPU0 foundation described by [boot](../kernel/boot.md), [memory](../kernel/memory.md), [interrupts](../kernel/interrupts.md) and [CPU scope](../kernel/smp.md). Use independent safe algorithms in kernel-core, processor mechanisms in arch/aarch64, platform discovery in platform and checked MMIO in hal. Native Cargo closure is only kernel and kernel-core plus compiler runtime. No Linux code or ABI is imported.

## Alternatives

Implement secondary CPU and EL0 services immediately; build a foreign-kernel compatibility base; keep untested simulated subsystems. Memory and synchronization alternatives are compared in the [implementation review](../architecture/implementation-review.md).

## Why rejected

Those choices violate the authorized order or introduce unproved ownership, retirement and IRQ dependencies. Familiar directory names are not evidence for inheriting an operating-system design.

## Consequences

The boot and test kernels are real no_std/no_main binaries. Two configured CPUs do not mean SMP; only CPU0 is active. Fixed capacities and identity mapping are explicit current contracts. Scheduler and EL0 work does not start automatically.

## Compatibility impact

None: no foreign ABI, loader, syscall adapter or compatibility dependency is implemented.

## Performance impact

Shared core, DEV-only diagnostic features, optimized PROD without tests, fixed metadata and bounded operations. Full SIMD preservation and full TLBI have visible costs. No global fastest-method claim is accepted without comparative evidence.

## Security impact

Actual W^X, RO/NX faults, validated DTB, reservation exclusion, private frame ownership and fatal unexpected exceptions. IRQ never depends on allocator or interrupted locks. The current EL1 kernel is not an EL0 isolation implementation.

## Testing

cargo xtask test runs both profiles, actual boot images, 23 in-kernel tests per profile and failing-host assertion/panic controls. Host tools check algorithms, translations, models and lint. A fresh source export must rebuild without target outputs; installed pinned tools and dependency caches are prerequisites.

## Reversibility

Replace a mechanism only with a new reviewed contract, invariant analysis and real negative tests; no temporary second implementation. Before SMP, establish remote-reader retirement and per-CPU state. Hardware and a second platform remain unverified.
