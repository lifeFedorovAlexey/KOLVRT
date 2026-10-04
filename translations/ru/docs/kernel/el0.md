# Фундамент EL0 и scheduling

Этап реализует настоящее исполнение AArch64 EL0 и timer-driven переключение на обоих CPU выбранной платформы. Восемь статически выбранных процессов работают в отдельных address spaces, по четыре закреплены за каждым CPU. Два worker завершаются; шесть других процессов вызывают faults, не завершая peers или ядро. Затем boot workload освобождает ресурсы и выключается. Это ограниченный фундамент, а не работающая ОС общего назначения. IPC, handles/capabilities, cancellation, service models и security domains остаются следующим этапом. Routing/adapters остаются вне kernel dependency closure.

## Исполнение и ownership

```text
CPU0: allocate/zero frames → create private roots → publish immutable task setup
Each CPU: acquire launch → rendezvous → acquire process ownership → TTBR0 switch
→ ERET EL0t → local timer or synchronous exception → EL1 stack/context
→ stop source/EOI → save task → release owner → select ready task → ERET
No runnable task: restore native root → completed local TLBI → return EL1
CPU0: acquire both completions → drop space guards/charges → release frames
```

| State/resource                          | Owner и mutability                                                               | Publication, lifetime и reclamation                                                                                     |
| --------------------------------------- | -------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Physical allocations и UserSpace guards | CPU0, непередаваемое заимствование Frame                                         | Zero/init до launch; постоянный per-space charge запрещает release при забытом guard                                    |
| Root/L2/L3 и code                       | CPU0 создаёт, immutable во время admission                                       | Девять native fixture pages плюс bounded optional image pages на процесс; publication/cache maintenance до launch       |
| Data и user stack                       | Один закреплённый процесс; его CPU обращается к data, пока процесс приостановлен | Нет cross-process user mapping; verifier читает только после завершения обоих CPU                                       |
| Ready queue и saved context             | Только CPU с соответствующим индексом, IRQ masked                                | UnsafeCell не является абстракцией shared writers; ссылки не переживают user execution                                  |
| Running ownership                       | Per-process atomic CPU identity                                                  | CAS запрещает duplicate execution; affinity check запрещает другой CPU; release при каждом terminal/preempted переходе  |
| Kernel stack                            | Существующий постоянный per-CPU EL1 stack                                        | User SP не используется для kernel exception entry; под user stack находится unmapped guard page                        |
| Setup и completion                      | Release launch, acquire consume; release done, acquire collect                   | Последовательные boot batches используют CAS admission и acquired completion; migration и hotplug support не заявляются |

Scheduling policy — чистый ограниченный round-robin selector в crates/kernel-core/src/scheduling.rs. Привилегированный механизм находится в crates/kernel/src/scheduler/mod.rs и arch/aarch64. Quantum — типизированный Duration 1 ms, выбранный scheduler; это не гарантия измеренной задержки. Ready process получает ход за один оборот при условии доставки local timer и ограниченных EL1 handlers. IRQ не требует allocation, UART logging, queue lock или lock прерванного ordinary code. Шестнадцать slices и deadline workload в две секунды ограничивают враждебные или незавершающиеся образы. Expiry помечает unfinished tasks как timed out; оно никогда не освобождает исполняющееся address space.

## Address spaces и context

Каждый native fixture process владеет тремя table pages, code page, data page и четырьмя stack pages; optional opaque images добавляют bounded RX pages. Общие kernel mappings остаются privileged-only. По общему user VA code является RX с запрещённым privileged execution; data/stack — RW и NX; guard отсутствует. Process-specific alias существует только в root этого процесса, позволяя настоящую проверку отказа по чужому адресу. Различные tags, записанные по одинаковому data VA, проверяют, что TTBR switching выбирает разные physical pages. Kernel code/data и устройства недоступны пользователю.

Native root всегда использует ASID zero. Owned process roots получают nonzero leases с учётом ширины из `ID_AA64MMFR0_EL1.ASIDBits` (8 или 16 бит; `TCR_EL1.AS` включает 16-битные tags, если поддерживаются). Lease содержит закреплённый CPU, slot и монотонный software epoch; hardware namespace локален для каждого CPU, поэтому одинаковые номера ASID на CPU0 и CPU1 не конфликтуют. Обычный switch устанавливает TTBR0 с ASID и не инвалидирует TLB. При terminal completion CPU-владелец выполняет `DSB ISH; TLBI ASIDE1; DSB ISH; ISB` до публикации retirement. Detach, local retirement и acquired scheduler completion предшествуют освобождению frames/tables и повторной выдаче tag. Creation rollback освобождает lease сразу только до публикации root.

