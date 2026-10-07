# Работа над производительностью CI

Document status: CURRENT
Evidence scope: экспериментальная реализация для issue #109; исторический baseline полного времени трёх CI runs и локальные проверки отделены от приёмки ускорения на hosted runners.
Current reference: [Система знаний](../knowledge-system.md)

Исторические publication controls #126 в DEV/PROD требовали удержанной публикации и CompletionPublicationTimeout. По требованию maintainer Phase 3.7 убирает эту mutation и произвольное coordination cutoff. Текущий completion-publication-state-inputs проверяет настоящий отказ Ownership и сохранённое состояние; это не эквивалент SYSTEM coverage удержанной публикации. Внешний timeout диагностирует отказ и никогда не считается успешной операцией или witness отрицательного входа.

<a name="ci-performance"></a>

## Интеграция supervisor и сопоставимость workload

Текущий source-bound plan содержит 144 задачи: четыре ordinary, десять legacy-failure, двадцать один invariant, девяносто три negative-input и шестнадцать mixed input/invariant tasks. Для 140 control slots сохранены разные observed requirements; positive invariants не эквивалентны mutation detection. Сохранённые планы на 130 задач и post-#126 на 142 задачи исторические. API-only reports оставляют check_inventory_id UNKNOWN и требуют отдельного reviewed equivalence для cross-inventory comparison.

## Измерение времени CI и оптимизация

