# ADR-0003 — Одна архитектура, DEV/STAGING/PROD

Status: **Accepted**. Date: 2026-10-02.

## Context

Debug instrumentation меняет timing; release должен сохранять safety (cases 10, 18, 30).

## Decision

STAGING — production configuration с bounded diagnostics; все profiles используют один native contract.

## Alternatives

Отдельный prod kernel; только debug testing; полная tracing всегда.

## Why rejected

Архитектурный fork расходится; debug скрывает timing failures; tracing всегда нарушает cost budget.

## Consequences

CI matrix включает optimizer и instrumentation combinations. Полностью stripped PROD не даёт dynamic counters.

## Compatibility impact

Профиль не выбирает compatibility route.

## Performance impact

Optional diagnostics устраняются compile-time; mandatory checks остаются.

## Security impact

Fault injection и live experiments недоступны PROD; validation не отключается.

## Testing

Cross-profile semantic tests и observer overhead experiments.

## Reversibility

Budget и defaults меняются без смены semantic API.

## Evidence

[Cases и первичные источники](../research/CASE_INDEX.md); [сравнение других ОС](../../research/other-systems/COMPARISON.md).
