# Performance trade-offs

## Evidence

No comparable latency, throughput, CPU cost or TCB-size result for the investigated systems was measured locally. Published numbers from different machines/configurations would not rank these designs. seL4 documents a fast IPC path with explicit eligibility conditions. [IPC fastpath](https://docs.sel4.systems/Tutorials/ipc.html). Capsicum's original paper illustrates incremental object-authority confinement in UNIX, without requiring wholesale microkernel conversion. [Watson et al., USENIX Security 2010](https://www.cl.cam.ac.uk/research/security/capsicum/papers/2010usenix-security-capsicum-website.pdf).

## Proposed experiment protocol

Use identical operations, security guarantees, failure outcomes, payload distributions, quotas and concurrency. Pin source, compiler, target, CPU count, accelerator/hardware, timer units and warm-up. Retain raw samples, failures, p50/p95/p99, throughput, CPU work, copies/bytes, memory and transition counts; report unknown where unavailable. QEMU TCG timings are emulator observations, not hardware latency claims.

| Candidate                            | Counterexample or cost to investigate                                    |
| ------------------------------------ | ------------------------------------------------------------------------ |
| Grant issuance plus local validation | Credential/revocation changes may invalidate an earlier decision         |
| Targeted state-dependent check       | Admission must serialize predicate changes; account contention           |
| Central policy evaluation            | Extra dispatch/state traversal may matter; cache without stale authority |
| Copied IPC                           | Bounded copies and snapshots; compare small payloads first               |
| Shared-memory lease                  | Mapping/pinning/cleanup cost, mutation, DMA and ordering proof burden    |
| Batching/asynchronous ring           | Amortized transitions versus latency, fairness, quotas and cancellation  |

## Placement consequence

An optimization cannot waive the effective-authority invariant or EL1 admission proof. Evaluate applicable alternatives; no requirement exists to implement every ring/zero-copy mechanism first. Reject using an expensive IPC benchmark alone to move a service into EL1. Current SMP work provides no equivalent-workload comparison for these future service designs. Prototypes start only after their scheduler/EL0/authority prerequisites are separately authorized.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/performance-tradeoffs.md)
