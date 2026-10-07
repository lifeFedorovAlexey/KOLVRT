# Пилот Arena для запросов clock

Document status: CURRENT
Evidence scope: принятый ограниченный пилот CLOCK issue #32 на исходниках 2978ad9; реальное выполнение QEMU DEV/PROD, точные artifacts и независимое review, не аппаратный performance или допустимые records Arena.
Current reference: [Контракт native CLOCK](../kernel/clock.md); [Контракт измерений Arena](../architecture/arena-measurement-contract.md)

<a name="kolvrt-arena-clock-passport"></a>

## Пилот и полезная операция

Этот внешний ELF-клиент вызывает единственную production-реализацию `native::clock_snapshot()` и существующий syscall CLOCK. Production-сервис не копируется; новая feature ядра, fault hook или временная политика не добавляются. Общий native bootstrap получает клиента как root и передаёт существующий непрозрачный `KOLVRT_NATIVE_ARGUMENT`: 1 выбирает recorder OFF, 2 — recorder ON, остальные значения недопустимы. Это необходимый отдельный клиент измерения ABI без аналога среди production-приложений.

Каждая итерация вызывает CLOCK трижды: начальный снимок, полезный запрос, конечный снимок. Оба режима выполняют одинаковые полезный запрос и oracle. Частота должна оставаться ненулевой и одинаковой; метки и накопленные счётчики должны быть монотонными, а полезный запрос — находиться внутри интервала конечных точек. Каждая разность READ_WINDOW должна быть нулевой. Проверяемая арифметика отклоняет отрицательные результаты и переполнение. Измеряемая длительность — интервал между двумя точками чтения счётчика ядром, включая полезный запрос, пользовательский учёт и части крайних вызовов. Это не чистая задержка syscall, пустой trap, задержка IPC или исключительная процессорная стоимость.

## Отчёт и oracle

Отчёт содержит ровно 64 беззнаковых 64-битных слова; сбор и импорт обязаны сохранять точность целых чисел. Слова 0–7 задают magic `0x434c4b01`, версию 1, режим, частоту, четыре прогрева, 24 измеряемых наблюдения, 28 завершённых полезных запросов и нулевую накопленную разность READ_WINDOW. Слова 8–35 содержат все 28 разностей времени; слова 36–63 — соответствующие разности окон исполнения. Первые четыре элемента каждого массива явно сохраняются как прогрев. Общий oracle выполняется после конечного снимка и проверяет каждое наблюдение до экспорта; агрегаты не скрывают отдельные отказы. Magic отчёта определяет клиент и проверяет host oracle.

Клиент экспортирует успешный отчёт schema 1 только после измеряемого цикла. При отказе наблюдения он вместо этого выдаёт частичную диагностику schema 0 после его интервала и завершается неуспешно; эта диагностика никогда не является результатом измерения производительности. Вызовы REPORT и записи массивов отчёта после конечного снимка находятся вне интервала. Отсутствующие, лишние или неверные отчёты, неизвестные режимы/версии, изменённые частота/числа, нарушенные проверки полезного запроса, ненулевой код выхода root, panic, timeout или отсутствие reclaim отклоняют запуск. Хост сохраняет журналы и учёт отказов, а не превращает частичное выполнение в успешное. Нормальное завершение обязано освободить владельцев, восстановить frames и оставить ноль живых процессов/доменов. Экспорт не даёт доступа к другому процессу и не меняет вместимость отчёта ядра.

## Согласованная стоимость recorder

ON записывает промежуточный снимок полезного запроса в фиксированный пользовательский буфер до конечного снимка. OFF исключает только этот дополнительный recorder. Оба сохраняют одинаковые три CLOCK-вызова, обязательный oracle, сбор конечных точек и экспорт отчёта. Сохраняемые volatile-записи предотвращают удаление recorder ON. Стоимость ветвления и записи относится к этому именованному протоколу наблюдения.

Парное сравнение оценивает только добавочную стоимость промежуточного recorder. OFF не свободен от инструментации: крайние CLOCK-пробы и обычный сбор результатов остаются. Ни один режим не измеряет всю стоимость наблюдения меток. Сохраняются оба сырых ряда; точная скорректированная задержка не вычитается, а стоимость syscall не выводится из их разности. Ряд OFF — основное измерение полезной операции; ON даёт парное свидетельство стоимости наблюдателя, а не вторую реализацию или заявление о превосходстве.

## Выборка и атрибуция

