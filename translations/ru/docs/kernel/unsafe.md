# Реестр unsafe-границ

Запустите `cargo xtask audit` для создания target/kernel/unsafe-audit.json. Отчёт перечисляет first-party Rust locations с контекстом инвариантов, SHA-256 ассемблера, развёртки register macros, native Cargo dependency closure и hashes закреплённых артефактов core/alloc/compiler_builtins. Compiler runtime явно доверенный, а не отдельно доказанный по исходникам. Lexical inventory включает unsafe declarations и attributes; число locations не является числом unsafe blocks.

## Инварианты

### INV-ENTRY

**Необходимость и владелец:** Startup требует privileged assembly; primary CPU владеет stack/BSS.

**Предусловия и проверка:** Вход EL1/EL2, выровненный стек, IRQ masked, только CPU0. EL1 boot test и linker bounds.

### INV-VECTOR

**Необходимость и владелец:** Assembly сохраняет exception ABI вне обычных вызовов Rust.

**Предусловия и проверка:** Все GPR/SIMD/FP state, стек 16 байт, vectors 2 KiB, без nesting. BRK/fault tests и намеренное повреждение SIMD в IRQ.

### INV-REG

**Необходимость и владелец:** Rust не выражает privileged register reads.

**Предусловия и проверка:** Runtime EL1, правильное архитектурное кодирование. EL, counter и MMU tests; compiler доверенный.

### INV-FDT

**Необходимость и владелец:** Firmware bytes требуют превращения в slice.

**Предусловия и проверка:** Закреплённый RAM start, immutable boot data, проверенный размер до ядра. Safe parser проверяет extents, blocks, cells, properties; malformed/truncated host tests.

### INV-MMIO

**Необходимость и владелец:** Device access требует volatile raw pointers.

**Предусловия и проверка:** Проверенные непересекающиеся platform regions, mapped device lifetime, проверенные width/alignment/overflow. UART identity и GIC delivery tests.

### INV-UART

**Необходимость и владелец:** Diagnostic output использует raw PL011 registers.

**Предусловия и проверка:** Адрес публикуется после проверки, только CPU0, без IRQ logging, bounded polling. Настоящие serial events, наблюдаемые host.

### INV-FRAME

**Необходимость и владелец:** Zeroing и доступ к mapped RAM требуют raw addresses.

**Предусловия и проверка:** Единственный physical pool, закрытый линейный Frame, initialized nonreserved RAM, borrowed Mapping, завершённый unmap/TLBI до free. Exhaustion/reuse/map/permission tests.

### INV-MMU

**Необходимость и владелец:** Установка translation tables требует system registers.

**Предусловия и проверка:** Выровненные initialized tables, текущие PC/SP mapped, section W^X, device attributes. Настоящие fault tests и ELF checks.

### INV-TLB

**Необходимость и владелец:** Retirement владения требует architectural completion.

**Предусловия и проверка:** Только CPU0; publication barrier, invalidation, completion barrier, instruction synchronization. Unmap fault и reuse tests; remote readers не заявляются.

### INV-HEAP

**Необходимость и владелец:** GlobalAlloc — unsafe pointer contract.

**Предусловия и проверка:** Постоянно reserved 64 KiB, locked disjoint live ranges, исходный Layout при free. Box/Vec и exhaustion tests.

### INV-LOCK

**Необходимость и владелец:** UnsafeCell и реализация Sync требуют исключения.

**Предусловия и проверка:** Acquire/Release, guard нельзя дублировать, T:Send для Lock Sync, guard Sync следует T:Sync; IRQ не использует lock. Host concurrent publication и kernel locking tests.

### INV-GIC

**Необходимость и владелец:** Конструирование device registers предполагает platform topology.

**Предусловия и проверка:** CPU0 affinity zero, первый redistributor, IRQ masked при setup, bounded RWP checks. Настоящие PPI masking/delivery/rearm tests.

### INV-IRQ

**Необходимость и владелец:** DAIF и GIC operations меняют execution state.

**Предусловия и проверка:** Инициализация до unmask, acknowledge точного ID, stop source до EOI, EOImode=0. Настоящие IRQ tests.

### INV-TIMER

**Необходимость и владелец:** Доступ к counter comparator архитектурный.

**Предусловия и проверка:** Проверенный deadline загрузочной эпохи, правильная frequency, готовый PPI. Conversion host tests и настоящая доставка timer.

### INV-PSCI

**Необходимость и владелец:** Firmware shutdown требует SMC.

**Предусловия и проверка:** Закреплённый virt PSCI SMC contract, без возвращённого владения. Завершение QEMU и host timeout fallback.

### INV-PROBE

**Необходимость и владелец:** Negative tests намеренно вызывают fault или перезаписывают registers.

**Предусловия и проверка:** Только kernel-test feature, точная регистрация PC/resume, bounded recovery, без PROD recovery. RO/NX/unmap и SIMD negative controls.

## Границы доказательств

Этот реестр документирует локальные proof obligations и наблюдаемые tests. Он не доказывает аппаратную корректность, все случаи malformed firmware или SMP safety. Boot firmware, toolchain, emulator и generated instructions остаются trust boundaries. Добавление CPU нарушает single-owner assumptions и требует [SMP review](smp.md). Unsafe не оправдывается одной производительностью; [правило выбора метода](../architecture/implementation-review.md) применяется до принятия.
