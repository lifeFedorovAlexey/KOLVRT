# KOLVRT

## Kernel Outside Legacy, Versioned Routing & Translation

> **Legacy может работать. Оно не определяет устройство ядра.**

KOLVRT — **ядро операционной системы на Rust с ARM64 как основной платформой**.

Проект начинается с намеренно неудобного вопроса:

**Как выглядело бы современное ядро, если бы десятилетия исторического поведения перестали считаться вечным архитектурным законом?**

KOLVRT не является форком Linux.

Это не Linux, переписанный на Rust.

Это не попытка повторить внутреннее устройство Linux с более безопасным синтаксисом.

Linux и другие зрелые ОС используются как **свидетельства**: десятилетия ошибок, регрессий, hardware quirks, удачных идей, неудачных абстракций и решений совместимости, которые стоит изучить.

Затем KOLVRT принимает собственное решение.

**Ядро остаётся чистым. Совместимость адаптируется вокруг него.**

---

## Идея

Традиционная совместимость обычно накапливается внутри:

```text
old software
     │
     ▼
special case
     │
     ▼
another special case
     │
     ▼
kernel
```

KOLVRT намерен вынести её наружу:

```text
                         ┌───────────────┐
legacy software ───────► │ compat v1/v2  │ ──────┐
                         └───────────────┘       │
                                                 ▼
native software ─────────────────────────► Native API
                                                 │
                                                 ▼
                                          ┌────────────┐
                                          │   KOLVRT   │
                                          │   kernel   │
                                          └────────────┘
```

Если старому ПО нужно старое поведение, ему место в **версионированном слое совместимости**.

Если ядро когда-то предоставило ошибочное поведение и ПО стало от него зависеть, native поведение исправляется. Совместимость с ошибкой находится вне native core.

Постоянный шрам не нужен лишь потому, что кто-то когда-то зависел от раны.

---

## Зачем это существует

KOLVRT строится вокруг нескольких намеренно строгих идей:

- **Native поведение является источником истины.**
- **Legacy semantics не относятся к native kernel API.**
- **Совместимость явная, версионированная и удаляемая.**
- **Разные приложения, драйверы и подсистемы могут одновременно использовать разные routes.**
- **Стоимость совместимости должна быть измерима.**
- **Старое поведение сохраняется лишь там, где оно кому-то требуется.**
- **Ошибки ядра исправляются, а не превращаются в вечную архитектуру.**
- **Предпочтителен safe Rust; `unsafe` — аудируемая граница, а не удобство.**
- **ARM64 имеет Tier 1.**
- **Другие ОС — источники для исследования, а не готовые схемы.**

Или короче:

> **Не переделывайте ядро под ошибочные предположения. Переводите предположения.**

---

## Текущее состояние

KOLVRT уже загружается как native AArch64 kernel в QEMU.

| Область                            | Статус                                                    |
| ---------------------------------- | --------------------------------------------------------- |
| Rust `no_std` ядро                 | ✅                                                        |
| AArch64 / ARM64                    | ✅ Tier 1                                                 |
| Загрузка QEMU `virt`               | ✅                                                        |
| Исполнение EL1                     | ✅                                                        |
| Проверка Device Tree               | ✅                                                        |
| PL011 UART                         | ✅                                                        |
| Векторы исключений                 | ✅                                                        |
| Physical memory allocator          | ✅                                                        |
| Page tables / MMU                  | ✅                                                        |
| W^X mappings                       | ✅                                                        |
| Kernel heap                        | ✅                                                        |
| GICv3                              | ✅ Оба CPU                                                |
| ARM physical timer IRQ             | ✅                                                        |
| DEV / PROD профили                 | ✅                                                        |
| Автоматический kernel test harness | ✅                                                        |
| Настоящие in-kernel tests          | ✅ 84 checks в exact-source матрице DEV/PROD              |
| Negative failure controls          | ✅                                                        |
| Отладка GDB                        | ✅                                                        |
| SMP                                | ✅ Основа двух CPU в QEMU                                 |
| EL0 / userspace                    | ✅ Ограниченный фундамент изолированных процессов         |
| Scheduler                          | ✅ Timer-driven с фиксированной per-CPU affinity          |
| Жизненный цикл процессов           | ✅ Ограниченные create/start/exit/reclaim на двух CPU     |
| Ожидание собственного события      | ✅ Ограниченный block/wakeup в `Registry::step()`         |
| Копирование из памяти пользователя | ✅ Ограниченный синхронный снимок Phase 3.2               |
| Runtime versioned routing          | ✅ Ограниченный EL0 vertical slice; native core независим |
| Migration advisor                  | ✅ Host planner; production integrations ещё не готовы    |
| Linux compatibility                | ⏳ Не начата                                              |