До выполнения фиксируются три свежие пары на профиль в порядке OFF/ON, ON/OFF, OFF/ON. Каждый запуск содержит четыре отмеченных прогрева и 24 измеряемых наблюдения. DEV и PROD дают всего двенадцать свежих загрузок. Нет повторных попыток, переиспользования исторических наблюдений, выборочного удаления выбросов или остановки по успешному результату. Неуспешная кампания сохраняет все попытки и незавершённые наблюдения и не публикует успешную приёмку кампании.

Единица повторения — независимая загрузка guest; наблюдения внутри одной загрузки могут коррелировать. Четыре прогрева — фиксированная подготовка, а не доказательство стабилизации. Сохраняются идентичность пары и порядок; 72 наблюдения нельзя считать независимыми испытаниями. Используются заранее заданные описательные медианы nearest-rank. Достаточность хвостов, выводы о p95/p99, стабильности регрессий и сравнительном превосходстве остаются INCONCLUSIVE для этой ограниченной кампании. Бюджет производительности не выдумывается.

Сохраняются разности времени, окон исполнения и READ_WINDOW вместе с проверяемым неатрибутированным остатком. DEV предоставляет существующую частичную атрибуцию окон исполнения, собранную ядром; PROD наблюдает тот же публичный ABI снаружи. Ни одна величина не является исключительным процессорным временем. Из остатка нельзя отдельно вывести обслуживание CLOCK, IRQ и планирование хоста. x3 равен нулю для этой нагрузки, поскольку READ_WINDOW не выполняется; это не утверждение о нулевой стоимости CLOCK. x4 не читается. DEV/PROD имеют отдельные профили и классы сравнения даже при видимости метрики SHARED.

## Паспорт и область безопасности

Замороженный [артефакт протокола](../../../../research/arena/clock-query/protocol.json) определяет версионированные семантику нагрузки и сравнение recorder. Профили закрепляют его байты независимо от изменяющихся метаданных приёмки этой feature; нормативный документ CLOCK задаёт отдельно закреплённый контракт API. Любое изменение протокола требует новой ревизии профиля и review применимости.

Используются существующие схемы профиля/запуска Arena, registry и evidence pipeline xtask. [Профиль DEV](../../../../research/arena/profiles/clock-query-dev.json) и [профиль PROD](../../../../research/arena/profiles/clock-query-prod.json) имеют статус REVIEWED для заявленного протокола после независимого review Codex; этот статус не подтверждает выполнение. Каждая пара сохраняет идентичность исходников/образа/конфигурации/toolchain/QEMU, замороженные снимки профиля/registry, отмеченные прогревы, сырые наблюдения и digests артефактов; неизменяемый манифест кампании связывает три пары. Вторая registry или универсальный score не вводятся. Отдельному механизму не нужна выдуманная альтернативная реализация. Существующий допуск остаётся не выше STRUCTURALLY_ADMISSIBLE; сохраняется `record_eligible=false`, а принадлежность вклада остаётся UNATTRIBUTED без отдельного принятого evidence.

Область безопасности охватывает границу наблюдения CLOCK только за текущим процессом и только для чтения, внешний oracle и допуск сохранённых результатов. Доверенные boot input, работа таймера и существующая изоляция процессов — предположения, а не заново доказанные свойства. Собственные SFR требуют правдивых полных наблюдений, отсутствия выводимой authority или селектора чужого процесса и нормального ограниченного завершения/reclamation. Обязательные SAR сочетают настоящее выполнение, review конкретных исходников и отрицательные host-входы. Пропуск операции, отражённый неполным числом полезных запросов, сломанный oracle, отсутствующие/повторяющиеся/усечённые наблюдения, нарушение обязательного SFR или отсутствие SAR обязаны отклонять запись. Эти controls меняют внешние входы или evidence, никогда реализацию ядра. Они не доказывают защиту от вредоносного ядра или производителя, способного подделать всю цепочку сохранности данных.

Grant авторизации clock не выдумывается. PMU, физическое оборудование, пики ресурсов, кампании надёжности, fuzz-покрытие, анализ уязвимостей, уровни attack potential и сертификация этим пилотом не подтверждаются. PERF и SEC остаются отдельными измерениями; другие измерения имеют явные пробелы области. Метка PASS производителя не заменяет evidence или независимый review применимости.

## Текущее evidence

