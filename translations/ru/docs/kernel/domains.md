# Native domains и scoped grants

Document status: CURRENT
Evidence scope: bounded Phase 3.4, один процесс на домен, два CPU с fixed affinity, Event-notification requests.
Current reference: [ADR-0022](../../../../docs/architecture-decisions/0022-domains-and-scoped-grants.md)

ABI contract: native.notification/1
Publication stage: EXPERIMENTAL
ABI-FREEZE: none

## Identity, ownership и limits

[Domain primitives](../../../../crates/kernel-core/src/domain.rs) связывают ровно один generation-aware ProcessId с retained DomainId. [Атомарный пул](../../../../crates/kernel-core/src/domain/pool.rs) содержит 32 cells с nonwrapping generations и максимум 1 024 references на cell. DomainId — внутренняя metadata; пользовательское поле не выбирает principal. Retained lease запрещает reuse cell. Exhaustion возвращает явную ошибку. Multi-process domains, root sharing и migration в этом subset не поддерживаются.

[Process owner](../../../../crates/kernel/src/process.rs) владеет domain и private address space. Namespace удерживает тот же domain и точную process identity. Bootstrap передаёт image, affinity, grants и явные `Spec.limits`; runtime валидирует и применяет эти входы. Текущие static boot/test launchers задают fixture configuration. Product policy и будущий installation protocol trusted EL0 supervisor отделены от kernel enforcement; package/model/network policy, зависимость от AI, global policy DSL и ambient allow не добавляются.

Memory pages включают owned page tables, code, data и guarded stacks, плюс loaded ELF pages. Memory limit проверяется до physical allocation. Handle charges учитывают каждый live namespace entry; transfer списывает receiver budget до publication. Request admission списывает consumer outstanding-request budget и service queue/request budgets. Denial откатывает все временные charges и references. Текущая namespace имеет восемь slots, а service — один queue cell; эти storage bounds не задают постоянные ABI maxima. Domain metadata и Event storage имеют отдельные явные fixed kernel pool bounds.

## Grants и revocation

[Reference primitives](../../../../crates/kernel-core/src/handles.rs) сохраняют generation/type/lifetime checks. Bare references не разрешают Event effects. Protected bootstrap создаёт explicit Event grant, привязанный к одному immutable target и точному service ProcessId. SEND=1 разрешает notification admission; TRANSFER=2 разрешает attenuated receiver-local delegation; REVOKE=4 разрешает revocation. Обычное создание reference по умолчанию даёт SEND/TRANSFER, но не REVOKE. Unknown bits и отсутствующие grants отклоняются до effects. EL0 grant-creation endpoint отсутствует.

Все delegated aliases разделяют target [admission gate](../../../../crates/kernel-core/src/wait.rs). Admission и revocation линеаризуются на одном atomic word. Request или transfer, admitted до revoke, может удерживать target; все последующие effect admissions отклоняются. Close удаляет reference без revocation других aliases. Strong immediate revoke/drain/reset не реализован. Перезапущенный service не получает grants старого service generation.

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

Service получает authority только для уже admitted concrete effect. Его собственные grants не могут заменить consumer target. Terminal receipt запрещает второй request до consumption; copied identity и sequence не дают stale outcomes связаться с более поздним процессом. Arbitrary payload, general endpoint, request cancellation API, automatic request-wait wakeup, deadline, persistent service и cross-CPU request queue отсутствуют. Это отдельные IPC/supervisor milestones, архитектура которых должна заново выводиться из accepted invariants, а не замораживаться этим pilot.

## Teardown и evidence

Exit/fault/budget termination закрывает domain admission на masked scheduler boundary. Namespace retirement освобождает handle charges. Accepted work удерживает target и domain request charge до completion либо cancellation. Умирающий service публикует cancelled-before-effect до освобождения work. Private memory sender можно reclaim после scheduler detachment, пока его copied request остаётся retained; user pointer не удерживается. Final pool release использует atomics, без heap allocation/deallocation или ordinary locks внутри scheduler ownership.

[Actual EL0 fixtures](../../../../crates/kernel/src/security/testing.rs) покрывают оба CPU, unknown rights, missing grants, spoofed scope, foreign memory, attenuation, receiver handle exhaustion, отдельные zero request/queue budgets, revoke racing admission между CPU, fault containment, process/service generation reuse и teardown с accepted work. Step fixtures reclaim sender до пробуждения service и проверяют, что surviving EL0 caller наблюдает service-fault cancellation. Trusted driver использует существующий coalescing wait/event primitive; он не реализует будущий general IPC wait contract. Test-only factory/identity/race helpers инжектируют входы, а authority requests используют production copied boundary.

Пять controls удаляют реальный budget, identity, teardown, revocation либо reserved-scope enforcement. DEV/PROD runner требует соответствующий failed machine test record и nonzero host status. Host tests и Clippy дополняют kernel execution. Retained exact-source receipts задают tested source и named matrix; QEMU TCG не доказывает physical ARM64 ordering или performance.

[Exact-source matrix](../../../../research/measurements/runs/1791128001893-phase3-4-final-e08829fce5b9.json) records 96 DEV/PROD checks and 80 negative controls. [Unsafe inventory](../../../../research/results/kernel-phase34-unsafe-audit.json) retains the reviewed source locations.

[Английский оригинал](../../../../docs/kernel/domains.md)
