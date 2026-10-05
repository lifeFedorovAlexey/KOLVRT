# Bounded native EL0 IPC

<a name="bounded-native-ipc"></a>

Document status: CURRENT
Evidence scope: Phase 3.5 implementation in progress. Real DEV/PROD EL0 peers exercise bounded transport, waits, deadlines, death, copy transactions and reclamation; milestone acceptance remains incomplete.
Current reference: [Architecture re-derivation](../architecture/ipc-phase35-review.md)

ABI contract: native.request/1
Publication stage: EXPERIMENTAL
ABI-FREEZE: none

<a name="authority-and-identities"></a>

## Authority and identities

A concrete endpoint is separate from Event and Completion. Trusted bootstrap binds it to one exact service ProcessId and fixed owner CPU. A nondelegable receiver binding authorizes receive, commit, reply and shutdown. A SEND grant authorizes submission to that endpoint. The executing namespace and immutable task supply caller identity; no packet field selects a principal. Service-private handles cannot substitute for caller SEND or an accepted service request token.

Handles retain the actual LE64 encoding: low 8 bits select a namespace slot and high 56 bits identify its nonwrapping generation. An endpoint's private pool identity has its own generation. Request receipts and service tokens are globally nonwrapping request identities, interpreted against the exact requester or receiver binding. Their numeric values confer no authority independently of those checks.

Caller request IDs are unique within requester ProcessId plus endpoint while queued, delivered, committed or retained terminal work exists. A successful collect or permitted abandon/death cleanup releases that ID. Closing SEND does not revoke aliases or erase accepted work. Shared revoke/admission serialization rejects later SEND while preserving earlier accepted requests.

<a name="bounded-storage-and-accounting"></a>

## Bounded storage and accounting

The fixed pool has eight endpoints. Queue capacity is explicitly 1 or 4 in the acceptance fixtures; request/result storage independently has eight slots per endpoint. Payloads contain at most 256 bytes. One exact receiver may register one readable wait; requester terminal waits use a bounded registry of sixteen retained wait records. Each process has one active owner-local blocking reason.

Endpoint ownership charges its service domain. Admission transactionally acquires a requester request charge, service queue charge and service request charge. Failed admission publishes no queue entry and releases prepared charges. Successful receive releases the queue charge. The sole terminal arbiter releases service work charges once; requester/result ownership remains until successful collection or cleanup. Closing-domain observation prevents uncommitted dead-requester work from acquiring commitment before deferred death drainage reaches that cell. Committed dead-requester work remains retained until actual terminal resolution.

<a name="copied-transport"></a>

## Copied transport

SVC `0xa0` accepts input address/length in x0/x1 and output address/capacity in x2/x3. It returns status in x0 and receipt, outcome or output length in x1. Input is an owned initialized snapshot copied under the exact executing task/root contract. Endpoint work occurs after leaving scheduler storage. Output copy reacquires that same executing task scope; no endpoint permit spans user access, scheduler access, sleep or context switch.

The input header is 40 bytes, with explicit little-endian fields:

| Offset | Bytes    | Field                                                      |
| ------ | -------- | ---------------------------------------------------------- |
| 0      | 2        | Version 1                                                  |
| 2      | 2        | Operation                                                  |
| 4      | 4        | Exact total input length                                   |
| 8      | 8        | Caller-local endpoint handle or accepted requester receipt |
| 16     | 8        | Submit client ID or commit/reply service token             |
| 24     | 8        | Submit absolute deadline in boot-local counter ticks       |
| 32     | 4        | Payload length, 0–256                                      |
| 36     | 4        | Reserved zero                                              |
| 40     | variable | Copied payload                                             |

Operations are submit 1, receive 2, commit 3, reply 4, wait-terminal 5, collect 6, cancel 7, shutdown 8 and abandon 9. Only submit/reply carry payload. Only submit carries deadline. Only submit/commit/reply carry the ID/token field. Unknown versions/operations and nonzero unused fields fail explicitly; there is no downgrade to native.request/0.

