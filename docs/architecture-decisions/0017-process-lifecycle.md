# ADR-0017 — Generation-safe native process lifecycle

Status: **Accepted for bounded Phase 3.1**. Date: 2026-10-03.

Document status: HISTORICAL MILESTONE
Evidence scope: accepted bounded Phase 3.1 process lifecycle for trusted bootstrap workloads on two CPUs; no public EL0 lifecycle API.
Current reference: [Process lifecycle contract](../kernel/processes.md)

## Context

The scheduler now owns per-CPU execution queues, but a process lifecycle also needs stable identity, retained address-space ownership, transactional creation, terminal results, and safe reclamation. Reusing a slot or releasing its frames before every scheduler reference is detached can turn stale references into access to a different process. Issue #20 tracks the broader native process and service lifecycle.

The current implementation is a trusted bootstrap mechanism for bounded workloads. It is not yet a general service manager or a user-facing process API.

## Decision

Accept the current Phase 3.1 protocol as a bounded kernel-internal milestone:

- Identify each process by a private slot and checked, nonzero `u64` generation. Never wrap or rewind a consumed generation, including after failed creation.
- Keep one private `Registry`, constructed by kernel startup and retained across workloads. A permanent singleton claim prevents another registry from resetting identity generations.
- Use four scheduler slots per CPU and fixed affinity for this milestone. Only trusted bootstrap code may create processes; explicit `Origin::El0` creation is denied before reservation.
- Create resources transactionally. Publish no runnable reference until the address space and initial context are fully prepared. Roll back every resource on failure while keeping the consumed identity stale.
- Treat `Prepared` and `Admitted` as separate states. `start` admits a prepared process; dispatch publishes only complete descriptors to the indexed CPU.
- Record terminal exit, contained fault, explicit workload deadline, or creation failure as a kernel-internal completion observation. This does not define a public blocking or wait API.
- Reclaim only after the exact generation is terminal, both CPU dispatches have acquired completion, scheduler roots are detached under the quiescent permit, and no execution owner remains. A timeout never authorizes reclamation.
- Retain the conservative two-CPU dispatch barrier. Do not admit another round or reclaim one process while a peer from the active round may still execute.
- Keep process identity separate from authority. This decision adds no IPC, handles, capabilities, security domains, ELF loader, migration, public create/wait interface, or general persistent-service policy.

Issue #20 remains open for public lifecycle authority, asynchronous lookup and waiting, and service admission policy. Any relaxation of the barrier needs a separately reviewed retained-reference and per-object quiescence protocol.

The existing obligations LAW-013, LAW-018, LAW-035, LAW-041 and LAW-043 continue to apply. No law or privilege exception is added. The process coordinator adds ownership bookkeeping around existing protected page-table publication, privileged context switching and quiescent retirement. It is not a new service-policy responsibility: an EL0 supervisor may choose future lifecycle policy, but cannot itself publish protected tables, activate TTBR or perform privileged TLBI. No existing authorized process-creation endpoint is available to EL0 in this bounded foundation. Keep these narrow mechanisms in EL1 and defer public policy until user-copy and authority contracts exist. Kernel memory/scheduler maintainers must revisit placement before admitting service callers; allocation strategy and service supervision belong outside this mechanism. Its TCB remains the frame owner, mapping writer, architectural switch and scheduler ownership protocol.

## Alternatives

- Reuse process slots without generations.
- Expose the internal registry as an EL0 create/wait service now.
- Reclaim each process as soon as it reports a terminal result, independently of peer CPUs.
- Add a general asynchronous service manager as part of this milestone.

## Why rejected

Slot reuse without generations cannot reject stale identities. Exposing creation before defining authority and fault-contained user-copy rules would make numeric identity or addresses look like authorization. Independent reclamation is unsafe until every CPU has released its scheduler roots and execution ownership. A general service manager would exceed the evidence and scope of the present implementation.

## Consequences

Bootstrap workloads gain stable process identities, rollback-safe creation, terminal observations, bounded capacity, and verified resource reclamation. The two-CPU barrier limits concurrency and reclamation latency. Registry and address-space ownership remain kernel-private. Issue #20 is not complete.

## Compatibility impact

This extends the native bootstrap path and does not change legacy compatibility routing or grant it process authority. Unsupported EL0 creation faults only the caller. Existing bootstrap fixtures use the same internal create/start/dispatch/reclaim path.

## Performance impact

Creation uses a bounded contiguous frame lease and performs no fallible heap growth. Fixed capacity and the synchronous two-CPU barrier limit throughput. No performance improvement is claimed without separate measurements.

## Security impact

Generations reject stale references; private ownership keeps address-space resources alive; rollback prevents partial publication; terminal isolation contains a task fault; quiescent detachment precedes frame release. Process IDs and numerical addresses confer no authority. This milestone does not establish user-copy safety, IPC, handles, capabilities, or security domains.

## Testing

The Phase 3.1 QEMU matrix exercises both DEV and PROD profiles on two CPUs, including creation and rollback boundaries, admission, exit and fault completion, capacity, stale generations, duplicate operations, sparse queues, physical-resource recovery, preemptible steps, and the bounded own-event wait/block path. The exact-source 67-check result is linked from [the lifecycle contract](../kernel/processes.md) and [wait contract](../kernel/wait.md); it is a bounded deterministic run, not long-term reliability, fuzzing, or hardware weak-memory evidence. The wait extension remains kernel-internal and does not add the public blocking/wait API excluded by this decision.

## Reversibility

This internal protocol can be revised before public callers depend on it. Once external APIs or persistent services rely on process identity and lifecycle semantics, changes will require an explicit compatibility plan.

[Russian translation](../../translations/ru/docs/architecture-decisions/0017-process-lifecycle.md)
