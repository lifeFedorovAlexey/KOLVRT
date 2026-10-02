# ADR-0012 — Multicore ownership и подтверждённый retirement

Статус: **Принят для двух CPU в QEMU**. Дата: 2026-10-02.

## Контекст

Активация CPU1 отменяет допущения о lifetime только CPU0. Случаи [09](../../../../research/cases/KOL-PATH-0009.json), [22](../../../../research/cases/KOL-PATH-0022.json) и [30](../../../../research/cases/KOL-PATH-0030.json) разделяют retained access, visibility и reclamation. Workload требует native SMP без scheduler или compatibility runtime.

## Решение

Использовать PSCI CPU_ON, постоянные отдельные stacks, DTB affinity и соответствующие redistributors. Принудительно сохранить physical ownership/изменение PTE за CPU0 в этом ограниченном этапе; оставить существующие TTAS locks для heap и tables. Публиковать одно checked shootdown generation. IRQ фиксирует его; CPU1 подтверждает на обычной границе reader quiescence после local TLBI/barriers. Retiring frame charge сохраняется при забытом guard. Подтвердить quiescence и CPU_OFF до SYSTEM_OFF. [SMP контракт](../kernel/smp.md) задаёт ownership, publication и failure boundaries.

## Альтернативы

Только broadcast TLBI; acknowledgement непосредственно из IRQ; concurrent allocation/PTE writers с RCU или epochs сейчас; оставить CPU1 offline; новый global kernel lock.

## Причины отклонения

Первые два варианта не устанавливают lifetime safety прерванных readers. Дополнительные writers/reclamation methods добавляют неподдержанное состояние без workload. Offline CPU1 не выполняет milestone. Global lock скрывает ownership и может блокировать completion, зависящий от IRQ.

## Последствия

Два CPU исполняют native code, но CPU1 имеет coordination loop и ограниченную test-only работу. Один retirement и постоянное хранилище делают obligations проверяемыми. Timeout является fatal и не разрешает reuse. Дополнительные CPU, hardware, remote mutable access и dynamic address spaces требуют пересмотра контракта.

## Влияние на совместимость

Standalone groundwork Phase 2 сохранён без kernel hooks или dependencies. Неизменяемые version/profile types независимы от числа CPU. Существующие standalone dispatch, switching и accounting являются отложенными runtime candidates. Будущая integration требует review consumer ownership, synchronized publication, pinned request generations и per-CPU accounting.

## Влияние на производительность

Второй постоянный stack добавляет 256 KiB. Busy polling и полная local invalidation требуют времени; fastest-method claim отсутствует. DEV и PROD используют один протокол; correctness acknowledgements остаются без diagnostics. [Доказательства](../../../../research/results/kernel-smp.json) сохраняют image sizes и timer observations; single-CPU и SMP runs не являются equivalent-workload benchmarks.

## Влияние на безопасность

EL0 isolation не заявляется. CPU1 не может безопасно обращаться к physical ownership или UART. Test pointer probes имеют явные retained-lifetime obligations. Secondary privileged failure останавливает систему; identity mapping не разрешает доступ после release.

## Тестирование

Одинаковые 39 tests в DEV/PROD, отдельные boot images и восемь negative controls. Пропуск remote TLBI должен приводить к ошибке после настоящего mapping read. Host checks покрывают отклонение CPU/conduit в DTB и dependency direction. QEMU не доказывает hardware weak memory.

## Обратимость

Будущие writers заменяют CPU0 admission только проверенным per-domain protocol. Сохранить drained generations и ownership; изменения CPU-count constant недостаточно. Доказательства Phase 1 остаются историческими. Изменения Kernel Laws не нужны; scoped enforcement усиливает LAW-013, LAW-018, LAW-035, LAW-041 и LAW-043.

[English source](../../../../docs/architecture-decisions/0012-multicore-retirement.md)
