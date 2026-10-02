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

**Предусловия и проверка:** Адрес публикуется после проверки, принудительно только CPU0, без IRQ logging, bounded polling; secondary failure публикуется atomics. Настоящие serial events, наблюдаемые host.

### INV-FRAME

**Необходимость и владелец:** Zeroing и доступ к mapped RAM требуют raw addresses.

**Предусловия и проверка:** Physical pool только CPU0, непередаваемый закрытый линейный Frame, initialized nonreserved RAM, borrowed Mapping/Retirement и remote reader acknowledgement до free. Exhaustion/reuse/map/permission tests и premature-release controls.

### INV-MMU

**Необходимость и владелец:** Установка translation tables требует system registers.

**Предусловия и проверка:** Выровненные initialized tables, текущие PC/SP mapped, section W^X, device attributes. Настоящие fault tests и ELF checks.

### INV-TLB

**Необходимость и владелец:** Retirement владения требует architectural completion.

**Предусловия и проверка:** CPU0 PTE writer освобождает table lock до ожидания. CPU1 подтверждает только после завершения reader и local invalidation/barriers. Generation проверяется; один retirement; failure не разрешает release. Настоящий remote fault и omitted-TLBI control.

### INV-HEAP

**Необходимость и владелец:** GlobalAlloc — unsafe pointer contract.

**Предусловия и проверка:** Постоянно reserved 64 KiB, locked disjoint live ranges, исходный Layout при free. Box/Vec и exhaustion tests.

### INV-LOCK

**Необходимость и владелец:** UnsafeCell и реализация Sync требуют исключения.

**Предусловия и проверка:** Acquire/Release, guard нельзя дублировать, T:Send для Lock Sync, guard Sync следует T:Sync; IRQ не использует lock. Host concurrent publication и kernel locking tests.

### INV-GIC

**Необходимость и владелец:** Конструирование device registers предполагает platform topology.

**Предусловия и проверка:** CPU0 инициализирует distributor; каждый CPU сопоставляет GICR_TYPER с полной MPIDR affinity в проверенном ограниченном регионе. Неподдерживаемый VLPI stride отклоняется; IRQ masked при local setup, bounded RWP checks. PPI обоих CPU и повторные bidirectional SGI tests.

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

## Границы Phase 1.1

### INV-SECONDARY

**Необходимость и владелец:** PSCI entry требует assembly; CPU1 владеет отдельным постоянным linker stack. Без BSS/shared initialization. Проверки настоящих EL1/MMU и stack bounds.

### INV-BOOT-PUBLISH

**Предусловия и проверка:** CPU0 публикует READY, очищает linked RAM и root tables до PoC с Cortex-A57 cache line, завершает DSB SY до CPU_ON. CPU1 включает тот же root до acquire shared publication. Boot обоих профилей; coherent-platform pin не доказывает произвольную hardware coherence.

### INV-CPUON

**Предусловия и проверка:** Проверенный DTB SMC conduit, CPU affinity и aligned native entry/root. SMCCC x0-x3 clobbers объявлены, PSCI result проверяется. CPU_OFF вызывается после quiescence; CPU0 также проверяет AFFINITY_INFO.

### INV-PERCPU

**Предусловия и проверка:** MPIDR сопоставляется с immutable affinity table. Stack pointer только наблюдается; mutable state и probes находятся в per-CPU atomics. Distinct ID/stack и независимые timer/fault tests.

### INV-GIC-AFFINITY и INV-SGI

**Предусловия и проверка:** Ограниченный validated MMIO region, GICR_TYPER match, проверенные SGI target bits. Publication до SGI и точный acknowledged INTID для EOI. Повторные IPI в обоих направлениях и timers.

### INV-SHOOTDOWN и INV-REMOTE-READER

**Предусловия и проверка:** Remote tests сохраняют borrowed Mapping/Frame до completion или acknowledged retirement; raw submission является unsafe test API. Reader admission исключается при retirement. IRQ только публикует generation. Ordinary quiescence, local TLBI, DSB/ISB и release acknowledgement предшествуют reclaim. Retained charge защищает от забытых guards. Delayed-reader, premature-release, omitted-ACK и omitted-TLBI controls.

Новое assembly и unsafe Rust остаются в target/kernel/unsafe-audit.json; машинные counts — inventory, а не доказательство.

## Границы доказательств

Этот реестр документирует локальные proof obligations и наблюдаемые tests. Он не доказывает аппаратную корректность, все случаи malformed firmware или SMP safety. Boot firmware, toolchain, emulator и generated instructions остаются trust boundaries. [SMP контракт](smp.md) ограничивает participation двумя CPU и явно удерживаемыми readers. Unsafe не оправдывается одной производительностью; [правило выбора метода](../architecture/implementation-review.md) применяется до принятия.
