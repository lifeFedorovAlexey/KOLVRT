# Native domains и scoped grants

Document status: CURRENT
Evidence scope: bounded Phase 3.4, один процесс на домен, два CPU с fixed affinity, Event-notification requests.
Current reference: [ADR-0023](../../../../docs/architecture-decisions/0023-domains-and-scoped-grants.md)

ABI contract: native.notification/1
Publication stage: EXPERIMENTAL
ABI-FREEZE: none

<a name="kolvrt-domains-scope"></a>

## Границы реализованной функции

Phase 3.4 реализована в обозначенных ограниченных границах. Читайте нижеследующие contracts и exact-source receipt; очередь notification и experimental ABI не определяют общую IPC архитектуру.

<a name="kolvrt-domains-identity"></a>

## Identity, ownership и limits

[Domain primitives](../../../../crates/kernel-core/src/domain.rs) связывают ровно один generation-aware ProcessId с retained DomainId. [Атомарный пул](../../../../crates/kernel-core/src/domain/pool.rs) содержит 32 cells с nonwrapping generations и максимум 1 024 references на cell. DomainId — внутренняя metadata; пользовательское поле не выбирает principal. Retained lease запрещает reuse cell. Exhaustion возвращает явную ошибку. Multi-process domains, root sharing и migration в этом subset не поддерживаются.

[Process owner](../../../../crates/kernel/src/process.rs) владеет domain и private address space. Namespace удерживает тот же domain и точную process identity. Bootstrap передаёт image, affinity, grants и явные `Spec.limits`; runtime валидирует и применяет эти входы. Текущие static boot/test launchers задают fixture configuration. Product policy и будущий installation protocol trusted EL0 supervisor отделены от kernel enforcement; package/model/network policy, зависимость от AI, global policy DSL и ambient allow не добавляются.

Memory pages включают owned page tables, code, data и guarded stacks, плюс loaded ELF pages. Memory limit проверяется до physical allocation. Handle charges учитывают каждый live namespace entry; transfer списывает receiver budget до publication. Request admission списывает consumer outstanding-request budget и service queue/request budgets. Denial откатывает все временные charges и references. Текущая namespace имеет восемь slots, а service — один queue cell; эти storage bounds не задают постоянные ABI maxima. Domain metadata и Event storage имеют отдельные явные fixed kernel pool bounds.

Конкретный [IPC transport](ipc.md) использует те же one-process domain identities и отдельные quota dimensions. `Limits.endpoints` учитывает принадлежащие service domain endpoint instances и не подменяет memory/handle/queue/request limits. Admission удерживает requester request charge, service queue charge и service active request charge. Receive освобождает queue charge; единственный terminal arbiter освобождает active work charge; retained response остаётся под requester charge до успешного collect либо cleanup после смерти. Endpoint charge освобождается только после actual request/wait/reference quiescence. Legacy notification pilot сохраняет one-cell bound; IPC имеет собственные ограниченные очереди и result storage.

<a name="kolvrt-domains-grants"></a>

## Grants и revocation

[Reference primitives](../../../../crates/kernel-core/src/handles.rs) сохраняют generation/type/lifetime checks. Unscoped references не разрешают deferred service effects. Protected bootstrap создаёт explicit Event grant, привязанный к одному immutable target и точному service ProcessId. SEND=1 разрешает notification admission; TRANSFER=2 разрешает attenuated receiver-local delegation; REVOKE=4 разрешает revocation. Trusted Event creation по умолчанию даёт SEND, а Completion creation — NONE; REVOKE требует explicit Event grant. Unknown bits и отсутствующие grants отклоняются до effects. EL0 grant-creation endpoint отсутствует.

Все delegated aliases разделяют target [admission gate](../../../../crates/kernel-core/src/wait.rs). Admission и revocation линеаризуются на одном atomic word. Request или transfer, admitted до revoke, может удерживать target; все последующие effect admissions отклоняются. Close удаляет reference без revocation других aliases. Strong immediate revoke/drain/reset не реализован. Перезапущенный service не получает grants старого service generation.

<a name="kolvrt-domains-notification"></a>

## Конкретная notification boundary

[EL0 request handling](../../../../crates/kernel/src/security.rs) использует SVC 0x90 и immutable 48-byte SafeCopy snapshot. Шесть LE64 words кодируют version=1, operation, local handle либо request sequence, затем три reserved zero words. Nonzero principal/scope fields и unknown versions/operations отклоняются. Caller identity берётся только из executing scheduler-owned process и его bound domain.

| Operation | Contract                                                                                                                                |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| 4         | Admit одно Event notification к same-CPU service из grant; вернуть status 0 и nonwrapping sequence в x1                                 |
| 5         | Выполнить revoke через caller-local REVOKE grant                                                                                        |
| 6         | Отклонить unprivileged grant issuance                                                                                                   |
| 7         | Прочитать current-domain page/handle/queue/request charges; parameter обязан быть zero                                                  |
| 8         | Inspect retained request текущего service; показать consumer closing state и retained request count без pointers или principal selector |
| 9         | Complete только admitted Event effect; parameter обязан быть zero и не может подставить service-private handle                          |
| 10        | Observe caller-bound exact request sequence: 0 pending, 1 completed, 2 cancelled before effect; terminal receipt потребляется один раз  |

