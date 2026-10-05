# ADR-0016 — Scheduler responsibilities and enforced per-CPU ownership

Status: **Accepted for bounded Phase 3.0**. Date: 2026-10-02.

Document status: HISTORICAL MILESTONE
Evidence scope: accepted Phase 3.0 scheduler-ownership milestone; issue #20 process lifecycle was future work at decision time.
Current reference: [Scheduler and process-lifecycle contract](../kernel/scheduler.md)

## Context

[Issue #16](https://github.com/lifeFedorovAlexey/KOLVRT/issues/16) and [issue #17](https://github.com/lifeFedorovAlexey/KOLVRT/issues/17) require decomposition and enforced ownership before dynamic processes. Review uses current main and [ADR-0015](0015-el0-versioned-routing.md), not the issue's historical commit. A comment-only UnsafeCell contract, mixed fixture fields and raw setup/inspection cannot safely serve as an extensible lifecycle boundary. LAW-013, LAW-018, LAW-035, LAW-041 and LAW-043 remain obligations; no law or privilege exception is added.

## Decision

Separate pure selection and safe atomic ownership, kernel runtime state, the AArch64 frame/trap boundary and trusted bootstrap verification. Encapsulate storage behind observed CPU/IRQ/phase/generation checks, nonblocking exclusive permits and higher-ranked closures. Keep immutable task definitions private, copy completed results and read reports in bounded chunks. Permanent per-CPU slots outlive workloads; only checked quiescent setup replaces a generation. Maintain fixed affinity and the existing per-task running-owner CAS. Enforce both ordinary-lock/scheduler access directions and prevent guard transfer. [Contract](../kernel/scheduler.md) defines the actual scope.

## Alternatives

Retain comment-only UnsafeCell access; add a global scheduler lock; expose mutable guard references across user entry; use one universal lifecycle object now; implement migration/work stealing; move all scheduling policy into a future EL0 service immediately.

## Why rejected

Comments do not reject stale or foreign access. A global lock hides ownership and introduces IRQ progress dependencies. Retained references across exception re-entry violate aliasing. A universal object or migration requires unrelated lifecycle proofs and changes the authorized workload. Service policy placement needs the future native process/IPC foundation; this change admits no new policy service into EL1.

## Consequences

The eight-process workload retains required faults, survivor progress, stacks, registers and reclamation. Tick publication moves into the EL0 fixture using the existing own-slices call; marker/fault verification occurs after completion. Old timing samples are not equivalent instruction workloads. Static queue capacity is separate from fixture count. A second native session tests reset/reuse without reboot. Dynamic per-process lifecycle, vacant slots and independent process generations remain #20 work; queue generations are not a user handle ABI.

## Compatibility impact

The native dependency closure remains kernel/kernel-core. Routing and translation stay in the optional EL0 image with unchanged native observation semantics and separate regression evidence. Scheduler imports no routing, adapter, profile, capability or IPC policy.

## Performance impact

Each short storage access adds checked atomics and CPU/IRQ observations; this is correctness enforcement, not a speed claim. No contention wait or global queue lock is introduced. Context preservation, full local TLBI and typed quantum remain unchanged. Results are copied without allocating a full set of evidence buffers; bounded report chunks avoid exhausting the fixed 64 KiB heap. Host/QEMU evidence is not a fastest-method comparison.

## Security impact

Under the [admission policy](../architecture/kernel-admission-policy.md), this narrows existing privileged enforcement rather than adding an EL1 service. Protected current-task attribution, exception return, timer preemption and TTBR/TLBI require trusted privileged mechanisms; an EL0 service cannot install the trusted return/root or attest another kernel slot. Allocation, images, budgets and fixture expectations remain bootstrap choices, not admitted permanent service policy. Native issuer remains trusted bootstrap; scope is the current fixed-affinity task, with no delegation or stable grant API. Immutable roots survive both CPU completions and local invalidation; timeout does not authorize reuse. Privileged ownership failure is fatal. Compiler/firmware/QEMU trust and unverified silicon remain explicit. [Threat model](../security/threat-model.md) records the same boundary in DEV/PROD.

## Testing

Host tests cover wrong role, unmasked access, re-entry/concurrent exclusion, stale generation, live reset/read, duplicate start, reader/reset exclusion and generation exhaustion. Compile-time context assertions and source guards preserve the split. DEV/PROD require all previous 53 tests plus generation reuse, non-test boots, eleven original failure controls and fifteen ownership controls in both profiles. Actual routing regression and native-only dependency/build checks must pass; [results](../../research/results/kernel-phase3.json) bind claims to exact sources. Unsafe inventory remains an audit aid, not a proof.

## Reversibility

Next #20 extends this owned boundary with per-process identities, vacant/admitted/terminal transitions, fallible rollback, persistent admission and acknowledged reclamation. It must preserve permit lifetimes and prove new callers/phases rather than export storage. CPU0 is the current bounded allocator/coordinator, not a permanent restriction on future admission architecture. Migration, IRQ nesting, ASID reuse and new privileged responsibilities require separate decisions. #20, user-copy and IPC are not started by completing this milestone.

[Russian translation](../../translations/ru/docs/architecture-decisions/0016-scheduler-ownership.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0016",
  "kind": "adr",
  "aliases": ["ADR-0016"],
  "summary": "Scoped architecture decision; acceptance is intended authority, not proof of all implementation.",
  "tags": ["architecture", "decision"]
}
```
