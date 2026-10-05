# Работа над производительностью CI

Document status: CURRENT
Evidence scope: экспериментальная реализация для issue #109; исторический baseline полного времени трёх CI runs и локальные проверки отделены от приёмки ускорения на hosted runners.
Current reference: [Система знаний](../knowledge-system.md)

<a name="ci-performance"></a>

## Измерение времени CI и оптимизация

[Issue #109](https://github.com/lifeFedorovAlexey/KOLVRT/issues/109) находится в работе. [Сохранённый baseline](../../../../research/measurements/ci-legacy-baseline.json) содержит три успешных Windows push runs с одинаковыми Git blob inventories ядра, workspace, scripts, assets, manifests, lockfiles и измеряемого workflow. Документация/evidence различаются; входы сборки — нет. Интервалы выполнения составляют 2002, 1970 и 1829 секунд: медиана 1970 секунд (32 минуты 50 секунд). Существенно более ранний run длительностью 908 секунд исключён, поскольку объём работы ядра отличался. Нагрузка runner не контролируется; это описательные наблюдения полного времени, а не парный вывод об ускорении или измерения CPU компилятора.

Сборщик после завершения workflow сохраняет каждый результат step GitHub и полное время, точные измеренные SHA/run/attempt, пять самых дорогих steps, интервал выполнения и путь объявленного DAG. Он извлекает доверенный код ветки по умолчанию с правами только на чтение и никогда не исполняет артефакты PR, вызвавшего запуск. Workflow измерений становится активным после появления в ветке по умолчанию. Отсутствующие наблюдения остаются недоступными; ошибка API сохраняет недоступность или завершает сбор ошибкой. Авторское сопоставление inventory kernel-130-v1 позволяет автоматически сравнивать полное время старого и разделённого графов; незнакомые графы, events или workflows не приравниваются молча. Сравнение повторов одинаковых исходников по-прежнему требует трёх различных успешных run IDs.

Shell wrapper записывает полное время настройки/скачивания и команд. Необязательный observer xtask записывает отдельные вызовы Cargo build и время жизни процессов QEMU, включая ошибки и timeout, в JSONL-файлы каждого процесса. Сводки сохраняют вложенные категории отдельно; сложение matrix, build и emulator totals учитывало бы работу дважды. Длительность вызова Cargo включает проверки fingerprints/кэша, а не исключительное CPU time rustc. Исключительное время компилятора и начальное ожидание runner остаются UNKNOWN. Длительность DAG суммирует времена jobs вдоль самого длинного объявленного пути зависимостей без очередей runners; интервал выполнения включает ожидания между jobs. Результаты кэша сохраняются отдельно в каждом job.json.

## Границы кэша и конфигураций

Скачивания Cargo registry/git и скомпилированные host/AArch64 artifacts используют версионированные точные keys, включающие Windows OS, Rust 1.99.0, host/guest targets, workload/shard, Cargo.lock, manifests, исходники crates, конфигурацию, build assets и scripts оркестрации. Широкого restore prefix нет. Кэш registry сохраняет только indexes и archives: извлечённые source trees заново создаются Cargo из archives с проверкой checksum из Cargo.lock. Кэш Git содержит только object databases и проверяется git fsck --full; checkouts заново создаются из закреплённых commits. DEV/PROD и feature/negative-control units сохраняют собственные конфигурационные fingerprints Cargo; каждая задача требует нового вызова, а её фактические features/ELF hash проверяются. Разные jobs/shards никогда не используют одну изменяемую директорию сборки. SHA-256 inventory проверяет каждый восстановленный скомпилированный файл до исполнения: добавления, отсутствующие файлы, повреждения и symlinks вызывают ошибку job. Метаданные кэша проверяют согласованность в границах доверия к кэшу GitHub; они не являются сервисом аутентифицированного происхождения артефактов или зависимостью TCB ядра.

Кэшируется только закреплённый архив QEMU. Каждый restore проверяет SHA-512, заново создаёт установку из этого архива и проверяет закреплённый SHA-256 бинарного файла и точную версию 10.1.0 перед записью provenance. Установленные DLLs и бинарные файлы создаются заново, а не принимаются на доверии из кэша распакованной установки. Кэш скачиваний npm сохраняет обязательный npm ci --ignore-scripts. Кэш инструмента аудита Ubuntu закрепляет OS, Rust и cargo-deny 0.19.9, проверяет inventory байтов до исполнения и сверяет версию; при miss инструмент собирается из locked package. Проверки supply chain не пропускаются.

Эксперимент переиспользования сборки выполняет cold, warm, negative-feature, restored-positive и повторные clean builds по одному target path. Он сравнивает ELF hashes выбранных DEV/PROD builds и проверяет, что negative binary отличается. Это не доказательство для всех feature sets или воспроизводимости между машинами. [Документация Rust для sccache](https://github.com/mozilla/sccache/blob/main/docs/Rust.md) требует отключения incremental compilation и исключает crates, вызывающие системный linker, в том числе bin crates. Для бинарных файлов ядра это существенное ограничение; sccache не включается без отдельного измеренного эксперимента пользы/корректности. Review также следует [контракту Cargo build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html).

## Inventory проверок и параллельный граф

| Исходная проверка                                                  | Текущая команда/job               | Решение                                                                    |
| ------------------------------------------------------------------ | --------------------------------- | -------------------------------------------------------------------------- |
| Dependency policy и rejection fixtures                             | check:static                      | Сохранены один раз.                                                        |
| Prettier, форматирование Taplo, cargo fmt, Markdown и Taplo lint   | check:static                      | Сохранены один раз.                                                        |
| Repository validation и impact/history от PR base                  | static                            | Сохранены; требуется настоящий PR base.                                    |
| Workspace Clippy, research tests, cargo test и native-state-models | check:host                        | Сохранены; добавлены rejection tests инфраструктуры CI.                    |
| Подмножество exception registry                                    | Workspace cargo test в host       | Удалена только повторная идентичная команда подмножества; тесты сохранены. |
| Routing all-features и no-default-features tests                   | check:host                        | Обе конфигурации сохранены.                                                |
| DEV diagnostics/kernel-tests architecture lint                     | check:kernel-dev                  | Сохранены точные исходные target/features.                                 |
| PROD release/no-default-features architecture lint                 | check:kernel-prod                 | Сохранены точные исходные target/features.                                 |
| Реальный kernel matrix и распространение ошибок на host            | matrix shards 0–3, затем evidence | Те же 4 positive и 126 negative tasks распределены по одному разу.         |
| Реальный EL0 routing matrix и controls                             | routing                           | Полный специализированный matrix сохранён.                                 |
| Восемь парных реальных 16-bit ASID comparisons                     | asid                              | Сохранены все шестнадцать boots и проверки фактических 16 bits.            |

Локальный npm run check выполняет check:static, затем check:host. check:qemu сохраняет все три дорогих runners, а именованные команды kernel lint доступны независимо. Static gate CI предшествует параллельным jobs host, DEV lint, PROD lint, четырём kernel shards, routing и ASID. Для shards отключён fail-fast. Evidence job независимо генерирует текущий план исходников/задач/конфигураций и отклоняет пропущенные/дублированные tasks или shards, устаревшие исходники, неверные profiles/features, различающиеся ELF hashes или аргументы QEMU. Каждая задача помечена executed. Итоговый статус foundation требует успеха каждого обязательного job; ошибка, отмена, отсутствие или skip не могут превратиться в PASS.

Последовательное и разделённое выполнение ядра используют один inventory задач и исходные обязательные failure predicates, включая разобранное именованное событие ошибки IPC. Частичные receipts shards не заменяют полную приёмку. Все artifacts относятся к текущему run и названы по точному head SHA, workload, shard и attempt. Кэшируемые paths исключают сохранённые результаты QEMU; неизменяемые исторические receipts сохраняются. Receipt ASID копируется в target evidence текущего run только после исполнения, поэтому host/lint jobs не могут опубликовать старый сохранённый ASID receipt как свежий evidence.

## Выполнение и оставшиеся этапы

Push/PR runs используют оптимизированный граф. workflow_dispatch mode=baseline сохраняет последовательный reference path без кэшей, включая его историческое повторное подмножество exception tests. Явный dispatch mode имеет приоритет. Отчёты и raw evidence сохраняются девяносто дней; baselines в репозитории сохраняются постоянно.

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

Повторные ASID boots — независимые статистические наблюдения с чередованием порядка, а не дублированные потребители evidence. Foundation, routing и ASID имеют разные features, payloads или CPU configurations и не могут разделять один positive boot receipt. Replay сохранённых результатов не вводится; каждое наблюдение остаётся executed. Дальнейшее переиспользование QEMU требует действительно идентичного tuple source/ELF/config/profile/features и явной семантики consumers. Планирование по impact сопоставляет точный inventory исходников ядра/сборки/runner с настоящим base и анализирует ownership и prerequisite impact по обоим графам знаний: принятому base и текущему. Изменения CI/verification inputs, kernel authority, неизвестные файлы или недоступный base требуют полного matrix. Пропуск разрешён только при одинаковых kernel inputs для производной навигации или объявленных host-only units без kernel prerequisite impact. Проверка path == docs не используется. При host-only run финальный gate требует успеха static/host и явного skip всех освобождённых kernel jobs; неожиданный failed job не маскируется.

PR feedback при прогретом кэше менее десяти минут и cold CI менее пятнадцати минут остаются целями приёмки до измерения на hosted runners. Оставшиеся обязательные этапы: cold/warm hosted runs, review cache hit/miss и повреждений, сравнение с опубликованным baseline, содержательные ограничения атрибуции и полное смысловое/EN-RU review. Issue остаётся открытым.

[Английский оригинал](../../../../docs/ci/performance.md)

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
        "implementation_scope": "Experimental timing, versioned integrity-checked caches, split local checks, four-way kernel matrix partitioning and exact-source aggregate validation; CI performance acceptance remains pending.",
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
          "research/measurements/ci-build-reuse.json"
        ],
        "issues": [109],
        "adrs": ["adr.0006"],
        "limitations": [
          "Hosted cold/warm acceleration, cache service behavior and semantic/EN-RU review remain pending. No compiler-exclusive CPU attribution or retained QEMU observation replay is claimed; conservative impact scheduling cannot prove semantic completeness."
        ],
        "next_gate": "Run the optimized graph on GitHub, compare cold/warm feedback with the three-run legacy baseline, review cache integrity and complete semantic/EN-RU review before issue closure.",
        "verification": [
          {
            "environment": "github-actions",
            "state": "UNKNOWN",
            "reason": "The reporting workflow has not run on GitHub; local fixture tests cannot establish runner measurements."
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
