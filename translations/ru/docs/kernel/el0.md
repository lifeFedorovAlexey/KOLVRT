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

| State/resource                          | Owner и mutability                                                               | Publication, lifetime и reclamation                                                                                    |
| --------------------------------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Physical allocations и UserSpace guards | CPU0, непередаваемое заимствование Frame                                         | Zero/init до launch; постоянный per-space charge запрещает release при забытом guard                                   |
| Root/L2/L3 и code                       | CPU0 создаёт, immutable во время admission                                       | Шесть удерживаемых страниц на процесс; publication/cache maintenance до launch                                         |
| Data и user stack                       | Один закреплённый процесс; его CPU обращается к data, пока процесс приостановлен | Нет cross-process user mapping; verifier читает только после завершения обоих CPU                                      |
| Ready queue и saved context             | Только CPU с соответствующим индексом, IRQ masked                                | UnsafeCell не является абстракцией shared writers; ссылки не переживают user execution                                 |
| Running ownership                       | Per-process atomic CPU identity                                                  | CAS запрещает duplicate execution; affinity check запрещает другой CPU; release при каждом terminal/preempted переходе |
| Kernel stack                            | Существующий постоянный per-CPU EL1 stack                                        | User SP не используется для kernel exception entry; под user stack находится unmapped guard page                       |
| Setup и completion                      | Release launch, acquire consume; release done, acquire collect                   | Одна boot workload session; reset/relaunch race, migration или hotplug support не заявляются                           |

Scheduling policy — чистый ограниченный round-robin selector в crates/kernel-core/src/scheduling.rs. Привилегированный механизм находится в crates/kernel/src/scheduler.rs и arch/aarch64. Quantum — типизированный Duration 1 ms, выбранный scheduler; это не гарантия измеренной задержки. Ready process получает ход за один оборот при условии доставки local timer и ограниченных EL1 handlers. IRQ не требует allocation, UART logging, queue lock или lock прерванного ordinary code. Шестнадцать slices и deadline workload в две секунды ограничивают враждебные или незавершающиеся образы. Expiry помечает unfinished tasks как timed out; оно никогда не освобождает исполняющееся address space.

## Address spaces и context

Каждый процесс владеет тремя table pages, code page, data page и stack page. Общие kernel mappings остаются privileged-only. По общему user VA code является RX с запрещённым privileged execution; data/stack — RW и NX; guard отсутствует. Process-specific alias существует только в root этого процесса, позволяя настоящую проверку отказа по чужому адресу. Различные tags, записанные по одинаковому data VA, проверяют, что TTBR switching выбирает разные physical pages. Kernel code/data и устройства недоступны пользователю.

Каждое переключение использует ASID zero и завершённый full local TLBI, включая возврат к native root. ASID allocator и migration отсутствуют; root допускается ровно на одном CPU. Shared native mapping changes запрещены во время user execution. Private roots immutable во время admission, поэтому reclamation опирается на возврат обоих CPU к native roots и завершение local invalidation, а не IRQ acknowledgement живого user reader. [Протокол SMP retirement](smp.md) продолжает защищать изменяемые native mappings.

User context размером 816 байт расширяет GPR/SIMD/FP frame размером 784 байта полями ELR, SPSR, SP_EL0 и TPIDR_EL0. Compile-time проверки offsets/size соответствуют assembly. Context kernel caller, SP_EL0 и TLS register восстанавливаются перед возвратом в EL1. Lower-EL synchronous и IRQ vectors используют kernel stack CPU. User fault завершает только текущий процесс; current-EL fault сохраняет fatal kernel policy. Ожидаемые registers/tags относятся к trusted workload verifier, а не authority checks trap mechanism. Один распознаваемый SVC завершает этот статический workload; это не stable syscall ABI и не capability operation.

## Проверки и ограничения

Завершение worker проверяется после faults всех трёх local peers. Это наблюдение возобновлённого полезного исполнения после faults, а не подсчёт worker, завершившегося раньше них. Итоговый measurement label — `el0-foundation`; предыдущие EL0 development records остаются историческими.

Матрица требует 53 tests на каждый DEV/PROD profile: существующие 39 foundation checks и 14 EL0 checks для процессов, timer switches, user stacks, memory isolation, fault containment, quiescent reclamation, GPR/SIMD/FP/TLS context, SMP ownership, отказов kernel/foreign memory, записи code, guard-page access, NX data и privileged instruction. Non-test boot также исполняет workload; machine validation требует его EL0 event до boot success.

Одиннадцать negative host commands должны завершаться ошибкой. Новые controls: `--user-context-control` (повреждение saved marker), `--user-root-control` (native root вместо переключения), `--user-retirement-control` (забывание space guard и попытка frame release). Они дополняют прежние восемь SMP/foundation controls. [Результаты](../../../../research/results/kernel-el0.json) сохраняют точные source hashes, artifact features/sizes, events и timer samples. [Unsafe inventory](../../../../research/results/kernel-el0-unsafe-audit.json) — перечень, а не доказательство safety.

QEMU TCG evidence не устанавливает physical weak-memory behavior, hardware throughput, hard real-time progress или корректность произвольных user programs. Статический allocator/bootstrap workload не является production process-creation interface; dynamic creation, ASID lifecycle, migration, work stealing, nested IRQ, lazy FPU, demand paging, user-copy и общий executable loading требуют отдельных контрактов и тестов. Эти процессы не устанавливают DMA boundary, полный native slice, IPC или security-domain гарантии.

[ADR-0014](../architecture-decisions/0014-el0-foundation.md) фиксирует placement, alternatives и reconsideration. Существующие [Kernel Laws](../architecture/kernel-laws.md), особенно LAW-001, LAW-013, LAW-025, LAW-031 и LAW-041, не меняются: dependency isolation, retained charges, scoped platform limits и explicit progress bounds продолжают действовать.

[Английский оригинал](../../../../docs/kernel/el0.md)
