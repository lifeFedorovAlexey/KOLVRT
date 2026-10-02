# ADR-0007 — Bug compatibility и удаление modules

Status: **Accepted**. Date: 2026-10-02.

## Context

Ошибка иногда становится зависимостью, но security bugs не должны сохраняться (cases 01, 07, 08, 23, 29).

## Decision

Fix native + regression; безопасная старая semantics получает отдельный opt-in module только при consumer evidence; removal по declared и observed dependencies.

## Alternatives

Вечно держать баг в native; немедленно ломать всех; автоматически генерировать shim каждого бага.

## Why rejected

Первое противоречит native truth; второе не даёт migration; третье плодит ненужный debt.

## Consequences

Нужны registry/tombstones, migration window и поддержка dormant/offline consumers.

## Compatibility impact

Security-incompatible request отклоняется; legacy semantics не равна право на exploit.

## Performance impact

Lifecycle accounting имеет стоимость вне hot path; runtime metrics optional по profile.

## Security impact

Нельзя вернуть unsafe behavior или выгрузить referenced code.

## Testing

Migration fixtures, zero consumers с неполной coverage не разрешает removal, native-only suite.

## Reversibility

Module можно вернуть новым supported artifact до support expiry; native bug не возвращается.

## Evidence

[Cases и первичные источники](../research/CASE_INDEX.md); [сравнение других ОС](../../research/other-systems/COMPARISON.md).
