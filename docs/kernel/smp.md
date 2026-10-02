# Two-CPU correctness foundation

Phase 1.1 runs two AArch64 EL1 CPUs in pinned QEMU virt. No scheduler, EL0 or runtime compatibility is connected. [ADR-0012](../architecture-decisions/0012-multicore-retirement.md) records the decision; [Phase 1 review](../research/phase-1-review.md) remains the historical CPU0 milestone.

## Boot and publication

Validated DTB CPU affinities and PSCI SMC determine the supported topology. Logical identity matches MPIDR, not launch order. Each CPU has a permanent 256 KiB linker stack, vectors, affinity-matched GIC redistributor and local timer. Per-CPU atomics contain state, actual stack pointer, timer/IPI delivery and TLB acknowledgement generation. Probe recovery is per-CPU and test-only.

```text
CPU0: DTB -> affinity table -> UART/vectors -> tables/heap -> global GIC
      -> release READY -> clean linked RAM to PoC -> DSB SY -> PSCI CPU_ON
CPU1: secondary_entry -> masked EL1h -> private stack -> FP setup -> native root
      -> acquire READY -> check MPIDR/EL/MMU -> matching GICR -> ONLINE -> IRQ
```

CPU1 never clears BSS or initializes shared resources. Cleaning the linked RAM, including tables and boot publication, establishes visibility for its MMU-off entry; coherent atomics apply after native MMU activation. The 64-byte cache line is part of the Cortex-A57 platform pin. CPU_ON and initial-state contracts follow [PSCI DEN0022D](https://documentation-service.arm.com/static/5f905c71f86e16515cdc1fd0?token=), sections 5.6 and 6.4.

## Ownership and synchronization

| Resource                        | Owner and synchronization                                | Lifetime                                  |
| ------------------------------- | -------------------------------------------------------- | ----------------------------------------- |
| Physical pool and PTE mutations | CPU0 affinity checks; non-transferable Physical/Frame    | Linear ownership through retirement       |
| Table storage and stacks        | Permanent storage; CPU0 initializes tables               | Entire boot                               |
| Heap metadata                   | Existing TTAS lock; ordinary code on either CPU          | Reserved permanent RAM                    |
| UART and GIC distributor setup  | CPU0 only; secondary failure uses atomics                | Entire boot                               |
| Redistributor, timer and probes | Matching per-CPU owner                                   | CPU online interval                       |
| Boot data                       | Release/acquire after cache cleaning                     | Immutable after startup                   |
| Remote work mailbox             | Test-only CPU0 producer/CPU1 executor; one item          | Borrowed data retained through completion |
| Retirement                      | CPU0 publishes; CPU1 acknowledges at ordinary quiescence | One checked generation; never wraps       |

CPU0 ownership is the enforced milestone scope, not a permanent future allocation/routing rule. Additional writers require a reviewed admission and serialization contract. No global lock was added. Existing TTAS uses Acquire CAS/Release unlock; IRQ takes no locks and allocates nothing. Two-CPU QEMU shared-pair execution checks publication and exclusion, not exhaustive weak-memory behavior or fairness.

## Shootdown and retirement

```text
OWNED -> MAPPED -> RETIRING -> RECLAIMABLE -> FREE
CPU0: retain retiring charge -> clear PTE -> release table lock -> local TLBI
      -> publish generation -> SGI
CPU1 IRQ: IAR -> publish pending generation -> EOI/deactivate
CPU1 ordinary boundary: complete prior reader -> TLBI VMALLE1 -> DSB ISH -> ISB
                        -> release acknowledgement
CPU0: acquire acknowledgement -> completion barrier -> clear retiring charge
      -> consuming release may return frame to pool
```

IRQ receipt cannot acknowledge an interrupted reader. The ordinary CPU1 loop acknowledges only after work completes. No table lock is held while waiting. Broadcast TLBI is not a substitute for acknowledgement. New mappings are rejected during pending retirement. Retirement borrows Frame; forgetting the guard retains its charge and release still fails. Existing mapped aliases also prevent release. Timeout/failure halts without authorizing reuse.

The permanent privileged identity alias remains. Ownership governs access; audited remote pointers exist only in tests. This does not retire arbitrary privileged pointers, DMA, unbounded readers, third CPUs or userspace mappings. Extending the scope needs reader leases and admission control across participants.

## Interrupts, failure and shutdown

Named SGI 1 carries coordination. GICR discovery scans bounded frames and matches full compressed MPIDR affinity; unsupported VLPI stride, absent affinity and unsupported SGI target ranges fail. Actual IAR IDs drive timer/IPI service and EOImode=0 deactivation. Timer sources stop before EOI.

Shutdown stops mailbox admission, drains accepted work, masks CPU1 IRQs, stops its timer, completes local TLBI and publishes QUIESCENT. CPU1 calls CPU_OFF; CPU0 waits for quiescence and AFFINITY_INFO=OFF before SYSTEM_OFF. Permanent stacks/tables are retained. Secondary panic/fault publishes FAILED and stops the CPU; CPU0 detects it and halts without reclaiming or recovering damaged privileged state.

## Checks and limits

`cargo xtask test` runs 39 named tests in DEV and optimized PROD, boots both non-test images and requires eight failing host controls. Sixteen SMP tests cover execution/IDs/stacks, independent per-CPU state, both IPI directions, 32 repeated IPIs, actual shared-lock publication, remote mapping reads, delayed acknowledgement, blocked reuse, remote translation fault, safe reuse, both timers and orderly CPU_OFF. SMP controls add secondary panic, retiring-frame release, missing acknowledgement and omitted remote TLBI. The last fails after a real remote read, preventing a boolean-only shootdown test.

[Machine evidence](../../research/results/kernel-smp.json) retains source hashes and exact results; [unsafe register](unsafe.md) states trust boundaries. QEMU TCG is kernel integration evidence, not silicon certification. Busy polling, two fixed CPUs, one outstanding retirement and fatal progress timeouts are explicit limits.

[Russian translation](../../translations/ru/docs/kernel/smp.md)
