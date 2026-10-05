# GICv3 и синхронизация

Проверенные платформой дескрипторы DTB задают distributor и redistributor. Каждый CPU сканирует ограниченный регион и сопоставляет GICR_TYPER affinity со своим MPIDR; неподдерживаемый VLPI stride или отсутствующая affinity завершаются ошибкой. Инициализация пробуждает его, настраивает nonsecure Group 1, physical timer PPI 30 с level trigger, приоритет и системный CPU interface при замаскированном IRQ. Опрос регистров имеет конечный предел итераций.

## Жизненный цикл IRQ

Acknowledge возвращает настоящий INTID. Обработка таймера отключает источник уровня, увеличивает атомарный счётчик доставки и выполняет EOI с EOImode=0, завершая сброс приоритета и деактивацию. Архитектурные барьеры дополняют volatile MMIO; одного volatile недостаточно для порядка эффектов устройств. Spurious IDs игнорируются; неожиданные реализованные IDs идут в fatal path. Тесты проверяют маскирование, доставку и повторное включение, а не заранее установленный boolean.

IRQ mask/unmask wrappers сохраняют compiler memory clobber вместе с архитектурным ISB; обращения к защищённой памяти и публикации, видимые IRQ, нельзя переставлять через DAIF transition. Тайм-аут timer probe остаётся ошибкой после прежнего предела 500 мс. При отказе диагностика читает local physical timer control/compare/counter, IRQ mask state на входе и GICR enable/pending/active bits для соответствующей affinity, не подтверждая interrupt и не меняя controller state. Эти наблюдения отделяют source state от фактической delivery; pending bit не заменяет delivery counter.

## Блокировки

Ограниченный TTAS lock ожидает через relaxed loads и использует Acquire CAS с Release unlock. Время жизни guard защищает доступ UnsafeCell; ограничения Send/Sync соответствуют защищаемому типу. Host tests проверяют публикацию четырьмя конкурентными потоками. IRQ никогда не берёт этот lock, не выделяет память и не пишет UART. Поэтому прерванный код не блокируется навсегда из-за IRQ, ожидающего его собственный lock. Миллион spin iterations — предел отказа, а не гарантия реального времени или доказательство справедливости. [Ревью](../architecture/implementation-review.md) фиксирует альтернативы.

## Multicore delivery

CPU0 однократно инициализирует distributor. Каждый CPU инициализирует только собственный redistributor, timer PPI и именованный coordination SGI. SGI target encoding сохраняет affinity levels и проверяет поддерживаемый target range. IRQ фиксирует настоящую delivery и pending TLB generation; обычный код подтверждает retirement после завершения readers и local TLBI. IRQ не захватывает table/heap locks и не пишет UART. Настоящие tests покрывают оба направления IPI, повторную delivery, независимые timers и Acquire/Release publication shared pair. См. [SMP](smp.md).

Lower-EL timer delivery запрашивает bounded deferred selection после остановки source и EOI. [EL0 switch](el0.md) выполняется с masked IRQ, не берёт queue locks и ничего не выделяет; одна SGI delivery не расходует timer slice.
