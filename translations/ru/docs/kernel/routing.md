# Контракт исполнения routing Phase 2

Document status: CURRENT
Document scope: bounded optional EL0 routing на проверенном two-CPU QEMU foundation; native IPC/services остаются незавершёнными.
Status reference: [Admission и evidence](../architecture-decisions/0015-el0-versioned-routing.md)

Ограниченный Versioned Routing & Translation slice исполняется в восьми настоящих EL0-процессах на двух CPU. Разные consumers одновременно выбирают native, inclusive-v1, counted-v2 и безопасный synthetic bug adapter. Native kernel зависит только от kernel-core и не декодирует legacy formats, не выбирает compatibility versions и не вызывает adapters. [ADR-0015](../architecture-decisions/0015-el0-versioned-routing.md) фиксирует placement, ownership, security и limits.

## Архитектура и границы репозитория

```text
xtask: validate profile → build selected user image → validate ELF → select opaque boot payload
CPU0 supervisor: create private roots/stacks → release admission
CPU0 EL0 consumers: native / inclusive-v1 / counted-v2 / empty-first bug
CPU1 EL0 consumers: native / inclusive-v1 / counted-v2 / empty-first bug
                    ↓ private route + synchronous transaction
                    ↓ adapter converts to current native half-open Span
                    ↓ native SVC; current runqueue supplies task identity
EL1: own execution-accounting snapshot → checked window read → initialized register reply
EL0: shared native window reduction → sum/word count
Both CPUs: native root + completed TLBI → release Done
CPU0: acquire both Done → inspect bounded reports → drop charges → reclaim frames
```

```text
crates/kernel-core/src/execution.rs     native experimental operation contract
crates/kernel-core/src/window.rs        native checked algorithm and Backend boundary
crates/kernel/src/execution.rs          protected own observations and checked register reply
crates/kernel/src/scheduler/mod.rs          current-task attribution, SVC, admission
crates/kernel/src/memory/mod.rs         retained immutable RX image and guarded stacks
crates/routing/src/lib.rs               profile, versions, private binding and transactions
crates/routing/src/conformance.rs       behavior and rejection fixtures
crates/window-compat/src/lib.rs         optional legacy conversion and safe bug behavior
crates/routing-demo/                    separately linked no_std EL0 image
crates/xtask/src/routing_demo.rs         build, inspection, measurement and validation
```

```text
native kernel → kernel-core
EL0 demo → routing → kernel-core
                  → window-compat [optional, feature-selected] → kernel-core
host xtask → routing + kernel-core + evidence tools
native kernel ↛ routing or window-compat
```

Profile с четырьмя entries — consumer template, независимо создаваемый на каждом CPU. Runtime consumer identity определяется native task ID, а не template slot или caller-supplied selector. Каждый instance владеет отдельными Rust Consumer и stack. Global mutable route table отсутствует.

## Native capability и authority

Timer handling записывает bounded own-task service intervals в u32 timer ticks с насыщением на u32::MAX. Capture один раз фиксирует восемь observations; повторный capture не заменяет snapshot. Ядро принимает проверенные u32 half-open endpoints и возвращает максимум восемь собственных observations в инициализированных регистрах; общий native reduction вычисляет sum/word count в EL0. User memory не dereference, foreign owner не принимается, raw root и privileged address не возвращаются. Empty native spans возвращают zero words; invalid widths, order и bounds отклоняются до чтения данных.

Только privileged code может подтверждать kernel-owned scheduling observations и устанавливать текущий task. EL0 выполняет translation и route policy. Bootstrap предоставляет каждому static task доступ только к собственной observation capability на его lifetime. Это bounded native authority contract, а не общая handles/capabilities или security-domain implementation. Adapters не могут его расширить. Все routes вызывают один backend; unsupported encoding и native rejection не включают fallback.

Каждый adapter использует один immutable owned record snapshot. v1 преобразует LE16 inclusive endpoints, включая all-ones empty sentinel; v2 преобразует checked BE32 start/count. Opt-in bug adapter отображает zero count в одно собственное word. Controlled demonstration отличается от исправленного empty native result без обхода bounds или раскрытия данных другого task. Сохраняется безопасное synthetic behavior, а не реальная историческая vulnerability.

