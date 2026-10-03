# ADR-0019 — Linear process-local handle namespaces

Status: **Accepted for bounded Phase 3.3**. Date: 2026-10-03.

Document status: CURRENT
Evidence scope: process-local identity/type/lifetime; validation in progress, no authority or transfer.
Current reference: [Handle contract](../kernel/handles.md)

## Context

Phase 3.2 supplies bounded immutable request snapshots and retained current-process memory. Issue #23 mixes handles with later capability/transfer guarantees; the explicit Phase 3.3 scope implements identity and lifetime while keeping authority separate. Existing process ownership and quiescent scheduler permits provide an exclusion mechanism without a new global lock.

## Decision

Use one non-cloned eight-entry namespace per process slot, retaining 56-bit slot generations across ProcessId reuse. Move the linear value into indexed CPU state for execution and return it after acquired quiescence. Own concrete wait Event/Completion targets and return synchronous borrowed Retained references. Check caller, encoding, generation, live state and kind. Burn reservation identities on failed publication and quarantine exhausted slots. Decode copied versioned words and expose only lookup/close to EL0.

## Alternatives

Global numeric IDs, raw kernel pointers, reset-on-process-reuse counters, generic KernelObject/Arc storage, global locks, rights placeholders and asynchronous leases were considered.

## Why rejected

Pointers/global selectors leak or bypass namespace attribution. Resetting counters aliases stale values. Current exclusive fixed-affinity ownership and direct concrete storage already exclude reclamation races. Universal storage, locks, asynchronous leases and undefined rights add unsupported contracts. Real delegation/accepted asynchronous work must revisit retention separately.

## Consequences

Each owned target carries an immutable kernel-only TargetId (creating ProcessId, slot and reservation generation), distinct from the process-local wire Handle. Moving a namespace preserves TargetId; reusing its slot creates a different target. TargetId is never serialized to EL0 and grants no authority.

Lookups retain synchronous borrows; close/retire cannot invalidate a used borrow. Targets are bounded and released with their namespace owner. Process slot reuse preserves handle generations. Copy publication errors cannot leave live entries. Shared process execution, transfer and asynchronous retention remain unsupported.

## Compatibility impact

The provisional LE64 identity and explicit 32-byte request have no native dependency on fd/HANDLE numbers, legacy errno or internal struct layout. Compatibility can map its identities above future independently authorized native operations.

## Performance impact

Bounded checked lookup and inline targets avoid lookup allocation. Five DEV/PROD per-CPU QEMU scopes measure lookup/create/close/reuse. No unchecked index or profile-specific correctness bypass is accepted.

## Security impact

The protected resource is kernel-owned wait/completion storage. EL0 cannot safely mutate protected scheduler namespace state or enforce kernel frame/borrow exclusion through an existing handle primitive because none exists. The minimum EL1 addition resolves/retains a concrete native reference and closes that reference; creation policy, resource effects, authority and delegation stay separate. A service-only reference table would not enforce the kernel-owned target lifetime. Table quota is eight targets and storage ownership is linear. No new production unsafe is needed; linked EL0 fixture slices use existing INV-USER-IMAGE. This adds no global object policy or implicit authority.

## Testing

Require real two-CPU copied-request execution, foreign/stale/wrong-kind rejection, close/rollback/exhaustion/reuse, exit/fault cleanup, compile-time retained-borrow exclusion, five real enforcement controls, existing lifecycle/user-copy/routing regressions, physical native-only removal, host/Clippy/repository checks, exact-source receipts and green CI. Current progress is not a completed acceptance claim.

## Reversibility

Change encoding/capacity only with explicit provisional ABI review. Relaxing affinity, synchronous borrows or private mappings requires a new proof. Capability admission can use unchanged identity/type lookup but must independently specify rights, revocation and accepted-work retention. Handles are not authority.

[Russian translation](../../translations/ru/docs/architecture-decisions/0019-process-local-handles.md)
