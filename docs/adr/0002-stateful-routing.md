# ADR-0002 — Маршрутизация по state domains

Status: **Accepted**. Date: 2026-10-02.

## Context

fd/OFD, locks, credentials и futexes связывают API families (cases 02, 03, 17, 26).

## Decision

Bind consumer/family/protocol на exact version/digest; shared-state closure ограничивает splitting и switching.

## Alternatives

Произвольный per-call router; один process-wide flag; routing по имени приложения.

## Why rejected

Per-call ломает state; flag слишком груб; имя приложения не является contract.

## Consequences

Потребуется domain graph и explicit handle export/import. Начать с конечной family table.

## Compatibility impact

Mixed consumers возможны; некоторые personalities неделимы без gateway.

## Performance impact

Resolve вне hot path; измерять dispatch и gateway отдельно.

## Security impact

Пересечение policies, fail closed на conflict/downgrade; no authority gain.

## Testing

Resolver conflict tests и model-check quiescent rebind/unload.

## Reversibility

Можно менять resolver implementation; pinned semantic bindings остаются совместимыми.

## Evidence

[Cases и первичные источники](../research/CASE_INDEX.md); [сравнение других ОС](../../research/other-systems/COMPARISON.md).
