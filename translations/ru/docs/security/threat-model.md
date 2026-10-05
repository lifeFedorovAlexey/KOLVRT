# Модель угроз безопасности

Document status: DESIGN BASELINE
Evidence scope: Принятые требования; реализация и проверка имеют отдельные границы доказательств.
Current reference: [Documentation policy](../documentation-policy.md)

Статус: межподсистемные требования проекта, 2026-10-02. Основная область безопасности за пределами первого native-сценария. [Модель первого сценария](../architecture/threat-model.md) сохраняет более узкий контракт приёмки. Одни свидетельства SMP не устанавливают изоляцию служб или DMA. Ограниченные наблюдения основы EL0 и маршрутизации ниже подтверждают лишь записанную область задач/адаптеров. [Область исследования](../../research/other-systems/kasperskyos/overview.md).

<a name="kolvrt-security-trust"></a>

## Цели и доверие

Защищать целостность ядра, native-разрешённую область ресурсов/эффектов, идентичность/время жизни, ограниченный учёт и правдивые исходы. Уязвимость не должна автоматически компрометировать чужие домены; область ущерба зависит от grants, общего состояния, посредников и оборудования. Safe Rust не доказывает авторизацию, логику и временную корректность. Утверждения об иммунитете к zero-days нет.

| Участник                     | Предполагаемое доверие и пределы                                                        |
| ---------------------------- | --------------------------------------------------------------------------------------- |
| Kernel core/architecture     | Доверенное обеспечение; invariant failure фатален                                       |
| HAL                          | Доверенная узкая privileged hardware граница                                            |
| Native service               | Потенциально faulty; в TCB свойств, которые обеспечивает только она                     |
| Драйвер                      | Device-scoped grants; DMA/reset определяют integrity TCB                                |
| Compat adapter               | Недоверенные входы; только независимо авторизованные эффекты caller                     |
| Native/legacy application    | Недоверенные запросы, поддельные handles, races/exhaustion                              |
| Device firmware/DMA hardware | Недоверенные только при реальном ограничении; иначе явное доверие или unsupported       |
| Boot/toolchain               | Доверенные provenance, compiler/runtime и machine assumptions; authenticity не показана |
| DEV tooling                  | Авторизованные scoped inspection/injection; контролировать deployment exposure          |

TCB определяется для свойства и включает применимые safe/generated code, службы и оборудование. [Анализ TCB](../../research/other-systems/kasperskyos/tcb.md).

<a name="kolvrt-security-threats"></a>

## Угрозы и условия проверки

| Угроза                              | Обеспечение и будущий отрицательный случай                                                                   |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Memory corruption/unsafe defects    | Настоящие адресные пространства и audited EL1; служба не пишет kernel memory                                 |
| Confused deputy/escalation          | Доверенная caller attribution и область эффектов; disk-service grant не открывает raw disk или чужой root    |
| Malformed IPC/compat input          | Собственный snapshot, bounds, enum/flags, alignment/conversions, поколение/тип handle и limits               |
| Resource exhaustion/DoS             | Учёт до публикации, bounded queues, retained charges, supervisor cleanup reserve; явные progress assumptions |
| Logic bugs/races/stale capabilities | Нужные live predicates синхронизированы с admission/отзывом; старые поколения отвергаются                    |
| Compromised driver/malicious DMA    | Scoped MMIO/IRQ/DMA, hardware restriction, reset и leases; unrestricted DMA не локализован                   |
| Replay/unknown effects              | Один terminal arbiter, без автоматического повтора при неизвестном отсутствии эффектов                       |

<a name="kolvrt-security-placement"></a>

## Привилегированное размещение и рассмотрение отказов

Каждая новая ответственность EL1 должна обосновать необходимость привилегий согласно
[политике допуска](../architecture/kernel-admission-policy.md), включая альтернативу сервиса
EL0 с существующими узкими примитивами. Авторитетное состояние и координация сами по себе
не требуют EL1. До смены размещения рассмотреть точный инвариант, минимальный механизм,
область вызывающего и эффектов, учёт ресурсов и последствия компрометации. Политика сервиса
по умолчанию относится к EL0; она может оставаться в TCB отдельного свойства без привилегий.

