<!-- markdownlint-disable MD041 -->
<!-- The stable knowledge anchor precedes the visible document heading. -->

<a name="bounded-native-ipc"></a>

# Bounded native EL0 IPC

Document status: CURRENT
Evidence scope: accepted bounded Phase 3.5 on main 9b4687a: 124 DEV/124 PROD checks and 46 focused mutations at that retained snapshot. The latest exact-source Cortex-A57 stand verifies 125 DEV/125 PROD checks and 56 IPC/supervision controls. Machine-readable READY applies only to accepted IPC Phase 3.5; supervision NOT_READY and human acceptance remain separate. Physical ARM is unverified.
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

## Accepted Phase 3.5 evidence

The measurement method, copy-count scope and raw observations are described in the [performance passport](ipc-performance.md). The retained final Phase 3.5 source snapshot was verified in 144 groups per profile, including hot/blocked receive, blocked requester and queue-full rejection.

Current real EL0 fixtures cover both cross-CPU directions and each same-CPU placement, capacity 1/4, initialized payload boundaries, mutation after submit/reply, duplicate IDs before collection, failed receive/collect copy-out, blocked expiry before/after commitment, service/requester death, cancellation via an independent IPC handshake, authorized denial, revoke/close retention, FIFO/full rejection and zero charges/frames after quiescence. Payload stress executes 24 requests per CPU/capacity configuration, including four passes over sizes 0/1/8/64/255/256.

Twenty-three focused mutation controls cover authority, queue bounds/FIFO, client-ID and internal request generation, service-token substitution, wait recheck, wake generation/publication, duplicate READY, wrong-process wake, blocked reclaim, storage scope, terminal arbitration, cancel/commit, deadlines, missing and duplicate charge release, endpoint teardown, copy transactions and service death. All 46 DEV/PROD runs were rejected by their exact expected event or named failing test; an arbitrary panic is insufficient. A CPU1 kernel failure retains its exact cause in a bounded atomic record for CPU0, which stops the continuous session without authorizing reclamation.

Retained source-bound artifacts are [kernel acceptance](../../research/results/ipc-phase35-acceptance.json), [physical package-removal](../../research/results/native-compat-removal-phase35.json), [six-configuration routing regression](../../research/results/routing-phase35-regression.json) and [unsafe inventory](../../research/results/kernel-phase35-unsafe-audit.json). The original receipts retain their source snapshots. The [main integration receipt](../../research/results/ipc-phase35-main-integration.json) records positive checks of its named source snapshot and the exact unchanged-enforcement review; package metadata and test-only IRQ finalization changed. The [acceptance review](../architecture/ipc-phase35-acceptance-review.md) records code/EN-RU semantic correspondence and the scoped readiness decision. Controls and readiness receipts supplement the historical records at their named snapshots. Issue #109 changes only the host execution/verification inventory: the current combined snapshot is verified by a Cortex-A57 stand receipt with 125 checks per DEV/PROD profile and 56 IPC/supervision controls. The [post-#126 functional review](../architecture/supervision-phase36-acceptance-review.md) is complete; performance scope remains open. Native IPC implementation and its prior bounded readiness acceptance remain unchanged; historical receipts do not verify the new runner. The stand timer_rearm timeout remains in history: IRQ wrappers now preserve compiler memory ordering, and read-only timer/GIC diagnostics preserve the unchanged strict failure bound. The timeout was not accepted as a revocation-control witness. The retained complete stand run passed, while the intermittent timeout cause remains unproven; bounded execution evidence does not establish general race-freedom. A later stripped routing boot failed without a panic identity after its initial EL0 check. Static failing process/IPC names and panic source location now survive stripping; no private data or addresses are logged. The [fresh post-#126 receipt](../../research/results/supervision-phase36-main126.json) verifies current-source functional regression separately from performance acceptance. The next timer diagnostic showed an enabled expired compare with ISTATUS/GIC pending clear, consistent with delayed QEMU timer-callback processing. Basic timer verification now idles with IRQ enabled instead of occupying the emulated CPU continuously; it still requires actual delivery and keeps the independent external watchdog. This is fixture scheduling, not a fabricated IRQ or a production IPC deadline change. The next gate is #27; the ABI remains EXPERIMENTAL and unfrozen.

Sources: [core endpoint state](../../crates/kernel-core/src/ipc.rs), [identity](../../crates/kernel-core/src/ipc/identity.rs), [wire](../../crates/kernel-core/src/ipc/wire.rs), [mailbox](../../crates/kernel-core/src/ipc/mailbox.rs), [native continuation](../../crates/kernel/src/ipc/native.rs), [storage](../../crates/kernel/src/ipc/storage.rs), [deferred work](../../crates/kernel/src/ipc/deferred.rs), [scheduler](../../crates/kernel/src/scheduler/mod.rs), [EL0 fixtures](../../crates/kernel/src/ipc_workload.rs), [runner](../../crates/xtask/src/main.rs).

[Russian translation](../../translations/ru/docs/kernel/ipc.md)

## Phase 3.6 checkpoint extension

