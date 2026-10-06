# Основа корректности двух CPU

Phase 1.1 запускает два AArch64 EL1 CPU в закреплённой QEMU virt. Последующий [фундамент EL0](el0.md) добавляет fixed-affinity queues на обоих CPU; runtime compatibility остаётся отключённым. [ADR-0012](../architecture-decisions/0012-multicore-retirement.md) фиксирует решение; [обзор Phase 1](../research/phase-1-review.md) остаётся историческим этапом CPU0.

## Запуск и публикация

Проверенные DTB CPU affinity и PSCI SMC определяют поддерживаемую topology. Логическая identity соответствует MPIDR, а не порядку запуска. Каждый CPU имеет постоянный linker stack 256 KiB, vectors, соответствующий affinity GIC redistributor и local timer. Per-CPU atomics содержат state, фактический stack pointer, timer/IPI delivery и TLB acknowledgement generation. Probe recovery является per-CPU и test-only.

```text
CPU0: DTB -> affinity table -> UART/vectors -> tables/heap -> global GIC
      -> release READY -> clean linked RAM to PoC -> DSB SY -> PSCI CPU_ON
CPU1: secondary_entry -> masked EL1h -> private stack -> FP setup -> native root
      -> acquire READY -> check MPIDR/EL/MMU -> matching GICR -> ONLINE -> IRQ
```

