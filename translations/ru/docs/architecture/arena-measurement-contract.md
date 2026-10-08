# Контракт измерений Arena

Document status: CURRENT
Evidence scope: экспериментальные общие offline-контракты A0/A1 и проверки допуска импортов плюс принятый ограниченный QEMU-пилот CLOCK; без record service, общей независимой аттестации или сертификации.
Current reference: [Arena](component-arena.md); [ADR-0006](../architecture-decisions/0006-metrics.md); [контракт знаний](../knowledge-system.md)

## Архитектура и область действия

Issue [#93](https://github.com/lifeFedorovAlexey/KOLVRT/issues/93) определяет общую модель Arena. Первая реализация использует существующую библиотеку repository-checks и фасад xtask. Новых зависимостей ядра, instrumentation, архитектурного реестра или score нет. Canonical Markdown владеет ID блоков/контрактов; текущие проверки профилей разрешают их существующим сборщиком знаний. Versioned edges runtime-графа и восстановление исторического графа пока не реализованы.

[Профиль диапазонов](../../../../research/arena/profiles/user-copy-range.json) — PROPOSED synthetic host profile существующего контракта проверки диапазонов kernel-core. Он не является принятым PERF/SEC-паспортом SafeCopy или измеренным kernel run. Собственная мера и SFR/SAR помечены KOLVRT_DEFINED. Текущие байты canonical текста консервативно фиксируются digest контракта; будущая версия с семантическим подмножеством требует reviewed rules, а не автоматической совместимости.

[Пилот CLOCK](../research/clock-passport.md) теперь задаёт ограниченный pipeline настоящего ядра с той же схемой и registry. Проверенные профили DEV/PROD измеряют интервал трёх CLOCK-вызовов с одним полезным запросом и парными наблюдениями промежуточного recorder ON/OFF. Существующие публичные счётчики окон исполнения дают частичную атрибуцию, а не исключительную процессорную стоимость. Принятое выполнение сохраняет двенадцать свежих успешных загрузок и шесть переносимых паспортов пар, review точных исходников и полное regression evidence. Это не меняет право на offline record и не заявляет полную стоимость timestamp-проб.

## Реестр стандартов

[Реестр](../../../../research/arena/standards.json) и [закрытая schema](../../../../schemas/arena-standards.schema.json) фиксируют stable ID/version, scope, authoritative source и source revision, вид источника, применимость к Arena, состояние публикации, дату проверки и supersession. ISO/IEC 25010:2023 задаёт taxonomy; ISO/IEC 25023:2016 предоставляет меры качества и отдельно хранит ожидаемую draft-замену; ISO/IEC 25040:2024 — framework оценки. Применимые части ISO/IEC 15408 имеют отдельные версии. CC:2022/CEM:2022 включают Release 1 и errata 1.2; ISO/IEC 18045:2026 — отдельный источник. NIST SP 800-55 Vol.1/2 (2024) представлены отдельными guidance entries.

Правила SPEC CPU 2017 дают принципы воспроизводимости, проверки результата, повторных запусков и раскрытия условий; Arena не выполняет suites SPEC CPU и не заявляет SPEC conformity. Linux perf bench v6.12 и fio 3.42 — закреплённые de-facto references, а не стандарты ISO. RFC 2544 и документы применимости/обновлений версионируются отдельно и не разрешают испытания действующих production networks. Методика KOLVRT_DEFINED версии 1 задаёт описанные ниже экспериментальную адаптацию и описательный анализ. Public metadata не доказывают mapping нормативных clauses ISO/CC или сертификацию.

Ключ записи реестра — ID/version. Дубликаты, неверные даты, плавающие branch locators, несовпадение вида/authority источника, отсутствующие successors, draft-successors и циклы supersession отвергаются. Обновление текущего реестра не меняет встроенные исторические snapshots. Подлинность publisher/reviewer и реальное соответствие заявленной source revision остаются обязанностью review. Команда registry выводит обратные ссылки metric/security из профилей.

## Measurement Profile и comparison class

[Schema профиля](../../../../schemas/arena-profile.schema.json) требует canonical target/functional version и digest контракта, workload/input/oracle identities и digests, environment/hardware/architecture/toolchain/config/resource policy, sampling/stopping/analysis, протокол observer cost, units/denominator, source sections/versions и видимость метрик. PERF, REL и RES — dimensions; RES означает resource usage/cost, а не новую ISO quality characteristic. SEC содержит TOE, SPD с assumptions/threats/policies/boundaries, явные собственные SFR и необходимые методы/scope SAR, а также исключения AVA_VAN/AP. Неопределённые связи SFR/SAR и незаявленные методики отвергаются.

Версия 1 использует консервативный comparison key по всем полям профиля кроме review_state и требует равенства каждой referenced frozen methodology definition. Изменения schema, версии профиля, functional/workload/security/methodology, environment, toolchain, budgets, sampling и observer protocol создают другие classes. SHARED декларирует видимость сбора и не смешивает DEV/PROD. DEV_INTERNAL требует внутреннего механизма DEV; PROD разрешает только EXTERNAL. Exact implementation commit/source/image identities хранятся в provenance run, поэтому изменение реализации не замораживает серию. Допуск всё равно требует новых exact artifacts и применимого evidence; прежние assertions не доказывают поведение нового image.

## Замороженные импорты и допуск

[Schema run](../../../../schemas/arena-run.schema.json) включает frozen registry/profile snapshots с canonical JSON SHA-256 digests, exact commit/source/image identities, роли submission/contributor, полный environment manifest и byte-digested bundle artifacts. Canonical JSON digests используют упорядоченные maps serde_json и компактную сериализацию; digests artifacts используют исходные байты без нормализации переводов строк. Raw observations и помеченный warmup остаются в receipt. Ссылки bundle должны быть относительными, после canonicalization оставаться внутри bundle и указывать на проверенные файлы. Каждый input/artifact ограничен 32 MiB. Импорты пока не сохраняют immutable history и не аттестуют выполнение.

В версии 1 все заявленные metrics обязательны для structural admission; допуск optional metrics требует рассмотренного изменения policy. Каждая metric встречается ровно один раз как доступные raw samples либо typed missing observation с причиной/count/evidence. Failures, dropped/unfinished samples, недостаточные заявленные sample/warmup counts, failed или отсутствующий correctness evidence, unreviewed profile, отсутствующие mandatory SFR/SAR и matched on/off observer observations дают INELIGIBLE. Неизвестные/повторные SFR/SAR/metric IDs, неверные digests и unrelated evidence не проходят validation. PASS assertion без всех обязательных SAR artifacts не подтверждает SFR. Matched observer protocol ID/version/workload должны совпадать; overhead представлен отдельным evidence и никогда не вычитается как точная поправка.

Команда показывает описательные nearest-rank median/p95/p99, арифметическое среднее или число raw observations по заявленной metric. Она не делает выводов о throughput, PMU, достаточности tails, repeated-run regression, uncertainty interval или superiority. Прошедший эти gates producer assertion получает только STRUCTURALLY_ADMISSIBLE. На этом этапе record_eligible всегда false: общий допуск с независимо проверенными execution/applicability/contribution, вычисление records/co-holders и append-only invalidation/history не реализованы. Пилот CLOCK сохраняет собственное ограниченное независимое review исходников и выполнения; это review не реализует общий процесс публикации records. Synthetic fixtures явно не допускаются как kernel evidence. До публикации необходимо добавить repeated-run/uncertainty и authenticity policy; переданные REVIEWED/PASS/ACCEPTED labels не являются доверенными attestations.

## Команды и проверка

Запуск из корня репозитория:

```text
pwsh -File scripts/run-arena.ps1
pwsh -File scripts/run-arena.ps1 registry
pwsh -File scripts/run-arena.ps1 profile
cargo xtask arena check
cargo xtask arena registry
cargo xtask arena map
cargo xtask arena map --html
cargo xtask arena map --html target/custom-arena-map.html
cargo xtask arena profile research/arena/profiles/user-copy-range.json
cargo xtask arena assess PATH_TO_RUN_JSON
cargo xtask arena compare LEFT_RUN_JSON RIGHT_RUN_JSON
cargo test --locked -p repository-checks --test arena
```

Общая проверка репозитория валидирует сохранённые registry и proposed profiles. assess печатает сохранённые причины отказа и завершает работу с ненулевым кодом при INELIGIBLE; compare отвергает несовместимые или структурно отклонённые imports. Команды не публикуют record. Tests изменяют provenance, methodology, environment, oracle, missing SAR, потерю observations, overhead и artifact paths/digests; synthetic numbers не становятся измеренными kernel results. Закрытые schemas также отвергают добавленные universal scores и отсутствие units/source fields.

Запуск одной командой: `scripts/run-arena.ps1` поддерживает check/registry/profile/assess/compare, по умолчанию выполняет check, а profile без пути выбирает proposed range profile. Скрипт запускает текущий source через существующий xtask, разрешает относительные пути от корня проекта и сохраняет exit code отказа. Cargo берётся из PATH либо из существующей `.toolchains` установки проекта (для worktree также из общего Git checkout); environment восстанавливается после запуска. `-Help` не запускает сборку. Сам launcher не добавляет публикации records; команды map ниже предоставляют ограниченную графическую проекцию.

Текущая [логическая проекция архитектуры](kernel-component-map.md) доступна через `cargo xtask arena map` и `cargo xtask arena map --html [OUTPUT]` (по умолчанию `target/arena-map.html`). Она показывает одну модель версии 2 с привязкой к исходникам: 13 групп EL1, две production-службы EL0 и 30 выбранных взаимодействий. Её логические элементы и типизированные рёбра описывают архитектуру, а не изолированные единицы развёртывания или измеренные частоты вызовов. Несовпадение хешей исходников отклоняет представление как STALE. Терминал и интерактивный HTML сохраняют явно неизвестные стоимости и добавляют только проверенные исторические наблюдения пути CLOCK, отдельно для DEV и PROD; распределять эти интервалы между отдельными группами нельзя. Восстановление исторической топологии, полное покрытие общего графа и веса рёбер исполнения этим не реализованы.

HTML-проекция — граф во всю область браузера с перемещением, масштабированием, командой «Вписать», компактными значками узлов и локальным переключателем RU/EN. Выбор корневого блока открывает внутренний состав, а «Назад» восстанавливает корневое представление; постоянной боковой панели и большой полосы сводки нет. Стабильные локальные идентичности охватывают 55 элементов. Внутренние представления не дублируют родительский блок. Все 15 групп имеют выбранные взаимодействия, проверенные по исходникам: всего 56 внутренних связей, включая восемь связей группы CLOCK и пять связей группы ожидания. Это не исчерпывающее покрытие зависимостей; фактические вызовы контроллера прерываний и writer консоли связаны с элементом MMIO группы загрузки и платформы через межгрупповые границы. Представление ожиданий отделяет уведомления событий STEP от обработки удерживаемых пробуждений IPC и запрета обычных блокировок; READY после пробуждения IPC означает перезапуск syscall, а не завершение RPC. Компактные карточки соответствуют содержимому и объясняют статус измерений подсказкой. Краткие подсказки названий и исходников не повторяют метрики. Количества элементов, входящих и исходящих связей являются фактами топологии и отделены от полей производительности внутри узлов. Замеры дочерних элементов явно не реализованы. Только исторический родительский узел CLOCK содержит наблюдения пути DEV/PROD; это не устанавливает стоимость текущих исходников или распределение стоимости по компонентам.

Внутренние представления продолжают корневой граф через границу выбранного блока. Все 30 корневых связей сохраняют идентичность через 41 проверенных пар `member_endpoints`, присоединяя входящие и исходящие связи к реально участвующим механизмам. Составная корневая связь может иметь несколько ветвей в подробном представлении; это не увеличивает число корневых связей. Карточки внешних групп открывают связанную группу. Краткие подписи действий обозначают связи, а подробные подсказки сохраняют полное взаимодействие и свидетельства. Это выбранное покрытие исходного кода, а не исчерпывающий граф вызовов или измеренная трасса исполнения.

## Оставшиеся gates

Ограниченный пилот CLOCK для #32 предоставляет первый принятый паспорт настоящего QEMU-механизма: внешние интервалы DEV/PROD, частичный учёт окон исполнения, парную стоимость recorder и ограниченные host-свидетельства допуска и безопасности. Он не устанавливает исключительную CPU-атрибуцию, широкое SEC-покрытие или физическую производительность. #33 предоставляет IPC после #26; #34/#35 — REL/fuzz/SAR campaigns; #36 интегрирует publication/regression gates; #50 отвечает за native/compat equivalence и COST-L attribution. Приёмка CLOCK не закрывает эти последующие области или полную acceptance #93. Reviewed source mappings, подписанный/независимый custody где требуется, accepted contribution verification, record history, восстановление исторической топологии и измеренные веса рёбер исполнения остаются открытыми. Physical ARM64 performance требует отдельной campaign; QEMU — отдельная среда воспроизводимости/regression.

[English original](../../../../docs/architecture/arena-measurement-contract.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.arena.measurement-contract",
  "kind": "subsystem-contract",
  "summary": "Экспериментальный реестр методик, замороженные профили измерений и офлайн-допуск импортов; без kernel records.",
  "depends_on": [
    "adr.0006",
    "adr.0003",
    "adr.0024",
    "kolvrt.memory.user-copy.api"
  ],
  "tags": ["arena", "standards", "measurement", "security"],
  "read_when": ["Arena профили измерений сравнимость SFR SAR допуск"]
}
```
