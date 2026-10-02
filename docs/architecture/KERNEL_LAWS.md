# KERNEL LAWS — v0.1

Статус: нормативные требования к будущему KOLVRT. Приняты для Phase 0; runtime ещё не реализован.
Historical evidence — конкретные случаи, а выводы и enforcement — требования KOLVRT, не заявления об уже выполненных tests.
Исключение не может отменять memory safety/authorization. Изменение закона требует нового ADR с контрпримером и тестом.

## LAW-001 — Native contract is authoritative

**Rule:** Native operation не ветвится по legacy consumer/version.

**Rationale:** Иначе внешняя история становится неустранимой внутренней зависимостью.

**Historical evidence:** [KOL-PATH-0006](../../research/pathology/KOL-PATH-0006.json), [KOL-PATH-0028](../../research/pathology/KOL-PATH-0028.json).

**Prevents:** Скрытые legacy special cases в native core.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Review API surface и dependency graph.

**Testing:** Один native request даёт одинаковый outcome при разных manifests.

## LAW-002 — Compatibility is a dependency direction

**Rule:** Core не импортирует compat crates, types, generated layouts или features.

**Rationale:** Quarantine требует проверяемой границы.

**Historical evidence:** [KOL-PATH-0006](../../research/pathology/KOL-PATH-0006.json), [KOL-PATH-0028](../../research/pathology/KOL-PATH-0028.json).

**Prevents:** Невозможность собрать native без compatibility dependencies.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Cargo graph и generated-code audit в будущем CI.

**Testing:** Native-only build и contract suite без adapters.

## LAW-003 — No global compatibility mode

**Rule:** Binding выбирается по consumer/family/state domain, не одним global flag.

**Rationale:** Сосуществование интерфейсов требует локального выбора.

**Historical evidence:** [KOL-PATH-0002](../../research/pathology/KOL-PATH-0002.json), [KOL-PATH-0020](../../research/pathology/KOL-PATH-0020.json).

**Prevents:** Взаимное исключение native и legacy consumers на одной системе.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Resolver schema и review config.

**Testing:** A mixed, B native, C personality работают одновременно.

## LAW-004 — Bind once, fail explicitly

**Rule:** Bind закрепляет version/digest; unsupported и ambiguous routes возвращают ошибку.

**Rationale:** Молчаливый fallback меняет контракт.

**Historical evidence:** [KOL-PATH-0020](../../research/pathology/KOL-PATH-0020.json), [KOL-PATH-0023](../../research/pathology/KOL-PATH-0023.json).

**Prevents:** Неявную смену semantics при обновлении module registry.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Resolver проверяет dependency closure до launch.

**Testing:** Missing module и два кандидата не приводят к запуску.

## LAW-005 — State determines routing granularity

**Rule:** Все операции над общей state machine используют согласованный binding.

**Rationale:** Close, dup и locks невозможно произвольно разделить.

**Historical evidence:** [KOL-PATH-0001](../../research/pathology/KOL-PATH-0001.json), [KOL-PATH-0002](../../research/pathology/KOL-PATH-0002.json), [KOL-PATH-0003](../../research/pathology/KOL-PATH-0003.json).

**Prevents:** Разные представления ownership одного объекта.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Каждая family объявляет state domain.

**Testing:** Split close/lock и wait/wake отклоняется.

## LAW-006 — Quiescence before switching

**Rule:** Rebind требует admission stop, drain, conversion и atomic generation commit.

**Rationale:** Замена кода не меняет безопасно уже опубликованное состояние.

**Historical evidence:** [KOL-PATH-0003](../../research/pathology/KOL-PATH-0003.json), [KOL-PATH-0026](../../research/pathology/KOL-PATH-0026.json), [KOL-PATH-0030](../../research/pathology/KOL-PATH-0030.json).

**Prevents:** Выполнение половины протокола на старом и половины на новом backend.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Migration state machine review.

