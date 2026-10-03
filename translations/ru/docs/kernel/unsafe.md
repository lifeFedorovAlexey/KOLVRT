# Реестр unsafe-границ

Запустите `cargo xtask audit` для создания target/kernel/unsafe-audit.json. Отчёт перечисляет first-party Rust locations с контекстом инвариантов, SHA-256 ассемблера, развёртки register macros, native Cargo dependency closure и hashes закреплённых артефактов core/alloc/compiler_builtins. Compiler runtime явно доверенный, а не отдельно доказанный по исходникам. Lexical inventory включает unsafe declarations и attributes; число locations не является числом unsafe blocks.

## Инварианты

### INV-ENTRY

**Необходимость и владелец:** Startup требует privileged assembly; primary CPU владеет stack/BSS.

**Предусловия и проверка:** Вход EL1/EL2, выровненный стек, IRQ masked, только CPU0. EL1 boot test и linker bounds.

### INV-VECTOR

**Необходимость и владелец:** Assembly сохраняет exception ABI вне обычных вызовов Rust.

**Предусловия и проверка:** Все GPR/SIMD/FP state, стек 16 байт, vectors 2 KiB, без nested IRQ. Синхронный user-copy abort может прервать lower-EL trap: EL1 register frame восстанавливается, а внешний lower-EL return state остаётся в saved Context. BRK/fault tests, намеренное повреждение SIMD в IRQ и precise user-copy recovery проверяют поддерживаемые пути.

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

**Предусловия и проверка:** Acquire/Release, guard нельзя дублировать, T:Send для Lock Sync, guard не является Send/Sync и не переносится между CPU/thread; IRQ не использует lock. Host concurrent publication и kernel locking tests.

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

### INV-USER-SPACE and INV-USER-RETIRE

CPU0 инициализирует проверенные эксклюзивно выделенные table/data/code/stack/image pages и удерживает Frame borrows и постоянные space charges. Забытый guard не разрешает release. Roots immutable во время admission; каждый закреплён за одним CPU. Оба CPU восстанавливают native root и завершают local TLBI до release completion. CPU0 acquires оба completion до снятия charges. EL0 fixture сам пишет tick word; kernel IRQ не пишет fixture data; tags читаются после остановки всех writers. Настоящие RO/NX/guard/foreign faults, выживание peers, возвращённый free count и отказ forgotten-guard проверяют обязательства.

### INV-USER-TTBR and INV-USER-IMAGE

Privileged root/barrier instructions требуют live aligned tables, сохраняющие kernel PC/SP. ASID zero требует full local invalidation при каждом switch и перед reclamation. Trusted immutable linker extents копируются в checked owned pages, очищаются до PoC и публикуются с instruction-cache maintenance до launch. Пропуск root-switch должен вызвать ошибку. Loader, migration, ASID reuse и доказательство physical cache-coherency не заявляются.

### INV-USER-CONTEXT and INV-RUNQUEUE

Lower-EL vectors используют per-CPU EL1 stack и полный aligned frame размером 816 байт; compile-time offsets совпадают с assembly. Kernel ABI/SP/TLS восстанавливается при возврате. Каждый CPU исключительно изменяет свою UnsafeCell queue с masked IRQ; ссылки не переживают user execution. Setup предшествует release launch; acquire done предшествует inspection. Per-CPU CAS admission и acquire terminal completion не позволяют stale polls войти в reset batch. Per-process ownership CAS и fixed affinity запрещают duplicate/wrong-CPU execution. IRQ не берёт locks и ничего не выделяет. Ожидаемые user registers относятся к verifier, а не authority logic. Настоящие GPR/SIMD/FP/TLS/stack checks, timer switching и отказ corrupted-context проверяют эти границы. См. [контракт EL0](el0.md).

Этот реестр документирует локальные proof obligations и наблюдаемые tests. Он не доказывает аппаратную корректность, все случаи malformed firmware или SMP safety. Boot firmware, toolchain, emulator и generated instructions остаются trust boundaries. [SMP контракт](smp.md) ограничивает participation двумя CPU и явно удерживаемыми readers. Unsafe не оправдывается одной производительностью; [правило выбора метода](../architecture/implementation-review.md) применяется до принятия.

Phase 2 добавляет INV-DEMO-ENTRY и INV-DEMO-SVC в отдельном EL0 image: private aligned guarded stack, fixed RX entry и explicit GPR clobbers для nonpointer native calls. ELF extraction отклоняет malformed extents и writable globals до selection. Native SVC handlers используют current-task state с IRQ masked, не удерживают user references и собирают bounded reports только с machine-events. [ADR-0015](../architecture-decisions/0015-el0-versioned-routing.md) фиксирует authority/necessity и actual tests; native dependency closure не меняется.

