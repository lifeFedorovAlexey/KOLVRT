# ADR-0025 — Concrete bounded IPC and continuous fixed-affinity execution

Status: **Accepted for the bounded Phase 3.5 mechanism; milestone verification pending**. Date: 2026-10-05.

Document status: CURRENT
Evidence scope: architectural decision and implemented bounded mechanism; preliminary QEMU fixtures do not complete Phase 3.5 acceptance or establish physical ARM behavior.
Current reference: [IPC contract](../kernel/ipc.md)

## Context

Issue #26 requires real EL0 peers, immutable bounded requests, initialized retained responses, scoped authority, blocking receive/wait, cancellation/deadline/death arbitration and actual reclamation. The Phase 3.4 Event-notification pilot is evidence, not architectural authority. [Re-derivation](../architecture/ipc-phase35-review.md) compared synchronization A/B/C and execution E1/E2/E3 against current laws and ADRs before implementation. The user authorized continuation through implementation, tests, documentation and PR following that review.

Ordinary locks cannot be introduced inside exclusive scheduler storage or IRQ callbacks. Early return from dispatch when no READY task exists cannot supply a persistent blocking transport. Independent admission/retirement would change root, namespace and owner lifetime across CPUs and is not required for this bounded milestone.

## Decision

Choose synchronization A: prepare/copy and retain under exact owner-local scheduler scope; release it; use a private nonblocking endpoint permit; release the permit; reacquire the exact executing task for output copy; release it; commit or roll back the owned reservation under endpoint exclusion. Permit contention is deferred separately from semantic BUSY. No guard spans scheduler storage, user access, sleep, ERET or another object guard. IRQ handlers publish flags; post-handler/native continuations perform bounded drainage.

Choose execution E1: add `dispatch_ipc()` as a continuous fixed-affinity session while retaining conservative whole-session root/namespace retirement. With no READY task, an owner remains Running and waits on its permanent native root/stack, servicing bounded deadline/wake/death work. Even a locally terminal CPU remains available until peer terminal state and retained source/mailbox drainage agree. Existing synchronous dispatch and step retain their distinct contracts. This adds neither migration nor independent admission.

Use a concrete endpoint, separate from Event/Completion. Trusted bootstrap creates its exact service binding and nondelegable receiver reference and installs scoped SEND grants. No PID, endpoint number or packet caller field creates authority. SEND/TRANSFER/REVOKE attenuation and a shared revoke/admission gate remain narrow mechanisms; there is no universal object hierarchy or generic invoke.

Use initialized owned payloads, bounded FIFO and separately bounded request/result slots. Delivery and collection are explicit copy transactions. Request ID reuse waits for result consumption or permitted cleanup. One endpoint-local terminal arbiter decides completion, cancellation, expiry and service failure/shutdown. Before commitment it may guarantee absence of authorized effect; after commitment it reports unknown effects rather than rollback. Closing-domain observation participates before later service authorization, even if deferred death cleanup skipped a busy cell.

Wait state binds exact process generation, globally unique nonwrapping sequence and object/condition identity. Source records retain wake ownership until target acknowledgement. Only the target owner changes BLOCKED to READY. Mailbox pending/ack state is atomic; publish and source retirement occur under the same endpoint exclusion. A stale wake cannot create a runnable owner. No READY task alone is never quiescence.

## Privileged necessity and policy boundary

EL1 must enforce caller/root provenance, handle rights, binding/token generations, private copied storage, domain charges, the common arbiter and owner-local execution transitions: untrusted EL0 cannot enforce these against itself or another domain. Selecting services, grants, limits, application effects, restarts and orchestration remains EL0 policy. Static launchers are trusted bootstrap fixtures, not a supervisor or policy interpreter. Performance is not the reason to move policy into EL1.

## Alternatives

Reuse Event/Completion as transport, authorize by a global selector, hold one lock across unrelated ownership domains, discard requests/results on copy failure, split terminal arbitration, or retire each CPU independently.

## Why rejected

These choices respectively conflate notification and transport, turn identifiers into authority, violate scheduler/user-copy ownership, lose accepted state, permit contradictory terminal results, or add independent root and namespace retirement without an accepted requirement.

## Consequences

Atomic multiword queues would require a larger publication/reclamation proof; a flag cannot conceal an ordinary lock in scheduler scope. Endpoint-owner command mailboxes would add remote admission and accepted-result ownership protocols. Independent CPU sessions would require independent root/namespace retirement now. Coordinator step pumping would retain polling and would not establish autonomous blocked deadline/wake behavior. Their possible later benefits do not justify importing them without their invariants.

## Compatibility impact

The native.request/1 contract remains experimental and unfrozen. Its actual handle encoding and scoped receiver authority must be documented with code-bound receipts before public support.

## Performance impact

The initial QEMU data is a regression baseline only. The required controlled same-CPU/cross-CPU paths and physical hardware measurements remain future evidence.

## Security impact

PID, endpoint selector and packet fields do not grant authority. Copied request data and exact generation checks are required before every object transition; permit scope must never include user copy, scheduler storage or IRQ work.

## Testing

DEV and PROD QEMU fixtures, focused exact-event mutation controls, host endpoint tests and repository documentation checks provide bounded evidence. They do not prove unexecuted mutations or physical ARM behavior.

## Reversibility

The protocol remains experimental; retain version rejection and explicit lifecycle notices. Future supervisor/service work must rederive the architecture from current laws and accepted decisions.

The selected mechanism introduces private fixed endpoint storage and audited synchronization/idle boundaries. Core state remains safe Rust. Fixed affinity, immutable mappings and whole-session lifetime bound scope and resources. Closed endpoints reclaim only with zero request/result and wait ownership plus final reference quiescence; session retirement also checks pending copy work and mailboxes. There is no grace-period substitute.

## Protocol publication

`native.request/1` is EXPERIMENTAL with ABI-FREEZE none. It uses the actual LE64 8/56 namespace handle and typed receipts/service tokens, replacing the historical 32/32 host proposal for EL0 transport. Version 0 is explicitly unsupported at the new entry; historical host models remain scoped under their lifecycle notice. Neither source publication nor this architectural ADR declares PUBLIC/STABLE support or freezes an encoding.

## Verification and next gates

Real DEV/PROD fixtures and mutation controls are preliminary evidence for the selected mechanism. Full wait/wake and arbiter/death/authority adversarial coverage, performance controlled paths, exact-source receipts, unsafe review and EN/RU knowledge/status audit remain required before milestone acceptance. QEMU is a regression platform, not silicon evidence.

Issue #27 is the next supervisor scope; #28 follows with an isolated persistent service. They must rederive execution, admission and retirement from their accepted invariants. Existing E1 code, effort or tests cannot force their architecture. Mutable mappings, zero-copy, wait-any, device/network/storage policy and independent service supervision remain excluded here.

[Russian translation](../../translations/ru/docs/architecture-decisions/0025-bounded-native-ipc.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0025",
  "kind": "adr",
  "summary": "Bounded native endpoint IPC and retained continuous fixed-affinity execution.",
  "aliases": ["ADR-0025"]
}
```
