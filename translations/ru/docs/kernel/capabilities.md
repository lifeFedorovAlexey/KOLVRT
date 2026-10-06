# Границы нативных полномочий и отзыв

Document status: DESIGN BASELINE
Evidence scope: ограниченные scoped grants и домены Phase 3.4 приняты в 3124d65; исторический синхронный Event-срез имеет отдельный CI receipt, общий IPC остаётся планом.
Current reference: [Границы полномочий](../architecture-decisions/0013-security-boundaries.md)

<a name="kolvrt-security-capability-revocation"></a>

## Ограниченный текущий контракт функции

Идентичность потребителя должна поступать из доверенного entry context. Нативные bootstrap grants и проверки ограниченных прав допускают effects только после проверки полномочий. Attenuation не создаёт прав. Revocation сериализуется с admission: уже принятая работа сохраняет удержание lifetime; новая отклоняется. Close не означает revoke. Немедленное аннулирование требует отдельного проверенного drain/reset contract.

Приватные grants службы не расширяют effects вызывающего, включая вложенные вызовы. Restart/rebind не обновляет authority и не стирает обязательства. Acceptance требует реального EL0 allow/deny, отказа unauthorized grants/transfer, attenuation и revoke/admission races, одинакового DEV/PROD enforcement и exact-source evidence. Универсальный policy interpreter и новые обязанности EL1 ради удобства не допускаются.

[Домены и scoped grants](domains.md) описывают принятую Phase 3.4 реализацию требований [issue #24](https://github.com/lifeFedorovAlexey/KOLVRT/issues/24) в границах одного процесса на домен и Event notification. Общий IPC, supervisor policy installation и немедленный drain остаются отдельными условиями. Читайте текущие законы, ADR, identity/lifetime, reclamation и user-copy через явное prerequisite closure; QEMU receipt не доказывает silicon.

<a name="kolvrt-security-event-revocation"></a>

## Ограниченный синхронный Event-срез

[ADR-0022](../architecture-decisions/0022-native-event-grants-and-revocation.md) задаёт отдельные явные права SEND/TRANSFER/REVOKE и общий атомарный state Event. Signal и revoke упорядочиваются этим state; уже принятое pending уведомление сохраняется, новые SEND отклоняются через все aliases. Close снимает одну ссылку и не означает revoke. Revocation не освобождает target и не разрешает Completion, device или общую службу.

[Сохранённый CI receipt](../../../../research/results/event-revocation-ci.json) импортирует реальные результаты принятого #72. Это исходный снимок QEMU DEV/PROD, а не новый запуск в этой задаче или silicon acceptance. Этот ранний снимок не проверяет последующее scoped deferred admission. Текущая Phase 3.4 описана в [контракте доменов](domains.md); общие IPC, supervisor policy installation и немедленный drain остаются отдельными этапами.

[Английский оригинал](../../../../docs/kernel/capabilities.md)

Controls Phase 3.7 вызывают настоящие операции с запрещёнными, scoped и revoked grants и наблюдают сохранение принятого эффекта и конкурентный revoke/admission. План matrix различает отрицательные входы и положительные инварианты retention/rebind; production-реализация revocation не отключается. No-op capability-negative features удалены. Предыдущие exact-source receipts сохраняют исторический scope и не проверяют мигрированный текущий runner.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.capabilities",
  "kind": "subsystem-contract",
  "summary": "Ограниченные grants, domains и revocation Phase 3.4; общие IPC и supervisor policy остаются отдельными этапами.",
  "units": [
    {
      "id": "kolvrt.security.capability-revocation",
      "anchor": "kolvrt-security-capability-revocation",
      "kind": "feature",
      "summary": "Текущие ограниченные Phase 3.4 grants и отзыв с явными оставшимися этапами.",
      "tags": [
        "capability",
        "revocation",
        "phase",
        "3.4",
        "отзыв",
        "полномочий",
        "полномочия"
      ],
      "read_when": ["проверь отзыв полномочий", "review capability revocation"],
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
            "reason": "Phase 3.7 removes source-copy mutation builds and application implementation copies. Tests use the actual production code; replacement fault/restart/shutdown and related acceptance scenarios remain incomplete. Prior receipts retain their historical scope; partial passes are not full current-source acceptance.",
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
      "summary": "Синхронные Event grants и отзыв допуска для всех aliases.",
      "tags": [
        "event",
        "revocation",
        "grant",
        "отзыв",
        "полномочий",
        "полномочия"
      ],
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
            "reason": "Phase 3.7 removes source-copy mutation builds and application implementation copies. Tests use the actual production code; replacement fault/restart/shutdown and related acceptance scenarios remain incomplete. Prior receipts retain their historical scope; partial passes are not full current-source acceptance.",
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
      },
      "read_when": ["проверь отзыв полномочий", "review capability revocation"]
    }
  ]
}
```