## Владение, переключение и retirement

| State                           | Owner и mutability                                                 | Publication/lifetime rule                                                                                      |
| ------------------------------- | ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| Validated profile и user code   | Trusted build/boot selection, immutable                            | Exact image/profile hashes сохранены; loader и runtime code replacement отсутствуют                            |
| Binding и route counters        | Один EL0 Consumer, exclusive mutable borrow                        | Shared references, IRQ access и CPU0 writer отсутствуют; разные consumers исполняются на разных CPU            |
| Transaction и generation        | Private consumer, synchronous backend calls                        | Guard удерживает binding; DEV switch только после завершения; забытый guard блокирует admission; wrap запрещён |
| Native history и frozen capture | CPU текущего task, IRQ masked                                      | Caller-selected owner отсутствует; captured values immutable; Rust reference не переживает ERET                |
| Queue reset и completion        | CPU0 готовит; каждый CPU выполняет CAS claim своего admitted batch | Idle → Admitted → Running → Done; acquire обоих terminal phases до reset; stale poll не получает idle state    |
| Tables, image pages и stacks    | CPU0 Frame/UserSpace ownership; процесс использует private stack   | Charges переживают забытые guards; оба CPU восстанавливают native root и завершают TLBI до reclamation         |

State-free synchronous reduction допускает DEV transaction-boundary switching. State migration, live module unloading, shared-object rebinding и experimental PROD switching отсутствуют. Stateful capability требует собственного transition proof или restart boundary. Четыре stack pages с unmapped guard и максимум 128 KiB RX payload — явные bootstrap limits. Timer quantum остаётся typed native scheduler setting; payload execution ограничен slice/deadline admission. IRQ allocation и новые queue/global routing locks не используются.

## DEV API и production profiles

```text
cargo xtask routing test
cargo xtask routing status
cargo xtask routing inspect 1
cargo xtask routing compare 1
cargo xtask routing top
cargo xtask routing profile native,inclusive,counted,bug target/routing-profile.bin
cargo xtask routing validate target/routing-profile.bin EXPECTED_SHA256
```

Inspection читает сохранённые test observations; это не удалённо доступный privileged inspector. Consumer::switch и Transaction::try_switch — настоящий DEV API, проверенный в EL0. Text labels понятны без цвета. LEGACY/COMPAT/MIXED/MOSTLY_NATIVE/NATIVE classification сохраняет проверенную multi-metric model; выдуманный scalar score не добавляется. Нулевой admission denominator остаётся unknown. Benchmark admissions показаны отдельно от bound consumer traffic; native backend calls не являются повторным external native admission.

Profiles кодируют schema/native version, nonzero generation, explicit consumer entries и implementation source identities. Отдельно собираемый image настраивается KOLVRT_ROUTING_PROFILE и независимо доверенным KOLVRT_ROUTING_DIGEST. Build и EL0 отклоняют integrity, identity, unsupported module и version errors. Digest не является signature или operator authorization; build/boot authenticity не реализована. Exact executable hashes отделены от semantic versions и source identities.

PROD содержит только feature-selected adapters и pinned profile. Rebind API, route experimentation и DEV counters отсутствуют. Evidence images явно добавляют conformance и bounded reporting; stripped image исключает эти fixtures и A/B code. Без machine-events user REPORT collection и его storage исключаются из native kernel при компиляции. Correctness checks, authority, bounds, page protection и lifetime сохраняются. Runtime module unload не заявлен; unused code удаляется на validated rebuild boundary.

## Измерения и проверки

[Routing results](../../../../research/results/routing-phase2.json) сохраняют user/kernel artifact hashes, profile bytes/digests, actual events, native oracle, raw marked warmup и measurement samples. Шесть evidence configurations покрывают DEV/full, PROD/full, PROD/v1, PROD/v2, PROD/bug и PROD/native. Отдельный stripped PROD/v1 boot исключает diagnostic features. Три controls обнаруживают corrupted profile, contained adapter fault и corrupted accounting. [Native results](../../../../research/results/kernel-phase2.json) сохраняют неизменную 53-test DEV/PROD foundation matrix и одиннадцать прежних controls. [Unsafe inventory](../../../../research/results/kernel-phase2-unsafe-audit.json) — перечень, а не доказательство.

