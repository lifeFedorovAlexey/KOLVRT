# Planned native capability revocation

Document status: DESIGN BASELINE
Evidence scope: issue #24 requirements over the accepted bounded handle foundation; no grants/revocation implementation or acceptance is claimed.
Current reference: [Authority boundaries](../architecture-decisions/0013-security-boundaries.md)

<a name="kolvrt-security-capability-revocation"></a>

## Planned feature contract

Trusted entry context must supply consumer identity. Native bootstrap grants and scoped rights checks admit effects only after authority validation. Attenuation cannot create rights. Revocation must serialize with admission: retained already-admitted work preserves lifetime; new work is denied. Close is not revoke. Immediate revocation needs a separately reviewed drain/reset contract.

Service-private grants cannot expand caller effects, including nested calls. Restart/rebind cannot refresh authority or erase obligations. Actual acceptance requires real EL0 allow/deny, unauthorized grants/transfer, attenuation and revoke/admission races, identical DEV/PROD enforcement and exact-source evidence. No universal policy interpreter or additional EL1 responsibility is admitted for convenience.

This canonical planned unit records [issue #24](https://github.com/lifeFedorovAlexey/KOLVRT/issues/24); it does not implement it. Current handles provide bounded SEND/TRANSFER and retained targets only. Read the relevant laws, accepted ADRs, handle identity/lifetime, process reclamation, user-copy and security requirements through the explicit prerequisite closure. Hardware and kernel verification remain UNKNOWN for this feature.

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
        "kolvrt.security.threats"
      ],
      "gaps": [
        "Issue #24 native revocation implementation and behavioral acceptance are missing."
      ],
      "feature": {
        "implementation": "PLANNED",
        "implementation_scope": "Issue #24 native grants, attenuation and revocation serialized with admission.",
        "sources": [],
        "acceptance": [],
        "issues": [24],
        "adrs": ["adr.0013"],
        "limitations": [
          "Not implemented; close is not revoke; retained admitted work needs its own lifetime."
        ],
        "next_gate": "Real EL0 acceptance from issue #24 with exact-source DEV/PROD negative controls.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "UNKNOWN",
            "reason": "Revocation implementation and acceptance are absent."
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
    }
  ]
}
```
