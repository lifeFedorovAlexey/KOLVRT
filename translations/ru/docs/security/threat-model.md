# Модель угроз безопасности

Статус: межподсистемные требования проекта, 2026-10-02. Основная область безопасности за пределами первого native-сценария. [Модель первого сценария](../architecture/threat-model.md) сохраняет более узкий контракт приёмки. Одни свидетельства SMP не устанавливают изоляцию служб или DMA. Ограниченные наблюдения основы EL0 и маршрутизации ниже подтверждают лишь записанную область задач/адаптеров. [Область исследования](../../research/other-systems/kasperskyos/overview.md).

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

## Инвариант совместимости

EffectiveAuthority_via_compat(C, O) является подмножеством NativeAuthorizedAuthority(C, O) в точке авторизации native-контракта. Учитывать вызванные через службы эффекты, а не только handles C. Служба использует независимые grants только для разрешённых caller эффектов. Отсутствие полномочий даёт DENIED/AUTHORITY_REQUIRED; новые права consumer требуют отдельной native authorization до admission. Без compatibility waiver. Nested/split operations, restart/rebind и PROD routing сохраняют инвариант. [Решение](../architecture-decisions/0013-security-boundaries.md).

## Отзыв и восстановление

Разделять отказ будущего admission и семантику принятой работы. Закрытие одного handle не означает автоматический отзыв descendants. Определить child/delegated grants, expiry, live mappings, reassignment и DMA до выбора реализации отзыва.

Recovery: остановить admission, отменить будущие полномочия, правдиво завершить/отклонить работу, обеспечить quiescence callbacks/IRQ/DMA, подтвердить reset при необходимости, освободить leases, затем создать новое поколение и отдельно авторизовать восстановленные grants/routes. Timeout не устанавливает quiescence; карантин или halt. Privileged invariant failure не является restartable service fault. Неизвестный эффект нельзя считать отсутствующим.

## Профили, диагностика и пределы

DEV поддерживает bounded traces, grant inspection и явную fault injection. PROD сохраняет authorization, проверки handles, isolation, ownership, quotas, barriers и outcomes. Необязательные logs не обеспечивают защиту. Inspector требует authentication/redaction; исключить payloads, raw addresses/usable handles и secrets. [Модель inspection](inspection-model.md).

Физические атаки, hostile boot/host administration и microarchitectural side channels вне текущих подтверждений. General-purpose availability, hostile firmware, настоящая IOMMU/reset containment и persistent recovery не проверены. Доверие устройству не покрывает враждебное оборудование. Будущие тесты требуют отдельно авторизованных scheduler/EL0/user-copy и контракта названного устройства.

## Observation boundary Phase 2

Ограниченный [EL0 routing slice](../kernel/routing.md) имеет явный [admission review каждой обязанности](../architecture-decisions/0015-el0-versioned-routing.md). Bootstrap admission разрешает только timer observations текущей задачи; caller-selected owner, pointer, delegation и general capability service отсутствуют. Native register reads повторяют width/bounds checks и инициализируют outputs; общая арифметика и вся legacy policy исполняются в EL0. Task state и roots удерживаются, пока оба CPU не восстановят native roots и не завершат TLBI. Ошибка adapter не расширяет scope и ограничена task; privileged enforcement compromise остаётся fatal. Evidence reporting — bounded instrumentation, исключённая из stripped PROD, а не authenticated deployment inspector. Trusted boot/compiler, static grants, fixed affinity, отсутствие DMA и непроверенные silicon/availability — явные ограничения. Пересмотреть capture publication, grants/revocation и inspection placement вместе с native IPC services.

## Граница scheduler Phase 3.0

[ADR-0016](../architecture-decisions/0016-scheduler-ownership.md) сужает существующее enforcement: foreign CPU/IRQ re-entry, stale state/task, duplicate running ownership, live reset/read и оба направления locks отклоняются до unsafe storage access или execution. Generations не могут переполниться, completed snapshots не могут заимствовать live state, а retained roots переживают native-root/TLBI completion. Оба profiles используют одинаковый mechanism; privilege violations фатальны. Новый authority issuer, dynamic process API, routing policy или IPC admission не добавляются. Queue-session generations не заменяют будущие per-process identities и lifecycle rollback.

[Английский оригинал](../../../../docs/security/threat-model.md)
