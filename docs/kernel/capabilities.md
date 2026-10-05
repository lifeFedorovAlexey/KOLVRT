# Native capability scope and revocation

Document status: DESIGN BASELINE
Evidence scope: bounded scoped grants and domains are accepted Phase 3.4 behavior at 3124d65; the historical synchronous Event slice has a separate CI receipt, while general IPC remains planned.
Current reference: [Authority boundaries](../architecture-decisions/0013-security-boundaries.md)

<a name="kolvrt-security-capability-revocation"></a>

## Bounded current feature contract

Trusted entry context must supply consumer identity. Native bootstrap grants and scoped rights checks admit effects only after authority validation. Attenuation cannot create rights. Revocation must serialize with admission: retained already-admitted work preserves lifetime; new work is denied. Close is not revoke. Immediate revocation needs a separately reviewed drain/reset contract.

Service-private grants cannot expand caller effects, including nested calls. Restart/rebind cannot refresh authority or erase obligations. Actual acceptance requires real EL0 allow/deny, unauthorized grants/transfer, attenuation and revoke/admission races, identical DEV/PROD enforcement and exact-source evidence. No universal policy interpreter or additional EL1 responsibility is admitted for convenience.

[Domains and scoped grants](domains.md) describe accepted Phase 3.4 [issue #24](https://github.com/lifeFedorovAlexey/KOLVRT/issues/24) behavior within one process/domain and Event notification. General IPC, supervisor policy installation and immediate drain remain separate gates. Read current laws, ADRs, identity/lifetime, reclamation and user-copy through the explicit prerequisite closure; QEMU receipts do not establish silicon.

<a name="kolvrt-security-event-revocation"></a>

## Bounded synchronous Event slice

[ADR-0022](../architecture-decisions/0022-native-event-grants-and-revocation.md) defines explicit SEND/TRANSFER/REVOKE rights and one shared atomic Event state. Signal and revoke linearize on that state; admitted pending notification survives while later SEND fails through all aliases. Close drops one reference and is not revoke. Revocation does not free the target or authorize Completion, device or general service effects.

[Retained CI receipt](../../research/results/event-revocation-ci.json) imports the actual passing results of accepted #72. This is the recorded QEMU DEV/PROD source snapshot, not a new run by this documentation task or silicon acceptance. This earlier snapshot does not verify later scoped deferred admission. Current Phase 3.4 is described by the [domain contract](domains.md); general IPC, supervisor policy installation and immediate drain remain separate gates.

[Russian translation](../../translations/ru/docs/kernel/capabilities.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.capabilities",
  "kind": "subsystem-contract",
  "summary": "Bounded Phase 3.4 grants, domains and revocation; general IPC and supervisor policy remain separate gates.",
  "units": [
    {
      "id": "kolvrt.security.capability-revocation",
      "anchor": "kolvrt-security-capability-revocation",
      "kind": "feature",
      "summary": "Current bounded Phase 3.4 capability revocation with explicit remaining gates.",
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
        "kolvrt.security.event-revocation",
        "kolvrt.security.domains",
        "kolvrt.security.phase34"
      ],
      "gaps": [
        "General IPC, multi-process domains, trusted supervisor policy installation and immediate revoke/drain are separate future gates."
      ],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "Merged bounded Phase 3.4 explicit scoped Event grants, generation-aware domains and retained service notification with revoke/admission serialization.",
        "sources": [
          "crates/kernel/src/security.rs",
          "crates/kernel-core/src/handles.rs",
          "crates/kernel-core/src/wait.rs"
        ],
        "acceptance": [
          "research/measurements/runs/1791130278634-phase3-4-revocation-integrated-f3be261c515b.json"
        ],
        "issues": [24],
        "adrs": ["adr.0013", "adr.0022", "adr.0023"],
        "limitations": [
          "The full general IPC/supervisor production contract is not delivered; one-process fixed-affinity domains and single-cell Event notification remain bounded."
        ],
        "next_gate": "Review #26/#27 general IPC/supervisor invariants and policy installation; refactor the early notification pilot if it constrains them.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "STALE",
            "reason": "IPC integration changes shared implementation/build inputs; retained historical receipts keep their scope, while current per-feature exact-source applicability is not asserted by the old receipt.",
            "scope": "Exactly the f3be261c515b source digests and DEV/PROD QEMU profiles recorded by this receipt, including 96 checks and 80 controls; broader IPC/supervisor policy and silicon excluded.",
            "receipt": "research/measurements/runs/1791130278634-phase3-4-revocation-integrated-f3be261c515b.json",
            "receipt_sha256": "f789eddc625e9c8e1fac74a78a011568dc847fd494ef08e8d774c5b4299a35a3"
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
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "Adopt then-current merged #75 Phase 3.4 behavior and exact-source receipt; the earlier planning snapshot cannot dictate current reality.",
            "acceptance": [
              "research/measurements/runs/1791130278634-phase3-4-revocation-integrated-f3be261c515b.json"
            ]
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
            "state": "STALE",
            "reason": "IPC integration changes shared implementation/build inputs; retained historical receipts keep their scope, while current per-feature exact-source applicability is not asserted by the old receipt.",
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