**Testing:** Rebind с pending waiter отказывает или drains; timeout сохраняет старый route.

## LAW-007 — No live-reference unload

**Rule:** Module не удаляется при bound consumers, callbacks или in-flight state.

**Rationale:** Удалённый код не должен оставаться целью completion.

**Historical evidence:** [KOL-PATH-0022](../../research/pathology/KOL-PATH-0022.json), [KOL-PATH-0030](../../research/pathology/KOL-PATH-0030.json).

**Prevents:** Вызов кода выгруженного adapter.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Manifest + reference/grace accounting.

**Testing:** Concurrent unload/completion не вызывает use-after-free.

## LAW-008 — Fix native bugs first

**Rule:** Native fix сопровождается regression test; старое поведение получает отдельный bug-compat ID только при нужде.

**Rationale:** Без этого исправление превращается в постоянную ветку.

**Historical evidence:** [KOL-PATH-0007](../../research/pathology/KOL-PATH-0007.json), [KOL-PATH-0008](../../research/pathology/KOL-PATH-0008.json), [KOL-PATH-0030](../../research/pathology/KOL-PATH-0030.json).

**Prevents:** Вечное сохранение ошибочной native semantics.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** ADR и consumer evidence перед bug-compat.

**Testing:** Fixture проходит исправленный native и явно выбранную безопасную старую semantics.

## LAW-009 — Security is not negotiable compatibility

**Rule:** Adapter не ослабляет native permissions, memory protection и quotas.

**Rationale:** Изоляцию нельзя продавать как версию поведения.

**Historical evidence:** [KOL-PATH-0007](../../research/pathology/KOL-PATH-0007.json), [KOL-PATH-0008](../../research/pathology/KOL-PATH-0008.json), [KOL-PATH-0017](../../research/pathology/KOL-PATH-0017.json), [KOL-PATH-0018](../../research/pathology/KOL-PATH-0018.json).

**Prevents:** Повторное введение vulnerability как compatibility feature.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Security review всех required grants.

**Testing:** Враждебный legacy request не повышает права.

## LAW-010 — Version behavior independently of code

**Rule:** Semantic version и implementation digest хранятся отдельно.

**Rationale:** Security fix может сохранять контракт при новом artifact.

**Historical evidence:** [KOL-PATH-0008](../../research/pathology/KOL-PATH-0008.json), [KOL-PATH-0020](../../research/pathology/KOL-PATH-0020.json).

**Prevents:** Смешение protocol compatibility и identity исполняемого artifact.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Signed/pinned release manifest design.

**Testing:** Замена digest не выдаётся за новую semantics и наоборот.

## LAW-011 — Typed extensible wire format

**Rule:** Public payload имеет size/version, fixed widths, byte order и checked reserved fields.

**Rationale:** Host language layout не является устойчивым ABI.

**Historical evidence:** [KOL-PATH-0005](../../research/pathology/KOL-PATH-0005.json), [KOL-PATH-0006](../../research/pathology/KOL-PATH-0006.json), [KOL-PATH-0020](../../research/pathology/KOL-PATH-0020.json).

**Prevents:** ABI drift, padding leaks и неоднозначный decoding.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Codec review и schema conformance.

**Testing:** Fuzz truncated, oversized, endian и unknown-required fields.

## LAW-012 — Time carries a clock domain

**Rule:** Deadline содержит clock domain и checked 64-bit representation; conversion не truncates.

**Rationale:** Ширина времени и выбор clock — разные гарантии.

**Historical evidence:** [KOL-PATH-0005](../../research/pathology/KOL-PATH-0005.json), [KOL-PATH-0026](../../research/pathology/KOL-PATH-0026.json).

**Prevents:** Time overflow и ожидание по неправильным часам.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** API type review.

**Testing:** 2038, overflow, realtime jumps и monotonic deadlines.

## LAW-013 — Object identity is not an integer name

**Rule:** Authority operations используют handle identity; recycled display ID не достаточен.