Fixed affinity остаётся обязательным. Root нельзя запускать на нескольких CPU; будущий shared-root или migration design должен отслеживать residency и ждать подтверждённой remote invalidation до reuse. User roots immutable во время admission, а shared native mapping changes по-прежнему запрещены при user execution. Если кодировка ASID width неизвестна, fallback использует ASID zero и full local flush на каждом switch. `asid-baseline` включает этот режим для сравнения. [Протокол SMP retirement](smp.md) продолжает защищать изменяемые native mappings.

User context размером 816 байт расширяет GPR/SIMD/FP frame размером 784 байта полями ELR, SPSR, SP_EL0 и TPIDR_EL0. Compile-time проверки offsets/size соответствуют assembly. Context kernel caller, SP_EL0 и TLS register восстанавливаются перед возвратом в EL1. Lower-EL synchronous и IRQ vectors используют kernel stack CPU. User fault завершает только текущий процесс; current-EL fault сохраняет fatal kernel policy. Ожидаемые registers/tags относятся к trusted workload verifier, а не authority checks trap mechanism. Один распознаваемый SVC завершает этот статический workload; это не stable syscall ABI и не capability operation.

## Проверки и ограничения

Завершение worker проверяется после faults всех трёх local peers. Это наблюдение возобновлённого полезного исполнения после faults, а не подсчёт worker, завершившегося раньше них. Итоговый measurement label — `el0-foundation`; предыдущие EL0 development records остаются историческими.

Исходная EL0 foundation содержала 53 checks; актуальные количества DEV/PROD связаны с исходниками сохранённой матрицы. ASID coverage сжимает pool до четырёх tags на CPU, доводит его до exhaustion, затем повторно использует tags с другой physical backing по тому же user VA на обоих CPU. `--asid-reuse-control` убирает retirement TLBI и должен провалить `asid_reuse_requires_invalidation`. QEMU TCG сохраняет isolation даже при пропуске, поэтому negative check напрямую проверяет обязательную инвалидацию. Non-test boot также исполняет process workload; machine validation требует его EL0 event до boot success.

Двенадцать failure-propagation controls должны завершаться ошибкой вместе с scheduler rejection controls. `--asid-reuse-control` намеренно пропускает reuse invalidation. [Результаты](../../../../research/results/kernel-el0.json) сохраняют исторический EL0 source scope. Парное QEMU сравнение #18 хранится отдельно в [ASID measurements](../../../../research/results/issue18-asid-measurements.json). [Unsafe inventory](../../../../research/results/kernel-el0-unsafe-audit.json) — перечень, а не доказательство safety.

Парный benchmark содержит 32 process create/run/reclaim цикла на каждом CPU и восемь counterbalanced QEMU-пар; он сохраняет timer ticks, TTBR switches, полные/scoped invalidation и ASID reuse. Это только QEMU TCG evidence. Оно не устанавливает physical weak-memory behavior, hardware throughput, hard real-time progress или корректность произвольных user programs. Fixed-affinity ASID lifecycle реализован; migration, work stealing, nested IRQ, lazy FPU, demand paging и общий executable loading требуют отдельных контрактов и тестов. Эти процессы не устанавливают DMA boundary, полный native slice, IPC или security-domain гарантии.

[ADR-0014](../architecture-decisions/0014-el0-foundation.md) фиксирует placement, alternatives и reconsideration. Существующие [Kernel Laws](../architecture/kernel-laws.md), особенно LAW-001, LAW-013, LAW-025, LAW-031 и LAW-041, не меняются: dependency isolation, retained charges, scoped platform limits и explicit progress bounds продолжают действовать.

Более поздний [workload Phase 2](routing.md) использует этот native mechanism с sequential CAS-admitted batches и opaque EL0 code. Исторические EL0 records сохраняют исходные one-page/single-batch source hashes; текущие geometry и admission описаны в ADR-0015.

## Обновление ownership Phase 3.0

Более поздний [контракт scheduler](scheduler.md) заменяет описание queue/setup выше: проверки наблюдаемых CPU/IRQ/phase/generation и nonblocking permits контролируют все обращения к storage. AArch64 frame и static verifier разделены. EL0 сам пишет fixture tick word через существующий own-slices call; kernel IRQ больше не пишет fixture data. Runtime storage переживает fixture и payload sessions. Текущая проверка расширяет матрицу до 69 tests и сохраняет ownership controls в обоих profiles. Исторические результаты выше сохраняют первоначальные source scope. Dynamic lifecycle, safe user-copy и IPC остаются отдельно разрешаемыми prerequisites Phase 3 — Native Process & Service Foundation.

[Английский оригинал](../../../../docs/kernel/el0.md)
