# Native domains and scoped grants

Document status: CURRENT
Evidence scope: bounded Phase 3.4, one process per domain, two fixed-affinity CPUs, Event-notification requests.
Current reference: [ADR-0023](../architecture-decisions/0023-domains-and-scoped-grants.md)

ABI contract: native.notification/1
Publication stage: EXPERIMENTAL
ABI-FREEZE: none

<a name="kolvrt-domains-scope"></a>

## Implemented feature scope

Phase 3.4 is implemented within the stated bounds. Read the contracts and exact-source receipt below; the notification queue and experimental ABI do not establish general IPC architecture.

<a name="kolvrt-domains-identity"></a>

## Identity, ownership and limits

[Domain primitives](../../crates/kernel-core/src/domain.rs) associate exactly one generation-aware ProcessId with a retained DomainId. The [atomic pool](../../crates/kernel-core/src/domain/pool.rs) holds 32 cells with nonwrapping generations and at most 1,024 references per cell. DomainId is internal metadata; no user field selects a principal. A retained lease excludes cell reuse. Exhaustion returns an explicit error. Multi-process domains, root sharing and migration are unsupported in this subset.

The [process owner](../../crates/kernel/src/process.rs) owns the domain and private address space. The namespace retains the same domain and exact process identity. Bootstrap supplies image, affinity, grants and explicit `Spec.limits`; runtime validates and applies these inputs. Current static boot/test launchers supply fixture configuration. Product policy and the future trusted EL0 supervisor's installation protocol are separate from kernel enforcement; no package/model/network policy, AI dependency, global policy DSL or ambient allow is introduced.

Memory pages include the owned page tables, code, data and guarded stacks, plus loaded ELF pages. The memory limit is checked before physical allocation. Handle charges cover each live namespace entry; transfer charges the receiver before publication. Request admission charges the consumer's outstanding-request budget and the service's queue/request budgets. Denial rolls back every temporary charge and reference. The current namespace has eight slots and each legacy notification service has one queue cell; these storage bounds do not establish permanent ABI maxima. Domain metadata and Event storage have explicit separate fixed kernel pool bounds.

Concrete [IPC transport](ipc.md) uses the same one-process domain identities and separate quota dimensions. `Limits.endpoints` accounts for endpoint instances owned by the service domain without overloading memory/handle/queue/request limits. Admission retains a requester request charge, service queue charge and service active request charge. Receive releases the queue charge; the sole terminal arbiter releases active work; the retained response stays under the requester charge until successful collection or death cleanup. The endpoint charge releases only after actual request/wait/reference quiescence. The legacy notification pilot retains its one-cell bound; IPC has its own bounded queues and result storage.

<a name="kolvrt-domains-grants"></a>

## Grants and revocation

[Reference primitives](../../crates/kernel-core/src/handles.rs) retain generation/type/lifetime checks. Unscoped references cannot admit a deferred service effect. Protected bootstrap creates an explicit Event grant bound to one immutable target and one exact service ProcessId. SEND=1 permits notification admission; TRANSFER=2 permits attenuated receiver-local delegation; REVOKE=4 permits revocation. Trusted Event creation defaults to SEND and Completion creation to NONE; REVOKE requires an explicit Event grant. Unknown bits and absent grants fail before effects. No EL0 grant-creation endpoint exists.

All delegated aliases share the target's [admission gate](../../crates/kernel-core/src/wait.rs). Admission and revocation linearize on one atomic word. A request or transfer admitted before revoke can retain its target; all later effect admissions fail. Close removes a reference without revoking other aliases. Strong immediate revoke/drain/reset is not implemented. A restarted service cannot acquire the old service generation's grants.

<a name="kolvrt-domains-notification"></a>

## Concrete notification boundary

[EL0 request handling](../../crates/kernel/src/security.rs) uses SVC 0x90 and an immutable 48-byte SafeCopy snapshot. Six LE64 words encode version=1, operation, local handle or request sequence, then three reserved zero words. Nonzero principal/scope fields and unknown versions/operations fail. Caller identity comes exclusively from the executing scheduler-owned process and its bound domain.

| Operation | Contract                                                                                                                                         |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| 4         | Admit one Event notification to the grant's same-CPU service; return status 0 and a nonwrapping sequence in x1                                   |
| 5         | Revoke through a caller-local REVOKE grant                                                                                                       |
| 6         | Reject unprivileged grant issuance                                                                                                               |
| 7         | Read current-domain page/handle/queue/request charges; parameter must be zero                                                                    |
| 8         | Inspect the current service's retained request; expose consumer closing state and retained request count, with no pointers or principal selector |
| 9         | Complete only that admitted Event effect; parameter must be zero and cannot substitute a service-private handle                                  |
| 10        | Observe a caller-bound exact request sequence: 0 pending, 1 completed, 2 cancelled before effect; consume a terminal receipt once                |

Errors reuse bounded handle statuses: invalid=1, stale=2, wrong type=3, foreign=4, inactive=5, capacity=6, generation exhausted=7, copy failure=8, rights denied=9, reference exhausted=10; scope/authority denied=11 and domain budget exhausted=12. This notification boundary reports failed input copies as invalid. No pointer, struct layout or compatibility errno is exported.

The service receives authority only for the already-admitted concrete effect. Its own grants cannot replace the consumer's target. A terminal receipt blocks a second request until consumed; copied identity and sequence prevent stale outcomes from binding to a later process. This notification boundary has no arbitrary payload, general endpoint, request cancellation API, automatic request-wait wakeup or deadline. Those mechanisms belong to the separate native IPC contract. Persistent service and supervisor policy remain later gates; this pilot does not freeze their architecture.

<a name="kolvrt-domains-teardown"></a>

## Teardown and evidence