**Rationale:** Повторный lookup может обратиться к другому объекту.

**Historical evidence:** [KOL-PATH-0001](../../research/pathology/KOL-PATH-0001.json), [KOL-PATH-0019](../../research/pathology/KOL-PATH-0019.json).

**Prevents:** Ошибочное действие над переиспользованным PID/fd.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Handle table design review.

**Testing:** Exit/reuse/signal и fd reuse не меняют адресата.

## LAW-014 — Close consumes once

**Rule:** Release outcome отделён от I/O result; повтор close не нужен для определения ownership.

**Rationale:** Ошибка completion не должна создавать неопределённый lifetime.

**Historical evidence:** [KOL-PATH-0001](../../research/pathology/KOL-PATH-0001.json).

**Prevents:** Двойное закрытие нового объекта после поздней ошибки старого.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Consuming API и result type.

**Testing:** Поздняя EIO сохраняется без повторного закрытия.

## LAW-015 — Explicit lock ownership

**Rule:** Native lock имеет owner token; unrelated close не снимает его.

**Rationale:** Implicit process-wide ownership усложняет локальное рассуждение.

**Historical evidence:** [KOL-PATH-0002](../../research/pathology/KOL-PATH-0002.json).

**Prevents:** Снятие блокировки побочным close в библиотеке.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Lock arbiter contract.

**Testing:** Другой fd того же файла не снимает lock.

## LAW-016 — Wait registration has lifetime

**Rule:** Registration token имеет cancel/drain и generation; stale events не переадресуются.

**Rationale:** Subscription и номер fd имеют разные lifetimes.

**Historical evidence:** [KOL-PATH-0003](../../research/pathology/KOL-PATH-0003.json), [KOL-PATH-0030](../../research/pathology/KOL-PATH-0030.json).

**Prevents:** Доставку stale event новому объекту.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Event contract review.

**Testing:** dup/close/cancel/reuse interleavings.

## LAW-017 — No resurrection

**Rule:** Weak upgrade не создаёт strong reference из Retiring/Dead.

**Rationale:** Наличие pointer под RCU не означает живой объект.

**Historical evidence:** [KOL-PATH-0030](../../research/pathology/KOL-PATH-0030.json).

**Prevents:** Воскрешение объекта после начала destruction.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Refcount/epoch wrapper safety proof.

**Testing:** Model-check last-drop versus upgrade.

## LAW-018 — Initialize before publish

**Rule:** Все observable fields и bytes задаются до публикации и после reuse.

**Rationale:** Старые metadata могут изменить permissions новой страницы.

**Historical evidence:** [KOL-PATH-0006](../../research/pathology/KOL-PATH-0006.json), [KOL-PATH-0008](../../research/pathology/KOL-PATH-0008.json).

**Prevents:** Использование старых flags или kernel stack bytes новым объектом.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Constructor-only publication; unsafe audit.

**Testing:** Poisoned allocator и recycled buffer fixtures.

## LAW-019 — COW is not authorization

**Rule:** COW state не снимает write permission checks и не смешивается с request rights.

**Rationale:** Fault retry не должен терять начальный смысл операции.

**Historical evidence:** [KOL-PATH-0007](../../research/pathology/KOL-PATH-0007.json).

**Prevents:** Обход write permissions через COW retry.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** MM state transition review.

**Testing:** Concurrent COW/unmap/write и software dirty backend.

## LAW-020 — Pinning is a lease

**Rule:** DMA pin отличается от CPU reference, имеет owner, quota и termination contract.

**Rationale:** Free/migration не могут игнорировать внешнего writer.

**Historical evidence:** [KOL-PATH-0009](../../research/pathology/KOL-PATH-0009.json).

**Prevents:** Освобождение страницы до завершения device access.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** DmaLease API review.

**Testing:** Device timeout и unmap не освобождают reachable pages.

## LAW-021 — Coherence is not ordering

**Rule:** DMA publication содержит требуемые barriers и cache ownership transitions.

