# First native slice

Document status: DESIGN BASELINE
Evidence scope: complete isolated-task workload defined here (IPC, handles, cancellation, and failure containment); bounded EL0, scheduler, and routing milestones have separate implementation evidence.
Current reference: [Scheduler ownership and Phase 3.0](../architecture-decisions/0016-scheduler-ownership.md)

Status: accepted full native workload design baseline; this document does not claim that workload is complete. The bounded EL0/address-space, timer-scheduler and Phase 2 routing milestones have separate implementation evidence. This document describes the broader isolated-task workload: a supervisor creates two isolated tasks, grants one endpoint, exchanges a bounded message, revokes future admission, observes cancellation or service death, and shuts down without leaked ownership. This is a correctness workload, not a performance claim.

Current IPC decision, 2026-10-05: the locking paragraph below records the earlier workload proposal. Phase 3.5 rederived synchronization from current scheduler ownership and selected split nonblocking endpoint permits plus continuous fixed-affinity execution in [ADR-0025](../architecture-decisions/0025-bounded-native-ipc.md). The general workload here remains a design baseline; its supervisor, application policy and persistent-service acceptance are not part of #26.

## Observable acceptance

| Scenario                         | Required observation                                                                          |
| -------------------------------- | --------------------------------------------------------------------------------------------- |
| Normal request                   | Receiver obtains the exact copied payload; caller receives one terminal outcome               |
| Unauthorized or stale handle     | Rejection before queue admission or resource retention                                        |
| Full queue or allocation failure | Resource-exhausted result; no partial publication or leaked charge                            |
| Close during an accepted request | New lookup fails; accepted request retains its object until completion                        |
| Revoke racing admission          | Serialized admission either precedes revocation and finishes, or follows it and fails         |
| Cancel before effect             | No effect; one cancelled outcome                                                              |
| Cancel after effect              | Completion or effect-unknown result; no promise of rollback or blind retry                    |
| Service fault                    | Only that task is terminated; supervisor observes failure and can start a fresh instance      |
| Two runnable tasks               | Each receives service under the declared scheduler assumptions                                |
| Shutdown under load              | Stop admission, drain or terminate requests, quiesce callbacks/interrupts, then free mappings |

## Bounded starting configuration

Two application tasks plus a supervisor use separate address spaces. Endpoint admission allows one outstanding request; each message carries at most 256 payload bytes. Each test domain has two handle slots. These are small test and first-slice budgets chosen to make exhaustion observable, not permanent ABI maxima or law counts. Increasing them requires quota and contention tests, not a new architectural law.

The per-object ordinary-lock design below is retained as historical workload analysis; it is not the accepted Phase 3.5 execution mechanism. Phase 3.5 uses the split ownership and bounded permit rules in ADR-0025 and [the IPC contract](../kernel/ipc.md). Future milestones must rederive their synchronization rather than inherit either proposal by implementation order.

Start with a single scheduling domain and equal round-robin task shares. A ready task is served within one rotation if timer interrupts arrive and non-preemptible sections finish within the declared budget. This is conditional progress, not a hard real-time guarantee. A stalled CPU or broken timer violates the assumption and invokes the platform failure policy. Pin the actual timer quantum during implementation measurements rather than inventing a latency target now.

## Scope boundaries

No filesystem, network stack, persistent transactions, DMA-capable driver, hardware hotplug, foreign ABI, live code replacement or compatibility score is required for this slice. Their contracts remain in the architecture; implementation is activated only by a later workload and its acceptance tests. Static task images are selected by the trusted supervisor; arbitrary executable loading and package trust are separate work.

The [threat model](threat-model.md), [candidate ABI](native-abi.md), [platform contract](platform-contract.md) and [law audit](law-audit.md) define the design baseline. Phase 0 established finite host models and encoding. Later accepted milestones provide evidence for privilege separation, faults, interrupts and page tables; the full workload still needs acceptance evidence for IPC, handles, cancellation and failure containment before it can be claimed as implemented in KOLVRT.

The later [EL0 foundation](../kernel/el0.md), [versioned routing](../architecture-decisions/0015-el0-versioned-routing.md) and [scheduler ownership](../architecture-decisions/0016-scheduler-ownership.md) milestones implement their bounded scopes. Phase 3.5 has a separate experimental IPC implementation; this broad IPC/handles/cancellation/supervisor workload remains unimplemented and needs its own acceptance checks.

[Russian translation](../../translations/ru/docs/architecture/first-native-slice.md)

## Experimental supervision integration

The [Phase 3.6 supervisor](../kernel/supervision.md) supplies experimental real isolated EL0 service policy and a bounded lifecycle mechanism. The full native slice remains incomplete: bounded functional architecture/code and EN/RU review are complete after #126, with performance readiness separate, persistent integration belongs to #28 and production bootstrap trust to #38. Current-source receipts are separate from accepted historical Phase 3.5 evidence.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.architecture.first-native-slice",
  "kind": "navigation",
  "summary": "Broader isolated-task workload design, scoped separately from experimental bounded Phase 3.5 IPC."
}
```
