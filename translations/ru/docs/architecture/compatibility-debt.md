# Реестр долга совместимости COST-L

Document status: CURRENT
Evidence scope: реализованный ограниченный offline-реестр и исследовательские кандидаты; без поддержки Linux-драйверов и runtime-учёта.
Current reference: [Реализация реестра](../../../../crates/repository-checks/src/cost_l.rs)

## Идентичность и границы

COST-L-#### обозначает конкретную повторяющуюся причину несовместимости, а не отдельный драйвер. COST означает измеримую цену совместимости/технического долга. L намеренно означает Linux и Legacy: Cost of Linux / Legacy, Цена Linux-наследия. Двойной смысл и подходящие русские поговорки — культура проекта; они не заменяют доказательства.

COST-L ID не является kernel API, security capability, package facility, идентификатором адаптера, исключением EXC или случаем KOL-PATH. Эти понятия могут ссылаться друг на друга. Native authority, semantic versions и implementation digests сохраняют значения из [native-модели](native-model.md), [модели совместимости](compatibility-model.md) и [модели routing](routing-model.md).

Native kernel не импортирует сведения реестра или Linux semantics. Translation не расширяет effective authority (LAW-009); недостающее право требует отдельной native authorization. Обязательная аппаратная защита — отдельное измерение по LAW-031. Маршрут совместимости не может быть NATIVE, но исключительно аппаратный workaround сам по себе не делает native software COMPAT.

## Формат и выделение IDs

[Schema записей v1](../../../../schemas/cost-l.schema.json) и [schema выделения IDs v1](../../../../schemas/cost-l-registry.schema.json) используют закрытую JSON Schema Draft 2020-12 и semantic checks. Формат следует conventions репозитория для JSON, duplicate keys и неизвестного provenance; связанные сведения сгруппированы без дублирования плоских счётчиков и универсального графа.

[Журнал выделения IDs](../../../../research/cost-l/registry.json) перечисляет каждый назначенный стабильный ID ровно один раз. Каждому выделению соответствует запись COST-L-####.json, каждой записи — выделение. Нельзя переиспользовать ID или удалять историческую запись. Для добавления записей не требуется числовая квота. Ноль зарезервирован. Согласованность журнала выявляет случайное удаление, но не злонамеренное изменение журнала и истории одновременно; необходимы проверяемая история версий и будущие CI checks истории.

Группы записи содержат identity/status/dates, историю с источниками, native decision, связи module/consumer, costs, support, lifecycle, confidence и open questions. URL источников имеют закреплённые commits или versions и locators. Ссылки KOL-PATH и EXC остаются внешними identities. Category/subsystem — ограниченные расширяемые labels; classification происхождения — проверяемый enum, включая historical constraints и обоснованные hardware/performance/security trade-offs.

Null с объяснением обозначает неизвестное происхождение/owner/observation; неизвестная цена использует state UNKNOWN, null value/provenance, unknown coverage и reason. Неизвестное не означает ноль, false или неприменимость. Пустые module/consumer arrays в research-only записях означают отсутствие заявленной реализации, а не отсутствие пользователей Linux. Выдуманные количества драйверов и metrics запрещены.

Files/receipts ограничены 64 KiB, реестр — 4096 записями, strings — 4096 символами, collections — пределами schema. Это host input limits, не kernel budgets или целевое количество записей. Пределы пересматриваются явно при необходимости реальных данных. Duplicate keys, unknown fields, malformed dates/IDs, неразрешённые ссылки, противоречивые module artifacts и lifecycle/support отклоняются.

## Lifecycle и support

| Переход                           | Обязательство review                                                                                        |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| CANDIDATE → CONFIRMED             | Повторяющаяся причина установлена относительно принятого native difference; сохранить sources и uncertainty |
| CANDIDATE или CONFIRMED → RETIRED | Отклонённый или ненужный исследовательский случай; сохранить историческую identity                          |
| CONFIRMED → ACTIVE                | Named consumers, actual module declarations, owner, конечный support и native migration/removal target      |
| ACTIVE → DEPRECATED               | Проверяемое решение migration/support; без преждевременной отмены обещаний                                  |
| DEPRECATED → ACTIVE               | Явное записанное renewal; successor artifact сам по себе недостаточен                                       |
| DEPRECATED → RETIRING             | Прекратить unsupported admission и разрешить obligations                                                    |
| RETIRING → DEPRECATED             | Записать заблокированное retirement и пересмотренный support disposition                                    |
| RETIRING → RETIRED                | Не осталось supported obligations; сохранить исторические implementation/evidence records                   |

