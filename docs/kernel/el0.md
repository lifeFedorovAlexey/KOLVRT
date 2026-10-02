# EL0 and scheduling foundation

This milestone implements real AArch64 EL0 execution and timer-driven switching on both pinned CPUs. Eight statically selected processes run in independent address spaces, four pinned to each CPU. Two workers finish; six other processes fault without terminating their peers or the kernel. The boot workload then reclaims resources and powers off. This is a bounded foundation, not a running general-purpose OS. IPC, handles/capabilities, cancellation, service models and security domains remain the next stage. Routing/adapters remain outside the kernel dependency closure.

## Execution and ownership

```text
CPU0: allocate/zero frames → create private roots → publish immutable task setup
Each CPU: acquire launch → rendezvous → acquire process ownership → TTBR0 switch
→ ERET EL0t → local timer or synchronous exception → EL1 stack/context
→ stop source/EOI → save task → release owner → select ready task → ERET
No runnable task: restore native root → completed local TLBI → return EL1
CPU0: acquire both completions → drop space guards/charges → release frames
```

| State/resource                            | Owner and mutability                                                            | Publication, lifetime and reclamation                                                                                     |
| ----------------------------------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Physical allocations and UserSpace guards | CPU0, non-transferable Frame borrow                                             | Zero/init before launch; persistent per-space charge rejects release of a forgotten guard                                 |
| Root/L2/L3 and code                       | CPU0 constructs, immutable while admitted                                       | Nine native fixture pages plus bounded optional image pages per process; publication/cache maintenance before launch      |
| Data and user stack                       | One pinned process; its owning CPU accesses data while the process is suspended | No cross-process user mapping; reads by the verifier only after both CPU completions                                      |
| Ready queue and saved context             | Indexed CPU only, IRQ masked                                                    | UnsafeCell is not a shared-writer abstraction; no references span user execution                                          |
| Running ownership                         | Per-process atomic CPU identity                                                 | CAS rejects duplicate execution; fixed-affinity check rejects another CPU; release on every terminal/preempted transition |
| Kernel stack                              | Existing permanent per-CPU EL1 stack                                            | User SP is never used for kernel exception entry; user stack has an unmapped lower guard page                             |
| Setup and completion                      | Release launch, acquire consume; release done, acquire collect                  | Sequential boot batches use CAS admission and acquired completion; no migration or hotplug support is claimed             |

Scheduling policy is the pure bounded round-robin selector in crates/kernel-core/src/scheduling.rs. Privileged mechanism lives in crates/kernel/src/scheduler.rs and arch/aarch64. The quantum is a typed 1 ms Duration selected by the scheduler; it is not a measured latency guarantee. A ready process receives a turn within one rotation assuming local timer delivery and bounded EL1 handlers. No IRQ allocation, UART logging, queue lock or interrupted ordinary-code lock is needed. Sixteen slices and a two-second workload deadline bound hostile/nonterminating images. Expiry marks unfinished tasks timed out; it never frees an executing address space.

## Address spaces and context

Each native fixture process owns three table pages, a code page, a data page and four stack pages; optional opaque images add bounded RX pages. The shared kernel mappings remain privileged-only. At the common user VA, code is RX with privileged execution disabled; data/stack are RW and NX; the guard is absent. A process-specific alias exists only in that process's root, enabling a real foreign-address rejection. Distinct tags written at the same data VA verify that TTBR switching reaches distinct physical pages. Kernel code/data and devices are not user-accessible.

All switches use ASID zero and completed full local TLBI, including the return to the native root. There is no ASID allocator or migration; a root is admitted on exactly one CPU. Shared native mapping changes are rejected while user execution is active. Private roots are immutable during admission, so their reclamation relies on both CPUs returning to native roots and completing local invalidation, not on an IRQ acknowledgement of a live user reader. The [SMP retirement protocol](smp.md) continues to protect native mutable mappings.

The 816-byte user context extends the 784-byte GPR/SIMD/FP frame with ELR, SPSR, SP_EL0 and TPIDR_EL0. Compile-time offset/size assertions match assembly. The kernel caller's context, SP_EL0 and TLS register are restored before returning to EL1. Lower-EL synchronous and IRQ vectors use the CPU's kernel stack. User faults mark only the current process terminal; current-EL faults retain the fatal kernel policy. Register/tag expectations belong to the trusted workload verifier, not authority checks in the trap mechanism. The one recognized SVC ends this static workload; it is not a stable syscall ABI or a capability operation.

## Checks and limits

Worker completion is verified after all three local peers have faulted. This witnesses resumed useful execution after faults, rather than counting a worker that exited before them. The final measurement label is `el0-foundation`; earlier EL0 development records remain historical.

The matrix requires 53 tests per DEV/PROD profile: the existing 39 foundation checks and 14 EL0 checks covering processes, timer switches, user stacks, memory isolation, fault containment, quiescent reclamation, GPR/SIMD/FP/TLS context, SMP ownership, kernel/foreign-memory rejection, code write rejection, guard-page access, NX data and a privileged instruction. The non-test boot also executes the workload; machine validation requires its EL0 event before boot success.

Eleven negative host commands must fail. New controls are `--user-context-control` (corrupt a saved marker), `--user-root-control` (retain the native root instead of switching), and `--user-retirement-control` (forget a space guard and attempt frame release). These supplement the existing eight SMP/foundation controls. [Results](../../research/results/kernel-el0.json) retain exact source hashes, artifact features/sizes, events and timer samples. [Unsafe inventory](../../research/results/kernel-el0-unsafe-audit.json) is an inventory, not a safety proof.

QEMU TCG evidence does not establish physical weak-memory behavior, hardware throughput, hard real-time progress or arbitrary user programs. The static allocator/bootstrap workload is not a production process-creation interface; dynamic creation, ASID lifecycle, migration, work stealing, nested IRQ, lazy FPU, demand paging, user-copy and general executable loading need separate contracts and tests. No DMA boundary, full native slice, IPC or security-domain claim follows from these processes.

[ADR-0014](../architecture-decisions/0014-el0-foundation.md) records placement, alternatives and reconsideration. Existing [Kernel Laws](../architecture/kernel-laws.md), especially LAW-001, LAW-013, LAW-025, LAW-031 and LAW-041, are unchanged: dependency isolation, retained charges, scoped platform limits and explicit progress bounds continue to apply.

The later [Phase 2 workload](routing.md) reuses this native mechanism with sequential CAS-admitted batches and opaque EL0 code. Historical EL0 records retain their original one-page/single-batch source hashes; current geometry and admission are documented in ADR-0015.

[Russian translation](../../translations/ru/docs/kernel/el0.md)
