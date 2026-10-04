# EL0 and scheduling foundation

This milestone implements real AArch64 EL0 execution and timer-driven switching on both pinned CPUs. Eight statically selected processes run in independent address spaces, four pinned to each CPU. Two workers finish; six other processes fault without terminating their peers or the kernel. The boot workload then reclaims resources and powers off. This is a bounded foundation, not a running general-purpose OS. This paragraph records the original foundation workload. Later milestones add bounded user-copy, process-local handles, transfer, ELF loading and fixed-affinity ASIDs; general IPC, native grants, cancellation, service models and security domains remain separate gates. Routing/adapters remain outside the kernel dependency closure.

## Execution and ownership

```text
CPU0: allocate/zero frames → create private roots → publish immutable task setup
Each CPU: acquire launch → rendezvous → acquire process ownership → TTBR0 switch
→ ERET EL0t → local timer or synchronous exception → EL1 stack/context
→ stop source/EOI → save task → release owner → select ready task → ERET
No runnable task: restore native ASID-0 root → return EL1
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

Scheduling policy is the pure bounded round-robin selector in crates/kernel-core/src/scheduling.rs. Privileged mechanism lives in crates/kernel/src/scheduler/mod.rs and arch/aarch64. The quantum is a typed 1 ms Duration selected by the scheduler; it is not a measured latency guarantee. A ready process receives a turn within one rotation assuming local timer delivery and bounded EL1 handlers. No IRQ allocation, UART logging, queue lock or interrupted ordinary-code lock is needed. Sixteen slices and a two-second workload deadline bound hostile/nonterminating images. Expiry marks unfinished tasks timed out; it never frees an executing address space.

## Address spaces and context

Each native fixture process owns three table pages, a code page, a data page and four stack pages; optional opaque images add bounded RX pages. The shared kernel mappings remain privileged-only. At the common user VA, code is RX with privileged execution disabled; data/stack are RW and NX; the guard is absent. A process-specific alias exists only in that process's root, enabling a real foreign-address rejection. Distinct tags written at the same data VA verify that TTBR switching reaches distinct physical pages. Kernel code/data and devices are not user-accessible.

The bounded executable loader accepts ELF64 little-endian AArch64 version 1, ET_EXEC only, with checked 64-byte file and 56-byte program headers. Its initial kernel execution profile accepts exactly one read/execute PT_LOAD at `0x20040000`, with file offset and virtual address aligned to 4 KiB and `p_align` exactly 4 KiB, within a 128 KiB user image window. The entry must point to file-backed bytes in that segment. Writable, RWX, overlapping, out-of-window, overflowing and malformed images are rejected before reserving a process slot or frames. The allocated image page is zeroed before file bytes are copied, so BSS and padding start at zero; instruction cache maintenance completes before process publication. ET_DYN, interpreters, PT_DYNAMIC, TLS, relocations, writable data segments and multi-segment execution are unsupported by this initial kernel adapter. Format validity does not grant execution authority; creation remains bootstrap-only.

The native root always uses ASID zero. Owned process roots receive nonzero leases from the width reported by `ID_AA64MMFR0_EL1.ASIDBits` (8 or 16 bits; `TCR_EL1.AS` enables 16-bit tags where supported). A lease carries its pinned CPU, slot and monotonic software epoch; the hardware namespace is per CPU, so equal ASID numbers on CPU0 and CPU1 are distinct. Ordinary root switches write TTBR0 with the leased ASID and do not invalidate the TLB. On terminal completion, the owning CPU executes `DSB ISH; TLBI ASIDE1; DSB ISH; ISB` before publishing retirement. Detach, local retirement and acquired scheduler completion all precede frame/table release and lease reuse. Creation rollback may release a lease only before its root was published.

Fixed affinity remains mandatory. A root is never admitted on multiple CPUs; any future shared-root or migration design must track residency and wait for acknowledged remote invalidation before reuse. User roots remain immutable while admitted, and native shared mapping changes remain rejected during user execution. If the ASID width encoding is unsupported, ASID zero plus full local flush on each switch remains the fallback. `asid-baseline` selects this same full-flush behavior for comparison. The [SMP retirement protocol](smp.md) continues to protect native mutable mappings.

The 816-byte user context extends the 784-byte GPR/SIMD/FP frame with ELR, SPSR, SP_EL0 and TPIDR_EL0. Compile-time offset/size assertions match assembly. The kernel caller's context, SP_EL0 and TLS register are restored before returning to EL1. Lower-EL synchronous and IRQ vectors use the CPU's kernel stack. User faults mark only the current process terminal; current-EL faults retain the fatal kernel policy. Register/tag expectations belong to the trusted workload verifier, not authority checks in the trap mechanism. The one recognized SVC ends this static workload; it is not a stable syscall ABI or a capability operation.

## Checks and limits

Worker completion is verified after all three local peers have faulted. This witnesses resumed useful execution after faults, rather than counting a worker that exited before them. The final measurement label is `el0-foundation`; earlier EL0 development records remain historical.

The original EL0 foundation had 53 checks; current DEV/PROD counts are bound to the retained matrix source inventory. Added ASID coverage forces a four-tag-per-CPU pool to exhaustion, then reuses tags against different physical backing at the same user VA on both CPUs. `--asid-reuse-control` omits retirement TLBI and must fail `asid_reuse_requires_invalidation`. QEMU TCG still observes isolation with the omission enabled, so the negative check enforces the required invalidation invariant directly. The non-test boot also executes the process workload; machine validation requires its EL0 event before boot success.

The matrix runs both DEV and PROD profiles, including real execution of two instances of the minimal ELF in isolated EL0 address spaces, zero-checked BSS/padding, and malformed-image rollback controls for W^X, foreign addresses, bad entries and range overflow.

Twelve failure-propagation controls must fail, alongside the scheduler rejection controls in DEV and PROD. `--asid-reuse-control` deliberately omits reuse invalidation. [Results](../../research/results/kernel-el0.json) retain the historical EL0 source scope. The issue #18 QEMU baseline/tagged comparison is retained separately in [ASID measurements](../../research/results/issue18-asid-measurements.json). [Unsafe inventory](../../research/results/kernel-el0-unsafe-audit.json) is an inventory, not a safety proof.

The retained paired benchmark measures 32 process create/run/reclaim cycles per CPU, counterbalanced across eight QEMU pairs; it reports timer ticks, TTBR switches, full/scoped invalidation counts and ASID reuse. This is QEMU TCG evidence only. It does not establish physical weak-memory behavior, hardware throughput, hard real-time progress or arbitrary user programs. The ELF subset is a loader validation slice, not support for general applications. Fixed-affinity ASID lifecycle and bounded safe-copy are implemented; trusted executable authorization, signer policy, dynamic linking, writable ELF segments, migration, work stealing, nested IRQ, lazy FPU, demand paging and a general executable ABI need separate contracts and tests. No DMA boundary, full native slice, IPC or security-domain claim follows from these processes.

[ADR-0014](../architecture-decisions/0014-el0-foundation.md) records placement, alternatives and reconsideration. Existing [Kernel Laws](../architecture/kernel-laws.md), especially LAW-001, LAW-013, LAW-025, LAW-031 and LAW-041, are unchanged: dependency isolation, retained charges, scoped platform limits and explicit progress bounds continue to apply.

The later [Phase 2 workload](routing.md) reuses this native mechanism with sequential CAS-admitted batches and opaque EL0 code. Historical EL0 records retain their original one-page/single-batch source hashes; current geometry and admission are documented in ADR-0015.

## Phase 3.0 ownership update

The later [scheduler contract](scheduler.md) supersedes the queue/setup description above: observed CPU/IRQ/phase/generation checks and nonblocking permits gate all storage accesses. The AArch64 frame and static verifier are separate. EL0 writes its own fixture tick word through the existing own-slices call; kernel IRQ no longer writes fixture data. Runtime storage outlives both fixture and payload sessions. Phase 3.1 validation added generation-safe process lifecycle, `Registry::step()` preemption checks and scheduler ownership controls; see the exact-source process and scheduler receipts. Phase 3.2 accepts a bounded [safe user-copy contract](user-copy.md); IPC and public authority remain separately gated work under Phase 3 — Native Process & Service Foundation.

[Russian translation](../../translations/ru/docs/kernel/el0.md)