Каждое событие имеет дату и decision. События упорядочены, начинаются с исходного candidate и заканчиваются текущим объявленным state; неподдержанные jumps и воскрешение RETIRED отклоняются. Confirmed/supporting states требуют принятого native difference. Research-only записи не могут выдавать себя за реализацию adapters.

Различать debt lifecycle, semantic-version support expiry и фактическую runtime quiescence. Ноль calls не отменяет named offline/recovery/archival obligations; hypothetical consumers не продлевают support. Ранее active запись в RETIRED сохраняет owner/artifact history и завершённые конечные support obligations. Checker не доказывает завершение callbacks, IRQ/DMA, reset или grace periods и никогда не удаляет code, не прекращает admission и не выгружает module. Перед реальным удалением применять [ADR-0007](../architecture-decisions/0007-bug-compat.md) и [security recovery rules](../security/threat-model.md).

## Связи и измерения

Один debt может относиться ко многим drivers/modules; один module может встречаться в нескольких debt records. Consumers закрепляют ID, semantic version и scope каждого declared module с direct/transitive kind и support deadline. Разные versions одного module могут сосуществовать; одинаковые module ID/version/scope должны обозначать одинаковый artifact. Consumer counts выводятся в named inventory; изменяемый consumer_count не хранится. Support start включён, а until — первый неподдерживаемый календарный день в объявленной UTC date domain; expiry не доказывает runtime quiescence.

Cost dimensions: CPU time/latency в ns, memory в bytes, copies/allocations/context switches в counts, throughput в operations/s. MEASURED требует value, denominator, scope, известного coverage, observation date и ограниченного local receipt с точным SHA-256. Partial coverage требует описания ограничения. Измеренный ноль отличается от UNKNOWN. Проверка hash устанавливает сохранённые bytes, но не producer honesty, causal attribution, payload correctness, equal work или statistical gain; это будущие accounting/benchmark/advisor obligations. Source history и runtime provenance разделены.

Реестр не содержит live drivers, authenticated inspectors, universal scores, per-call parsing или kernel dispatch. Существующие текстовые states LEGACY/COMPAT/MIXED/MOSTLY_NATIVE/NATIVE и дополнительные colors не меняются. Сохранять результаты, в которых compatibility действительно быстрее; не добавлять artificial penalties.

## Команды и текущие доказательства

```text
cargo run --locked -p repository-checks -- check-cost-l
cargo run --locked -p repository-checks -- check-cost-l --directory research/cost-l
cargo run --locked -p repository-checks -- validate
cargo test --locked -p repository-checks --test cost_l
```

Обычные check/validate включают этот реестр. [Behavioral tests](../../../../crates/repository-checks/tests/cost_l.rs) проверяют malformed records, duplicate JSON, потерю allocation, missing evidence, lifecycle/support failures, offline obligations, UNKNOWN и measured zero, digest tampering и реальные CLI exit codes. Synthetic active/retired/measurement fixtures — тесты, а не supported deployments.

Три записи с источниками находятся в CANDIDATE: [allocation context](../../../../research/cost-l/COST-L-0001.json), [sysfs text ABI](../../../../research/cost-l/COST-L-0002.json) и [ioctl layout](../../../../research/cost-l/COST-L-0003.json). Каждая закрепляет документацию Linux v6.12, оставляет исходное introduction и actual consumers/cost неизвестными и не заявляет implemented module. [Seed review](../../research/compatibility/taxonomy.md) также фиксирует deferred/non-debt cases вместо принудительного превращения каждого API в запись.

Foundation реализован локально для issue #46; окончательное подтверждение несовместимости требует evidence #45 и native decisions. [Bounded offline queries](../research/cost-l-queries.md) реализованы над проверенными declarations. [Ограниченная проверка module declarations/CI](compatibility-manifests.md) проверяет три синтетических window-контракта без выдуманного Linux-долга. Reviewed production module data и closure (#47), смысловое и EN/RU-принятие уже интегрированных queries #48, runtime accounting, benchmarks, migration и authorized inspection (#49–#52) остаются открытыми. Здесь нет Linux Driver Host, hardware survey или публикации личного inventory.

[Английский оригинал](../../../../docs/architecture/compatibility-debt.md)
