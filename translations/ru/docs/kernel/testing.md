# Воспроизводимые проверки ядра

Framing и семантика отказов следуют [политике вывода](../architecture/output-policy.md). cargo xtask run показывает human-консоль, run --prod — PROD, run --machine — framed evidence. cargo xtask test включает события в обоих профилях; console summaries скрывают JSON, а raw logs сохраняют его.

Установите Rust 1.99.0 с rustfmt, clippy и target aarch64-unknown-none, Node.js 18 или новее, а на Windows — 7-Zip. Первое скачивание зависимостей и инструментов требует сети. Windows setup QEMU извлекает закреплённый installer в игнорируемый cache, проверяет SHA-512 и записывает происхождение; service не устанавливается. Другие hosts могут указать QEMU 10.1.0 через QEMU_AARCH64.

## Команды

```powershell
rustup toolchain install 1.99.0 --profile minimal --component rustfmt --component clippy
rustup target add aarch64-unknown-none --toolchain 1.99.0
./scripts/setup-qemu.ps1
npm ci --ignore-scripts
npm run check
cargo xtask test
```

Матрица xtask собирается с locked dependencies, выполняет 53 настоящих kernel tests в каждом профиле и отдельно загружает DEV и PROD без test features. Образы отрицательной assertion, panic, второго physical owner, освобождения retained mapping, secondary panic, release retiring frame, отсутствия acknowledgement и пропуска remote TLBI запускаются дочерними host commands: все одиннадцать обязаны завершиться с ненулевым кодом и ожидаемыми свидетельствами. Положительная матрица отказывает, если какой-либо контроль ошибочно проходит. JSON events обязаны содержать точный набор тестов и итоговое количество suite. Отсутствие результатов, ошибка эмулятора или timeout в 30 секунд завершают команду ошибкой.

Артефакты в target/kernel включают ELF, SHA-256 и build reports features/размеров, полную версию и аргументы QEMU, UART logs, structured results и unsafe inventory. ELF bytes включают debug information; load_bytes считает файловую нагрузку PT_LOAD; memory_bytes включает обнуляемую память. PROD использует release optimization и сохраняет debug information для исследования; diagnostics и kernel-tests отсутствуют в его boot image.

Каждый успешный test run также проверяет raw measurement samples и пересчитывает их quantiles. Используйте `cargo xtask test --record LABEL` для сохранения запуска и дальнейшего сравнения; см. [историю измерений](../../research/measurements/README.md). Host test allocator проверяет линейный объём сканирования относительно той же реализации с отключённым cursor. Эти проверки не означают, что каждый алгоритм глобально самый быстрый.

## Отладка

```text
cargo xtask debug
aarch64-none-elf-gdb target/kernel/dev-boot.elf
(gdb) target remote 127.0.0.1:1234
(gdb) break kernel_main
(gdb) continue
(gdb) info registers
(gdb) bt
```

Используйте отдельно установленный GDB с поддержкой AArch64. QEMU останавливается перед входом через -S и открывает debug server на loopback. Rust symbols и frame pointers поддерживают исследование; ассемблерные границы исключений не имеют полного DWARF unwind metadata, поэтому backtrace через исключение не гарантирован.

## Границы свидетельств

Host tests проверяют безопасные алгоритмы, отклонение повреждённого DTB, кодирование дескрипторов и конкурентную публикацию lock. In-kernel tests проверяют наблюдаемые аппаратурой translation faults и timer IRQ, включая полное восстановление SIMD. Model checks остаются проектными свидетельствами Phase 0. CI повторяет local checks в .github/workflows/kernel.yml; настроенный workflow не является выполненным remote CI run. См. [трактовку unsafe report](unsafe.md) и [границу CPU](smp.md).

## SMP controls

Одинаковый протокол работает с diagnostics и без них. Матрица требует 16 именованных SMP tests и настоящего CPU_OFF, а не только ONLINE flag. Дополнительные flags: `--secondary-panic-control`, `--retirement-control`, `--shootdown-control` и `--remote-tlbi-control`. Каждый обязан завершиться ошибкой; положительный runner проверяет и ненулевой статус, и failure marker. Образ без TLBI должен завершаться ошибкой после чтения mapping CPU1. `cargo test --locked` также отклоняет native dependency reversal. Standalone checks Phase 2: `cargo test --locked -p routing --all-features` и `cargo test --locked -p routing --no-default-features`; они не подключаются к kernel.

[Фундамент EL0](el0.md) добавляет 14 проверок и настоящее non-test execution. Machine boot требует его EL0 result до terminal boot success. Новые negative controls: `--user-context-control`, `--user-root-control`, `--user-retirement-control`; они повреждают saved context, пропускают root switch и пытаются выполнить release после забывания space guard. Workers обязаны завершиться после faults своих local peers, доказывая продолжение исполнения.

Отдельная [матрица Phase 2](routing.md) запускается `cargo xtask routing test`: настоящее EL0 native/v1/v2/bug coexistence, feature-minimal PROD profiles, native-only consumers, stripped boot и три routing failure controls. 53 native checks и одиннадцать foundation controls не меняются. Host decoding tests не заменяют EL0 execution.

## Проверки user-copy Phase 3.2

[Контракт copy](user-copy.md) добавляет реальные EL0 copies и snapshot mutation на обоих CPU, precise mid-copy fault injection, initialized output и generation/lifetime rejection. Текущая DEV/PROD matrix требует 69 именованных checks и 57 failure controls, включая live-reread и disabled-recovery controls в обоих профилях. Четыре size/failure scope сохраняют raw bounded benchmarks для каждого CPU. Та же matrix выполняется без compatibility source packages; повторяется Phase 2 routing matrix. Host tests/Clippy/repository checks дополняют real kernel evidence, но не заменяют его.

## Проверки handles в Phase 3.3

[Контракт handles](handles.md) требует 72 kernel checks и 67 negative controls в DEV/PROD. Пять actual enforcement mutations проверяют generation validation, owner validation, kind validation, unsafe reuse и missing retirement cleanup. Пять handle measurement scopes на CPU добавляются к четырём copy scopes: nineteen unique measurement records вместе с lock baseline. Physical compatibility-package removal и Phase 2 routing должны успешно повториться на том же source candidate.
