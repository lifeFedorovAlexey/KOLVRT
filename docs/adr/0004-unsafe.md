# ADR-0004 — Safe Rust и audited unsafe boundaries

Status: **Accepted**. Date: 2026-10-02.

## Context

Dirty Pipe, DMA ordering и reclamation требуют явных lifetime proofs (cases 08, 10, 22).

## Decision

Core safe by default; unsafe только в boundary crates с invariant inventory и SAFETY comments.

## Alternatives

Unsafe во всех crates при локальном удобстве; полный запрет unsafe включая MMIO.

## Why rejected

Первое размывает audit surface; второе не позволяет реализовать hardware boundary.

## Consequences

Требуются review и разные виды тестирования; safe Rust не доказывает semantic correctness.

## Compatibility impact

Legacy parsing не получает exemption из unsafe policy.

## Performance impact

Безопасная abstraction может иметь overhead; оптимизация требует proof и measurements.

## Security impact

Dependencies, generated code, Send/Sync и asm входят в TCB inventory.

## Testing

Host models/Miri где применимы, concurrency litmus, QEMU и real hardware; не выполнено сейчас.

## Reversibility

Wrapper implementation заменяем, public safe guarantees нельзя ослабить незаметно.

## Evidence

[Cases и первичные источники](../research/CASE_INDEX.md); [сравнение других ОС](../../research/other-systems/COMPARISON.md).