Каждый benchmark использует одни frozen native data и equivalent nonempty useful result для всех routes. Сохранены шестнадцать warmup samples и 128 measured samples на route. Разные consumers чередуют forward/reverse route order. Clock SVC boundary одинаков; raw latency включает privilege transitions и preemption. black_box применяется симметрично к inputs, чтобы fixed fixture не исчезла при компиляции. Source-boundary copies/conversions учитывают исполненные logical translation work, а не physical memory-copy instructions. Allocation и locking на этих bounded call paths отсутствуют. Per-run timer preemptions записаны; exclusive CPU attribution и saturation throughput недоступны.

Nearest-rank median/p95/p99, mean и sample variance описывают эти observations. Warmup фиксирован и не доказывает stabilization; восемь concurrent fixtures не являются independent hardware trials. Tail confidence и fastest-path claim остаются inconclusive. Более быстрый compatibility median сохраняется для native-path investigation без artificial penalty. Emulator observations не устанавливают physical hardware throughput, side-channel protection или hard real-time progress.

Native-only builds исполняют ту же foundation suite. Отдельная pruned-workspace проверка собирает систему без routing/adapter source packages. Cargo dependency closure проверяет все native features; source guards запрещают известные compatibility imports, legacy input types и branches. Negative fixtures проверяют эти guards, malformed ELF bounds, missing/duplicate/foreign reports, изменение каждого profile byte, provider denial и отсутствие PROD switching. Generated code и произвольные будущие identifiers всё ещё требуют review.

[Доказательства физического удаления исходников](../../../../research/results/native-compat-removal.json) подтверждают 35 одинаковых native/kernel-core/harness source files после удаления пакетов routing, window-compat и routing-demo; изменены только workspace/tool dependency composition и lockfile. Те же 53 теста в каждом профиле и одиннадцать host controls проходят. [Performance investigation #15](https://github.com/lifeFedorovAlexey/KOLVRT/issues/15) отслеживает восемь наблюдений с меньшей compatibility-медианой в сохранённом DEV run; неопределённость и следующие проверки записаны в [investigation record](../../../../research/results/routing-performance-investigation.json). Это не устанавливает более быстрый hardware path.

## Сохранённая groundwork и следующие кандидаты

Сохранённая pure model: Route, semantic names, Profile schema/version/digest, immutable configuration, dependency rules, status definitions и native Span/Reduction. Сохранённые runtime candidates: Consumer/Transaction, dispatch и adapter conversion. Их backend boundary теперь вызывает actual native capability из EL0; local providers остаются conformance fixtures. Полезная groundwork не откатывалась. Прежнее ограничение «no multi-CPU access» заменено exclusive per-consumer ownership при concurrent execution разных consumers. Прежние one-page image/stack и single boot session заменены bounded retained payloads и CAS admission.

Native → compatibility hook не подключён. Routing runtime подключён вне native core. IPC, handles/capabilities, cancellation, services, security domains, arbitrary loaders, shared state-domain migration, signatures и dynamic unloading остаются дальнейшей работой. Кандидаты Phase 3: native IPC/authority slice, затем конкретный service consumer; device/DMA и Linux personalities требуют самостоятельных contracts и evidence. Демонстрация не устанавливает global production security policy.

Более поздний [scheduler Phase 3.0](scheduler.md) разделяет runtime, architectural context и bootstrap verification, добавляя checked generations и per-CPU storage permits. Adapter binding и native observation semantics не меняются. [Regression results](../../../../research/results/routing-phase3-regression.json) относятся к текущим scheduler sources; первоначальные измерения Phase 2 выше остаются историческими. Native foundation теперь также проверяет повторное использование queue без reboot.

[Английский оригинал](../../../../docs/kernel/routing.md)
