# ADR-0026 — Isolated EL0 supervision and bounded lifecycle rendezvous

Status: **Accepted for bounded Phase 3.6 after #126; Codex semantic review at the maintainer’s request, readiness separate**. Date: 2026-10-06.

Document status: CURRENT

Evidence scope: accepted bounded Phase 3.6 architecture after #126; current-source QEMU verification is separate from the open performance question and production/hardware readiness.

Current reference: [Supervisor contract](../kernel/supervision.md)

## Context

Issue #27 requires a real isolated EL0 supervisor, readiness authentication, fresh restart, bounded failure policy and truthful shutdown. The reviewed base is main `4e713a2194742fa2d5c75fd58d2dd7608e7617be`. The required foundation is accepted Phase 3.5, not its historical planning snapshot. Read [ADR-0025](0025-bounded-native-ipc.md), [process lifecycle](../kernel/processes.md), [scheduler ownership](0016-scheduler-ownership.md), [user-copy](0018-safe-user-copy.md), [handles](0019-process-local-handles.md), [retention](0020-handle-transfer-and-retention.md), [ASIDs](0021-asid-lifecycle.md), [grants](0022-native-event-grants-and-revocation.md), [domains](0023-domains-and-scoped-grants.md) and [Kernel Laws](../architecture/kernel-laws.md), especially LAW-009, LAW-013, LAW-025 and LAW-043.

Existing whole-session IPC retirement prevents replacement until every peer exits. Earlier implementation does not authorize that constraint for supervision. Conversely, moving manifests, dependency resolution, readiness or restart decisions into the scheduler would violate the accepted policy boundary. The proposed extension retains existing invariants while explicitly changing the session admission boundary.

## Decision

Use an isolated EL0 supervisor and immutable finite bootstrap image/CPU/quota grants. Only its executing exact identity can invoke the lifecycle mechanism. Service selectors are explicit initial grant selectors; replacement tokens are nondelegable, fresh and bound to that supervisor and entry. They cannot select unrelated processes or widen images, placement, limits or instance credits. Seal extinguishes unused initial grants; separately retained service capabilities remain finite and accounted. Scope retirement disables invocation; bootstrap installation is single use within a boot.

Use explicit two-CPU lifecycle checkpoints. Owners return to native roots after finite execution boundaries; CPU0 acquires both completions and detached execution owners before touching process ownership or allocating/reclaiming frames. IPC wait identities and owned continuation state remain retained across the checkpoint. Pending copy transactions finish before namespace transfer. Endpoint/source/mailbox lifetime is separate from root detachment; blocked contexts never become reclaimable because a checkpoint returned.

Keep source/ack drainage outside scheduler storage. SGI-only entry cannot spend the checkpoint timer quantum before EL0 executes. Continuation of round-robin selection requires the cursor's exact live process generation. Termination is an exact owner-local terminal transition with ASID retirement before frame reuse. Quotas are charged transactionally before publication; failed creation rolls back its unpublished resources.

The EL0 manifest and supervisor choose dependency order, readiness probes, deadline, backoff, restart budget, discovery and shutdown. Readiness is an initialized completed native IPC response from the exact authorized receiver, never a caller-written READY bit. Shutdown stops admission and observes real commitment-aware terminal results before owner termination and final source/ack reclamation. The fixture demonstrates one worker and one healthy peer; generic process creation, migration, wait-any and persistent services remain outside scope.

## Alternatives

Preserve the existing whole-session barrier and pre-create spare services; move policy into EL1; implement independent live admission and per-process asynchronous retirement; or use bounded checkpoint membership changes with preserved IPC ownership.

## Why rejected

Pre-created spares do not establish fresh allocation/restart identity or extinguished bootstrap grants. EL1 policy violates privileged necessity. Independent live admission would require a larger asynchronous root/namespace publication and per-object retirement protocol. Checkpoints provide a reviewable bounded extension with acquired owner/root quiescence, actual fresh instances and explicit retained IPC ownership. Their pause overhead and lifecycle query polling are declared limits, not a claim of autonomous persistent supervision.

## Consequences

Checkpoint publication must have an independent finite coordination deadline even without workload expiry. Timeout is fatal and retains resources; it never manufactures completion or permits reclaim. Workload expiry remains BudgetExpired and is not reused as a join duration. Pending-copy drainage must also have a finite failure bound.

Lifecycle membership changes pause healthy peers at acquired checkpoints. Scope, authority credits and retained transport ownership remain explicit; failed quiescence requires quarantine or global stop, never deadline-based reclaim.

## Security impact

EL1 alone can enforce the executing process/root, private memory, exact receiver/SEND bindings, finite launch grants, quotas and owner-local ASID retirement against untrusted children. It does not choose when to restart or what READY means. Static images and manifest provide development assurance only. Production use requires #38 to bind authorized exact images, grants and current trust/freshness policy; CANON/Phoenix Root, reproducibility and anti-rollback are not implied.

## Compatibility impact

The lifecycle register entry is EXPERIMENTAL and unfrozen. Existing native.request/1 framing and terminal arbiter are preserved. Native dependency closure excludes routing/compatibility/advisor policy.

## Performance impact

The checkpoint pauses peers during membership change; it is not a hardware throughput or real-time claim.

## Testing

Run real isolated EL0 images on both fixed-affinity CPUs in DEV and PROD, full prior controls, focused supervisor authority/stale-token/wait-retention mutations, host Clippy/tests and repository checks. Retain exact source and artifact identities. Proposals, host fixtures and QEMU do not establish production or physical ARM64 acceptance. The [current post-#126 review](../architecture/supervision-phase36-acceptance-review.md) records completed Codex architecture/code and EN/RU semantic review. Performance acceptance and independent human sign-off are not claimed.

## Reversibility

The experimental entry can be replaced without freezing internal representations; retained historical measurements and authority/lifetime obligations survive replacement.

[Russian translation](../../translations/ru/docs/architecture-decisions/0026-el0-supervision.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,

  "id": "adr.0026",

  "kind": "adr",

  "summary": "Accepted bounded isolated EL0 supervision with finite completion publication after #126.",

  "aliases": ["ADR-0026"]
}
```
