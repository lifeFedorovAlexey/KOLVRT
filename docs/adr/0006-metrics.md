# ADR-0006 — Dependency vector и честный A/B

Status: **Accepted**. Date: 2026-10-02.

## Context

Call counts, CPU и обязательная редкая зависимость не эквивалентны (cases 15, 21, 25).

## Decision

Нет universal score. Публиковать multidimensional vector, текстовые states, coverage и preregistered paired benchmarks.

## Alternatives

Weighted score 0..100; native всегда зелёный по заявлению; сравнение только среднего.

## Why rejected

Вес произволен; декларация не доказывает путь; среднее скрывает tail/failures.

## Consequences

Tooling хранит raw data и attribution. MOSTLY_NATIVE требует consumer-specific migration budget.

## Compatibility impact

Compat faster — допустимый результат; software dependency и hardware translation раздельны.

## Performance impact

Observer cost измеряется ON/OFF, unsupported PMU = unavailable.

## Security impact

Tracing ограничен по правам, памяти и payload collection.

## Testing

Zero denominator, double-count nested calls, dropped events, censored latency и missing counters.

## Reversibility

Новые dimensions добавляются versioned schema; старые raw samples не переписываются.

## Evidence

[Cases и первичные источники](../research/CASE_INDEX.md); [сравнение других ОС](../../research/other-systems/COMPARISON.md).
