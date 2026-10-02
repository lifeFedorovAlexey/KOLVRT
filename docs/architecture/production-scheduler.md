# Production scheduling and request lifecycle: next implementation order

Document status: DESIGN BASELINE
Document scope: user-directed implementation sequence; production scheduler, blocking and IPC are not implemented by this document.
Status reference: [Current scheduler foundation](../kernel/scheduler.md)

## Scope and existing mechanism

The user selected this sequence on 2026-10-03 for [native foundation #37](https://github.com/lifeFedorovAlexey/KOLVRT/issues/37). It refines implementation order and does not relax accepted authority, isolation, ownership or retirement contracts. The current timer-driven scheduler already preempts EL0 at a typed quantum. The remaining architectural problem is synchronous round completion: the coordinator waits for all admitted programs to terminate. An indefinitely living process and monopolizing a CPU are different conditions. Persistent services may live indefinitely while receiving bounded execution slices.

Keep the current synchronous dispatcher as a bootstrap/test execution path after introducing the next production mechanism. Do not repair it by adding a hidden dispatch timeout, terminating every persistent service after a fixed lifetime, or treating timeout as quiescence. Evidence for the existing bootstrap path remains separately scoped.

## Ordered implementation gates

| Order | Mechanism                         | Acceptance boundary and coordination                                                                                                                                                                                                                       |
| ----- | --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1     | Preemption and deadline handling  | Timer preemption continues without cooperative yield; CPU fairness/progress and requested budget/deadline expiry do not depend on all peers exiting. Preserve architectural context and fixed owner checks. Extend scheduler/process work #17/#20.         |
| 2     | Wait/block primitive              | READY → RUNNING → BLOCKED; atomic wait registration plus condition recheck, generation-safe wakeup and no lost wakeup. No ready tasks means idle/wait for an event, not runtime shutdown. Coordinate #20/#26.                                              |
| 3     | IPC transport                     | Bounded immutable messages, admission/backpressure, publication and event-driven wakeup. Initially only explicitly authorized bootstrap peers; prerequisite user-copy #22, identity/domain checks and retained ownership remain mandatory. Coordinate #26. |
| 4     | Handles and capabilities          | Caller-local generation-safe handles and per-operation native authorization before public send/receive/create. Process IDs, endpoint numbers and protocol opcodes confer no rights. Coordinate #23/#24/#25.                                                |
| 5     | Resource grants and revocation    | Temporary bounded access, retained resource ownership, attenuation and admission/revocation linearization. Closing/revoking access does not free resources still retained by accepted work. Coordinate #24/#25 and scoped device contracts.                |
| 6     | Complete request outcome protocol | One terminal arbiter for completed / cancelled / unknown effect, deadline, caller/service death and shutdown. Cancellation before commitment guarantees no effect; uncertainty after commitment prohibits blind replay. Coordinate #26/#27/#30.            |

This is an implementation sequence, not permission to expose an unauthorized IPC API at step 3 or to add outcome correctness only at step 6. Initial transport must already identify requests, retain accepted work and reject duplicate terminal publication. Define its restricted effects/cancellation limits before execution. Step 6 completes the outcome protocol across races, revocation, deadlines, failures and actual service effects; it is required before a persistent production service is admitted under #28.

## Production execution model

Separate process lifetime/identity, scheduler execution state and request outcomes. READY means eligible for execution; RUNNING has one indexed CPU owner; BLOCKED means registered on a specific retained event/condition. Terminal process states remain distinct. Request cancellation is not process exit, and expired scheduling budget does not prove that a request had no effect.

The timer trap saves a complete context, performs bounded accounting and publishes/reschedules preemption. IRQ paths allocate nothing, take no blocking locks and do not decide service policy. Event publication wakes an eligible exact generation; duplicate/stale wakeups must not create two runnable owners. State ownership, CPU migration and cross-CPU wakeup require an explicit protocol before relaxing fixed affinity. Do not introduce a universal object hierarchy or arbitrary-command capability.

A production scheduler returns to its event loop even when a peer remains alive or blocked. It must not borrow all process owners until every peer terminates. Reclamation needs retained lookup/admission references, exact terminal identity, scheduler detachment and acquired CPU/TLB quiescence for the particular object. A deadline stops or cancels admission/work according to its contract; memory is released only after actual retirement. Preserve the conservative existing barrier until the replacement protocol has its own evidence.

## Required evidence before each admission

- Preemption: noncooperative infinite EL0 loop beside a productive peer on each CPU, full context preservation, bounded peer progress and scheduler operation without terminating the infinite process merely to return from dispatch.
- Deadline: expiry during READY/RUNNING/BLOCKED and native-call/IRQ boundaries, exactly one observed expiry, no unauthorized effect or premature reclamation. Specify wall-time versus CPU-budget units explicitly.
- Blocking: event before registration, during registration/recheck and after blocking; duplicate/stale wakeup; empty ready queue; close/death/shutdown races. No busy-loop worker or lost event.
- IPC/rights: full queue, malformed frame, failed copy, unauthorized/stale handle, foreign buffer, revoke-versus-admission, retained references after close and denial before effect.
- Outcomes: cancel before commitment, cancellation after uncertain effect, delayed completion after restart, timeout versus completion, service death and shutdown under load. Exactly one truthful terminal result; unknown effect is retained, never relabeled as rollback.

These are future executable-test requirements, not recorded passing results. Use real two-CPU QEMU DEV/PROD execution, meaningful negative controls, exact-source receipts and separate physical-hardware scope where applicable. Host protocol tests complement execution evidence. ASID optimization #18, ELF #21 and benchmark/reliability tooling are separate work and do not justify bypassing the safety gates above.

[Russian translation](../../translations/ru/docs/architecture/production-scheduler.md)
