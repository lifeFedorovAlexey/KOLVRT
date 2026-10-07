# Native clock observations

Document status: CURRENT
Evidence scope: existing read-only CLOCK register contract and its SDK snapshot; no exclusive CPU-time, physical-hardware or performance acceptance.
Current reference: [Clock passport pilot](../research/clock-passport.md); [routing decision](../architecture-decisions/0015-el0-versioned-routing.md)

<a name="kolvrt-clock-query"></a>

## Clock query

The existing `svc #0x53` reads the architectural counter and the executing process's accumulated observations. The owned scheduler context selects the process; the caller supplies no process selector. Calling CLOCK requires no device or IPC grant and grants no authority. The SDK `native::clock_snapshot()` names the existing outputs; `native::clock()` retains its `(ticks, frequency)` result. Neither API creates a new kernel operation or instrumentation path.

<a name="kolvrt-clock-query-api"></a>

## Register contract

| Register | Meaning                                                                                                           |
| -------- | ----------------------------------------------------------------------------------------------------------------- |
| x0       | Architectural counter timestamp sampled inside CLOCK.                                                             |
| x1       | Counter frequency in ticks per second.                                                                            |
| x2       | Cumulative execution-window ticks for the current process, accounted at synchronous exception and IRQ boundaries. |
| x3       | Cumulative ticks around completed `READ_WINDOW` native service bodies.                                            |

The SDK declares x4 clobbered but does not expose it. Its conditional `ipc-benchmark` meaning is not part of this observation contract. Counters use unsigned 64-bit values; consumers validate a nonzero, consistent frequency and use checked differences. These observations are internal and unfrozen, not a stable external ABI promise.

## Accounting boundaries

Trap-entry accounting updates x2 before CLOCK returns the snapshot. The next execution window starts after the native-call handler and ends at a later vector-entry timestamp. A difference between snapshots therefore covers completed execution windows between them, including portions of context restoration/saving. It can include QEMU host descheduling and is not exclusive EL0 CPU time. It does not include the current CLOCK service body as a measured service counter.

The x3 counter is updated only after a `READ_WINDOW` handler completes. A subsequent CLOCK sees that completed accounting. CLOCK itself does not increment x3. For a client that performs only CLOCK queries between snapshots, each x3 difference is zero; zero does not mean the CLOCK handler costs nothing.

For checked snapshots A and B, retain `B.ticks-A.ticks`, the x2 difference and the x3 difference separately. Their checked residual is unattributed elapsed time. Underflow rejects an observation rather than clamping it. The residual does not separate kernel work, interrupts, scheduler delay or host scheduling. No IRQ-service counter or PMU cycles are inferred. The same public fields exist in DEV and PROD; measurement classes remain separate.

## Ownership and limits

Snapshot collection reads current-process state without allocating, mapping, granting or retaining user resources. Existing scheduler generation and ownership checks remain authoritative. Exposing already returned registers through the SDK neither relaxes isolation nor adds a privileged test hook. Broader clock policy, timer access, multi-clock synchronization and hardware precision require their own evidence. The [pilot](../research/clock-passport.md) is an external ABI client, not a replacement production service.

[Russian translation](../../translations/ru/docs/kernel/clock.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.clock",
  "kind": "subsystem-contract",
  "summary": "Read-only current-process clock and partial execution-window observations.",
  "units": [
    {
      "id": "kolvrt.clock.query",
      "anchor": "kolvrt-clock-query",
      "kind": "subsystem-contract",
      "summary": "Existing native CLOCK query and snapshot SDK.",
      "depends_on": ["adr.0015", "law.036"]
    },
    {
      "id": "kolvrt.clock.query.api",
      "anchor": "kolvrt-clock-query-api",
      "kind": "api-contract",
      "summary": "CLOCK x0/x1/x2/x3 outputs, checked deltas and attribution limits.",
      "depends_on": ["kolvrt.clock.query"]
    }
  ]
}
```