Receive output uses the initialized 40-byte header: version, reserved zero, total length, service token, client ID, deadline, payload length, reserved zero, then payload. Collect output uses 24 bytes: version, outcome, total length, client ID, payload length, reserved zero, then response. No Rust structure layout or padding crosses EL0.

<a name="copy-transactions-and-terminal-arbitration"></a>

## Copy transactions and terminal arbitration

Receive reserves and snapshots the queue head. Failed copy-out preserves its position, payload and charges; a token cannot commit queued work. Successful copy commits delivery and dequeues once. Collect reserves an immutable terminal result. Failed copy-out keeps that result and its client ID; successful copy consumes it once. Reservations retain exact endpoint/request identity across phases and reject concurrent cancellation or slot reuse safely.

Exactly one terminal arbiter handles completion, cancellation, expiry, service death and shutdown. Before commitment, cancellation/death reports cancelled-before-effect and expiry reports expired-before-effect. After commitment, these paths report effect-unknown. Completed results carry the initialized service response. Later terminal attempts cannot overwrite the winner or release charges twice. Commitment is transport authorization, not durability or a rollback promise.

<a name="blocking-cross-cpu-wake-and-idle"></a>

## Blocking, cross-CPU wake and idle

Readable and terminal waits check, register, recheck and block only while their condition remains false. A ready condition may return immediately. Producers retain an exact wait identity until the target owner acknowledges its mailbox record. The mailbox stores one pending-or-acknowledged sequence atomically; publication and source acknowledgement retirement share endpoint exclusion, preventing delayed publication after source removal. Busy control storage retains work for deferred retry.

Only the target CPU changes its task from BLOCKED to READY, matching process generation and the saved globally unique wait sequence. SGI/physical timer handlers publish flags; bounded native continuations drain object state outside scheduler storage. SGI rescheduling does not fabricate timer slices. Endpoint permit contention is distinct from state BUSY and uses kernel retry without an EL0 polling loop.

`Registry::dispatch_ipc()` keeps a continuous admitted session. With no READY task, the owner returns to its permanent native root/stack and waits for IRQ, retaining contexts, roots and namespaces. The nearest request/session deadline and bounded retry timer drive deferred work while peers are blocked. Idle IRQ flags are discarded before resuming task accounting. No READY task is not a completion condition.

<a name="teardown-and-reclamation"></a>

## Teardown and reclamation

Death/closing stops new admission and resolves or retains accepted work according to commitment. Shutdown wakes readable and terminal waiters. Source wait records survive until exact acknowledgement; stale target work never creates a runnable owner. The endpoint is reclaimable only when closed, with no requests/results, no wait records and only its own reference remaining. Whole-session retirement additionally requires both CPUs terminal, no pending copy completion, drained mailboxes and native-root ownership. Reclamation never depends on an elapsed grace delay.

Independent admission, migration, mutable user mappings, supervisor/service policy, generic wait-any, generic invoke and zero-copy are excluded. Later #27/#28 must rederive their architecture rather than treating this implementation order as authority.

## Evidence and remaining acceptance

The reproducible QEMU data and copy-count scope are described in the [performance passport](ipc-performance.md); its raw per-profile observations are retained in the linked JSON artifact. Dedicated hot/blocked/queue-full paths have been measured and verified in 144 groups per profile; the exact-source baseline must be refreshed after final source edits.

Current real EL0 fixtures cover both cross-CPU directions and each same-CPU placement, capacity 1/4, initialized payload boundaries, mutation after submit/reply, duplicate IDs before collection, failed receive/collect copy-out, blocked expiry before/after commitment, service/requester death, cancellation via an independent IPC handshake, authorized denial, revoke/close retention, FIFO/full rejection and zero charges/frames after quiescence. Payload stress executes 24 requests per CPU/capacity configuration, including four passes over sizes 0/1/8/64/255/256.

Twenty focused mutation controls cover authority, queue bounds/FIFO, request identity reuse, service-token substitution, wait recheck, wake generation/publication, duplicate READY, wrong-process wake, blocked-task reclamation, terminal arbitration, cancellation/commitment, deadlines, charge release, endpoint teardown, copy transactions and service death. The runner requires the exact rejection event or named failing real kernel test in a fresh parsed sidecar. The complete final matrix and exact-source receipt are being verified; human semantic/EN/RU review remains required. Physical ARM remains an unverified environment rather than an extra prerequisite for the specified QEMU acceptance.