**Rationale:** Coherent allocation не упорядочивает descriptor writes.

**Historical evidence:** [KOL-PATH-0010](../../research/pathology/KOL-PATH-0010.json).

**Prevents:** Чтение устройством частично опубликованного descriptor.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Platform primitive contract + instruction audit.

**Testing:** Weak-memory litmus и noncoherent backend.

## LAW-022 — Zero-copy retains provenance

**Rule:** Page sharing не передаёт право записи без explicit grant.

**Rationale:** Экономия copies не отменяет backing-page ownership.

**Historical evidence:** [KOL-PATH-0008](../../research/pathology/KOL-PATH-0008.json), [KOL-PATH-0009](../../research/pathology/KOL-PATH-0009.json).

**Prevents:** Запись через alias в read-only backing page.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Buffer type/provenance review.

**Testing:** Read-only page нельзя сделать mergeable writable buffer.

## LAW-023 — Validate the executed snapshot

**Rule:** Authorization и execution используют один immutable request либо доказанный pin protocol.

**Rationale:** Mutable foreign pointers допускают TOCTOU.

**Historical evidence:** [KOL-PATH-0018](../../research/pathology/KOL-PATH-0018.json).

**Prevents:** TOCTOU между policy decision и исполнением.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Decoder-to-executor dataflow audit.

**Testing:** Mutation после проверки не меняет выполненную операцию.

## LAW-024 — Delegation is monotonic in rights

**Rule:** Identity mapping и dropping metadata не расширяют native grants.

**Rationale:** POSIX credential изменения не всегда уменьшают доступ.

**Historical evidence:** [KOL-PATH-0017](../../research/pathology/KOL-PATH-0017.json).

**Prevents:** Расширение прав через credential/namespace преобразование.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Capability lattice review.

**Testing:** Nested delegation и restrictive group fixture.

## LAW-025 — Shared resource has one arbiter

**Rule:** Native и compat consumers одного lock/endpoint/resource согласуют ownership через один authority.

**Rationale:** Два независимых ledgers могут выдать конфликтующие разрешения.

**Historical evidence:** [KOL-PATH-0002](../../research/pathology/KOL-PATH-0002.json), [KOL-PATH-0016](../../research/pathology/KOL-PATH-0016.json), [KOL-PATH-0024](../../research/pathology/KOL-PATH-0024.json).

**Prevents:** Два конфликтующих разрешения на один shared resource.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Shared-object gateway review.

**Testing:** Cross-personality conflict даёт один разрешённый owner.

## LAW-026 — Queue topology is not ordering

**Rule:** I/O API явно описывает dependencies, completion и durability.

**Rationale:** Multiqueue completion не обязана следовать submit order.

**Historical evidence:** [KOL-PATH-0015](../../research/pathology/KOL-PATH-0015.json).

**Prevents:** Ложную durability и зависимости от случайного completion order.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Block protocol conformance.

**Testing:** Reordered completion и flush failure fixtures.

## LAW-027 — Scheduling policy is visible

**Rule:** Priority всегда сопровождается domain; group и task weights различны.

**Rationale:** Implicit grouping меняет ожидания от nice.

**Historical evidence:** [KOL-PATH-0021](../../research/pathology/KOL-PATH-0021.json).

**Prevents:** Необъяснимую смену fairness из-за скрытой группировки.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Diagnostics и scheduler API review.

**Testing:** Один workload в разных domains имеет объяснимые shares.

## LAW-028 — Reclamation waits for readers

**Rule:** Removed shared object не освобождается до доказанного завершения readers.

**Rationale:** Removal и reclamation — разные события.

**Historical evidence:** [KOL-PATH-0022](../../research/pathology/KOL-PATH-0022.json).

**Prevents:** Use-after-free у reader старой версии.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Guard/lifetime proof и bounded backlog policy.

**Testing:** Stalled reader, CPU offline и cancellation.