Ошибки используют bounded handle statuses: invalid=1, stale=2, wrong type=3, foreign=4, inactive=5, capacity=6, generation exhausted=7, copy failure=8, rights denied=9, reference exhausted=10; scope/authority denied=11 и domain budget exhausted=12. Эта notification boundary сообщает input copy failure как invalid. Pointer, struct layout и compatibility errno не экспортируются.

Service получает authority только для уже admitted concrete effect. Его собственные grants не могут заменить consumer target. Terminal receipt запрещает второй request до consumption; copied identity и sequence не дают stale outcomes связаться с более поздним процессом. Эта notification boundary не содержит arbitrary payload, general endpoint, request cancellation API, automatic request-wait wakeup или deadline. Такие механизмы принадлежат отдельному native IPC contract. Persistent service и supervisor policy остаются следующими gates; pilot не замораживает их архитектуру.

<a name="kolvrt-domains-teardown"></a>

## Teardown и evidence

Exit/fault/budget termination закрывает domain admission на masked scheduler boundary. Namespace retirement освобождает handle charges. Accepted work удерживает target и domain request charge до completion либо cancellation. Умирающий service публикует cancelled-before-effect до освобождения work. Private memory sender можно reclaim после scheduler detachment, пока его copied request остаётся retained; user pointer не удерживается. Final pool release использует atomics, без heap allocation/deallocation или ordinary locks внутри scheduler ownership.

[Actual EL0 fixtures](../../../../crates/kernel/src/security/testing.rs) покрывают оба CPU, unknown rights, missing grants, spoofed scope, foreign memory, attenuation, receiver handle exhaustion, отдельные zero request/queue budgets, revoke racing admission между CPU, fault containment, process/service generation reuse и teardown с accepted work. Step fixtures reclaim sender до пробуждения service и проверяют, что surviving EL0 caller наблюдает service-fault cancellation. Trusted driver использует существующий coalescing wait/event primitive; он не реализует будущий general IPC wait contract. Test-only factory/identity/race helpers инжектируют входы, а authority requests используют production copied boundary.

Пять controls удаляют реальный budget, identity, teardown, deferred revocation либо reserved-scope enforcement. DEV/PROD runner требует соответствующий failed machine test record и nonzero host status. Host tests и Clippy дополняют kernel execution. Retained exact-source receipts задают tested source и named matrix; QEMU TCG не доказывает physical ARM64 ordering или performance.

[Exact-source matrix](../../../../research/measurements/runs/1791130278634-phase3-4-revocation-integrated-f3be261c515b.json) records 96 DEV/PROD checks and 80 negative controls. [Unsafe inventory](../../../../research/results/kernel-phase34-unsafe-audit.json) retains the reviewed source locations.

[Native-only verification](../../../../research/results/native-compat-removal-phase34.json) compares 70 identical native/harness files and repeats the same matrix without compatibility packages. [Routing regression](../../../../research/results/routing-phase34-regression.json) retains six configurations and three rejected controls.

[Английский оригинал](../../../../docs/kernel/domains.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.domains",
  "kind": "subsystem-contract",
  "summary": "Текущие домены и grants Phase 3.4.",
  "units": [
    {
      "id": "kolvrt.security.domains",
      "kind": "feature",
      "anchor": "kolvrt-domains-scope",
      "summary": "Домены, budgets и retained notification outcomes.",
      "tags": [
        "domain",
        "security",
        "grant",
        "notification"
      ],
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
        "issues": [
          24,
          25,
          75
        ],
        "adrs": [
          "adr.0023"
        ],
        "limitations": [
          "One process/domain and fixed-affinity single-cell notification; no general IPC, supervisor installation policy, multi-process domains, immediate revoke/drain, migration or silicon verification."
        ],
        "next_gate": "Rederive general IPC and supervision contracts for #26/#27; early notification encoding, queue and grants cannot freeze later architecture.",
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
      "summary": "Контракт: Identity, ownership и limits"
    },
    {
      "id": "kolvrt.security.domains.grants",
      "kind": "contract-section",
      "anchor": "kolvrt-domains-grants",
      "summary": "Контракт: Grants и revocation"
    },
    {
      "id": "kolvrt.security.domains.notification",
      "kind": "contract-section",
      "anchor": "kolvrt-domains-notification",
      "summary": "Контракт: Конкретная notification boundary"
    },
    {
      "id": "kolvrt.security.domains.teardown",
      "kind": "contract-section",
      "anchor": "kolvrt-domains-teardown",
      "summary": "Контракт: Teardown и evidence"
    }
  ]
}
```
