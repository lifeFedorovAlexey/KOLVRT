# Завершение Phase 1.1 и граница Phase 2

Document status: HISTORICAL MILESTONE
Evidence scope: редакция ревью Phase 1.1; на момент подготовки отчёта интеграция Phase 2 ещё предстояла.
Current reference: [Действующий контракт планировщика](../kernel/scheduler.md); [контракт маршрутизации Phase 2](../kernel/routing.md)

Phase 1 остаётся завершённой исторической native основой. Phase 1.1 добавляет настоящее исполнение двух CPU, PSCI startup, GICv3 с корректной affinity, SGI/IPI, подтверждённый reader/TLB retirement и корректный CPU_OFF. [SMP контракт](../kernel/smp.md) содержит схемы architecture/boot/shootdown, ownership table и ограничения. [ADR-0012](../architecture-decisions/0012-multicore-retirement.md) фиксирует альтернативы; существующие Kernel Laws не требуют изменений.

## Доказательства

[Машинная запись](../../../../research/results/kernel-smp.json) содержит точные source hashes, 39 именованных tests для каждого DEV/PROD image, настоящие boot без tests, raw timer samples и artifact sizes. Восемь negative commands должны завершаться ненулевым кодом, включая secondary panic, premature release, отсутствие acknowledgement и пропуск remote TLBI. Host checks включают отклонение настоящего DTB CPU/PSCI, native dependency closure и существующий concurrent lock test. CI настроен; remote CI execution не заявляется. QEMU evidence не сертифицирует physical hardware или произвольные weak-memory interleavings.

[Запись](../../../../research/results/kernel-foundation.json) Phase 1 не изменяется. Её single-CPU timings не сравнимы с новым SMP workload. Таблица размеров текущего отчёта различает ELF/debug size и PT_LOAD file/memory size; второй постоянный stack добавляет 256 KiB памяти. Unsafe inventory фиксирует lexical locations и assembly hash, а не доказательство unsafe blocks.

| Boot image без tests | Phase 1, байт | Phase 1.1, байт | Разница, байт |
| -------------------- | ------------: | --------------: | ------------: |
| DEV ELF              |       1611120 |         1721552 |       +110432 |
| DEV PT_LOAD file     |         95571 |          116703 |        +21132 |
| DEV PT_LOAD memory   |        914771 |         1202047 |       +287276 |
| PROD ELF             |        266368 |          322504 |        +56136 |
| PROD PT_LOAD file    |         37484 |           47268 |         +9784 |
| PROD PT_LOAD memory  |        852588 |         1128516 |       +275928 |

[Текущий unsafe inventory](../../../../research/results/kernel-smp-unsafe-audit.json) содержит 67 lexical locations против 48 в Phase 1, включая declarations и test-only probes. Добавленные границы охватывают secondary startup/publication, PSCI, affinity/SGI, per-CPU state и remote reader retirement. Каждая граница связана с [unsafe review](../kernel/unsafe.md). Воспроизводимая итоговая матрица имеет label `phase-1-1`; предыдущие development runs сохранены как исторические доказательства. Более ранняя запись `smp-foundation` предшествует исправлению числа negative controls и не является записью завершения.

Полная проверка репозитория прошла до параллельных research изменений. Следующий прогон обнаружил незавершённую ссылку из `research/other-systems/kasperskyos/microkernel.md` на `docs/architecture/kernel-admission-policy.md`; пользователь подтвердил, что эти документы пишет другая задача. Эта работа сохранена. Итоговые SMP source hashes не изменились, а изолированные routing tests/Clippy проходят после исправления констант. Поэтому общая проверка документации текущего workspace не заявляется успешной.

## Сохранённые файлы Phase 2

| Файлы                                                                            | Классификация и текущая граница                                                                |
| -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `crates/kernel-core/src/window.rs`, объявление в `lib.rs`                        | Чистая bounded native reduction и host oracle; нет kernel call/hook                            |
| `crates/routing/Cargo.toml`, `build.rs`                                          | Изолированные dependency features и source-identity generation; нет kernel dependency          |
| `crates/routing/src/lib.rs`: Route, Error, Input, Work, Status, Profile          | Types, version names, profile schema/integrity model и descriptions                            |
| `crates/routing/src/lib.rs`: Consumer, Transaction, dispatch, classify, counters | Standalone runtime candidates; сохранены, не подключены и не приняты как multicore integration |
| `crates/routing/src/conformance.rs`                                              | Standalone contracts versions, validation, isolation и switching; только host evidence         |
| `crates/window-compat/Cargo.toml`, `src/lib.rs`                                  | Standalone optional adapter candidates; нет kernel dispatch                                    |
| Корневые `Cargo.toml`, `Cargo.lock`                                              | Workspace registration/locked dependencies; registration не является kernel dependency         |

Файлы Phase 2 не удалялись и не откатывались. Необходимые formatting, именованные layout constants и standalone build checks сохраняют пригодный groundwork. Normal/build dependency closure native executable по-прежнему содержит только kernel и kernel-core; dependency regression отклоняет routing/adapters в этом closure.

## Отложенная runtime работа

Kernel hooks, shared routing registry, live kernel switching, compatibility dispatch, production routing profile и translation benchmark не добавлялись. Standalone Consumer использует exclusive borrowing, busy transaction flag и local counter arrays. Это ownership candidate, а не доказательство consumer migration между CPU, shared publication, remote draining или точности accounting в concurrent kernel. Завершение Phase 2 и measured overhead не заявляются.

## Найденные предположения и условия integration

| Наблюдение                                                                 | Что нужно до Phase 2 integration                                                                                                     |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Consumer mutation/counters предполагают одного exclusive owner             | Определить owner transfer, state-domain synchronization и per-CPU aggregation; избегать global mode/temporary global lock            |
| Transaction drain охватывает только synchronous borrow scope               | Удерживать route generation и adapter lifetime для всей admitted work и всех CPU; отклонять недоказанные live changes                |
| Tick accounting начисляется вручную без kernel clock boundary              | Реализовать ограниченное настоящее measurement, overflow/loss handling и PROD без instrumentation                                    |
| Profile identity хеширует source files, а не полный executable closure     | Разделить immutable semantic identity, точную built artifact identity и trusted profile authenticity                                 |
| Routing build script читает adapter source даже без compatibility features | Устранить build-time source requirement до доказательства полного удаления adapter source; kernel сам не имеет этой зависимости      |
| Четыре demonstration consumers и одна family                               | Сохранять явные bounds; расширять только для подтверждённого workload с shared-state review                                          |
| Compatibility adapters являются privileged library candidates              | Проверить placement/threat model; EL0 isolation или safety bypass не подразумеваются                                                 |
| Physical ownership/изменение PTE ограничены CPU0 в Phase 1.1               | Будущий routing не должен считать CPU0 постоянным единственным writer; использовать проверенные SMP ownership/publication primitives |

На момент этого ревью следующим этапом была интеграция Phase 2 после проверки и принятия соответствующего ADR. Более поздние свидетельства Phase 2 и Phase 3.0 приведены отдельно в [политике статуса документации](../documentation-policy.md); этот отчёт остаётся свидетельством только для Phase 1.1.

[English source](../../../../docs/research/phase-1-1-review.md)