## LAW-029 — Owner death is an outcome

**Rule:** Synchronization сообщает owner death и не обещает восстановление application data.

**Rationale:** Уведомить waiter не значит починить защищаемое состояние.

**Historical evidence:** [KOL-PATH-0025](../../research/pathology/KOL-PATH-0025.json).

**Prevents:** Вечное ожидание погибшего owner и ложное обещание восстановленных данных.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Wait outcome type и recovery protocol review.

**Testing:** Kill owner во всех registration/acquire переходах.

## LAW-030 — Wait-any cannot lose wakeups

**Rule:** Registration/recheck/wake реализуют единый atomic protocol; partial setup откатывается.

**Rationale:** Последовательные waits не эквивалентны multiwait.

**Historical evidence:** [KOL-PATH-0026](../../research/pathology/KOL-PATH-0026.json).

**Prevents:** Lost wakeup и утечку partial registrations.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Concurrency model перед реализацией.

**Testing:** Wake между каждым шагом setup и cancel/deadline race.

## LAW-031 — Hardware workarounds are scoped

**Rule:** Каждый quirk содержит affected IDs/revisions, owner, native invariant и removal condition.

**Rationale:** Workaround нужен hardware domain, а не всем приложениям.

**Historical evidence:** [KOL-PATH-0011](../../research/pathology/KOL-PATH-0011.json), [KOL-PATH-0027](../../research/pathology/KOL-PATH-0027.json).

**Prevents:** Разрастание device-specific обходов по generic core.

**Allowed exceptions:** Machine-wide scope допустим только при evidence о machine-wide fault; workaround может быть обязательным для native target.

**Enforcement:** Platform support manifest audit.

**Testing:** Unaffected device не получает quirk; affected не запускается без него.

## LAW-032 — Firmware input is translated

**Rule:** DT/ACPI input валидируется в frontend и преобразуется в native descriptor.

**Rationale:** Внешний firmware ABI не должен стать core layout.

**Historical evidence:** [KOL-PATH-0012](../../research/pathology/KOL-PATH-0012.json), [KOL-PATH-0013](../../research/pathology/KOL-PATH-0013.json).

**Prevents:** Зависимость core от внешних firmware property names.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Frontend parser schema и core import audit.

**Testing:** Legacy DT fixture и malformed phandle/AML input.

## LAW-033 — Reconnect is not identity proof

**Rule:** Смена power session отзывает generation без доказанного recovery identity.

**Rationale:** Совпадение descriptors не гарантирует тот же device.

**Historical evidence:** [KOL-PATH-0014](../../research/pathology/KOL-PATH-0014.json).

**Prevents:** Передачу старых полномочий новому физическому устройству.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Device lifecycle/security review.

**Testing:** Подмена USB с тем же VID/PID не наследует handles.

## LAW-034 — No stable internal Rust binary ABI promise

**Rule:** Native private types не экспортируются внешним drivers как вечный layout.

**Rationale:** Внутреннее исправление должно оставаться возможным.

**Historical evidence:** [KOL-PATH-0028](../../research/pathology/KOL-PATH-0028.json).

**Prevents:** Фиксацию ошибочных внутренних layouts ради binary drivers.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Public API/FFI surface review.

**Testing:** Old protocol adapter работает без доступа к private layout.

## LAW-035 — Profiles preserve semantics

**Rule:** DEV/STAGING/PROD имеют одинаковые mandatory checks и outcomes.

**Rationale:** Диагностика не является механизмом корректности.

**Historical evidence:** [KOL-PATH-0007](../../research/pathology/KOL-PATH-0007.json), [KOL-PATH-0010](../../research/pathology/KOL-PATH-0010.json), [KOL-PATH-0018](../../research/pathology/KOL-PATH-0018.json).

**Prevents:** Release-only нарушение безопасности после удаления diagnostics.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Один contract suite для всех profiles.

**Testing:** Diagnostics on/off и optimized/unoptimized дают одинаковый oracle.

