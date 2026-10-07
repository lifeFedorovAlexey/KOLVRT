# Wait and wake foundation

Document status: CURRENT
Evidence scope: the `0x55` own-process event latch under trusted bootstrap coordination; the separate `native.request/1` IPC waiter has its own retained-source contract and evidence.
Current reference: [Production scheduling gates](../architecture/production-scheduler.md)

## Contract

The experimental `0x55` SVC waits on the executing process's own event in the
`Registry::step()` path. The protected runqueue supplies identity; userspace
provides no process ID, event address or buffer. The synchronous bootstrap
`dispatch()` does not admit this operation.

Registration publishes the waiter before rechecking the pending signal. If no
signal is consumed, RUNNING becomes BLOCKED and returns to the native coordinator.
The trap and coordinator recheck publication before retaining the blocked state.
BLOCKED tasks are excluded from selection, keep their context and memory, and
cannot be reclaimed. A step with no READY task returns without executing EL0 or
publishing process completion.

Only trusted CPU0 coordination can call `Registry::signal(id)`. It validates the
exact admitted generation before touching that slot's event. Prepared, completed
and stale identities are rejected. A consumed notification changes BLOCKED to
READY once. Pending notifications coalesce; a new notification after consumption
is retained for the next wait. Events carry no payload or message count.

The fixed slot's latch resets during creation, while execution is inactive and
the registry owns the reserved slot. The statement that there are no external
retained publisher handles applies only to this bootstrap event latch. The separate
IPC endpoint retains exact waiter identities and cross-CPU wake records until the
target acknowledges them; see the [IPC wait contract](ipc.md).

## Evidence and limits

[QEMU source receipt](../../research/results/scheduler-wait-block.json) records
67 checks per DEV/PROD profile and 53 negative controls. The wait scenario covers
notification before registration, coalescing, two successive waits, wake after
blocking, an empty runnable set, reclaim rejection and stale generation after
slot reuse. Host tests race publication against registration and check retained
notifications. The current native publisher is serialized by the coordinator;
the host race is not evidence of a concurrent native publisher implementation.

The waiting EL0 program performs one SVC per wait and consumes no CPU slices
while BLOCKED. The two-CPU barrier and existing bootstrap coordination remain
for this `step()` event path. The Phase 3.5 IPC path separately has an EL0
continuous dispatcher, readable and terminal waits, deadline processing while
blocked, service/requester teardown and generation-keyed cross-CPU wakes. Those
mechanics are specific to bounded IPC and do not turn the `0x55` latch into a
general wait API. Bounded Issue #26 acceptance is complete. The [current native integration](native-applications.md) records later source-bound regression separately from the historical latch receipt and mutation controls.

[Russian translation](../../translations/ru/docs/kernel/wait.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.wait",
  "kind": "subsystem-contract",
  "summary": "Trusted bootstrap 0x55 event latch scoped separately from the native IPC wait source."
}
```
