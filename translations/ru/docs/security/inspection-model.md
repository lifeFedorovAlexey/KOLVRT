# Модель артефакта проверки безопасности

Схема исследования, не runtime CLI/API или stable ABI. [Offline-схема домена](../../../../schemas/security-domain.schema.json) определяет записи ревью; пределы ограничивают артефакт, а не постоянные бюджеты ядра.

## Поля и ограничения

Учитывать ID, generation, privilege, implementation/evidence state, native issuer и object/effect scope grants, service-private статус, compat-зависимости, relationships, device ownership, TCB по свойствам и source provenance. Названия прав — review IDs, а не допуск новых capability-типов. Proposed records не устанавливают реальные grants. Фиктивный CLI не реализован.

Учёт служебных capabilities не равен учёту полномочий caller. Явно задавать согласованность snapshot, authorization, redaction, bounded enumeration и unavailable counters. Не открывать payloads, usable handles, raw addresses и secrets. Будущие inspect-команды требуют собственных native grants.

## Будущие проверки

Отвергать unauthorized inspection и неполные поколения; проверять redaction и bounded truncation при concurrent revocation. Обязательное обеспечение остаётся при исчезновении diagnostic consumers. Schema validation доказывает только shape/bounds, а не authority, minimum privilege и hardware containment. [Спецификации сценариев](../../../../research/fixtures/security-boundaries.json) отмечены как not executed.

[Английский оригинал](../../../../docs/security/inspection-model.md)
