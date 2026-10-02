# ADR-0008 — Размещение adapters и drivers по protection domains

Status: **Proposed — RESEARCH_REQUIRED**. Date: 2026-10-02.

## Context

Starnix, Linuxulator, WSL1 и Rust kernels показывают разные boundaries; одна библиотечная граница не гарантирует isolation.

## Decision

Зафиксировать interface boundaries сейчас; не выбирать microkernel/monolithic placement без прототипа IPC и threat model. Privileged adapter требует отдельного обоснования.

## Alternatives

Все adapters в kernel; все adapters в userspace; per-operation placement без общей state model.

## Why rejected

Первое увеличивает TCB; второе может потребовать сложных native primitives; третье ломает ownership.

## Consequences

Phase 0.2 сравнит два настоящих host models одной операции, затем архитектурный ADR. Никаких fake kernel paths.

## Compatibility impact

Некоторые Linux families потребуют одной personality domain; placement не меняет semantic contract.

## Performance impact

Измерять copies, context switches, CPU и p99; нет заранее назначенного победителя.

## Security impact

Сформулировать malicious-adapter и malicious-device модели; in-process boundary не называть sandbox.

## Testing

Fault containment, crash recovery, cancellation, shared handle transfer и quota enforcement.

## Reversibility

Решение открыто; public contracts должны допускать смену placement при сохранении guarantees.

## Evidence

[Cases и первичные источники](../research/CASE_INDEX.md); [сравнение других ОС](../../research/other-systems/COMPARISON.md).
