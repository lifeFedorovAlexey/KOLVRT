# ADR-0022 — One-process domains and scoped Event grants

Status: **Accepted for bounded Phase 3.4**. Date: 2026-10-04.

Document status: CURRENT
Evidence scope: generation-aware one-process domains, fixed-affinity notification services and retained terminal outcomes on two QEMU CPUs.
Current reference: [Domain contract](../kernel/domains.md)

## Context

Private address spaces and reference identity do not authorize effects. Issue #24 requires explicit grants and revoke/admission serialization; issue #25 requires domain lifetime and budgets, including teardown during accepted work. LAW-005 requires privileged enforcement necessity, not performance alone. Existing handles or early IPC fixtures cannot dictate the later architecture.

## Decision

Use one process per domain in this subset. Trusted bootstrap supplies explicit grants and limits; process runtime enforces identity, memory ownership, handle/request/queue charges and closing. Product policy, initial grant/limit selection and orchestration belong to the trusted EL0 policy/supervisor layer; current static launchers are bootstrap fixtures, not a policy interpreter. Domain metadata uses a fixed 32-cell atomic pool with nonwrapping generation and retained references.

An Event grant binds one immutable target and exact service ProcessId. SEND, TRANSFER and REVOKE are independent rights. Delegation attenuates rights and charges the receiver. Target aliases share a single revoke/admission gate. Admission before revoke retains work; later admission fails. Close is not revoke. No universal object hierarchy, magic root handle or generic invoke is introduced.

The concrete enforcement reader is a same-CPU EL0 notification service with one queue cell. Accepted work holds consumer request and service queue/request charges. The service may finish only the consumer's admitted target, regardless of its private grants. Process/service generation and request sequence bind each terminal receipt. Sender teardown stops new admissions; accepted work may outlive the sender's reclaimed private pages. Service failure publishes cancellation before effect and releases ownership once.

The experimental notification encoding is not architectural authority for general IPC. Before implementing #26/#27, rederive queue topology, waits, cancellation, outcomes and policy installation from current invariants; refactor or remove this pilot if it constrains that design.

## Alternatives

Use only private roots and integer labels; authorize through an EL0 policy interpreter on every operation; allocate domain lifetime with Arc; preserve an earlier queue ABI because its tests already pass; create a universal capability/object layer.

## Why rejected

Labels and roots do not enforce resource authority or quotas. Untrusted callers cannot enforce their own grants, mapping isolation or lifetime; those narrow checks require EL1, while choosing policy does not. A final Arc drop can take the ordinary heap lock inside masked scheduler ownership, violating its accepted lock contract; a fixed atomic pool avoids that path and handles exhaustion. Earlier implementation effort does not justify an ABI or topology. The named Event reader does not justify a universal abstraction.

## Consequences

Domain fault/exhaustion is contained to its caller. Exact identity and closing checks precede authority admission. Physical memory releases only after scheduler detachment; copied accepted requests retain no user pointers. Pool reuse waits for final retained release. Unknown rights, absent grants, spoofed scope, stale service binding and quota failure cannot silently fall back to another effect.

## Compatibility impact

Reference lookup/close/attenuated transfer remain bounded. Scoped issued Event grants add REVOKE=4; bare reference defaults remain SEND/TRANSFER. The new notification contract is EXPERIMENTAL with no ABI-FREEZE. General IPC, multi-process domains, migration and immediate revoke/drain are unsupported.

## Performance impact

Admission/terminal paths use bounded owned state and atomics, without heap allocation, deallocation or ordinary locks. Existing exact-source copy/handle benchmarks and kernel footprint accompany the matrix. No speedup or silicon throughput claim is made; later contention or topology changes require fresh measurements.

## Security impact

Kernel enforcement is necessary for trusted current identity, page ownership, linear reference retention, explicit grants and resource charges. The alternative EL0 layer selects policy and can request installed limits/grants through a future trusted installation contract; it cannot bypass native enforcement. AI/routing/package decisions do not enter this mechanism. Domain and request generations prevent stale rebinding; service-private authority cannot replace consumer scope.

## Testing

Actual EL0 tests exercise both CPUs, separate zero budgets, receiver handle exhaustion, unknown rights/missing grants, spoofed scope/foreign memory, attenuation, cross-CPU revoke/admission, fault containment, restart and request outcomes. Step execution reclaims a sender before completing its retained request and observes service-fault cancellation from a surviving caller. Five mutations disable actual identity, budget, closing, revoke or scope enforcement; DEV/PROD must witness failed machine records and nonzero host status. Host checks, architecture Clippy, native-only and routing regressions supplement QEMU execution; physical ARM64 evidence remains separate.

## Reversibility

No public/stable ABI is frozen. A later IPC/supervisor reader must revisit this implementation against accepted laws and decisions, including its fixed-affinity and single-process assumptions. Prior implementation, effort and compatibility with current tests are insufficient reasons to preserve a constraining design.

[Russian translation](../../translations/ru/docs/architecture-decisions/0022-domains-and-scoped-grants.md)
