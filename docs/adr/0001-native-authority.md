# ADR-0001 — Native authority и legacy quarantine

Status: **Accepted**. Date: 2026-10-02.

## Context

Старые contracts затрудняют изменения, но Linux сам меняет внутренние API (cases 06, 28).

## Decision

Core зависит только от native contracts. Legacy behavior находится в versioned adapters. Native-only build обязателен.

## Alternatives

Linux-compatible core; global compatibility mode; отсутствие любой compatibility.

## Why rejected

Первое связывает внутреннюю модель чужим ABI; второе не допускает mixed consumers; третье исключает реальную миграцию.

## Consequences

Нужны contracts, adapters и conformance fixtures. Не вся Linux semantics будет выразима.

## Compatibility impact

Старые consumers получают явный supported behavior; никакого silent fallback.

## Performance impact

Translation cost измеряется; native superiority не предполагается.

## Security impact

Adapters не могут обходить native rights; placement отдельно.

## Testing

Native-only graph/build и одинаковый native oracle с adapters/без них.

## Reversibility

До ABI release граница уточняется ADR; после release старые contracts остаются в adapters.

## Evidence

[Cases и первичные источники](../research/CASE_INDEX.md); [сравнение других ОС](../../research/other-systems/COMPARISON.md).