Phase 3.0 ограничивает scheduler UnsafeCell access операциями preparation, mutation и inspection в local.rs. Наблюдаемые CPU, masked IRQ, phase, generation и nonblocking permit проверяются до каждого разыменования; higher-ranked closures запрещают borrowed results. Acquired Done и release финального access предшествуют inspection/reset, а native-root/TLBI completion — reclamation. Per-CPU scope/lock tracking отклоняет оба направления locks; guards нельзя переносить между CPU. [Контракт scheduler](scheduler.md) перечисляет реальные rejection controls и оставшиеся пробелы. Ни reference, ни permit не переживают ERET или waits. Context expectations теперь полностью относятся к bootstrap verification после quiescence.

## Изменения владения в Phase 3.1

[Жизненный цикл процессов](processes.md) не добавляет небезопасного хранилища или реализации `Sync`: `Registry` владеет линейными кадрами в значении Rust, которое не реализует `Send` и `Sync`. При допуске адресные пространства заимствуются на время синхронной диспетчеризации; получение `Done` и освобождение последнего разрешения предшествуют отсоединению корней и копированию результата завершения. В `local.rs` остаются один `Sync` и три места разыменования хранилища, доступные только при исключительных разрешениях. `INV-USER-SPACE`, `INV-USER-RETIRE` и `INV-RUNQUEUE` включают проверки поколения процесса, отката и отсоединения очереди до освобождения. Новый небезопасный код ограничен выделением срезов из доверенного образа компоновщика, чтением меток после безопасной остановки и проверкой контракта CPU1 без указателей; применяются `INV-USER-IMAGE`, `INV-USER-RETIRE` и `INV-REMOTE-READER`. [Обновлённая инвентаризация](../../../../research/results/kernel-phase31-unsafe-audit.json) фиксирует эти границы; модель и координатор небезопасного кода не добавляют. См. [ADR-0017](../architecture-decisions/0017-process-lifecycle.md).

## Изменение copy в Phase 3.2

### INV-USER-COPY

**Необходимость и владелец:** Права EL0 и precise EL1 abort containment требуют privileged AT и unprivileged byte assembly. Indexed executing CPU владеет синхронной copy; native memory/architecture boundary отвечает за review. [Access](../../../../crates/kernel/src/user_copy.rs), [Permission queries](../../../../crates/kernel/src/arch/aarch64/mod.rs), [Loops](../../../../crates/kernel/src/arch/aarch64/entry.S) и [Решение](../architecture-decisions/0018-safe-user-copy.md) определяют границу.

**Предусловия и ошибки:** Checked non-null aperture, overflow/size bounds, права EL0 каждой страницы, точные retained process/queue generation и root, fixed affinity, exclusive scheduler scope и masked IRQ. Immutable private mappings и Registry admission borrows исключают mutation/retirement до return. Initialized disjoint kernel byte storage не выдаёт user Rust references и не сериализует struct/padding. Восстанавливаются только точные LDTRB/STTRB data-abort PC при active guard, valid in-range FAR и matching direction; остальные EL1 faults остаются fatal. Ошибка input не публикует snapshot; ошибка output сообщает записанный prefix. Нет allocation/ordinary lock/yield или reference через ERET. PROD сохраняет enforcement без optional counters.

**Проверки и ограничения:** [Реальные EL0 checks](../../../../crates/kernel/src/user_copy/testing.rs) проверяют два CPU, range/permission failures, cross-page/maximum copy, snapshot mutation, terminal/unlinked identity и stale generation. Controls с отключённым recovery/live reread проваливаются в DEV/PROD. Recovery сохраняет saved outer EL0 return state и восстанавливает EL1 registers. Существующие native-root/TLBI completion и reclamation остаются обязательными. Не подтверждены mutable/shared mappings, asynchronous exit, migration, DMA или silicon proof.

### INV-USER-COPY-TEST

**Необходимость и владелец:** Kernel-test-only injection пропускает page preflight для восьми bounded bytes: четыре допустимых байта перед unmapped guard. Fixture владеет initialized scratch и удерживает current process/root. Production assembly и recovery возвращают четыре завершённых байта в обоих направлениях; input suffix остаётся poisoned, peers продолжают работу и reclaim. Проверяется hardware fault containment, не поддерживаемый unmap race. Trusted immutable fixture slicing использует INV-USER-IMAGE. Новые unsafe Sync и shared mutable process storage отсутствуют.