Скомпрометированный сервис не должен публиковать произвольные таблицы страниц, получать
более широкие полномочия или освобождать память, удерживаемую читателями или устройствами.
Отрицательные обязательства включают поддельную область, устаревшую идентичность, исчерпание
квот, раннее освобождение, отсутствие подтверждения и небезопасное восстановление по сроку.
Существующие ограниченные проверки MMU/SMP подтверждают только сохранённую область;
общий допуск полномочий отображения и враждебный DMA остаются непроверенными. Отказ доверенного
привилегированного обеспечения фатален для области ядра. Более быстрый путь не заменяет эту
границу отказа без явного ADR, обновлённой модели угроз и сохранения гарантий.

<a name="kolvrt-security-compat-authority"></a>

## Инвариант совместимости

EffectiveAuthority_via_compat(C, O) является подмножеством NativeAuthorizedAuthority(C, O) в точке авторизации native-контракта. Учитывать вызванные через службы эффекты, а не только handles C. Служба использует независимые grants только для разрешённых caller эффектов. Отсутствие полномочий даёт DENIED/AUTHORITY_REQUIRED; новые права consumer требуют отдельной native authorization до admission. Без compatibility waiver. Nested/split operations, restart/rebind и PROD routing сохраняют инвариант. [Решение](../architecture-decisions/0013-security-boundaries.md).

<a name="kolvrt-security-revocation"></a>

## Отзыв и восстановление

Разделять отказ будущего admission и семантику принятой работы. Закрытие одного handle не означает автоматический отзыв descendants. Определить child/delegated grants, expiry, live mappings, reassignment и DMA до выбора реализации отзыва.

Recovery: остановить admission, отменить будущие полномочия, правдиво завершить/отклонить работу, обеспечить quiescence callbacks/IRQ/DMA, подтвердить reset при необходимости, освободить leases, затем создать новое поколение и отдельно авторизовать восстановленные grants/routes. Timeout не устанавливает quiescence; карантин или halt. Privileged invariant failure не является restartable service fault. Неизвестный эффект нельзя считать отсутствующим.

<a name="kolvrt-security-profiles"></a>

## Профили, диагностика и пределы

DEV поддерживает bounded traces, grant inspection и явную fault injection. PROD сохраняет authorization, проверки handles, isolation, ownership, quotas, barriers и outcomes. Необязательные logs не обеспечивают защиту. Inspector требует authentication/redaction; исключить payloads, raw addresses/usable handles и secrets. [Модель inspection](inspection-model.md).

Физические атаки, hostile boot/host administration и microarchitectural side channels вне текущих подтверждений. General-purpose availability, hostile firmware, настоящая IOMMU/reset containment и persistent recovery не проверены. Доверие устройству не покрывает враждебное оборудование. Будущие тесты требуют отдельно авторизованных scheduler/EL0/user-copy и контракта названного устройства.

<a name="kolvrt-security-phase2"></a>

## Observation boundary Phase 2

Ограниченный [EL0 routing slice](../kernel/routing.md) имеет явный [admission review каждой обязанности](../architecture-decisions/0015-el0-versioned-routing.md). Bootstrap admission разрешает только timer observations текущей задачи; caller-selected owner, pointer, delegation и general capability service отсутствуют. Native register reads повторяют width/bounds checks и инициализируют outputs; общая арифметика и вся legacy policy исполняются в EL0. Task state и roots удерживаются, пока оба CPU не восстановят native roots и не завершат TLBI. Ошибка adapter не расширяет scope и ограничена task; privileged enforcement compromise остаётся fatal. Evidence reporting — bounded instrumentation, исключённая из stripped PROD, а не authenticated deployment inspector. Trusted boot/compiler, static grants, fixed affinity, отсутствие DMA и непроверенные silicon/availability — явные ограничения. Пересмотреть capture publication, grants/revocation и inspection placement вместе с native IPC services.

<a name="kolvrt-security-phase30"></a>

## Граница scheduler Phase 3.0

[ADR-0016](../architecture-decisions/0016-scheduler-ownership.md) сужает существующее enforcement: foreign CPU/IRQ re-entry, stale state/task, duplicate running ownership, live reset/read и оба направления locks отклоняются до unsafe storage access или execution. Generations не могут переполниться, completed snapshots не могут заимствовать live state, а retained roots переживают native-root/TLBI completion. Оба profiles используют одинаковый mechanism; privilege violations фатальны. Новый authority issuer, dynamic process API, routing policy или IPC admission не добавляются. Queue-session generations не заменяют будущие per-process identities и lifecycle rollback.

