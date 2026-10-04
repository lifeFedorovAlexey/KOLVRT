# ADR-0014 — Fixed-affinity EL0 execution foundation

Document status: CURRENT
Evidence scope: Accepted intended obligations; implementation and verification remain separately scoped.
Current reference: [Documentation policy](../documentation-policy.md)

Status: **Accepted for the bounded foundation**. Date: 2026-10-02.

## Context

After verified SMP, the user authorized address spaces, EL1/EL0 transitions, stacks, context switching, timer scheduling, process isolation and contained faults on both CPUs. IPC, handles, cancellation, service models and security domains are explicitly deferred. [EL0 contract](../kernel/el0.md) defines the actual workload; the full native slice is not complete.

## Decision

Use independent per-CPU queues with fixed process affinity and an atomic executing owner per process. Preserve the complete architectural context and share only privileged kernel mappings between private roots. ASID zero plus completed local TLBI on every switch avoids premature ASID reuse. Retain all space guards until both CPUs have restored the native root and published completion. No global scheduler lock, remote address-space mutation or migration is introduced.

## Alternatives

One CPU only; a global locked queue; migration/work stealing; ASID-tagged targeted invalidation; cooperative switching; lazy SIMD/FP saving.

## Why rejected

The first does not exercise the requested SMP boundary. A shared queue adds IRQ lock ownership and progress obligations without this workload needing cross-CPU admission. Migration and ASID reuse need participant/lifetime protocols absent from the fixed-affinity contract. Cooperative-only execution cannot bound a non-yielding process. Lazy FP requires separate trap/ownership handling. Full local invalidation has a visible cost and no fastest-method claim.

## Consequences

Eight processes run in one static boot session. Dynamic loading/creation, migration and general process management remain unsupported. Pure round-robin selection is separated from architecture mechanisms. The trusted boot fixture chooses images and budgets; its validation assertions are not a user authority service.

## Compatibility impact

The kernel dependency closure remains kernel and kernel-core. Routing/adapter groundwork is preserved and disconnected. No foreign semantics, syscall compatibility or version-dependent task behavior is introduced.

## Performance impact

The chosen quantum is a typed 1 ms Duration. Fairness is conditional on timer delivery and bounded handlers; TCG observations do not establish a latency or throughput ranking. Every switch saves all GPR/SIMD/FP/TLS state and flushes local translations. ASIDs, lazy state or targeted flushes require evidence before optimization.

## Security impact

ERET/exception vectors, TTBR/TLBI and timer enforcement require privileged instructions; an EL0 process cannot safely install its own trusted exception return or protection root. This is the privileged necessity argument under [admission policy](../architecture/kernel-admission-policy.md). Allocation/image selection and round-robin policy are bootstrap choices, not irreducible global-authority services. Reconsider their placement when IPC/supervision exists. Privileged identity aliases remain within the TCB; no DMA or security-domain claim is made.

[Arm exception model](https://documentation-service.arm.com/static/67ac57fb091bfc3e0a9479cc), sections 5.1–5.2, describes vector/stack selection and ERET restoration from SPSR/ELR. [Arm memory management](https://developer.arm.com/-/media/Arm%20Developer%20Community/PDF/Learn%20the%20Architecture/LearnTheArchitecture-MemoryManagement-101811_0100_00_en.pdf), sections 5.2 and 8, motivates ASID identity and TLB maintenance. These architectural sources support mechanisms; actual correctness evidence is the pinned kernel workload.

## Testing

Both DEV/PROD require 53 real tests, actual non-test boot execution and eleven negative host controls. EL0 tests exercise same-VA private data tags, register/SIMD/FP/TLS and stack restoration, kernel/foreign memory rejection, RO/NX/guard faults, privileged instruction rejection, peer survival and reclaimed frames. Root/context/forgotten-guard controls must fail. [Unsafe register](../kernel/unsafe.md) states the local proof obligations; hardware weak-memory behavior remains unverified.

## Reversibility

Future scheduling domains or EL0 policy services may replace the static selector only with explicit ownership, admission, lifetime, timeout and remote quiescence contracts. Migration/ASID reuse require a new reviewed decision. This milestone creates no stable userspace ABI.

[Russian translation](../../translations/ru/docs/architecture-decisions/0014-el0-foundation.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0014",
  "kind": "adr",
  "aliases": ["ADR-0014"],
  "summary": "Scoped architecture decision; acceptance is intended authority, not proof of all implementation.",
  "tags": ["architecture", "decision"]
}
```