## LAW-036 — Telemetry states its denominator

**Rule:** Каждая доля указывает scope, window, coverage и attribution; unknown не ноль.

**Rationale:** Fast path и редкий обязательный compat call искажают единый score.

**Historical evidence:** [KOL-PATH-0025](../../research/pathology/KOL-PATH-0025.json), [KOL-PATH-0026](../../research/pathology/KOL-PATH-0026.json).

**Prevents:** Фиктивный native score и двойной учёт nested work.

**Allowed exceptions:** Counters могут отсутствовать в PROD; тогда значение unavailable, не фиктивное измерение.

**Enforcement:** Metrics schema и report review.

**Testing:** Zero calls, disabled counters и dropped events дают unknown.

## LAW-037 — Measure compatibility without bias

**Rule:** Оба paths получают одинаковые safety, limits и workload; native не получает искусственной форы.

**Rationale:** Performance tradeoff должен подтверждаться экспериментом.

**Historical evidence:** [KOL-PATH-0015](../../research/pathology/KOL-PATH-0015.json), [KOL-PATH-0021](../../research/pathology/KOL-PATH-0021.json), [KOL-PATH-0028](../../research/pathology/KOL-PATH-0028.json).

**Prevents:** Предопределённого победителя benchmark.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Predeclared benchmark manifest.

**Testing:** Faster compat result сохраняется в report без штрафов.

## LAW-038 — Report tails and failures

**Rule:** A/B публикует raw runs, warm-up, p95/p99, variance и errors без cherry-picking.

**Rationale:** Среднее скрывает очереди, contention и failure cost.

**Historical evidence:** [KOL-PATH-0015](../../research/pathology/KOL-PATH-0015.json), [KOL-PATH-0021](../../research/pathology/KOL-PATH-0021.json), [KOL-PATH-0026](../../research/pathology/KOL-PATH-0026.json).

**Prevents:** Скрытие tail latency и неуспешных операций.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Benchmark artifact review.

**Testing:** Timeout-heavy fixture не даёт ложный low latency winner.

## LAW-039 — Unsafe needs a local proof obligation

**Rule:** Каждый unsafe имеет SAFETY, invariant, necessity, boundary tests и review.

**Rationale:** Язык не проверяет assumptions устройства или foreign memory.

**Historical evidence:** [KOL-PATH-0008](../../research/pathology/KOL-PATH-0008.json), [KOL-PATH-0010](../../research/pathology/KOL-PATH-0010.json), [KOL-PATH-0022](../../research/pathology/KOL-PATH-0022.json), [KOL-PATH-0030](../../research/pathology/KOL-PATH-0030.json).

**Prevents:** Непроверяемый unsafe surface и неявные hardware assumptions.

**Allowed exceptions:** Boundary unsafe разрешён именно при выполнении unsafe policy; это не разрешение unsafe для удобства.

**Enforcement:** Unsafe inventory и boundary-crate lint.

**Testing:** Negative cases aliasing, initialization, teardown и SMP.

## LAW-040 — Evidence gates architectural claims

**Rule:** Каждый case имеет primary source, decision, native semantics, tests и benchmark requirement.

**Rationale:** Название pathology не делает Linux решение ошибкой.

**Historical evidence:** [KOL-PATH-0007](../../research/pathology/KOL-PATH-0007.json), [KOL-PATH-0028](../../research/pathology/KOL-PATH-0028.json), [KOL-PATH-0029](../../research/pathology/KOL-PATH-0029.json), [KOL-PATH-0030](../../research/pathology/KOL-PATH-0030.json).

**Prevents:** Архитектурные решения на основе лозунгов и выдуманной истории.

**Allowed exceptions:** Нет неявных исключений. Изменение требования — только новым ADR до реализации.

**Enforcement:** Phase 0.1 JSON validator + ручная проверка источников.

**Testing:** Malformed/contradictory records отклоняются; неизвестная история помечается явно.