Оба настроенных CPU исполняют native EL1 code. QEMU matrix проверяет secondary boot, per-CPU ownership, двусторонние IPI, подтверждённый remote TLB retirement и multicore shutdown. Настоящее hardware остаётся непроверенным.

---

## Что дальше

Порядок разработки выбран намеренно:

```text
Native ARM64 kernel foundation        ✅
        │
        ▼
SMP correctness foundation            ✅
        │
        ▼
EL0 + address spaces                  ✅
        │
        ▼
Scheduler + context switching        ✅
        │
        ▼
Versioned Routing & Translation      ✅ bounded EL0 slice
        │
        ▼
Process lifecycle + own-event wait  ✅ bounded Phase 3.1
        │
        ▼
Safe user-copy                      ✅ bounded Phase 3.2
        │
        ▼
Process-local handles               ✅ bounded Phase 3.3
        │
        ▼
Scoped grants and security domains  implemented (#24/#25)
IPC and services                    next (#26)
        │
        ▼
Compatibility personalities
        │
        ▼
Linux ABI compatibility where useful
```

Compatibility не подключается к single-CPU kernel с последующим исправлением для SMP.

Сначала native execution model. Ограниченный фундамент EL0/address spaces и timer scheduler запускает процессы с фиксированной affinity на двух CPU. Phase 2 routing исполняется в optional isolated EL0 image; native core остаётся независим. В Phase 3.1 добавлены управляемые ядром создание, запуск, завершение и освобождение процессов, а также ограниченный latch собственного события для вытесняемого пути `step()`. Это механизмы доверенной bootstrap-координации, а не публичные EL0 API процессов или ожидания. Phase 3.2 добавляет bounded safe user-copy, Phase 3.3 — process-local handles и receiver-local transfer. Phase 3.4 добавляет scoped Event grants, revocation, resource budgets и [security domains](docs/kernel/domains.md). Общий IPC, cancellation и постоянные services остаются будущей работой; полный native slice не завершён. См. [жизненный цикл процессов](docs/kernel/processes.md), [контракт ожидания](docs/kernel/wait.md), [handles](docs/kernel/handles.md) и [scheduler](docs/kernel/scheduler.md). Issue [#24](https://github.com/lifeFedorovAlexey/KOLVRT/issues/24) фиксирует gate для scoped grants, attenuation и revocation; [#26](https://github.com/lifeFedorovAlexey/KOLVRT/issues/26) — следующий IPC gate.

---

## Versioned Routing & Translation

Долгосрочная модель — **не**:

```text
SYSTEM = NATIVE
```

или:

```text
SYSTEM = COMPAT
```

Routing должен иметь точную область действия.

Например:

```text
Browser
├── memory          → native
├── filesystem      → native
├── networking      → native
└── old_sync_api    → compat-v2

Database
└── everything      → native

Old driver
└── device API      → compat-v1

New driver
└── device API      → native
```

Native и compatibility consumers могут сосуществовать.

Совместимость не становится свойством всей операционной системы.

---

## Совместимость имеет стоимость. Измеряйте её

KOLVRT не намерен искусственно замедлять совместимость.

Это было бы обманом.

Diagnostic builds должны показывать **настоящую стоимость** translation. Следующие layout и числа служат лишь иллюстрацией; это не результаты измерений:

```text
Component: example-driver

Route: COMPAT v2

Calls                  1,842,991
Translations             291,440
Extra copies              18,202

                COMPAT       NATIVE
median latency   14.2 µs      9.1 µs
p99              31.8 µs     19.7 µs
CPU               3.8 %       2.9 %
memory           18.4 MB      14.1 MB
```

Если native быстрее, разработчик видит, что даёт миграция.

Если compatibility быстрее, **это повод исследовать performance bug native path**.

Без искусственных штрафов.

Без маркетинговых benchmarks.

Без сокрытия неудобных чисел.

### Migration advisor

Read-only [migration advisor](docs/architecture/migration-advisor.md) работает как host subsystem: описывает capability-based dependency alternatives, проверяет evidence и contract receipts, анализирует парные измерения и готовит rollback proposals. Отдельный signed authorization gate реализован, но он не устанавливает packages и не меняет routes. Production catalog/installed-state collection, полный OS telemetry, настоящие contract/rollback executors, custody production keys, validated dependence/power analysis и physical ARM64 A/B evidence остаются открытыми по [issue #14](https://github.com/lifeFedorovAlexey/KOLVRT/issues/14).

---

## DEV и PROD решают разные задачи

KOLVRT проектируется с двумя execution profiles.

### DEV / DIAGNOSTIC

Для исследования:

- проверки invariants
- подробные свидетельства panic
- tracing
- учёт compatibility
- исследование routes
- A/B measurements
- fault injection
- аудит unsafe boundaries
- performance counters

### PROD

Для исполнения уже проверенной configuration:

- release optimization
- без экспериментального route switching
- без ненужной diagnostic instrumentation
- только необходимые compatibility modules
- предопределённый routing
- минимальный runtime overhead

Это **не отдельные ядра**.

Одна архитектура должна работать в обоих режимах.

---

## Linux — источник исследования, а не религия

KOLVRT ведёт структурированную базу реальных механизмов отказа ОС.

Проект изучает:

- регрессии Linux kernel
- ограничения ABI
- историческое поведение, ставшее требованием совместимости
- нарушения memory safety
- ошибки concurrency
- сложность driver model
- hardware quirks
- security fixes
- находки syzkaller / syzbot
- устаревшие и выведенные из использования interfaces
- проектные решения других ОС

Каждый интересный случай должен в итоге ответить:

```text
Что произошло?
Почему это произошло?
Было ли это ошибкой?
Какие ограничения существовали тогда?
Сохраняются ли эти ограничения?
Что сделал бы KOLVRT?
Относится ли это к native behavior?
Относится ли это к compatibility?
Нужно ли это вообще?
```

Выживание свидетельствует, что что-то работало.

Это **не свидетельство того, что это нужно копировать**.

---

## Законы ядра

Архитектура KOLVRT ограничена явными **Kernel Laws**.

Они не дают архитектуре постепенно деградировать до набора разумных исключений.

Примеры принципов:

```text
Native semantics не должны зависеть от compatibility semantics.

Compatibility должна удаляться без нарушения native execution.

Unsafe code должен иметь явный invariant.

Наблюдаемое legacy behavior не становится native specification автоматически.

Hardware quirks не должны молча становиться общей архитектурой.

Compatibility requirement должен назвать своего consumer.

Заявления performance требуют измерений.
```

Полные нормативные правила находятся здесь:

[`docs/architecture/kernel-laws.md`](docs/architecture/kernel-laws.md)

---

## Unsafe требует объяснения

Разработка ядра неизбежно пересекает границы, которые Rust не может доказать.

KOLVRT не утверждает обратного.

Каждая необходимая `unsafe` boundary должна ответить:

```text
Почему здесь нужен unsafe?
Какой invariant обеспечивает корректность?
Кто устанавливает этот invariant?
Кто может его нарушить?
Как это проверяется?
```

Проект генерирует unsafe inventory в составе kernel verification.

См.:

[`docs/kernel/unsafe.md`](docs/kernel/unsafe.md)

---

## Быстрый старт

### Требования

Текущая основа разработки:

- Rust **1.99.0**
- target `aarch64-unknown-none`
- `rustfmt`
- `clippy`
- Node.js **18+**
- QEMU **10.1.0**
- 7-Zip на Windows для автоматической настройки QEMU

### Настройка Windows

```powershell
rustup toolchain install 1.99.0 --profile minimal --component rustfmt --component clippy
rustup target add aarch64-unknown-none --toolchain 1.99.0

./scripts/setup-qemu.ps1

npm ci --ignore-scripts
npm run check
cargo xtask test
```

### Запуск матрицы проверок

```bash
cargo xtask test
```

Test command собирает и запускает настоящие AArch64 kernel images в QEMU.

Отсутствие event, panic, fatal exception, ошибка emulator или timeout завершают host command ошибкой.

### Отладка с GDB

```bash
cargo xtask debug
```

Затем:

```text
aarch64-none-elf-gdb target/kernel/dev-boot.elf

(gdb) target remote 127.0.0.1:1234
(gdb) break kernel_main
(gdb) continue
(gdb) info registers
(gdb) bt
```

---

## Тесты должны обнаруживать ошибки ядра

Последняя сохранённая exact-source интеграционная матрица фиксирует **96 kernel checks в каждом профиле DEV и PROD** и 80 отрицательных host controls. Она включает transfer process-local handles, ограниченные пути preemption/event-wait, ELF loader и ASID lifecycle. [Отчёт](../../research/measurements/runs/1791130278634-phase3-4-revocation-integrated-f3be261c515b.json) связывает исходники, artifacts и scope QEMU TCG; он не доказывает поведение на физическом ARM64 или более поздних изменениях исходников. Исторические milestone counts сохранены в исходных decision records.

Она также выполняет negative controls, которые обязаны корректно завершаться ошибкой.

Это различие существенно.

Зелёный test suite, не обнаруживающий намеренно внесённую ошибку, — украшение.

Tests KOLVRT проверяют настоящее kernel behavior: allocation, mapping, memory access, unmapping, freeing, exception paths и timer delivery.

Генерируемые artifacts включают:

- ELF images
- SHA-256 hashes
- UART logs
- structured test events
- аргументы и версию QEMU
- отчёты build size
- отчёты features
- unsafe inventory
- measurement samples

---

## Структура репозитория

```text
.
├── crates/          Rust implementation
├── docs/            architecture and kernel documentation
├── research/        source-backed OS research and measurements
├── schemas/         machine-readable research schemas
├── scripts/         development/bootstrap tooling
├── translations/    translated project documentation
└── assets/          project branding
```

Начните здесь:

- [`Vision`](docs/vision.md)
- [`Kernel Laws`](docs/architecture/kernel-laws.md)
- [`Native model`](docs/architecture/native-model.md)
- [`Compatibility model`](docs/architecture/compatibility-model.md)
- [`Routing model`](docs/architecture/routing-model.md)
- [`Kernel boot`](docs/kernel/boot.md)
- [`Testing`](docs/kernel/testing.md)
- [`SMP boundary`](docs/kernel/smp.md)
- [`Unsafe boundaries`](docs/kernel/unsafe.md)
- [`Architecture decisions`](docs/architecture-decisions/)
- [`Research`](../../research/)

Русская документация:

[`translations/ru/`](.)

---

## Участие в разработке

KOLVRT находится на раннем этапе.

Архитектурные изменения приветствуются.

Бездоказательные архитектурные заявления — нет.

Предлагая исключение совместимости на уровне kernel, ответьте:

1. Кому нужно это поведение?
2. Почему оно не может находиться вне native core?
3. Каков lifetime исключения?
4. Как измеряется его использование?
5. Как оно удаляется?
6. Что защищает native consumers от его стоимости?

Правила formatting, checks и участия описаны здесь:

[`CONTRIBUTING.md`](CONTRIBUTING.md)

---

## Экспериментальность означает экспериментальность

KOLVRT — активно разрабатываемый исследовательский проект ОС.

Это ещё не general-purpose production ОС.

Stable userspace ABI пока отсутствует.

Обещания Linux compatibility пока нет.

Широкая hardware support пока не заявляется.

Заявления должны следовать свидетельствам.

Нереализованные возможности должны быть названы в документации.

Неизмеренные возможности не должны иметь benchmark number.

Без доказательства SMP safety возможность нельзя называть SMP-safe.

---

## Лицензия

Лицензия проекта ещё не выбрана.

Сторонние материалы сохраняют свои лицензии.

---

## Итоговое правило

> **Совместимость разрешена. Legacy изолировано. Native остаётся чистым.**

Или на менее формальном языке проекта:

> **Не плюй в ядро — сам из него пить будешь.**

[Контракт фундамента EL0](docs/kernel/el0.md) фиксирует ownership, retirement, tests и ограничения; [ADR-0014](docs/architecture-decisions/0014-el0-foundation.md) рассматривает механизмы.

В [контракте Phase 2](docs/kernel/routing.md) описаны настоящие EL0 routes, profile validation, measurements, authority boundary и limits. Полный IPC/service native slice остаётся незавершённым.

[English source](../../README.md)

## Safe user-copy в Phase 3.2

[Граница user-copy](docs/kernel/user-copy.md) поддерживает bounded current-process byte copies, immutable input snapshots и precise fault recovery на обоих CPU. Её DEV/PROD suite содержит 69 checks и 57 failure controls. [ADR-0018](docs/architecture-decisions/0018-safe-user-copy.md) сохраняет синхронное исключение mapping/lifetime races и отделяет memory validity от authority.

## Локальные handles процессов в Phase 3.3

[Handles](docs/kernel/handles.md) дают bounded caller-local opaque references, generation/type/live/rights checks, receiver-local transfer с rights attenuation, retained Event targets и deterministic exit/fault cleanup. EL0 transfer сейчас адресует namespace на том же CPU; cross-CPU delegation подготавливает coordinator, а затем тест проверяет конкурентные EL0 close/lookup. [ADR-0019](docs/architecture-decisions/0019-process-local-handles.md) фиксирует исходное решение об identity/lifetime; [ADR-0020](docs/architecture-decisions/0020-handle-transfer-and-retention.md) описывает transfer и retention. Исходная Phase 3.3 [матрица](../../research/results/kernel-phase33.json) фиксирует 72 checks; exact-source [матрица issue #23](../../research/measurements/runs/1791022558822-issue23-transfer-bf3f9b298688.json) фиксирует 73 DEV/PROD checks и 69 negative controls. [Phase 3.4](docs/kernel/domains.md) добавляет scoped grants и domains; общий IPC остаётся будущей работой.

## Канонические статусы функций

Этот производный обзор пилота показывает границы реализации независимо от порядка дорожной карты. Перед утверждением о проверке прочитайте канонический контракт и границы исторической записи.

[Карта документации](docs/index.md)

<!-- feature-summary:start -->

| Каноническая функция                                                                                       | Реализация          | Граница доказательств                                  |
| ---------------------------------------------------------------------------------------------------------- | ------------------- | ------------------------------------------------------ |
| [kolvrt.docs.navigation](docs/knowledge-system.md#kolvrt-docs-navigation)                                  | BOUNDED_IMPLEMENTED | host-process: VERIFIED; physical-arm64: NOT_APPLICABLE |
| [kolvrt.handles.local](docs/kernel/handles.md#kolvrt-handles-local)                                        | BOUNDED_IMPLEMENTED | qemu-arm64: VERIFIED; physical-arm64: UNKNOWN          |
| [kolvrt.memory.user-copy](docs/kernel/user-copy.md#kolvrt-memory-user-copy)                                | BOUNDED_IMPLEMENTED | qemu-arm64: VERIFIED; physical-arm64: UNKNOWN          |
| [kolvrt.process.lifecycle](docs/kernel/processes.md#kolvrt-process-lifecycle)                              | BOUNDED_IMPLEMENTED | qemu-arm64: VERIFIED; physical-arm64: UNKNOWN          |
| [kolvrt.security.capability-revocation](docs/kernel/capabilities.md#kolvrt-security-capability-revocation) | PLANNED             | qemu-arm64: UNKNOWN; physical-arm64: UNKNOWN           |
| [kolvrt.security.event-revocation](docs/kernel/capabilities.md#kolvrt-security-event-revocation)           | BOUNDED_IMPLEMENTED | qemu-arm64: VERIFIED; physical-arm64: UNKNOWN          |

<!-- feature-summary:end -->