The experimental [supervisor](supervision.md) adds a distinct lifecycle checkpoint over the same native.request/1 state. Exact IPC wait identities and source/mailbox ownership survive acquired two-CPU root detachment. The original continuous transport, arbiter and scoped Phase 3.5 acceptance remain separate; [ADR-0026](../architecture-decisions/0026-el0-supervision.md) is accepted for bounded functional scope after #126 with readiness separate. Current exact-source regression applicability is recorded separately from historical receipts.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.ipc",
  "kind": "subsystem-contract",
  "summary": "Accepted bounded native EL0 IPC mechanism and source-grounded Phase 3.5 readiness.",
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
        "adr.0025",
        "doc.kolvrt.architecture.native-abi",
        "doc.kolvrt.architecture.first-native-slice",
        "doc.kolvrt.architecture.production-scheduler",
        "doc.kolvrt.kernel.wait"
      ],
      "gaps": [
        "Physical ARM is unverified; QEMU does not establish silicon behavior."
      ],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "Complete bounded native.request/1 IPC mechanism, READY for Phase 3.5 and the #27 reader under fixed affinity, immutable mappings and whole-session retirement; not overall production/platform readiness.",
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
          "crates/kernel/src/arch/aarch64/entry.S",
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
          "crates/kernel/src/ipc_paths_benchmark.S",
          "crates/kernel/src/ipc_revoke_workload.S",
          "crates/kernel/src/test_support/irq_wait.rs",
          "crates/xtask/src/matrix.rs"
        ],
        "acceptance": [
          "research/results/ipc-phase35-acceptance.json",
          "research/results/native-compat-removal-phase35.json",
          "research/results/routing-phase35-regression.json",
          "research/measurements/ipc-phase35-baseline.json",
          "research/results/kernel-phase35-unsafe-audit.json",
          "research/results/ipc-phase35-main-integration.json",
          "research/results/routing-phase35-upgrades.json",
          "research/results/phase35-dependency-upgrades.json",
          "research/results/ipc-phase35-controls-current.json",
          "research/results/ipc-phase35-readiness.json"
        ],
        "issues": [26],
        "adrs": ["adr.0025"],
        "limitations": [
          "ABI remains experimental and unfrozen.",
          "Fixed affinity, immutable mappings and conservative whole-session retirement; no supervisor, service discovery, migration, zero-copy or universal wait/invoke.",
          "Physical ARM and production bootstrap trust are unverified.",
          "READY is scoped only to bounded Phase 3.5; no physical-platform, production bootstrap or stable-ABI acceptance."
        ],
        "next_gate": "Phase 3.6/#27 isolated EL0 supervisor; rederive its policy, admission and retirement from current accepted invariants. Phase 3.7/#28 follows separately.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "STALE",
            "reason": "Phase 3.7 removes source-copy mutation builds and application implementation copies. Tests use the actual production code; replacement fault/restart/shutdown and related acceptance scenarios remain incomplete. Prior receipts retain their historical scope; partial passes are not full current-source acceptance.",
            "receipt": "research/results/supervision-phase36-main126.json",
            "receipt_sha256": "52a48b8246f516b7d393c1cecb6a7a0a58bac5a9af31822cda15390747c2907f",
            "scope": "Bounded native.request/1 transport regression on two fixed-affinity QEMU CPUs; not acceptance of the new supervisor/checkpoint policy or physical ARM64."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical ARM64 receipt exists."
          }
        ],
        "readiness": "READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Record the implemented bounded IPC contract while Issue #26 acceptance remains incomplete.",
            "acceptance": []
          },
          {
            "from": "EXPERIMENTAL",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "Fresh 23 DEV/23 PROD exact-source enforcement mutations, source-matched 124 checks per profile and explicit code/EN-RU semantic review complete the requested bounded Phase 3.5 acceptance; no ABI freeze or physical ARM claim.",
            "acceptance": [
              "research/results/ipc-phase35-controls-current.json",
              "research/results/ipc-phase35-readiness.json"
            ]
          }
        ],
        "roadmap_gate": "Phase 3.5",
        "readiness_acceptance": ["research/results/ipc-phase35-readiness.json"]
      },
      "aliases": ["kolvrt.ipc"]
    },
    {
      "id": "kolvrt.ipc.endpoint",
      "anchor": "authority-and-identities",
      "kind": "contract-section",
      "summary": "Concrete endpoint, service binding and scoped SEND authority.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.queue",
      "anchor": "bounded-storage-and-accounting",
      "kind": "contract-section",
      "summary": "Bounded queue, result slots and domain charges.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.request",
      "anchor": "copied-transport",
      "kind": "contract-section",
      "summary": "Copied native.request/1 request framing and operations.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.payload",
      "anchor": "copied-transport",
      "kind": "contract-section",
      "summary": "Initialized bounded payload snapshots and response bytes.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.receive",
      "anchor": "copy-transactions-and-terminal-arbitration",
      "kind": "contract-section",
      "summary": "Transactional receive delivery and retained queue state.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.wait",
      "anchor": "blocking-cross-cpu-wake-and-idle",
      "kind": "contract-section",
      "summary": "Blocking readable and terminal conditions with register and recheck.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.wakeup",
      "anchor": "blocking-cross-cpu-wake-and-idle",
      "kind": "contract-section",
      "summary": "Exact owner wake identity, mailbox acknowledgement and idle progress.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.terminal",
      "anchor": "copy-transactions-and-terminal-arbitration",
      "kind": "contract-section",
      "summary": "Exactly one retained terminal result and commitment boundary.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.cancel",
      "anchor": "copy-transactions-and-terminal-arbitration",
      "kind": "contract-section",
      "summary": "Cancellation before and after service effect commitment.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.deadline",
      "anchor": "copy-transactions-and-terminal-arbitration",
      "kind": "contract-section",
      "summary": "Absolute monotonic request deadline and terminal arbitration.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.cross-cpu",
      "anchor": "blocking-cross-cpu-wake-and-idle",
      "kind": "contract-section",
      "summary": "Fixed-affinity cross-CPU signaling and retained wake drainage.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.teardown",
      "anchor": "teardown-and-reclamation",
      "kind": "contract-section",
      "summary": "Death, shutdown, retained authority and endpoint reclamation.",
      "depends_on": ["kolvrt.ipc.transport"]
    }
  ]
}
```
