# Security threat model

Status: cross-subsystem design requirements, 2026-10-02. Canonical security scope beyond the first native slice. The [first-slice model](../architecture/threat-model.md) retains its narrower acceptance contract. Current SMP evidence does not establish EL0, driver or compat isolation. [Research scope](../../research/other-systems/kasperskyos/overview.md).

## Objectives and trust

Protect kernel integrity, native-authorized resource/effect scope, identity/lifetime, bounded charges and truthful outcomes. A vulnerability should not automatically compromise unrelated domains; blast radius depends on grants, shared state, mediators and hardware. Safe Rust does not prove authorization, logic or temporal correctness. No zero-day immunity claim exists.

| Actor                        | Intended trust and limits                                                                   |
| ---------------------------- | ------------------------------------------------------------------------------------------- |
| Kernel core/architecture     | Trusted enforcement; invariant failure is fatal                                             |
| HAL                          | Trusted narrow privileged hardware boundary                                                 |
| Native service               | Potentially faulty; in TCB for properties it alone enforces                                 |
| Driver                       | Device-scoped grants; DMA/reset determines integrity TCB membership                         |
| Compat adapter               | Untrusted inputs; independently authorized caller effects only                              |
| Native/legacy application    | Untrusted requests, forged handles, races/exhaustion                                        |
| Device firmware/DMA hardware | Untrusted only with real restriction; otherwise explicit trust or unsupported               |
| Boot/toolchain               | Trusted provenance, compiler/runtime and machine assumptions; authenticity not demonstrated |
| DEV tooling                  | Authenticated scoped inspection/injection; deployment exposure controlled                   |

TCB is per property and includes relevant safe/generated code, services and hardware. [TCB analysis](../../research/other-systems/kasperskyos/tcb.md).

## Threats and validation gates

| Threat                              | Enforcement and future negative case                                                                                |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Memory corruption/unsafe defects    | Actual address spaces and audited EL1; service cannot write kernel memory                                           |
| Confused deputy/escalation          | Trusted caller attribution and effect scope; disk-service grant cannot expose raw disk or another root              |
| Malformed IPC/compat input          | Owned snapshot, bounds, enum/flags, alignment/conversions, handle generation/type and limits                        |
| Resource exhaustion/DoS             | Charge before publication, bounded queues, retained charges, supervisor cleanup reserve; state progress assumptions |
| Logic bugs/races/stale capabilities | Required live predicates serialized with admission/revocation; old generations fail                                 |
| Compromised driver/malicious DMA    | Scoped MMIO/IRQ/DMA, hardware restriction, reset and leases; unrestricted DMA is not contained                      |
| Replay/unknown effects              | One terminal arbiter, no automatic replay when absence of effects is unknown                                        |

## Privileged placement and failure review

Every new EL1 responsibility must demonstrate privileged necessity under the
[admission policy](../architecture/kernel-admission-policy.md), including an EL0 service
using existing narrow primitives. Authoritative state and coordination do not themselves
require EL1. Review the exact invariant, smallest mechanism, caller/effect scope,
resource charges and consequences of compromise before changing placement. Service
policy defaults to EL0; it can remain in the property-specific TCB without becoming privileged.

A compromised service must not publish arbitrary page tables, acquire broader grants
or reclaim memory retained by readers/devices. Negative obligations include forged scope,
stale identity, exhausted quotas, premature release, missing acknowledgement and unsafe
timeout recovery. Existing bounded MMU/SMP checks establish only their recorded scope;
general mapping-grant admission and hostile DMA remain unverified. Failure of trusted
privileged enforcement is fatal to its kernel domain. A faster path cannot replace that
failure boundary without an explicit ADR, updated threat model and preserved guarantees.

## Compatibility invariant

EffectiveAuthority_via_compat(C, O) is a subset of NativeAuthorizedAuthority(C, O) at the native contract's authorization point. Include effects induced through services, not just C's handles. A service can use independent grants only for caller-authorized effects. Missing authority yields DENIED/AUTHORITY_REQUIRED; additional consumer rights require separate native authorization before admission. No compatibility waiver. Nested/split operations, restart/rebind and PROD routing preserve this invariant. [Decision](../architecture-decisions/0013-security-boundaries.md).

## Revocation and recovery

Specify future-admission denial versus accepted-work semantics. Closing one handle is not automatically descendant revocation. Define child/delegated grants, expiry, live mappings, reassignment and DMA before choosing a revocation implementation.

Recovery: stop admission, invalidate future authority, finish/fail accepted work truthfully, quiesce callbacks/IRQ/DMA, establish reset when necessary, release leases, then create a fresh generation and separately authorize restored grants/routes. Timeout does not establish quiescence; quarantine or halt. Privileged invariant failure is not a restartable service fault. Unknown effects cannot be treated as absent.

## Profiles, diagnostics and exclusions

DEV supports bounded traces, grant inspection and explicit fault injection. PROD retains all authorization, handle checks, isolation, ownership, quotas, barriers and outcomes. Optional logging cannot implement enforcement. Inspector access requires authentication/redaction; omit payloads, raw addresses/usable handles and secrets. [Inspection model](inspection-model.md).

Physical attacks, hostile boot/host administration and microarchitectural side channels are outside current assurance. General-purpose availability, hostile firmware, actual IOMMU/reset containment and persistent recovery are unverified. A trusted-device assumption does not cover hostile hardware. Future tests require separately authorized scheduler/EL0/user-copy and a named device contract.

[Russian translation](../../translations/ru/docs/security/threat-model.md)
