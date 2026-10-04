# ADR-0022 — Bounded native Event grants and revocation

Status: **Accepted for the minimal Event authority slice**. Date: 2026-10-04.

Supersedes only ADR-0020's rights-set and deferred-revocation clauses for this slice. The
identity, transfer, retention, close, pool, quota and fixed-affinity decisions in ADR-0020
remain in force.

## Context

Issue #24 requires explicit native grants, attenuation and revocation serialized with
admission. ADR-0020 deliberately fixed SEND/TRANSFER and deferred revocation to a separate
decision. The current process-local handle table and shared Event pool provide an existing
bounded authority/lifetime mechanism; a universal policy interpreter or object hierarchy
is not needed for one Event publication operation.

## Decision

- Keep authority on the caller-local handle. Decode the caller from the scheduler-owned
  task and bound namespace; never accept a caller identity or authority-bearing pointer
  from EL0, routing metadata, diagnostics or package policy.
- Define known rights as `SEND=1`, `TRANSFER=2`, and `REVOKE=4`. Unknown bits fail.
  Bootstrap-created Events default to SEND, Completion records default to no rights, and
  broader grants require an explicit trusted `create_*_with_rights` call. REVOKE is
  Event-specific; granting it to a Completion is rejected before slot generation is
  reserved or anything is published. There is no EL0 grant or object-creation operation
  in this slice.
- TRANSFER still requires the source TRANSFER right and a known subset of source rights.
  The transferred entry gets a fresh receiver-local token and keeps the same TargetId and
  Event state. REVOKE may be delegated only when the source grant includes it.
- A handle with REVOKE may permanently revoke future SEND admissions for its shared
  Event target. Signal admission and revocation race on the target's atomic state: if the
  signal CAS wins, its coalesced pending effect is committed; if revoke wins, the signal
  returns `Revoked` (wire status 11). The operation is synchronous and has no deferred
  post-admission effect to drain.
- Revocation does not free the Event, erase already-pending work or invalidate retained
  references. Close removes one caller-local entry and is not revoke. The final target
  reference releases the pool slot; a later target receives a new nonwrapping generation
  and fresh state. Revocation is permanent for one target generation; renewal creates a
  new target and a new explicit grant.
- Keep the mechanism resource-specific. It does not authorize Completion mutation, device
  access, arbitrary service calls, security-domain management, or cancellation/drain of
  asynchronous work. Future async operations require their own admission and terminal
  lifetime contract before sharing this mechanism.

## Invariant derivation

This preserves LAW-009 by validating the caller's actual SEND/REVOKE right at the native
operation boundary, and LAW-013 by keeping target identity, admission state and storage
lifetime distinct. LAW-025's single-arbiter rule is met by storing revocation in the same
shared Event state used for signal admission; there is no second policy ledger. The
bounded SVC path uses safe copied request bytes and existing per-CPU namespace ownership;
EL1 adds no global interpreter, allocation, lock or service policy.

## Alternatives

- Leave Event grants at implicit `ALL` and defer revocation. This fails the missing-grant
  and revoke/admission requirements in issue #24.
- Revoke only one handle entry. This leaves delegated aliases authorized and does not
  implement target-scoped authority revocation.
- Invalidate the target under already accepted work or free it at revoke. This confuses
  authority with lifetime and violates ADR-0020 retention semantics.
- Add a generic EL1 policy interpreter or universal capability/object framework. The
  single bounded Event operation does not justify those privileged mechanisms.

## Why rejected

Implicit ALL grants make missing authority indistinguishable from default construction.
Per-handle close is intentionally local and cannot stop admission through aliases.
Freeing a revoked target would invalidate retained borrowers or erase a committed pending
event. A universal interpreter would add arbitrary policy and failure paths beyond the
native operation that needs enforcement.

## Consequences

The provisional 48-byte request adds operation 4 (`revoke`) without changing frame size.
Status 11 is added; previous status values keep their meanings. Existing decoders reject
unknown operations. This is not a frozen ABI and does not complete issue #24: issuer
independence, restart/rebind policy for production services, nested consumer/service
authority, security domains and broader IPC grants remain separate work.

## Compatibility impact

The provisional 48-byte request adds operation 4 (`revoke`) without changing frame size.
Status 11 is added; previous status values keep their meanings. Existing decoders reject
unknown operations. The format remains provisional and unfrozen.

## Performance impact

Admission and revoke add one bounded compare/exchange on the shared Event's atomic byte.
No allocation, global lock or additional table scan is introduced. No throughput claim is
made before the QEMU/profile measurements land.

## Security impact

Only a caller holding SEND may publish and only a caller holding REVOKE may stop future
publication. Transfer can only attenuate. Revocation state is shared with the target, so
all aliases observe one authoritative decision. The Event remains allocated until its
normal final-reference release.

## Testing

Host checks cover attenuation, explicit rights, close-versus-revoke, alias lifetime and
the signal/revoke linearization. The real EL0 fixture must exercise revoke in both DEV
and PROD, followed by a denied SEND admission. CI and exact-source evidence are required
before claiming implementation acceptance.

## Reversibility

Replacing the right bits, wire operation, linearization point or accepted-work semantics
requires a reviewed successor decision and migration evidence. No compatibility exception
may expand authority.

[Russian translation](../../translations/ru/docs/architecture-decisions/0022-native-event-grants-and-revocation.md)