<a name="kolvrt-security-phase31"></a>

## Граница процессов Phase 3.1

[Жизненный цикл процессов](../kernel/processes.md) отделяет удерживаемое владение адресными пространствами и защищённую поколением идентичность от полномочий. Единственный `Registry` отклоняет повторное создание; задача выполняется только на CPU с соответствующим индексом. Проверки источника, CPU и IRQ выполняются до выделения памяти; у EL0 нет точки создания процесса, дескриптора или полномочия на ожидание. Завершение фиксируется после фактической безопасной остановки с восстановлением собственного корня и `TLBI`, затем очередь планировщика отсоединяется. Устаревшая идентичность не выбирает повторно использованный слот; неудачное создание возвращает каждый полученный кадр и снимает учёт памяти. В DEV и PROD применяются одинаковые ограничения. Конкурентный допуск и освобождение, удерживаемые операции копирования данных пользователя и публичная выдача полномочий не реализованы. [ADR-0017](../architecture-decisions/0017-process-lifecycle.md) обосновывает необходимость привилегированного кода и указывает, что размещение будущей политики EL0 ещё предстоит определить.

[Английский оригинал](../../../../docs/security/threat-model.md)

<a name="kolvrt-security-phase32"></a>

## Граница памяти в Phase 3.2

[Safe user-copy](../kernel/user-copy.md) не предоставляет operation authority. Проверяются точные executing process/root, bounds и аппаратные права EL0; immutable mappings/lifetime удерживаются, input публикует только завершённые initialized snapshots. Precise copy faults возвращают ошибки, а остальные EL1 invariant failures остаются fatal. Частичные output writes сообщают свой prefix; internal Rust layout/padding не экспортируются. Parsing, authority и effects должны использовать тот же snapshot в указанном порядке. Mutable/shared mappings, asynchronous exit и DMA не поддерживаются; [ADR-0018](../architecture-decisions/0018-safe-user-copy.md) определяет privileged necessity и review gate для обоих профилей.

<a name="kolvrt-security-phase33"></a>

## Ограниченная reference identity Phase 3.3

Исторический этап: следующий абзац описывает ADR-0019 до принятия расширения ADR-0020.

[Handle namespaces](../kernel/handles.md) отклоняют stale/forged/foreign/wrong-kind references без выбора чужой таблицы или dereference user object pointer. Concrete wait/completion storage защищено и принадлежит kernel; lookup возвращает synchronous retained borrow, cleanup exit/fault предшествует frame reclaim. Eight slots и generation quarantine ограничивают resource use и исключают stale aliasing. Эти гарантии не дают authority; rights, delegation, asynchronous retained work и revocation — отдельные gates. General object dispatch не добавлен.

<a name="kolvrt-security-handle-retention"></a>

## Историческая граница ADR-0020: полномочия и удержание

[ADR-0020](../architecture-decisions/0020-handle-transfer-and-retention.md) добавляет проверки SEND/TRANSFER, передачу с сужением прав, дескрипторы в пространстве получателя и ограниченные владеющие ссылки на Event. Идентичность вызывающего берётся из выполняемой задачи планировщика; EL0 не может выбрать исходного субъекта полномочий. Передача из EL0 допускается только живому получателю на том же CPU с фиксированной привязкой. Закрытие одной записи сохраняет остальные удерживаемые ссылки и не отзывает права производных дескрипторов. Этот абзац сохраняет область ADR-0020 до Phase 3.4: тогда общая выдача прав, домены безопасности и отзыв прав оставались отдельными условиями. Удержание общего Event само по себе не доказывает наличие IPC-контракта для нескольких ожидающих. Текущая ограниченная реализация описана ниже.

<a name="kolvrt-security-phase34"></a>

## Текущая ограниченная граница Phase 3.4

[ADR-0022](../architecture-decisions/0022-native-event-grants-and-revocation.md) и [ADR-0023](../architecture-decisions/0023-domains-and-scoped-grants.md) добавляют явные grants Event и домены с защищённой поколением идентичностью. [Контракт доменов](../kernel/domains.md) фиксирует один процесс на домен, два CPU с фиксированной привязкой и одно место в очереди уведомлений Event. Идентичность вызывающего берётся только из выполняемого процесса и его домена; неизменяемый SafeCopy-снимок не позволяет выбрать субъекта, область или цель из приватных полномочий службы. Права и бюджеты проверяются до публикации, отказ возвращает временные ссылки и учёт.