[Issue #109](https://github.com/lifeFedorovAlexey/KOLVRT/issues/109) находится в работе. [Сохранённый baseline](../../../../research/measurements/ci-legacy-baseline.json) содержит три успешных Windows push runs с одинаковыми Git blob inventories ядра, workspace, scripts, assets, manifests, lockfiles и измеряемого workflow. Документация/evidence различаются; входы сборки — нет. Интервалы выполнения составляют 2002, 1970 и 1829 секунд: медиана 1970 секунд (32 минуты 50 секунд). Существенно более ранний run длительностью 908 секунд исключён, поскольку объём работы ядра отличался. Нагрузка runner не контролируется; это описательные наблюдения полного времени, а не парный вывод об ускорении или измерения CPU компилятора.

Сборщик после завершения workflow сохраняет каждый результат step GitHub и полное время, точные измеренные SHA/run/attempt, пять самых дорогих steps, интервал выполнения и путь объявленного DAG. Он извлекает доверенный код ветки по умолчанию с правами только на чтение и никогда не исполняет артефакты PR, вызвавшего запуск. Workflow измерений становится активным после появления в ветке по умолчанию. Отсутствующие наблюдения остаются недоступными; ошибка API сохраняет недоступность или завершает сбор ошибкой. Сохранённый kernel-130-v1 baseline остаётся historical. Job и step names API не подтверждают source-bound число задач: supervisor integration исторически расширила plan до 142 задач; Phase 3.7 расширила его до 144 задач и переклассифицировала мигрированные controls, не меняя shard topology. Поэтому automatic collector оставляет check_inventory_id UNKNOWN, а не относит новый plan к kernel-130-v1. Automatic comparison ограничен одинаковыми source SHA и job/step graph. Для сравнения разных исходников/inventory нужен независимо reviewed source-bound equivalence evidence, которого API-only collector не предоставляет; вручную добавленные inventory labels не разрешают такое сравнение. Сравнение повторов одинаковых исходников по-прежнему требует трёх различных успешных run IDs.

Shell wrapper записывает полное время настройки/скачивания и команд. Необязательный observer xtask записывает отдельные вызовы Cargo build и время жизни процессов QEMU, включая ошибки и timeout, в JSONL-файлы каждого процесса. Сводки сохраняют вложенные категории отдельно; сложение matrix, build и emulator totals учитывало бы работу дважды. Длительность вызова Cargo включает проверки fingerprints/кэша, а не исключительное CPU time rustc. Исключительное время компилятора и начальное ожидание runner остаются UNKNOWN. Длительность DAG суммирует времена jobs вдоль самого длинного объявленного пути зависимостей без очередей runners; интервал выполнения включает ожидания между jobs. Результаты кэша сохраняются отдельно в каждом job.json.

## Границы кэша и конфигураций

Скачивания Cargo registry/git и скомпилированные host/AArch64 artifacts используют версионированные точные keys, включающие Windows OS, Rust 1.99.0, host/guest targets, workload/shard, Cargo.lock, manifests, исходники crates, конфигурацию, build assets и scripts оркестрации. Широкого restore prefix нет. Кэш registry сохраняет только indexes и archives: извлечённые source trees заново создаются Cargo из archives с проверкой checksum из Cargo.lock. Кэш Git содержит только object databases и проверяется git fsck --full; checkouts заново создаются из закреплённых commits. DEV/PROD и feature/negative-control units сохраняют собственные конфигурационные fingerprints Cargo; каждая отличающаяся исполняемая конфигурация требует нового build/run, а её фактические features/ELF hash проверяются; эквивалентные ordinary consumers следуют описанному ниже контракту reuse в пределах одного вызова. Разные jobs/shards никогда не используют одну изменяемую директорию сборки. SHA-256 inventory проверяет каждый восстановленный скомпилированный файл до исполнения: добавления, отсутствующие файлы, повреждения и symlinks вызывают ошибку job. Метаданные кэша проверяют согласованность в границах доверия к кэшу GitHub; они не являются сервисом аутентифицированного происхождения артефактов или зависимостью TCB ядра.

Кэшируется только закреплённый архив QEMU. Каждый restore проверяет SHA-512, заново создаёт установку из этого архива и проверяет закреплённый SHA-256 бинарного файла и точную версию 10.1.0 перед записью provenance. Установленные DLLs и бинарные файлы создаются заново, а не принимаются на доверии из кэша распакованной установки. Кэш скачиваний npm сохраняет обязательный npm ci --ignore-scripts. Кэш инструмента аудита Ubuntu закрепляет OS, Rust и cargo-deny 0.19.9, проверяет inventory байтов до исполнения и сверяет версию; при miss инструмент собирается из locked package. Проверки supply chain не пропускаются.

Эксперимент переиспользования сборки выполняет cold, warm, negative-feature, restored-positive и повторные clean builds по одному target path. Он сравнивает ELF hashes выбранных DEV/PROD builds и проверяет, что negative binary отличается. Это не доказательство для всех feature sets или воспроизводимости между машинами. [Документация Rust для sccache](https://github.com/mozilla/sccache/blob/main/docs/Rust.md) требует отключения incremental compilation и исключает crates, вызывающие системный linker, в том числе bin crates. Для бинарных файлов ядра это существенное ограничение; sccache не включается без отдельного измеренного эксперимента пользы/корректности. Review также следует [контракту Cargo build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html).

## Inventory проверок и параллельный граф

| Исходная проверка                                                  | Текущая команда/job               | Решение                                                                                                                           |
| ------------------------------------------------------------------ | --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Dependency policy и rejection fixtures                             | check:static                      | Сохранены один раз.                                                                                                               |
| Prettier, форматирование Taplo, cargo fmt, Markdown и Taplo lint   | check:static                      | Сохранены один раз.                                                                                                               |
| Repository validation и impact/history от PR base                  | static                            | Сохранены; требуется настоящий PR base.                                                                                           |
| Workspace Clippy, research tests, cargo test и native-state-models | check:host                        | Сохранены; добавлены rejection tests инфраструктуры CI.                                                                           |
| Подмножество exception registry                                    | Workspace cargo test в host       | Удалена только повторная идентичная команда подмножества; тесты сохранены.                                                        |
| Routing all-features и no-default-features tests                   | check:host                        | Обе конфигурации сохранены.                                                                                                       |
| DEV diagnostics/kernel-tests architecture lint                     | check:kernel-dev                  | Сохранены точные исходные target/features.                                                                                        |
| PROD release/no-default-features architecture lint                 | check:kernel-prod                 | Сохранены точные исходные target/features.                                                                                        |
| Реальный kernel matrix и распространение ошибок на host            | matrix shards 0–3, затем evidence | Текущие 144 задачи / 140 классифицированных control slots распределены по одному разу; historical baseline содержал 126 negative. |
| Реальный EL0 routing matrix и controls                             | routing                           | Полный специализированный matrix сохранён.                                                                                        |
| Восемь парных реальных 16-bit ASID comparisons                     | asid                              | Сохранены все шестнадцать boots и проверки фактических 16 bits.                                                                   |

Локальный npm run check выполняет check:static, затем check:host. check:qemu сохраняет шесть команд (test, routing test, asid-bench, selftest, app-smoke и service-run), а именованные команды kernel lint доступны независимо. Static gate CI предшествует параллельным jobs host, DEV lint, PROD lint, четырём kernel shards, routing, ASID и native. Для shards отключён fail-fast. Evidence job независимо генерирует текущий план исходников/задач/конфигураций и отклоняет пропущенные/дублированные tasks или shards, устаревшие исходники, неверные profiles/features, различающиеся ELF hashes или аргументы QEMU. Каждая задача помечена executed или reused с проверенным provenance donor. Итоговый статус foundation требует успеха каждого обязательного job; ошибка, отмена, отсутствие или skip не могут превратиться в PASS.

Последовательное и разделённое выполнение ядра используют один inventory задач и исходные обязательные failure predicates, включая разобранное именованное событие ошибки IPC и точные supervision-reject/error для supervision: controls. Частичные receipts shards не заменяют полную приёмку. Все artifacts относятся к текущему run и названы по точному head SHA, workload, shard и attempt. Кэшируемые paths исключают сохранённые результаты QEMU; неизменяемые исторические receipts сохраняются. Receipt ASID копируется в target evidence текущего run только после исполнения, поэтому host/lint jobs не могут опубликовать старый сохранённый ASID receipt как свежий evidence.

## Переиспользование evidence matrix в пределах одного вызова

B5 меняет host runner и проверку receipts, а не поведение production kernel или приложений. План matrix сохраняет schema 1; receipts последовательного выполнения, shards и aggregate используют schema 2. Каждый обычный donor обязан успешно выполниться в этом вызове. Только последующая задача того же shard может переиспользовать этот более ранний executed donor; цепочки reuse, предыдущие вызовы, исторические артефакты и donors из других shards запрещены. Должны совпадать source plan, DEV/PROD profile, features, идентичность ELF и конфигурация QEMU. Fatal controls и отличающиеся сборки по-прежнему выполняются независимо.

Каждый consumer сохраняет собственную идентичность задачи и проверяет все обязательные именованные assertions по сохранённым events donor. Отсутствующие, failed или повторяющиеся assertions отклоняют consumer. Provenance receipt различает executed и reused и указывает reused_from; один лишь одинаковый бинарный файл не подтверждает успешный assertion. Наблюдения времени Cargo и QEMU описывают только настоящие запуски процессов: reuse не добавляет фиктивный запуск с нулевой длительностью.

Для текущего плана на 144 обязательные проверки ожидаются 38 executed и 106 reused в четырёх shards (10/26, 11/25, 9/27 и 8/28) либо 32 executed и 112 reused при последовательном выполнении. Counts четырёх shards подтверждены сохранённым локальным выполнением ниже; counts последовательного режима остаются выведенными из плана. KOLVRT_MATRIX_REUSE=off даёт всегда заново исполняемый reference с теми же обязательствами и predicates; reuse по умолчанию не вводит retry после failure. Эти counts не являются измеренным ускорением или приёмкой hosted CI. 140 классифицированных control slots и исторические receipts, включая 144 настоящих исполнения на 43af402, сохраняют исходный смысл.

## Выполнение и оставшиеся этапы

Push в main, PR и ручные запуски используют один и тот же разделённый граф. Старый последовательный job, переключатель ручного запуска и обработчик workload удалены; итоговый gate всегда требует разделённые workloads. Исторические timing baselines остаются неизменяемыми свидетельствами для сравнения. Push в feature-ветку не дублирует synchronize run открытого PR; до открытия PR доступны локальные проверки и ручной запуск workflow. Новые commits PR автоматически отменяют устаревшие runs того же workflow/PR. Разные workflows не отменяют друг друга; активный main run сохраняется. Отчёты и raw evidence сохраняются девяносто дней; baselines в репозитории сохраняются постоянно.

```text
npm run check
npm run check:kernel-dev
npm run check:kernel-prod
npm run check:qemu
cargo xtask matrix-plan
cargo xtask matrix-shard 0 4
node scripts/ci-build-reuse.cjs
node scripts/ci-timings.cjs OWNER/REPO RUN_ID target/ci-timings research/measurements/ci-legacy-baseline.json
```

Сборщику GitHub требуется GITHUB_TOKEN с правом чтения Actions. В одной локальной checkout четыре shards выполняются последовательно, поскольку paths артефактов общие; CI выполняет каждый shard на отдельном runner. Агрегация проверяет все четыре receipts против заново сгенерированного текущего плана.

Повторные ASID boots — независимые статистические наблюдения с чередованием порядка, а не дублированные потребители evidence. Foundation, routing и ASID имеют разные features, payloads или CPU configurations и не могут разделять один positive boot receipt. Только эквивалентные ordinary consumers в пределах одного вызова переиспользуют сохранённые events по контракту schema 2. Replay исторических результатов не вводится. Планирование по impact сопоставляет точный inventory исходников ядра/сборки/runner с настоящим base и анализирует ownership и prerequisite impact по обоим графам знаний: принятому base и текущему. Изменения CI/verification inputs, kernel authority, неизвестные файлы или недоступный base требуют полного matrix. Пропуск разрешён только при одинаковых kernel inputs для производной навигации или объявленных host-only units без kernel prerequisite impact. Проверка path == docs не используется. При host-only run финальный gate требует успеха static/host и явного skip всех освобождённых kernel jobs; неожиданный failed job не маскируется.

PR feedback при прогретом кэше менее десяти минут и cold CI менее пятнадцати минут остаются целями приёмки до измерения на hosted runners. Оставшиеся обязательные этапы: cold/warm hosted runs, review cache hit/miss и повреждений, сравнение с опубликованным baseline, содержательные ограничения атрибуции и полное смысловое/EN-RU review. Issue остаётся открытым.

[Английский оригинал](../../../../docs/ci/performance.md)

Обязательный native workload отдельно запускает DEV/PROD guest Clippy, настоящие component methods, три external ABI actors, обычные application sessions, production crash/recovery/shutdown, отдельный app smoke и persistent-runtime observations. Negative inputs вызывают настоящие методы/public ABI или проверяют host oracle; source-copy mutations не используются. Этот workload отделён от foundation plan на 144 задачи и участвует в итоговом gate. Apps включены в compiled-cache source key; этот отдельный native workload по-прежнему заново выполняет свои emulator observations.

IRQ/SIMD control теперь наблюдает настоящий round trip как positive invariant coverage; saved-frame corruption и skipped-restore detection не заявляются. Текущие counts: четыре ordinary, десять legacy-failure, двадцать один invariant, девяносто три negative-input и шестнадцать mixed tasks. Прежние receipts на 130 задач и post-#126 на 142 задачи сохраняют исходный inventory/source scope.

[Сохранённое измерение B5](../../../../research/measurements/ci-b5-reuse.json) содержит одну локальную пару контрольного и оптимизированного shard 0: 309.7595683 с при отключённом reuse и 37.077867 с при включённом; идентичность build/ELF/QEMU совпала для всех 36 задач. Четыре оптимизированных shard прошли все 144 обязательства: 38 executed и 106 reused observations. Это одно последовательное локальное сравнение с неконтролируемым состоянием хоста и кэшей, а не гарантия hosted-ускорения или сравнение с историческим baseline из 130 задач. Полные контрольный и aggregate receipts сохранены рядом со сводкой. Пара измерений сохраняет исходный source plan до эквивалентной переработки guard по Clippy. Отдельный четырёхшардовый receipt на окончательных исходниках проверяет итоговый runner; прежнее измерение не переобозначается как timing comparison на точных окончательных исходниках.

Свежее выполнение с машинным evidence через общий runner xtask требует ровно одного проверенного [загрузочного наблюдения устройства](../kernel/devices.md). Отсутствие события инвентаризации приводит к отказу текущей проверки выполнения, даже если остальные события workload успешны. Чтение исторических свидетельств отделено и допускает потоки до появления дескриптора; эта совместимость не разрешает новый запуск без наблюдения. Меняется допуск evidence, а не производительность ядра, методика измерений или право на Arena record.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.ci.performance",
  "kind": "subsystem-contract",
  "summary": "Измерение CI и поэтапная оптимизация без ослабления доказательств ядра.",
  "depends_on": ["doc.kolvrt.docs.knowledge", "adr.0006"],
  "units": [
    {
      "id": "kolvrt.ci.performance",
      "anchor": "ci-performance",
      "kind": "feature",
      "summary": "Измеренный старый baseline, проверка целостности кэшей, параллельные проверки и полная агрегация четырёх QEMU shards.",
      "tags": ["ci", "timing", "cache", "tooling"],
      "feature": {
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "Experimental timing, versioned integrity-checked caches, split local checks, four-way kernel matrix partitioning and exact-source aggregate validation; CI performance acceptance remains pending. Phase 3.7 adds a mandatory separate native workload for production-method UNIT tests, external ABI/SYSTEM scenarios, actual application smoke/runtime and original-source dependency closure proof. Matrix coverage distinguishes negative inputs, positive invariants and remaining legacy failures; this is correctness integration, not new CI speed acceptance. B5 reuses fresh same-invocation ordinary events within each shard with schema-2 donor provenance and independent consumer assertions; feature-specific controls and statistical ASID runs remain separately executed.",
        "sources": [
          ".github/workflows/kernel.yml",
          ".github/workflows/dependencies.yml",
          ".github/workflows/ci-timings.yml",
          ".github/workflows/ci-windows-workload.yml",
          "package.json",
          "scripts/setup-qemu.ps1",
          "scripts/run-timed.ps1",
          "scripts/ci-workload.ps1",
          "scripts/ci-timings.cjs",
          "scripts/ci-observations.cjs",
          "scripts/cache-integrity.cjs",
          "scripts/ci-gate.cjs",
          "scripts/ci-evidence.cjs",
          "scripts/ci-build-reuse.cjs",
          "scripts/tests/ci-timings.test.cjs",
          "scripts/tests/ci-infrastructure.test.cjs",
          "crates/xtask/src/main.rs",
          "crates/xtask/src/routing_demo.rs",
          "crates/xtask/src/matrix.rs",
          "crates/xtask/src/timing.rs",
          "scripts/ci-impact.cjs"
        ],
        "acceptance": [
          "research/measurements/ci-legacy-baseline.json",
          "research/measurements/ci-build-reuse.json",
          "research/measurements/ci-b5-reuse.json"
        ],
        "issues": [109],
        "adrs": ["adr.0006"],
        "limitations": [
          "Hosted cold/warm acceleration, cache service behavior and semantic/EN-RU review remain pending. No compiler-exclusive CPU attribution or historical QEMU observation replay is claimed; invocation-local ordinary reuse preserves each required assertion, and conservative impact scheduling cannot prove semantic completeness."
        ],
        "next_gate": "Validate B5 on hosted CI, retain comparable cold/warm runs and cache outcomes, and complete remaining B0-B4 acceptance. Local B5 execution and EN/RU semantic review do not establish hosted performance targets or resolve supervision Issue #133.",
        "verification": [
          {
            "environment": "github-actions",
            "state": "STALE",
            "reason": "This feature retains historical receipts from before the Phase 3.7 source changes; its verification remains STALE pending a feature-scoped current-source review. The separately accepted bounded native-application fault/restart/shutdown evidence is recorded in docs/kernel/native-applications.md and research/results/native-phase37-43af402.json; it does not automatically renew this feature verification."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Begin issue #109 with an observational B0 reporting implementation; preserve every existing CI check.",
            "acceptance": []
          }
        ]
      }
    }
  ]
}
```
