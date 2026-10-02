# ADR-0013 — Effective authority and privileged necessity

Status: **Accepted for boundary rules only**. Date: 2026-10-02.

## Context

The issue review identified ambiguity between service-private capabilities and consumer authority, and between global coordination and privileged necessity. The user approved both clarifications. [Research lessons](../../research/other-systems/kasperskyos/kolvrt-lessons.md) distinguish external facts, proposals and implemented evidence.

## Decision

For consumer C and operation O, EffectiveAuthority_via_compat(C,O) must be a subset of NativeAuthorizedAuthority(C,O), evaluated in the applicable native authorization/revocation context. Service-private disk/device capabilities may differ from consumer capabilities, but may produce only caller-authorized objects and effects. Compat cannot mint a missing consumer grant, borrow another consumer's grant or waive protection, quotas or delegation restrictions. Additional consumer authority requires a separate native grant flow before admission. No compatibility exception may expand effective authority.

Global state ownership, authority and coordination alone do not justify EL1. Require proof that EL0 using existing narrow privileged primitives cannot safely enforce the invariant; admit only the irreducible privileged mechanism. Performance alone is insufficient. The [admission policy](../architecture/kernel-admission-policy.md) records that argument.

Issue #7 extends the boundary review to capability-type admission under LAW-009 and
LAW-013. Require a named workload and a demonstrated resource/operation, scope,
delegation, revocation or lifetime distinction that existing bounded interfaces cannot
express safely. Record issuer/effects, threat analysis and negative tests. Protocol opcodes
and driver convenience alone are insufficient; an arbitrary-command universal capability
is also rejected. The [native model](../architecture/native-model.md) contains the record
requirements and worked decision to reuse the bounded echo endpoint rather than add an
equivalent type. This creates no capability type, kernel grant API or new law.

Issue #11 requires a separate common-object admission comparison with at least two
distinct concrete workloads, narrow interfaces, shared/conflicting authority, lifetime
and failure invariants, counterexamples and deferral. The
[native model](../architecture/native-model.md) compares the host echo endpoint with
private EL0 worker memory. Bounded arithmetic/accounting may be shared, but handle
close, accepted requests and page/TLBI retirement are not interchangeable. Retain narrow
primitives and defer a universal supertype/state machine; no hypothetical file/socket/
driver hierarchy is approved. This refines LAW-009/LAW-013 without adding a law.

## Alternatives

Literal equality of service and caller capability inventories; compatibility-specific grants or waivers; EL1 placement for globally authoritative services or lower IPC cost.

## Why rejected

Inventory equality prevents legitimate mediation without proving caller-effect confinement. Waivers create confused deputies. Authority and speed do not establish a requirement for privileged execution.

## Consequences

Authorization tracks consumer, operation, scope and revocation context through mediation. Service restart or rebind cannot reset obligations. Proposed runtime mechanisms need separate design and implementation evidence.

## Compatibility impact

Preserve semantics only within native-authorized effects. No exception registry entry authorizes extra rights. Fallback, support windows and archived-package policy remain unresolved in the issue review.

## Performance impact

No benchmark or runtime overhead is established by this documentation. Future proposals measure IPC, batching and shared memory before privileged placement.

## Security impact

Clarify LAW-009 without removing memory, quota or delegation guarantees. Prevent confused deputies and authority laundering. Driver EL0 placement alone does not establish DMA containment. [Threat model](../security/threat-model.md) separates trust per property.

## Testing

[Future scenario specifications](../../research/fixtures/security-boundaries.json) include denied grants, nested/split operations, private-disk mediation and restart/rebind. They are not executed tests; repository checks verify document consistency only. Existing SMP results establish their stated bounded foundation.

## Reversibility

Mechanisms remain revisable; any replacement preserves effective authority and demonstrates privileged necessity. No new policy engine, capability taxonomy, universal object model or frozen ABI is accepted.

[Russian translation](../../translations/ru/docs/architecture-decisions/0013-security-boundaries.md)