Sources: [core endpoint state](../../crates/kernel-core/src/ipc.rs), [identity](../../crates/kernel-core/src/ipc/identity.rs), [wire](../../crates/kernel-core/src/ipc/wire.rs), [mailbox](../../crates/kernel-core/src/ipc/mailbox.rs), [native continuation](../../crates/kernel/src/ipc/native.rs), [storage](../../crates/kernel/src/ipc/storage.rs), [deferred work](../../crates/kernel/src/ipc/deferred.rs), [scheduler](../../crates/kernel/src/scheduler/mod.rs), [EL0 fixtures](../../crates/kernel/src/ipc_workload.rs), [runner](../../crates/xtask/src/main.rs).

[Russian translation](../../translations/ru/docs/kernel/ipc.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.ipc",
  "kind": "subsystem-contract",
  "summary": "Experimental bounded native EL0 IPC contract and current Phase 3.5 evidence.",
  "units": [
    {
      "id": "kolvrt.ipc.transport",
      "anchor": "bounded-native-ipc",
      "kind": "feature",
      "summary": "Bounded endpoint IPC with copied requests, blocking waits and exact terminal outcomes.",
      "tags": ["ipc", "endpoint", "native", "el0"],
      "depends_on": [
        "kolvrt.process.identity",
        "kolvrt.handles.local",
        "kolvrt.handles.lifetime",
        "kolvrt.security.domains",
        "kolvrt.memory.user-copy",
        "law.009",
        "law.013",
        "law.025",
        "adr.0016",
        "adr.0017",
        "adr.0018",
        "adr.0019",
        "adr.0020",
        "adr.0021",
        "adr.0022",
        "adr.0023",
        "adr.0025"
      ],
      "gaps": [
        "Final exact-source acceptance, remaining death/shutdown coverage and EN/RU review are pending.",
        "QEMU does not establish physical ARM behavior."
      ],
      "feature": {
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "A bounded native.request/1 endpoint mechanism is implemented with real EL0 fixtures; Issue #26 acceptance and publication stability are still under review.",
        "sources": [
          "crates/kernel-core/Cargo.toml",
          "crates/kernel-core/src/domain.rs",
          "crates/kernel-core/src/handles.rs",
          "crates/kernel-core/src/lib.rs",
          "crates/kernel-core/src/process.rs",
          "crates/kernel-core/src/ipc.rs",
          "crates/kernel-core/src/ipc/identity.rs",
          "crates/kernel-core/src/ipc/wire.rs",
          "crates/kernel-core/src/ipc/mailbox.rs",
          "crates/kernel-core/src/ipc/tests.rs",
          "crates/kernel-core/tests/handle_contract.rs",
          "crates/kernel/Cargo.toml",
          "crates/kernel/src/arch/aarch64/mod.rs",
          "crates/kernel/src/boot_workload.rs",
          "crates/kernel/src/handles.rs",
          "crates/kernel/src/handles/testing.rs",
          "crates/kernel/src/main.rs",
          "crates/kernel/src/process.rs",
          "crates/kernel/src/process_workload.rs",
          "crates/kernel/src/scheduler/local.rs",
          "crates/kernel/src/scheduler/mod.rs",
          "crates/kernel/src/scheduler/task.rs",
          "crates/kernel/src/security/testing.rs",
          "crates/kernel/src/smp.rs",
          "crates/kernel/src/tests.rs",
          "crates/kernel/src/user_copy/testing.rs",
          "crates/kernel/src/ipc.rs",
          "crates/kernel/src/ipc/native.rs",
          "crates/kernel/src/ipc/storage.rs",
          "crates/kernel/src/ipc/deferred.rs",
          "crates/kernel/src/ipc_workload.rs",
          "crates/kernel/src/ipc_workload.S",
          "crates/kernel/src/ipc_death_workload.S",
          "crates/kernel/src/ipc_authority_workload.S",
          "crates/kernel/src/ipc_payload_workload.S",
          "crates/kernel/src/ipc_queue_workload.S",
          "crates/kernel/src/ipc_benchmark.rs",
          "crates/kernel/src/ipc_benchmark.S",
          "crates/xtask/src/main.rs",
          "crates/xtask/src/output.rs",
          "crates/kernel/src/ipc_multi_workload.S",
          "crates/kernel/src/ipc_paths_benchmark.S"
        ],
        "acceptance": [],
        "issues": [26],
        "adrs": ["adr.0025"],
        "limitations": [
          "The ABI remains experimental and unfrozen.",
          "QEMU is regression evidence only; physical ARM is unverified.",
          "Final exact-source receipt, full acceptance and human semantic/EN/RU review remain pending."
        ],
        "next_gate": "Complete every Issue #26 functional, negative-control, DEV/PROD, source-receipt, performance, EN/RU knowledge and native-only/routing acceptance gate before changing implementation status.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "UNKNOWN",
            "reason": "Preliminary DEV/PROD fixtures and focused mutation controls exist, but the full requested matrix and exact final-source milestone receipt are not yet complete."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical ARM64 receipt exists."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Record the implemented bounded IPC contract while Issue #26 acceptance remains incomplete.",
            "acceptance": []
          }
        ],
        "roadmap_gate": "Phase 3.5"
      },
      "aliases": ["kolvrt.ipc"]
    },
    {
      "id": "kolvrt.ipc.endpoint",
      "anchor": "authority-and-identities",
      "kind": "contract-section",
      "summary": "Concrete endpoint, service binding and scoped SEND authority."
    },
    {
      "id": "kolvrt.ipc.queue",
      "anchor": "bounded-storage-and-accounting",
      "kind": "contract-section",
      "summary": "Bounded queue, result slots and domain charges."
    },
    {
      "id": "kolvrt.ipc.request",
      "anchor": "copied-transport",
      "kind": "contract-section",
      "summary": "Copied native.request/1 request framing and operations."
    },
    {
      "id": "kolvrt.ipc.payload",
      "anchor": "copied-transport",
      "kind": "contract-section",
      "summary": "Initialized bounded payload snapshots and response bytes."
    },
    {
      "id": "kolvrt.ipc.receive",
      "anchor": "copy-transactions-and-terminal-arbitration",
      "kind": "contract-section",
      "summary": "Transactional receive delivery and retained queue state."
    },
    {
      "id": "kolvrt.ipc.wait",
      "anchor": "blocking-cross-cpu-wake-and-idle",
      "kind": "contract-section",
      "summary": "Blocking readable and terminal conditions with register and recheck."
    },
    {
      "id": "kolvrt.ipc.wakeup",
      "anchor": "blocking-cross-cpu-wake-and-idle",
      "kind": "contract-section",
      "summary": "Exact owner wake identity, mailbox acknowledgement and idle progress."
    },
    {
      "id": "kolvrt.ipc.terminal",
      "anchor": "copy-transactions-and-terminal-arbitration",
      "kind": "contract-section",
      "summary": "Exactly one retained terminal result and commitment boundary."
    },
    {
      "id": "kolvrt.ipc.cancel",
      "anchor": "copy-transactions-and-terminal-arbitration",
      "kind": "contract-section",
      "summary": "Cancellation before and after service effect commitment."
    },
    {
      "id": "kolvrt.ipc.deadline",
      "anchor": "copy-transactions-and-terminal-arbitration",
      "kind": "contract-section",
      "summary": "Absolute monotonic request deadline and terminal arbitration."
    },
    {
      "id": "kolvrt.ipc.cross-cpu",
      "anchor": "blocking-cross-cpu-wake-and-idle",
      "kind": "contract-section",
      "summary": "Fixed-affinity cross-CPU signaling and retained wake drainage."
    },
    {
      "id": "kolvrt.ipc.teardown",
      "anchor": "teardown-and-reclamation",
      "kind": "contract-section",
      "summary": "Death, shutdown, retained authority and endpoint reclamation."
    }
  ]
}
```
