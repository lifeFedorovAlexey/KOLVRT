# Current CPU boundary

Two CPUs are configured in QEMU; only CPU0 executes the kernel. CPU1 stays powered off through the firmware PSCI contract. No CPU_ON call, secondary boot success, multicore test or SMP implementation is claimed.

## Next milestone

The user explicitly permitted completing the preceding working level when SMP cannot yet be implemented correctly. [ADR-0010](../architecture-decisions/0010-kernel-foundation.md) accepts that boundary. Before enabling another CPU, review per-CPU stacks and vectors, affinity-based redistributor discovery, release/acquire boot publication, interrupt routing, cache coherence, remote TLB acknowledgements and frame retirement. A broadcast TLBI instruction alone does not prove safe reclamation while another CPU uses a mapping.

Host concurrent lock tests validate the lock's publication mechanism; they do not validate AArch64 multicore boot. Scheduler, userspace and compatibility work requires a separate user instruction.
