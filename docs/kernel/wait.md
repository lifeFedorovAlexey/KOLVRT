# Wait and wake foundation

Document status: CURRENT
Evidence scope: bounded own-process event latch under trusted bootstrap coordination; not a production event loop or public IPC transport.
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
the registry owns the reserved slot. There are currently no external retained
publisher handles. This lifecycle restriction must be replaced with a retained
publisher/revocation contract before introducing concurrent native event sources.

## Evidence and limits

[QEMU source receipt](../../research/results/scheduler-wait-block.json) records
67 checks per DEV/PROD profile and 53 negative controls. The wait scenario covers
notification before registration, coalescing, two successive waits, wake after
blocking, an empty runnable set, reclaim rejection and stale generation after
slot reuse. Host tests race publication against registration and check retained
notifications. The current native publisher is serialized by the coordinator;
the host race is not evidence of a concurrent native publisher implementation.

The waiting EL0 program performs one SVC per wait and consumes no CPU slices
while BLOCKED. The two-CPU barrier and existing bootstrap polling remain. A
production idle/event loop, generalized wait sources, close/death/shutdown races,
deadline handling while blocked, IPC and capability-bearing handles remain gated
by the production scheduling plan.

[Russian translation](../../translations/ru/docs/kernel/wait.md)
