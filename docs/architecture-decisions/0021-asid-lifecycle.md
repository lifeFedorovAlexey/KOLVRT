# ADR-0021 — Fixed-affinity AArch64 ASID lifecycle

Status: **Accepted for bounded process roots**. Date: 2026-10-03.

Document status: CURRENT
Evidence scope: two fixed-affinity CPUs, private immutable process roots, AArch64 QEMU and the declared ASID-zero fallback; no migration or shared-root execution.
Current reference: [EL0 address-space contract](../kernel/el0.md)

## Context

The EL0/process foundation assigned ASID zero to every root and completed a full local TLBI after each TTBR0 switch. That was a simple correctness baseline, but it invalidated translations unrelated to the outgoing process. ASID reuse can recover this cost only if the previous owner has stopped using the tag and every CPU that could retain the translation has invalidated it before reuse. The process lifecycle already enforces fixed affinity and waits for scheduler detachment before frame reclamation.

## Decision

- Reserve ASID zero for the native root and the explicit full-flush fallback. Read `ID_AA64MMFR0_EL1.ASIDBits`; use the reported 8-bit namespace, or enable `TCR_EL1.AS` and use 16 bits when that encoding is supported. Unknown encodings disable tagged mode.
- Give each owned process root a nonzero lease containing its ASID, a monotonically increasing software epoch, its fixed owner CPU and allocator slot. ASID uniqueness is required among live roots on one CPU; hardware ASID namespaces are per CPU, so two CPUs may hold the same numeric ASID concurrently.
- For tagged roots, an ordinary TTBR0 switch writes root plus ASID and performs no TLBI. Native ASID-zero entry in tagged mode also retains unrelated translations.
- Process mappings remain immutable after admission. On terminal completion, the fixed owner CPU executes `DSB ISH; TLBI ASIDE1; DSB ISH; ISB`; only after that local invalidation has completed may it publish the lease as retired. Acquired scheduler completion and detachment precede frame/table release and ASID reuse. A creation rollback may release a lease immediately only if the root was never published.
- Keep root activation pinned to one CPU. No root can be resident on multiple CPUs in this scope. Any future shared-root or migration support must add residency tracking and acknowledged remote invalidation before retirement/reuse; ASID support alone does not provide migration.
- Preserve the ASID-zero/full-local-flush mode both as unsupported-hardware fallback and as the explicit `asid-baseline` measurement build.

## Alternatives

- Continue full `VMALLE1` invalidation after every switch.
- Reuse tags without a retirement acknowledgement, or rely on the physical allocator returning the same frames.
- Use an inner-shareable remote TLBI for every local process exit despite the fixed-affinity contract.
- Treat an ASID number as a globally unique process identity or as permission to migrate a root.

## Why rejected

Full invalidation discards translations from unrelated live roots. Reuse without a completed owner-local invalidation permits a stale VA-to-PA translation to reach a later process. Allocator coincidence is not an architectural guarantee. Remote invalidation is not needed when a root is provably pinned to one CPU; adding it would enlarge the current SMP contract without a reader. ASIDs are translation tags, not process identity, ownership or authority.

## Consequences

Each live process root has its own non-global translation namespace on its owner CPU. A terminal root is locally invalidated before its lease and backing frames can be reused. The existing global kernel mappings are shared unchanged; private user leaves are marked not-global. The bounded scheduler still uses the existing completion barrier, retains fixed affinity, and exposes no user-controlled mapping changes. Unsupported ASID encodings retain the original full-flush behavior.

## Compatibility impact

The ASID allocator is private to EL0 process roots. The native root keeps ASID zero, and unsupported or unknown hardware ASID widths retain the existing ASID-zero/full-flush contract. The change does not alter legacy routing, native authority, process image format, or fixed CPU affinity.

## Performance impact

The issue #18 benchmark runs eight counterbalanced QEMU pairs of 32 process create/run/reclaim cycles on each CPU. It records timer ticks, TTBR switches, full and ASID-scoped TLBI counts, process-tag reuse, build/run metadata and exact source inventory in [issue18-asid-measurements.json](../../research/results/issue18-asid-measurements.json). Positive paired differences favor tagged mode. These are QEMU TCG observations only; they do not establish silicon throughput or a hardware speedup.

## Security impact

The per-root epoch rejects stale software retirement operations. The allocator does not make a lease reusable until the pinned owner has completed ASID invalidation and the coordinator has acquired scheduler quiescence. The QEMU test pool is restricted to four nonzero tags per CPU to force exhaustion; same-VA tests hold retired physical backing so a missing invalidation cannot pass through allocator address reuse. `--asid-reuse-control` omits that TLBI and is required to fail. These checks do not prove physical weak-memory behavior or remote invalidation for an unimplemented shared-root design.

## Testing

The DEV/PROD matrix checks same-VA isolation, exhaustion on both CPUs, generation-safe reuse, stale-translation rejection, scheduler detachment and physical reclamation. The focused omitted-invalidation control must fail `asid_reuse_requires_invalidation` based on the observed per-CPU retirement TLBI counts, rather than a compile-time feature flag. Hardware TTBR readback checks the effective tag on every tagged activation; both 8-bit and 16-bit tags begin at bit 48. QEMU TCG still observes isolation with the invalidation omitted; the negative control verifies that the required retirement operation was actually issued. `cargo xtask asid-bench` retains paired tagged and ASID-zero/full-flush runs. Repository host checks, Clippy, formatting and source-diff checks remain required. QEMU does not substitute for hardware review before any hardware performance claim.

## Reversibility

The allocator and switch contract are private to the bounded kernel process runtime. Before increasing CPU count, allowing root sharing, mutating process mappings, or admitting migration, revisit ASID residency and remote shootdown obligations. The ASID-zero baseline remains available for comparison and fallback.

[Russian translation](../../translations/ru/docs/architecture-decisions/0021-asid-lifecycle.md)
