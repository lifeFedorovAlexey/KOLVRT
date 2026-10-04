# Native capability scope and revocation

Document status: DESIGN BASELINE
Evidence scope: broader issue #24 requirements remain planned; the synchronous Event grant/revocation slice is separately bounded implemented at aa75629, with retained CI reports.
Current reference: [Authority boundaries](../architecture-decisions/0013-security-boundaries.md)

<a name="kolvrt-security-capability-revocation"></a>

## Planned feature contract

Trusted entry context must supply consumer identity. Native bootstrap grants and scoped rights checks admit effects only after authority validation. Attenuation cannot create rights. Revocation must serialize with admission: retained already-admitted work preserves lifetime; new work is denied. Close is not revoke. Immediate revocation needs a separately reviewed drain/reset contract.

Service-private grants cannot expand caller effects, including nested calls. Restart/rebind cannot refresh authority or erase obligations. Actual acceptance requires real EL0 allow/deny, unauthorized grants/transfer, attenuation and revoke/admission races, identical DEV/PROD enforcement and exact-source evidence. No universal policy interpreter or additional EL1 responsibility is admitted for convenience.

This broader canonical planned unit records [issue #24](https://github.com/lifeFedorovAlexey/KOLVRT/issues/24); it does not implement it. Current handles also provide the separate bounded SEND/TRANSFER/REVOKE Event slice with retained targets. Broader issuer/service/domain authority remains planned. Read the relevant laws, accepted ADRs, handle identity/lifetime, process reclamation, user-copy and security requirements through the explicit prerequisite closure. Hardware and kernel verification remain UNKNOWN for this feature.

<a name="kolvrt-security-event-revocation"></a>

## Bounded synchronous Event slice

[ADR-0022](../architecture-decisions/0022-native-event-grants-and-revocation.md) defines explicit SEND/TRANSFER/REVOKE rights and one shared atomic Event state. Signal and revoke linearize on that state; admitted pending notification survives while later SEND fails through all aliases. Close drops one reference and is not revoke. Revocation does not free the target or authorize Completion, device or general service effects.

[Retained CI receipt](../../research/results/event-revocation-ci.json) imports the actual passing results of accepted #72. This is the recorded QEMU DEV/PROD source snapshot, not a new run by this documentation task or silicon acceptance. Independent issuers, restart/rebind, nested consumer/service authority and async drain remain full #24 gates.

[Russian translation](../../translations/ru/docs/kernel/capabilities.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.capabilities",
  "kind": "subsystem-contract",
  "summary": "Planned native capability admission and revocation; current bounded handles are not general grants.",
  "units": [
    {
      "id": "kolvrt.security.capability-revocation",
      "anchor": "kolvrt-security-capability-revocation",
      "kind": "feature",
      "summary": "Planned Phase 3.4 capability revocation and admission race review.",
      "tags": ["capability", "revocation", "phase", "3.4"],
      "read_when": ["review Phase 3.4 capability revocation"],
      "depends_on": [
        "kolvrt.handles.local",
        "kolvrt.handles.identity",
        "kolvrt.handles.lifetime",
        "kolvrt.process.reclamation",
        "kolvrt.memory.user-copy",
        "kolvrt.native.authority",
        "kolvrt.native.capability-admission",
        "kolvrt.security.revocation",
        "kolvrt.security.handle-retention",
        "law.009",
        "law.013",
        "law.018",
        "law.025",
        "law.040",
        "law.041",
        "law.042",
        "law.043",
        "adr.0013",
        "kolvrt.security.trust",
        "kolvrt.security.threats",
        "kolvrt.security.event-revocation"
      ],
      "gaps": [
        "Issue #24 broader issuer independence, service restart/rebind, nested authority, domains and IPC acceptance remain missing; the synchronous Event slice is implemented separately."
      ],
      "feature": {
        "implementation": "PLANNED",
        "implementation_scope": "Broader issue #24 service/issuer authority, restart/rebind and general capability admission beyond synchronous Event grants.",
        "sources": [],
        "acceptance": [],
        "issues": [24],
        "adrs": ["adr.0013"],
        "limitations": [
          "The ADR-0022 Event slice is separate bounded implementation; full issuer/service/domain authority and asynchronous drain are not delivered."
        ],
        "next_gate": "Real EL0 and exact-source acceptance for remaining issuer/service/domain obligations; re-derive architecture before extending Event mechanisms.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "UNKNOWN",
            "reason": "Broader issuer/service/domain acceptance is absent; scoped Event verification is a different feature."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical hardware acceptance exists."
          }
        ],
        "readiness": "NOT_READY",
        "roadmap_gate": "Phase 3.4",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "PLANNED",
            "reason": "Adopt issue #24 requirements without an implementation claim.",
            "acceptance": []
          }
        ]
      }
    },
    {
      "id": "kolvrt.security.event-revocation",
      "anchor": "kolvrt-security-event-revocation",
      "kind": "feature",
      "summary": "Synchronous Event grants and target-wide admission revocation.",
      "tags": ["event", "revocation", "grant"],
      "depends_on": [
        "kolvrt.handles.identity",
        "kolvrt.handles.lifetime",
        "kolvrt.memory.user-copy",
        "law.009",
        "law.013",
        "law.025"
      ],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "Trusted explicit Event SEND/TRANSFER/REVOKE grants, attenuation and atomic target-wide revocation serialized with synchronous signal admission.",
        "sources": [
          "crates/kernel-core/src/handles.rs",
          "crates/kernel-core/src/wait.rs",
          "crates/kernel/src/handles.rs",
          "crates/kernel/src/handles/testing.rs"
        ],
        "acceptance": ["research/results/event-revocation-ci.json"],
        "issues": [24, 72],
        "adrs": ["adr.0022"],
        "limitations": [
          "Event-only synchronous publication; no issuer independence, general service/domain grants, async cancellation/drain or silicon acceptance."
        ],
        "next_gate": "Review remaining issue #24 contracts independently; earlier Event implementation must not freeze broader authority/lifetime architecture.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "VERIFIED",
            "reason": "Original merged PR #72 CI profiles imported with source and ELF digests; not a rerun by this documentation change.",
            "receipt": "research/results/event-revocation-ci.json",
            "receipt_sha256": "b403eab6ca342921cff86c1d48c07395ebe5c2a18d4f4f1a6a331c17cbb8d8bf",
            "scope": "Only synchronous Event SEND/TRANSFER/REVOKE and retained positive DEV/PROD CI profiles at 4313feb82ea345e15d4a309889a3986b3558848a. Actual max/TCG/QEMU configuration is recorded; no issuer/service/domain, async drain, current-host-tool or silicon verification."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical hardware acceptance exists."
          }
        ],
        "readiness": "NOT_READY",
        "roadmap_gate": "Phase 3.4 bounded Event slice",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "Adopt already-merged PR #72 with its actual passing CI receipt; this documentation change implements no kernel behavior.",
            "acceptance": ["research/results/event-revocation-ci.json"]
          }
        ]
      }
    }
  ]
}
```
