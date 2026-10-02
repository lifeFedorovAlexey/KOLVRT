# ADR-0010 — Самостоятельный фундамент EL1

Статус: **Принято**. Дата: 2026-10-02.

## Context

Пользователь разрешил настоящее ядро до scheduler, userspace и compatibility. Ранее спроектированный native EL0 slice остаётся последующим milestone. Допускается законченный предыдущий уровень, если SMP пока нельзя корректно обеспечить.

## Decision

Принять работающий фундамент CPU0, описанный в [загрузке](../kernel/boot.md), [памяти](../kernel/memory.md), [IRQ](../kernel/interrupts.md) и [границах CPU](../kernel/smp.md). Независимые безопасные алгоритмы находятся в kernel-core, механизмы процессора — в arch/aarch64, обнаружение платформы — в platform, проверенный MMIO — в hal. Native Cargo closure содержит только kernel и kernel-core плюс compiler runtime. Код и ABI Linux не импортируются.

## Alternatives

Сразу реализовать secondary CPU и EL0 services; строить основу совместимости с чужим ядром; оставить непроверенные simulated subsystems. Альтернативы памяти и синхронизации сравниваются в [ревью реализации](../architecture/implementation-review.md).

## Why rejected

Эти варианты нарушают разрешённый порядок либо вводят недоказанные зависимости владения, retirement и IRQ. Знакомые имена каталогов не обосновывают наследование архитектуры ОС.

## Consequences

Boot и test kernels — настоящие no_std/no_main binaries. Два настроенных CPU не означают SMP; активен только CPU0. Фиксированные ёмкости и identity mapping — явные текущие контракты. Scheduler и EL0 не начинаются автоматически.

## Compatibility impact

Отсутствует: foreign ABI, loader, syscall adapter и compatibility dependency не реализованы.

## Performance impact

Общий core, diagnostic features только DEV, оптимизированный PROD без tests, фиксированные metadata и ограниченные операции. Полное сохранение SIMD и полный TLBI имеют явную стоимость. Глобальное утверждение о самом быстром методе не принимается без сравнительных данных.

## Security impact

Настоящие W^X, RO/NX faults, проверенный DTB, исключение reservations, закрытое владение frame и fatal unexpected exceptions. IRQ не зависит от allocator и прерванных locks. Текущее ядро EL1 не реализует изоляцию EL0.

## Testing

cargo xtask test выполняет оба профиля, настоящие boot images, по 23 in-kernel tests и controls assertion/panic с ошибкой host. Host tools проверяют алгоритмы, переводы, модели и lint. Чистый экспорт исходников обязан собраться без target outputs; установленные закреплённые tools и dependency caches — предусловия.

## Reversibility

Заменять механизм только с новым проверенным контрактом, анализом инвариантов и настоящими отрицательными tests; без временной второй реализации. До SMP обеспечить retirement удалённых readers и состояние каждого CPU. Аппаратура и вторая платформа остаются непроверенными.