Exit/fault/budget termination closes domain admission at the masked scheduler boundary. Namespace retirement drops handle charges. Accepted work retains its target and domain request charge until completion or cancellation. A dying service publishes cancelled-before-effect before releasing work. A sender's private memory can be reclaimed after scheduler detachment while its copied request remains retained; no user pointer is held. Final pool release uses atomics, without heap allocation, deallocation or ordinary locks inside scheduler ownership.

[Actual EL0 fixtures](../../crates/kernel/src/security/testing.rs) cover both CPUs, unknown rights, missing grants, spoofed scope, foreign memory, attenuation, receiver handle exhaustion, separate zero request/queue budgets, revoke racing admission across CPUs, fault containment, process/service generation reuse, and teardown with accepted work. Step fixtures reclaim the sender before waking its service, and verify that a surviving EL0 caller observes service-fault cancellation. Their trusted driver uses the existing coalescing wait/event primitive; it does not implement the future general IPC wait contract. Test-only factory/identity/race helpers inject inputs, while authority requests use the production copied boundary.

Five controls remove actual budget, identity, teardown, deferred revocation or reserved-scope enforcement. The DEV/PROD runner requires the corresponding failed machine test record and nonzero host status. Host tests and Clippy supplement kernel execution. Retained exact-source receipts define the tested source and named matrix; QEMU TCG does not prove physical ARM64 ordering or performance.

[Exact-source matrix](../../research/measurements/runs/1791130278634-phase3-4-revocation-integrated-f3be261c515b.json) records 96 DEV/PROD checks and 80 negative controls. [Unsafe inventory](../../research/results/kernel-phase34-unsafe-audit.json) retains the reviewed source locations.

[Native-only verification](../../research/results/native-compat-removal-phase34.json) compares 70 identical native/harness files and repeats the same matrix without compatibility packages. [Routing regression](../../research/results/routing-phase34-regression.json) retains six configurations and three rejected controls.

[Russian translation](../../translations/ru/docs/kernel/domains.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.domains",
  "kind": "subsystem-contract",
  "summary": "Current bounded Phase 3.4 domains and scoped grants.",
  "units": [
    {
      "id": "kolvrt.security.domains",
      "kind": "feature",
      "anchor": "kolvrt-domains-scope",
      "summary": "Bounded domains, budgets and retained notification outcomes.",
      "tags": ["domain", "security", "grant", "notification"],
      "depends_on": [
        "kolvrt.security.domains.identity",
        "kolvrt.security.domains.grants",
        "kolvrt.security.domains.notification",
        "kolvrt.security.domains.teardown",
        "kolvrt.process.identity",
        "kolvrt.process.reclamation",
        "kolvrt.handles.identity",
        "kolvrt.handles.lifetime",
        "kolvrt.memory.user-copy",
        "law.009",
        "law.013",
        "law.025",
        "kolvrt.handles.local"
      ],
      "gaps": [
        "General IPC and trusted supervisor policy-installation acceptance are not delivered by this bounded notification pilot."
      ],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "Generation-aware one-process domains, explicit limits and grants, same-CPU Event notification, retained terminal outcomes and revoke/admission serialization.",
        "sources": [
          "crates/kernel-core/src/domain.rs",
          "crates/kernel-core/src/domain/pool.rs",
          "crates/kernel/src/security.rs",
          "crates/kernel/src/process.rs"
        ],
        "acceptance": [
          "research/measurements/runs/1791130278634-phase3-4-revocation-integrated-f3be261c515b.json"
        ],
        "issues": [24, 25, 75],
        "adrs": ["adr.0023"],
        "limitations": [
          "One process/domain and fixed-affinity single-cell notification; no general IPC, supervisor installation policy, multi-process domains, immediate revoke/drain, migration or silicon verification."
        ],
        "next_gate": "Rederive general IPC and supervision contracts for #26/#27; early notification encoding, queue and grants cannot freeze later architecture.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "STALE",
            "reason": "The resident_pages observation helper now requires machine-events and boot-payload together. The retained Phase 3.4 receipt predates this process.rs source change; current-source domain verification requires a new exact-source receipt.",
            "scope": "Exactly the f3be261c515b source digests and DEV/PROD QEMU profiles recorded by this receipt, including 96 checks and 80 controls; broader IPC/supervisor policy and silicon excluded.",
            "receipt": "research/measurements/runs/1791130278634-phase3-4-revocation-integrated-f3be261c515b.json",
            "receipt_sha256": "f789eddc625e9c8e1fac74a78a011568dc847fd494ef08e8d774c5b4299a35a3"
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical ARM64 receipt exists."
          }
        ],
        "readiness": "NOT_READY",
        "roadmap_gate": "Phase 3.4",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "Adopt merged #75 bounded domain contract and exact-source acceptance; this navigation change introduces no kernel behavior.",
            "acceptance": [
              "research/measurements/runs/1791130278634-phase3-4-revocation-integrated-f3be261c515b.json"
            ]
          }
        ]
      }
    },
    {
      "id": "kolvrt.security.domains.identity",
      "kind": "contract-section",
      "anchor": "kolvrt-domains-identity",
      "summary": "Domain contract: Identity, ownership and limits"
    },
    {
      "id": "kolvrt.security.domains.grants",
      "kind": "contract-section",
      "anchor": "kolvrt-domains-grants",
      "summary": "Domain contract: Grants and revocation"
    },
    {
      "id": "kolvrt.security.domains.notification",
      "kind": "contract-section",
      "anchor": "kolvrt-domains-notification",
      "summary": "Domain contract: Concrete notification boundary"
    },
    {
      "id": "kolvrt.security.domains.teardown",
      "kind": "contract-section",
      "anchor": "kolvrt-domains-teardown",
      "summary": "Domain contract: Teardown and evidence"
    }
  ]
}
```