CPU1 не очищает BSS и не инициализирует shared resources. Cleaning linked RAM, включая tables и boot publication, обеспечивает видимость для MMU-off entry; coherent atomics действуют после native MMU activation. Cache line размером 64 bytes входит в Cortex-A57 platform pin. Контракты CPU_ON и initial state следуют [PSCI DEN0022D](https://documentation-service.arm.com/static/5f905c71f86e16515cdc1fd0?token=), разделы 5.6 и 6.4.

Сборки проверок ядра сохраняют отдельное поколение завершённой TLBI, публикуемое только
после возврата вторичного CPU из `DSB; TLBI; DSB; ISB`, до подтверждения. Проверка
`smp_remote_ack` требует это наблюдение исполнения для подтверждённого поколения.
Поэтому контроль пропуска процедуры отказывает даже при исчезновении старой трансляции
по другой причине; одного сбоя трансляции недостаточно для доказательства исполнения
процедуры. Наблюдение отсутствует в обычных сборках и не заменяет аппаратную проверку сбоя.

## Ownership и синхронизация

| Ресурс                           | Владелец и синхронизация                                        | Lifetime                                       |
| -------------------------------- | --------------------------------------------------------------- | ---------------------------------------------- |
| Physical pool и изменения PTE    | CPU0 affinity checks; непередаваемые Physical/Frame             | Линейное ownership через retirement            |
| Хранилище tables и stacks        | Постоянное хранилище; CPU0 инициализирует tables                | Весь запуск                                    |
| Heap metadata                    | Существующий TTAS lock; обычный код любого CPU                  | Постоянно зарезервированная RAM                |
| UART и настройка GIC distributor | Только CPU0; secondary failure использует atomics               | Весь запуск                                    |
| Redistributor, timer и probes    | Соответствующий per-CPU owner                                   | Пока CPU online                                |
| Boot data                        | Release/acquire после cache cleaning                            | Не меняются после startup                      |
| Remote work mailbox              | Только tests: producer CPU0/executor CPU1; один item            | Заимствованные data удерживаются до completion |
| Retirement                       | CPU0 публикует; CPU1 подтверждает на обычной границе quiescence | Одно checked generation; без переполнения      |

Ownership CPU0 — принудительно проверяемая область milestone, а не постоянное правило будущих allocation/routing. Дополнительные writers требуют review контракта admission и serialization. Global lock не добавлен. Существующий TTAS использует Acquire CAS/Release unlock; IRQ не захватывает locks и не выделяет память. Исполнение shared-pair test на двух CPU в QEMU проверяет publication и exclusion, а не исчерпывающее weak-memory behavior или fairness.

## Shootdown и retirement

```text
OWNED -> MAPPED -> RETIRING -> RECLAIMABLE -> FREE
CPU0: retain retiring charge -> clear PTE -> release table lock -> local TLBI
      -> publish generation -> SGI
CPU1 IRQ: IAR -> publish pending generation -> EOI/deactivate
CPU1 ordinary boundary: complete prior reader -> TLBI VMALLE1 -> DSB ISH -> ISB
                        -> release acknowledgement
CPU0: acquire acknowledgement -> completion barrier -> clear retiring charge
      -> consuming release may return frame to pool
```

Получение IRQ не может подтвердить завершение прерванного reader. Обычный loop CPU1 подтверждает только после завершения работы. При ожидании table lock не удерживается. Broadcast TLBI не заменяет acknowledgement. Новые mappings отклоняются во время pending retirement. Retirement заимствует Frame; забытый guard сохраняет charge, и release всё равно отклоняется. Существующие mapped aliases также запрещают release. Timeout/failure останавливает систему без разрешения reuse.

Постоянный privileged identity alias сохраняется. Ownership управляет доступом; аудированные remote pointers существуют только в tests. Это не выполняет retirement произвольных privileged pointers, DMA, неограниченных readers, третьего CPU или userspace mappings. Расширение области требует reader leases и admission control участников.

## Interrupts, failure и shutdown

Именованный SGI 1 обслуживает coordination. GICR discovery сканирует ограниченные frames и сопоставляет полную сжатую MPIDR affinity; неподдерживаемый VLPI stride, отсутствующая affinity и неподдерживаемые SGI target ranges завершаются ошибкой. Настоящие IAR ID определяют обслуживание timer/IPI и deactivation в EOImode=0. Timer sources останавливаются до EOI.

Shutdown прекращает mailbox admission, завершает принятую работу, маскирует IRQ CPU1, останавливает timer, завершает local TLBI и публикует QUIESCENT. CPU1 вызывает CPU_OFF; CPU0 ждёт quiescence и AFFINITY_INFO=OFF до SYSTEM_OFF. Постоянные stacks/tables сохраняются. Secondary panic/fault публикует FAILED и останавливает CPU; CPU0 обнаруживает это и останавливает систему без reclamation или восстановления повреждённого privileged state.

## Проверки и ограничения

Историческая evidence Phase 1.1: `cargo xtask test` запускал 39 именованных tests в DEV и оптимизированном PROD, загружает оба образа без tests и требует восемь failing host controls. Шестнадцать SMP tests покрывают execution/IDs/stacks, независимое per-CPU state, оба направления IPI, 32 повторных IPI, фактическую publication общего lock, remote mapping reads, задержанное acknowledgement, запрещённый reuse, remote translation fault, безопасный reuse, оба timer и корректный CPU_OFF. SMP controls добавляют secondary panic, release retiring frame, отсутствие acknowledgement и пропуск remote TLBI. Последний приводит к ошибке после настоящего remote read, исключая boolean-only shootdown test.

Этот исторический campaign не является текущим matrix plan. Текущие coverage kinds и ограничения missing-ACK/skipped-TLBI описаны в [native application contract](native-applications.md).

[Машинные доказательства](../../../../research/results/kernel-smp.json) сохраняют source hashes и точные результаты; [unsafe register](unsafe.md) задаёт trust boundaries. QEMU TCG подтверждает kernel integration, но не сертифицирует silicon. Busy polling, два фиксированных CPU, один outstanding retirement и fatal progress timeouts — явные ограничения.

Более поздний [EL0 routing workload](routing.md) исполняет independent consumers одновременно на обоих CPU. Batch admission использует per-CPU CAS phases; completion и native-root/TLBI quiescence предшествуют CPU0 reset/reclamation. Shared routing writer и routing lock не добавляются.

[English source](../../../../docs/kernel/smp.md)

CPU1 читает STOP с Acquire до чтения admitted mailboxes. Если STOP появился после этого snapshot, shutdown выполняется на следующей итерации после повторного чтения work; accepted command не теряется между пустым mailbox snapshot и STOP. TLB_REQUEST является authoritative mailbox, IPI — уведомлением. Actual local TLBI предшествует release ACK; задержка IRQ не скрывает принятый retirement request.
