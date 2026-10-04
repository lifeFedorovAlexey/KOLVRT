# Планируемое аннулирование нативных полномочий

Document status: DESIGN BASELINE
Evidence scope: требования issue #24 поверх принятой ограниченной основы handles; реализация и acceptance grants/revocation не заявляются.
Current reference: [Границы полномочий](../architecture-decisions/0013-security-boundaries.md)

<a name="kolvrt-security-capability-revocation"></a>

## Планируемый контракт функции

Идентичность потребителя должна поступать из доверенного entry context. Нативные bootstrap grants и проверки ограниченных прав допускают effects только после проверки полномочий. Attenuation не создаёт прав. Revocation сериализуется с admission: уже принятая работа сохраняет удержание lifetime; новая отклоняется. Close не означает revoke. Немедленное аннулирование требует отдельного проверенного drain/reset contract.

Приватные grants службы не расширяют effects вызывающего, включая вложенные вызовы. Restart/rebind не обновляет authority и не стирает обязательства. Acceptance требует реального EL0 allow/deny, отказа unauthorized grants/transfer, attenuation и revoke/admission races, одинакового DEV/PROD enforcement и exact-source evidence. Универсальный policy interpreter и новые обязанности EL1 ради удобства не допускаются.

Канонический planned unit фиксирует [issue #24](https://github.com/lifeFedorovAlexey/KOLVRT/issues/24), не реализуя её. Текущие handles дают только ограниченные SEND/TRANSFER и retained targets. Читайте подходящие законы, принятые ADR, identity/lifetime handles, process reclamation, user-copy и security requirements через явное prerequisite closure. Аппаратная и ядерная проверка этой функции остаётся UNKNOWN.

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