Допуск и отзыв синхронизируются на одном атомарном состоянии: уже принятая работа удерживает цель и учёт домена, а новые допуски эффектов через любой делегированный alias отклоняются. Закрытие дескриптора не является отзывом. При завершении службы публикуется исход cancelled-before-effect перед освобождением; перезапуск не возобновляет полномочия старого поколения. Ограниченные задачи #24/#25 завершены, но общий IPC, несколько процессов в домене, межпроцессорные очереди, установка политики доверенным супервизором и немедленный drain/reset остаются отдельными условиями. [Исходные свидетельства Phase 3.4](../kernel/domains.md#kolvrt-domains-teardown) описывают QEMU DEV/PROD; физический ARM64, DMA и общая изоляция сервисов ими не подтверждены.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.security.threat-model",
  "kind": "security-analysis",
  "summary": "Навигация по документу: Модель угроз безопасности. Доказательства имеют указанные границы.",
  "units": [
    {
      "id": "kolvrt.security.trust",
      "anchor": "kolvrt-security-trust",
      "kind": "contract-section",
      "summary": "Каноническая секция: Цели и доверие.",
      "depends_on": []
    },
    {
      "id": "kolvrt.security.threats",
      "anchor": "kolvrt-security-threats",
      "kind": "contract-section",
      "summary": "Каноническая секция: Угрозы и условия проверки.",
      "depends_on": []
    },
    {
      "id": "kolvrt.security.placement",
      "anchor": "kolvrt-security-placement",
      "kind": "contract-section",
      "summary": "Каноническая секция: Привилегированное размещение и рассмотрение отказов.",
      "depends_on": []
    },
    {
      "id": "kolvrt.security.compat-authority",
      "anchor": "kolvrt-security-compat-authority",
      "kind": "contract-section",
      "summary": "Каноническая секция: Инвариант совместимости.",
      "depends_on": []
    },
    {
      "id": "kolvrt.security.revocation",
      "anchor": "kolvrt-security-revocation",
      "kind": "contract-section",
      "summary": "Каноническая секция: Отзыв и восстановление.",
      "depends_on": []
    },
    {
      "id": "kolvrt.security.profiles",
      "anchor": "kolvrt-security-profiles",
      "kind": "contract-section",
      "summary": "Каноническая секция: Профили, диагностика и пределы.",
      "depends_on": []
    },
    {
      "id": "kolvrt.security.phase2",
      "anchor": "kolvrt-security-phase2",
      "kind": "contract-section",
      "summary": "Каноническая секция: Observation boundary Phase 2.",
      "depends_on": []
    },
    {
      "id": "kolvrt.security.phase30",
      "anchor": "kolvrt-security-phase30",
      "kind": "contract-section",
      "summary": "Каноническая секция: Граница scheduler Phase 3.0.",
      "depends_on": []
    },
    {
      "id": "kolvrt.security.phase31",
      "anchor": "kolvrt-security-phase31",
      "kind": "contract-section",
      "summary": "Каноническая секция: Граница процессов Phase 3.1.",
      "depends_on": []
    },
    {
      "id": "kolvrt.security.phase32",
      "anchor": "kolvrt-security-phase32",
      "kind": "contract-section",
      "summary": "Каноническая секция: Граница памяти в Phase 3.2.",
      "depends_on": []
    },
    {
      "id": "kolvrt.security.phase33",
      "anchor": "kolvrt-security-phase33",
      "kind": "contract-section",
      "summary": "Каноническая секция: Ограниченная reference identity Phase 3.3.",
      "depends_on": []
    },
    {
      "id": "kolvrt.security.handle-retention",
      "anchor": "kolvrt-security-handle-retention",
      "kind": "contract-section",
      "summary": "Историческая область полномочий и удержания ADR-0020 до Phase 3.4.",
      "depends_on": []
    },
    {
      "id": "kolvrt.security.phase34",
      "anchor": "kolvrt-security-phase34",
      "kind": "contract-section",
      "summary": "Текущая ограниченная безопасность Phase 3.4 с отдельными будущими условиями.",
      "depends_on": [
        "kolvrt.security.domains",
        "kolvrt.security.event-revocation"
      ]
    }
  ]
}
```
