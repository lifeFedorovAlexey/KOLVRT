# ADR-0005 — ARM64-first с явными platform contracts

Status: **Accepted**. Date: 2026-10-02.

## Context

QEMU virt удобен для старта; DT, DMA и silicon quirks не являются одинаковыми проблемами (cases 10–13, 27).

## Decision

Первый target: AArch64 EL1 MMU SMP GICv3 timer DT UART VirtIO; core architecture-independent contracts; PCIe затем по transport needs.

## Alternatives

x86-first; hardcoded QEMU board layout; сразу все firmware interfaces.

## Why rejected

Не соответствует цели; мешает machine-version evolution; расширяет scope без evidence.

## Consequences

Exact toolchain/machine pins выбираются Phase 0.2. Нет boot code в Phase 0.1.

## Compatibility impact

AArch32 и x86 execution не обещаются Linux personality; hardware adapters scoped отдельно.

## Performance impact

QEMU numbers не считаются silicon performance.

## Security impact

EL1 interrupts/MMU/foreign descriptors требуют audited boundaries; DMA isolation policy открыта.

## Testing

DT fixtures, mock platform contracts, затем emulator/hardware integration.

## Reversibility

Второй порт реализует contracts; если требует core rewrite, это повод пересмотреть abstraction.

## Evidence

[Cases и первичные источники](../research/CASE_INDEX.md); [сравнение других ОС](../../research/other-systems/COMPARISON.md).
