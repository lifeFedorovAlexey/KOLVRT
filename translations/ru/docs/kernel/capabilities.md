# Границы нативных полномочий и отзыв

Document status: DESIGN BASELINE
Evidence scope: общие требования issue #24 остаются планом; синхронный Event-срез grants/revocation отдельно реализован в ограниченных границах aa75629 и имеет сохранённые CI reports.
Current reference: [Границы полномочий](../architecture-decisions/0013-security-boundaries.md)

<a name="kolvrt-security-capability-revocation"></a>

## Планируемый контракт функции

Идентичность потребителя должна поступать из доверенного entry context. Нативные bootstrap grants и проверки ограниченных прав допускают effects только после проверки полномочий. Attenuation не создаёт прав. Revocation сериализуется с admission: уже принятая работа сохраняет удержание lifetime; новая отклоняется. Close не означает revoke. Немедленное аннулирование требует отдельного проверенного drain/reset contract.

Приватные grants службы не расширяют effects вызывающего, включая вложенные вызовы. Restart/rebind не обновляет authority и не стирает обязательства. Acceptance требует реального EL0 allow/deny, отказа unauthorized grants/transfer, attenuation и revoke/admission races, одинакового DEV/PROD enforcement и exact-source evidence. Универсальный policy interpreter и новые обязанности EL1 ради удобства не допускаются.

Канонический unit общего плана фиксирует [issue #24](https://github.com/lifeFedorovAlexey/KOLVRT/issues/24), не реализуя её. Текущие handles также обеспечивают отдельный Event-срез SEND/TRANSFER/REVOKE и retained targets. Общий issuer/service/domain контракт остаётся планом. Читайте подходящие законы, принятые ADR, identity/lifetime handles, process reclamation, user-copy и security requirements через явное prerequisite closure. Аппаратная и ядерная проверка этой функции остаётся UNKNOWN.

<a name="kolvrt-security-event-revocation"></a>

## Ограниченный синхронный Event-срез

[ADR-0022](../architecture-decisions/0022-native-event-grants-and-revocation.md) задаёт отдельные явные права SEND/TRANSFER/REVOKE и общий атомарный state Event. Signal и revoke упорядочиваются этим state; уже принятое pending уведомление сохраняется, новые SEND отклоняются через все aliases. Close снимает одну ссылку и не означает revoke. Revocation не освобождает target и не разрешает Completion, device или общую службу.

[Сохранённый CI receipt](../../../../research/results/event-revocation-ci.json) импортирует реальные результаты принятого #72. Это исходный снимок QEMU DEV/PROD, а не новый запуск в этой задаче или silicon acceptance. Отдельные issuer, restart/rebind, вложенные consumer/service полномочия и асинхронный drain остаются условиями полного #24.

[Английский оригинал](../../../../docs/kernel/capabilities.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.capabilities",
  "kind": "subsystem-contract",
  "summary": "Навигация по документу: Планируемое аннулирование нативных полномочий. Доказательства имеют указанные границы.",
  "units": [
    {
      "id": "kolvrt.security.capability-revocation",
      "anchor": "kolvrt-security-capability-revocation",
      "kind": "feature",
      "summary": "Каноническая секция: Планируемый контракт функции.",
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
      "summary": "Синхронные Event grants и отзыв допуска для всех aliases.",
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