[Приёмка](../../../../research/results/clock-passport-acceptance.json) фиксирует двенадцать успешных загрузок на чистых исходниках `2978ad9`, шесть независимо перепроверенных переносимых паспортов, пять проверок CLOCK oracle и семнадцать проверок допуска Arena. Полная foundation matrix выполнила 144 обязательства: 32 настоящих исполнения и 112 проверенных использований свежих результатов того же invocation; каждая suite DEV/PROD прошла 147 проверок. Все 151 digest исполняемых исходников совпадают между кампанией и matrix. [Неизменяемый bundle](../../../../research/arena/runs/clock-query-2978ad9/campaign.json) сохраняет каждую пару и сырые artifacts; прежние неудачные подготовительные host-проверки сохранены отдельно. Review исходников, протокола и полной пары EN/RU завершено. READY ограничено pipeline этого пилота; tails остаются inconclusive, physical hardware — unknown, record eligibility — false.

[Английский оригинал](../../../../docs/research/clock-passport.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.research.clock-passport",
  "kind": "subsystem-contract",
  "summary": "Ограниченный внешний протокол интервала CLOCK и отдельное evidence наблюдателя и безопасности.",
  "units": [
    {
      "id": "kolvrt.arena.clock-passport",
      "anchor": "kolvrt-arena-clock-passport",
      "kind": "feature",
      "summary": "Пилот паспорта настоящего механизма с частичным учётом и парным протоколом recorder.",
      "depends_on": [
        "kolvrt.clock.query.api",
        "doc.kolvrt.arena.measurement-contract",
        "adr.0006",
        "law.036",
        "law.044"
      ],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "Accepted external CLOCK ABI pilot: fixed twelve-boot DEV/PROD campaign, exact original ELF/source/configuration evidence, partial execution-window accounting, matched incremental recorder observations and six structurally admissible passports through the existing Arena validator.",
        "sources": [
          "apps/native-runtime/src/lib.rs",
          "tests/native-apps/Cargo.toml",
          "tests/native-apps/src/clock-client.rs",
          "crates/xtask/src/arena_clock.rs",
          "crates/xtask/src/arena_clock/passport.rs",
          "crates/xtask/src/native_apps.rs",
          "crates/xtask/src/main.rs",
          "crates/kernel/src/scheduler/mod.rs",
          "crates/kernel/src/scheduler/task.rs",
          "crates/kernel/src/arch/aarch64/entry.S",
          "research/arena/profiles/clock-query-dev.json",
          "research/arena/profiles/clock-query-prod.json",
          "research/arena/clock-query/protocol.json",
          "research/arena/clock-query/input.json",
          "research/arena/clock-query/resources.json",
          "research/arena/clock-query/dev-environment.json",
          "research/arena/clock-query/prod-environment.json",
          "crates/xtask/src/arena_common.rs"
        ],
        "acceptance": ["research/results/clock-passport-acceptance.json"],
        "issues": [32],
        "adrs": ["adr.0006", "adr.0015"],
        "limitations": [
          "No pure syscall latency, full timestamp observer cost, exclusive CPU attribution, physical evidence, adequate tails or eligible Arena record."
        ],
        "next_gate": "Extend separate IPC, reliability and fuzz scopes in #33/#34/#35; independent record publication, adequate tails and physical hardware remain unverified.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "STALE",
            "reason": "The accepted 2978ad9 campaign remains historical. Shared Arena execution and native report export changed for issue33; current-source applicability and regression require renewed evidence.",
            "receipt": "research/results/clock-passport-acceptance.json",
            "receipt_sha256": "8bea832e800a4d58f401d8916b2c9f302c548ae99a7a0d0c6c719b003ed3743f",
            "scope": "Clean campaign source 2978ad9693b7df1e70487ffbf01d5ffdfe27fe31, pinned QEMU 10.1.0 ARM64 TCG, DEV/PROD; useful CLOCK envelope and incremental recorder cost only. Full matrix uses identical 151 execution source hashes."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical benchmark campaign."
          }
        ],
        "readiness": "READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Introduce the external pilot protocol and proposed profiles without execution acceptance."
          },
          {
            "from": "EXPERIMENTAL",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "Independent source, EN/RU and actual evidence review accepted twelve real boots, exact raw artifacts and current-source regression; no eligible records or hardware performance claim.",
            "acceptance": ["research/results/clock-passport-acceptance.json"]
          }
        ],
        "readiness_acceptance": [
          "research/results/clock-passport-acceptance.json"
        ]
      }
    }
  ]
}
```
