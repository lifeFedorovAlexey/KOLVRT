# Production scheduling and request lifecycle: next implementation order

Document status: DESIGN BASELINE
Evidence scope: user-directed implementation sequence. The current bounded preemption step and own-process event wait/block/wakeup foundation pass DEV/PROD QEMU checks; this document does not implement a production service loop, general event sources, or IPC.
Current reference: [Current scheduler foundation](../kernel/scheduler.md)

## Scope and existing mechanism

The user selected this sequence on 2026-10-03 for [native foundation #37](https://github.com/lifeFedorovAlexey/KOLVRT/issues/37). It refines implementation order and does not relax accepted authority, isolation, ownership or retirement contracts. The current timer-driven scheduler already preempts EL0 at a typed quantum. The remaining architectural problem is synchronous round completion: the coordinator waits for all admitted programs to terminate. An indefinitely living process and monopolizing a CPU are different conditions. Persistent services may live indefinitely while receiving bounded execution slices.

Keep the current synchronous dispatcher as a bootstrap/test execution path after introducing the next production mechanism. Do not repair it by adding a hidden dispatch timeout, terminating every persistent service after a fixed lifetime, or treating timeout as quiescence. Evidence for the existing bootstrap path remains separately scoped.

## Ordered implementation gates

The first incremental code change added `Registry::step()`: each fixed CPU returns
after one timer quantum or terminal event. Surviving processes remain admitted;
their architectural context, slice accounting, native observations and diagnostic
report stream survive the next step. A retained round-robin cursor gives peers
execution across steps. Returning does not publish process completion or permit
reclamation. The synchronous `dispatch()` remains the bootstrap completion driver.

The executable `process_quantum_return_and_peer_progress` check runs a
noncooperative EL0 loop beside a finishing peer on each CPU. Steps return while
the loops remain alive, reject their reclamation, preserve register-backed loop
progress, and let peers finish before the loops exhaust explicitly requested CPU
slice budgets. This is a preemption foundation: the two-CPU rendezvous/barrier
remains, with no production event loop or wall-time deadline protocol. A bounded one-event-per-process wait/block/wakeup path has since been added to the step driver; it is described below and does not provide general event sources or public wait authority.

The bounded wait addition uses a coalescing event latch retained per process slot, with reuse reset only after scheduler quiescence. During `Registry::step()`, one reserved harness SVC registers and rechecks the event; if still absent, the scheduler saves context, marks the task BLOCKED, detaches it and returns. Kernel bootstrap signals by exact live `ProcessId`; stale generations and terminal processes are rejected. The passing QEMU check covers retained signals, all tasks blocked, no runnable work, one task waking while its peer remains blocked, and slot-generation reuse; a host test races publication and registration. The [wait contract](../kernel/wait.md) records exact limits. This does not create a public EL0 wait API, generic wait queues, interrupt/device event sources, cross-process signaling or a production event loop.

| Order | Mechanism                         | Acceptance boundary and coordination                                                                                                                                                                                                                                                      |
| ----- | --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1     | Bounded preemption step           | Implemented and exercised by the two-CPU noncooperative-loop QEMU check; preserve context, fixed affinity and explicit CPU-slice budgets. General deadline and persistent service policy remain open under #17/#20.                                                                       |
| 2     | Bounded own-event wait/block      | The own-process coalescing latch, generation-safe wakeup and empty-runnable-set step are implemented and tested. General retained event sources, idle/event-loop integration, and close/death/shutdown race coverage remain. Coordinate #20/#26.                                          |
| 3     | IPC transport                     | Bounded immutable messages, admission/backpressure, publication and event-driven wakeup. The bounded synchronous user-copy prerequisite is accepted in Phase 3.2; explicit bootstrap authorization, identity/domain checks and retained ownership still remain mandatory. Coordinate #26. |
| 4     | Handles and capabilities          | Caller-local generation-safe handles and per-operation native authorization before public send/receive/create. Process IDs, endpoint numbers and protocol opcodes confer no rights. Coordinate #23/#24/#25.                                                                               |
| 5     | Resource grants and revocation    | Temporary bounded access, retained resource ownership, attenuation and admission/revocation linearization. Closing/revoking access does not free resources still retained by accepted work. Coordinate #24/#25 and scoped device contracts.                                               |
| 6     | Complete request outcome protocol | One terminal arbiter for completed / cancelled / unknown effect, deadline, caller/service death and shutdown. Cancellation before commitment guarantees no effect; uncertainty after commitment prohibits blind replay. Coordinate #26/#27/#30.                                           |

This is an implementation sequence, not permission to expose an unauthorized IPC API at step 3 or to add outcome correctness only at step 6. Initial transport must already identify requests, retain accepted work and reject duplicate terminal publication. Define its restricted effects/cancellation limits before execution. Step 6 completes the outcome protocol across races, revocation, deadlines, failures and actual service effects; it is required before a persistent production service is admitted under #28.

## Production execution model

Separate process lifetime/identity, scheduler execution state and request outcomes. READY means eligible for execution; RUNNING has one indexed CPU owner; BLOCKED means registered on a specific retained event/condition. Terminal process states remain distinct. Request cancellation is not process exit, and expired scheduling budget does not prove that a request had no effect.

The timer trap saves a complete context, performs bounded accounting and publishes/reschedules preemption. IRQ paths allocate nothing, take no blocking locks and do not decide service policy. Event publication wakes an eligible exact generation; duplicate/stale wakeups must not create two runnable owners. State ownership, CPU migration and cross-CPU wakeup require an explicit protocol before relaxing fixed affinity. Do not introduce a universal object hierarchy or arbitrary-command capability.

A production scheduler returns to its event loop even when a peer remains alive or blocked. It must not borrow all process owners until every peer terminates. Reclamation needs retained lookup/admission references, exact terminal identity, scheduler detachment and acquired CPU/TLB quiescence for the particular object. A deadline stops or cancels admission/work according to its contract; memory is released only after actual retirement. Preserve the conservative existing barrier until the replacement protocol has its own evidence.

## Required evidence before each admission

- Preemption: noncooperative infinite EL0 loop beside a productive peer on each CPU, full context preservation, bounded peer progress and scheduler operation without terminating the infinite process merely to return from dispatch.
- Deadline: expiry during READY/RUNNING/BLOCKED and native-call/IRQ boundaries, exactly one observed expiry, no unauthorized effect or premature reclamation. Specify wall-time versus CPU-budget units explicitly.
- General blocking: the current bounded own-event foundation covers notification before registration, registration/recheck, post-block wake, coalescing, stale identity, and an empty runnable set under QEMU; host tests race signal against registration. General event-source registration, close/death/shutdown races, idle integration, and a production worker that does not busy-loop still require execution evidence.
- IPC/rights: full queue, malformed frame, failed copy, unauthorized/stale handle, foreign buffer, revoke-versus-admission, retained references after close and denial before effect.
- Outcomes: cancel before commitment, cancellation after uncertain effect, delayed completion after restart, timeout versus completion, service death and shutdown under load. Exactly one truthful terminal result; unknown effect is retained, never relabeled as rollback.

The bounded foundations above have passing executable evidence; the remaining general mechanisms are future executable-test requirements. Use real two-CPU QEMU DEV/PROD execution, meaningful negative controls, exact-source receipts and separate physical-hardware scope where applicable. Host protocol tests complement execution evidence. Fixed-affinity ASIDs (#18) and the bounded ELF loader (#21) are implemented in their separate scopes; benchmark/reliability tooling remains a separate concern and do not justify bypassing the safety gates above.

The original preemption increment has an [exact-source receipt](../../research/results/scheduler-quantum-step.json): 66 checks per profile. The follow-on [wait/block receipt](../../research/results/scheduler-wait-block.json) records 67 checks per DEV/PROD profile and 53 host rejection controls. The [Phase 3.2 user-copy receipt](../../research/results/kernel-phase3-2.json) records 69 checks per profile and 57 host negative controls. Remaining general event sources, production idle/event-loop integration, IPC and full admission gates require their own evidence.

[Russian translation](../../translations/ru/docs/architecture/production-scheduler.md)
