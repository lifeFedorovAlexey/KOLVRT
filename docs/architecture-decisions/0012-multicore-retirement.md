# ADR-0012 — Multicore ownership and acknowledged retirement

Document status: CURRENT
Evidence scope: Accepted intended obligations; implementation and verification remain separately scoped.
Current reference: [Documentation policy](../documentation-policy.md)

Status: **Accepted for two-CPU QEMU**. Date: 2026-10-02.

## Context

Activating CPU1 invalidates CPU0-only lifetime assumptions. Cases [09](../../research/cases/KOL-PATH-0009.json), [22](../../research/cases/KOL-PATH-0022.json) and [30](../../research/cases/KOL-PATH-0030.json) distinguish retained access, visibility and reclamation. The workload requires native SMP, without scheduler or compatibility runtime.

## Decision

Use PSCI CPU_ON, permanent separate stacks, DTB affinities and matching redistributors. Enforce CPU0 physical ownership/PTE mutation for this bounded milestone; retain existing TTAS locks for heap and tables. Publish one checked shootdown generation. IRQ records it; CPU1 acknowledges at ordinary reader quiescence after local TLBI/barriers. A retiring frame charge survives forgotten guards. Confirm quiescence and CPU_OFF before SYSTEM_OFF. The [SMP contract](../kernel/smp.md) specifies ownership, publication and failure boundaries.

## Alternatives

Broadcast TLBI alone; acknowledgement directly from IRQ; concurrent allocation/PTE writers with RCU or epochs now; CPU1 remaining offline; a new global kernel lock.

## Why rejected

The first two do not establish lifetime safety for interrupted readers. Extra writers/reclamation methods add unsupported state without a workload. Offline CPU1 fails the milestone. A global lock hides ownership and can block IRQ-dependent completion.

## Consequences

Two CPUs execute native code, but CPU1 has a coordination loop and test-only bounded work. One retirement and permanent storage keep obligations reviewable. Timeout is fatal, never permission to reuse. More CPUs, hardware, remote mutable access and dynamic address spaces require reopening the contract.

## Compatibility impact

Standalone Phase 2 groundwork is preserved without kernel hooks or dependencies. Immutable version/profile types are independent of CPU count. Existing standalone dispatch, switching and accounting are deferred runtime candidates. Future integration requires consumer ownership, synchronized publication, pinned request generations and per-CPU accounting review.

## Performance impact

The second permanent stack adds 256 KiB. Busy polling and full local invalidation cost time; no fastest-method claim is made. DEV and PROD use one protocol; correctness acknowledgements remain without diagnostics. [Evidence](../../research/results/kernel-smp.json) retains image sizes and timer observations; single-CPU and SMP runs are not equivalent-workload benchmarks.

## Security impact

No EL0 isolation is claimed. CPU1 cannot safely access physical ownership or UART. Test pointer probes have explicit retained-lifetime obligations. Secondary privileged failure halts the system; the identity mapping does not authorize access after release.

## Testing

Same 39 tests in DEV/PROD, separate boot images and eight negative controls. Omitted remote TLBI must fail after a real mapping read. Host checks cover DTB CPU/conduit rejection and dependency direction. QEMU is not a hardware weak-memory proof.

## Reversibility

Future writers replace CPU0 admission only through a reviewed per-domain protocol. Preserve drained generations and ownership; changing a CPU-count constant is insufficient. Phase 1 evidence remains historical. No Kernel Law changes are needed; scoped enforcement advances LAW-013, LAW-018, LAW-035, LAW-041 and LAW-043.

[Russian translation](../../translations/ru/docs/architecture-decisions/0012-multicore-retirement.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0012",
  "kind": "adr",
  "aliases": ["ADR-0012"],
  "summary": "Scoped architecture decision; acceptance is intended authority, not proof of all implementation.",
  "tags": ["architecture", "decision"]
}
```
