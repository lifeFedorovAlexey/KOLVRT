# First native slice

Status: accepted EL0 workload design baseline, not a completed EL0 implementation. A native EL1 kernel and two-CPU foundation now exist; this document describes the later isolated-task workload. The slice demonstrates native isolation and bounded communication: a supervisor creates two isolated tasks, grants one endpoint, exchanges a bounded message, revokes future admission, observes cancellation or service death, and shuts down without leaked ownership. This is a correctness workload, not a performance claim.

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

Use per-object locking and explicit retained references before considering lock-free reclamation. Handle lookup and reference acquisition occur under the same object-table lock. Lock order is domain table, object, queue; never hold these locks across allocation, external calls or blocking waits. No lock is acquired by an interrupt handler that can interrupt its holder. An interrupt only acknowledges its source and schedules bounded deferred work.

Start with a single scheduling domain and equal round-robin task shares. A ready task is served within one rotation if timer interrupts arrive and non-preemptible sections finish within the declared budget. This is conditional progress, not a hard real-time guarantee. A stalled CPU or broken timer violates the assumption and invokes the platform failure policy. Pin the actual timer quantum during implementation measurements rather than inventing a latency target now.

## Scope boundaries

No filesystem, network stack, persistent transactions, DMA-capable driver, hardware hotplug, foreign ABI, live code replacement or compatibility score is required for this slice. Their contracts remain in the architecture; implementation is activated only by a later workload and its acceptance tests. Static task images are selected by the trusted supervisor; arbitrary executable loading and package trust are separate work.

The [threat model](threat-model.md), [candidate ABI](native-abi.md), [platform contract](platform-contract.md) and [law audit](law-audit.md) define the design baseline. Phase 0 verifies finite host models and encoding; Phase 1 must implement and test real privilege separation, faults, interrupts and page tables before claiming this workload works in KOLVRT.

[Russian translation](../../translations/ru/docs/architecture/first-native-slice.md)
