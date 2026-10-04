# ADR-0018 — Bounded synchronous safe user copy

Status: **Accepted for bounded Phase 3.2**. Date: 2026-10-03.

Document status: CURRENT
Evidence scope: current-process memory boundary on two fixed-affinity CPUs; no public pointer ABI.
Current reference: [User-copy contract](../kernel/user-copy.md)

## Context

Issue #22 requires protected copying before future IPC payload admission. [Phase 3.1](0017-process-lifecycle.md) supplies retained immutable private roots and exact process generations. Numerical EL0 addresses cannot safely become kernel pointers; permission preflight without lifetime exclusion would leave a validation/use race. Snapshot admission must not depend on later user mutation.

## Decision

Use a synchronous current-task Access borrowed under existing indexed scheduler exclusion, with explicit bounded initialized Snapshot capacity and byte-slice output. Require exact root/process/queue identity and executing ownership; keep immutable mappings and the existing two-CPU retirement barrier. Hardware AT S1E0R/W preflight checks all covered pages; LDTRB/STTRB enforce EL0 permissions at actual access. Recover only precise translation/access/permission faults at those exact instructions while a per-CPU copy guard is active, with matching syndrome, access direction and in-range FAR. Input publishes all-or-nothing snapshots; output errors report the completed prefix. Keep ordinary EL1 invariant faults fatal.

## Alternatives

Raw pointer casts, software-only mapping checks, physical aliases, universal pinning, mapping locks and generic UserObject abstractions were considered.

## Why rejected

Raw casts lose permissions/lifetime. Software-only checks omit architectural faults. Physical aliases can bypass user permissions. Pinning, locks and generic objects add unneeded mechanisms while mappings cannot change. A future mapping writer or shared user process must revisit exclusion; this decision grants no future concurrency exception.

## Consequences

Copies are bounded and synchronous. Failed input publishes no snapshot; output may publish a prefix only with an explicit error count. Immutable current mappings and exact process generation retain lifetime until return; asynchronous/shared-mapping access remains unsupported.

## Compatibility impact

The memory API has no Linux errno, compatibility pointer format or internal Rust struct ABI. Compatibility translates its semantics above the same native boundary.

## Performance impact

Preflight checks each page and byte loops prioritize fault precision over throughput. Fixed caller-selected snapshot capacity bounds stack initialization. DEV/PROD raw QEMU samples establish regression observations; no fastest-system claim follows.

## Security impact

EL0 services cannot attest current trusted TTBR/owner or contain EL1 copy faults. Only hardware access and current-process lifetime enforcement belong in EL1; parsing, authority policy and future service effects do not. The native closure stays kernel/kernel-core with no compatibility-specific errno, layout or routing dependency. This enforces LAW-001, LAW-013, LAW-018, LAW-026, LAW-035, LAW-036 and LAW-041 without changing a law. [Unsafe policy](../architecture/unsafe-policy.md) and [security model](../security/threat-model.md) remain applicable.

## Testing

The exact-source [Phase 3.2 receipt](../../research/results/kernel-phase32.json) passes 69 kernel checks in each DEV/PROD profile and 57 host negative controls; all 70 recorded source hashes identify its exact milestone candidate; subsequent changes require new evidence. Require actual two-CPU DEV/PROD EL0 copies and snapshot mutation, mid-copy fault injection, generation/lifetime rejection, initialized outputs, restored native roots/TLBI, returned frame counts, lifecycle regression, controls detecting disabled recovery and live reread, raw bounded benchmarks, native-only/Phase 2 matrices, host/Clippy/repository checks and exact-source evidence for future changes. These are bounded QEMU observations; host models and inventories do not replace kernel execution. [User-copy contract](../kernel/user-copy.md) defines the detailed failure policy and non-goals.

## Reversibility

Revisit the 12 KiB limit and per-request stack capacities only with actual workload/evidence. Relaxing synchronous lifetime, immutable mappings, affinity, IRQ exclusion or precise fault recovery requires a new proof and decision. Handles, capabilities, security domains, IPC, stable public ABI and Linux semantics remain separate work.

[Russian translation](../../translations/ru/docs/architecture-decisions/0018-safe-user-copy.md)
