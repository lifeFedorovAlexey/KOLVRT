# Модель угроз безопасности

Статус: межподсистемные требования проекта, 2026-10-02. Основная область безопасности за пределами первого native-сценария. [Модель первого сценария](../architecture/threat-model.md) сохраняет более узкий контракт приёмки. Текущие SMP-свидетельства не устанавливают изоляцию EL0, драйверов и compat. [Область исследования](../../research/other-systems/kasperskyos/overview.md).

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

## Инвариант совместимости

EffectiveAuthority_via_compat(C, O) является подмножеством NativeAuthorizedAuthority(C, O) в точке авторизации native-контракта. Учитывать вызванные через службы эффекты, а не только handles C. Служба использует независимые grants только для разрешённых caller эффектов. Отсутствие полномочий даёт DENIED/AUTHORITY_REQUIRED; новые права consumer требуют отдельной native authorization до admission. Без compatibility waiver. Nested/split operations, restart/rebind и PROD routing сохраняют инвариант. [Решение](../architecture-decisions/0013-security-boundaries.md).

## Отзыв и восстановление

Разделять отказ будущего admission и семантику принятой работы. Закрытие одного handle не означает автоматический отзыв descendants. Определить child/delegated grants, expiry, live mappings, reassignment и DMA до выбора реализации отзыва.

Recovery: остановить admission, отменить будущие полномочия, правдиво завершить/отклонить работу, обеспечить quiescence callbacks/IRQ/DMA, подтвердить reset при необходимости, освободить leases, затем создать новое поколение и отдельно авторизовать восстановленные grants/routes. Timeout не устанавливает quiescence; карантин или halt. Privileged invariant failure не является restartable service fault. Неизвестный эффект нельзя считать отсутствующим.

## Профили, диагностика и пределы

DEV поддерживает bounded traces, grant inspection и явную fault injection. PROD сохраняет authorization, проверки handles, isolation, ownership, quotas, barriers и outcomes. Необязательные logs не обеспечивают защиту. Inspector требует authentication/redaction; исключить payloads, raw addresses/usable handles и secrets. [Модель inspection](inspection-model.md).

Физические атаки, hostile boot/host administration и microarchitectural side channels вне текущих подтверждений. General-purpose availability, hostile firmware, настоящая IOMMU/reset containment и persistent recovery не проверены. Доверие устройству не покрывает враждебное оборудование. Будущие тесты требуют отдельно авторизованных scheduler/EL0/user-copy и контракта названного устройства.

[Английский оригинал](../../../../docs/security/threat-model.md)
