# KOLVRT

## Kernel Outside Legacy, Versioned Routing & Translation

KOLVRT — экспериментальное ядро операционной системы на Rust. Основная платформа — ARM64.

> **Старое ПО может работать, но не диктует устройство ядра.**

У собственного API ядра есть явные контракты. Историческое поведение сохраняется в удаляемых слоях совместимости с отдельными версиями. Linux и другие ОС служат материалом для исследований, а не образцом для копирования.

## Текущее состояние

Ядро загружается на QEMU `virt` и запускает изолированные процессы EL0 на двух процессорах. Это пока исследовательское ядро: стабильного пользовательского ABI, совместимости с Linux и проверки на физическом ARM64 ещё нет.

| Область                      | Что реализовано                                                                                                                           |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Загрузка и оборудование      | EL1, проверка Device Tree, PL011 UART, исключения, GICv3 и прерывания таймера                                                             |
| Память                       | Распределение физических страниц, таблицы страниц, MMU, W^X и куча ядра                                                                   |
| Работа двух процессоров      | Запуск, IPI, освобождение отображений после подтверждённого сброса TLB и выключение                                                       |
| Процессы и планировщик       | Изолированные адресные пространства, вытеснение по таймеру, закрепление за процессором, жизненный цикл и ожидание собственного события    |
| Доступ к памяти пользователя | Копирование с явными пределами, неизменяемые снимки запросов и восстановление после ошибок доступа                                        |
| Дескрипторы и безопасность   | Локальные ссылки, передача с уменьшением прав, разрешения с заданной областью действия, отзыв, квоты и сохранение результатов уведомлений |
| ELF и ASID                   | Ограниченный загрузчик AArch64, проверка выравнивания точки входа и безопасное освобождение и повторное использование ASID                |
| Версионированные вызовы      | Пути native/v1/v2 в изолированных задачах EL0; ядро не зависит от адаптеров                                                               |
| Инструменты разработки       | Планировщик миграции, поиск по документации и запросы к реестру COST-L без сети                                                           |
| Чистота сборки               | Предупреждения запрещены в сборках AArch64 для DEV, PROD и отрицательных контролей                                                        |

Bounded IPC Phase 3.5 реализован и готов для объявленного контракта: изолированные EL0 request/response, blocking/wakeup, отмена, deadlines, death/shutdown arbitration и retained ownership. Supervision policy (#27) и persistent isolated services (#28) — следующие gates; physical ARM и stable ABI имеют отдельный scope.

Ранний код не определяет архитектуру следующих этапов. Перед расширением решение выводится из действующих инвариантов и принятых решений; мешающий им код перерабатывается или удаляется.

Экспериментальный [supervisor EL0](docs/kernel/supervision.md) реализует bounded static service workload со fresh lifecycle identities, аутентифицированным readiness, конечным restart/backoff и честным shutdown. Human architecture и complete EN/RU acceptance задачи #27 ещё не завершены; #28 отвечает за persistent integration, #38 — за production bootstrap trust. Physical ARM64 и stable ABI остаются отдельными gates.

## Архитектура

- Поведение ядра определяется действующими контрактами и не зависит от совместимости.
- Совместимость явная, версионированная и удаляемая; у каждого требования указан потребитель.
- Разные потребители и семейства API могут одновременно использовать разные пути вызовов.
- Предпочтителен безопасный Rust. Для каждого участка `unsafe` указаны инвариант, владелец и способ проверки.
- Заявления о производительности требуют измерений; особенности оборудования не становятся общим правилом без явного решения.

Нормативные требования — в [законах ядра](docs/architecture/kernel-laws.md). Подробнее: [модель ядра](docs/architecture/native-model.md), [модель совместимости](docs/architecture/compatibility-model.md) и [аудит unsafe](docs/kernel/unsafe.md).

## Совместимость и миграция

Приложение может использовать собственные API ядра для памяти и сети, а один устаревший API вызывать через `compat-v2`. Совместимость выбирается для потребителя и семейства API, а не для всей системы. [Контракт маршрутизации](docs/kernel/routing.md) описывает реализованный вариант EL0 и его ограничения.

DEV предназначен для исследования, просмотра путей вызовов и внедрения ошибок. PROD использует оптимизацию и заранее выбранные пути без экспериментального переключения. Архитектура у профилей общая.

Стоимость совместимости измеряется без искусственного замедления: задержки, работа процессора, память, копирования и преобразования учитываются отдельно. Результаты QEMU не доказывают производительность на оборудовании.

[Советник миграции](docs/architecture/migration-advisor.md) проверяет результаты и контракты, сравнивает измерения и предлагает план отката. Он работает только на чтение: подписанное разрешение не устанавливает пакеты и не меняет пути вызовов. Интеграция с рабочей ОС, телеметрия, выполнение решений, безопасное хранение ключей, проверка статистики и испытания на физическом ARM64 остаются задачами [#14](https://github.com/lifeFedorovAlexey/KOLVRT/issues/14).

[Утилита COST-L](docs/research/cost-l-queries.md) предоставляет команды `show`, `consumers`, `deps`, `list` и `top`, JSON, фильтры и ограниченный вывод. Она читает проверенные записи реестра, а не состояние работающей системы. Стандартные запросы проверяют взаимные ссылки манифестов модулей; `--manifests PATH` явно объединяет пользовательские перечни. Production-данные/полнота и принятие человеком остаются открытыми в [#47](https://github.com/lifeFedorovAlexey/KOLVRT/issues/47) и [#48](https://github.com/lifeFedorovAlexey/KOLVRT/issues/48).

## Сборка и проверки

[Локальный сборщик](docs/research/host-survey.md) формирует исследовательские отчёты из разрешённых полей, с указанием пробелов и локальной проверкой. Сведения о компьютере не подтверждают поддержку оборудования в KOLVRT.

Команды выполняются из корня репозитория. Нужны Rust **1.99.0** с `rustfmt`, `clippy` и целью `aarch64-unknown-none`, Node.js **18+** и QEMU **10.1.0**. Для автоматической настройки Windows также нужен 7-Zip.

```powershell
rustup toolchain install 1.99.0 --profile minimal --component rustfmt --component clippy
rustup target add aarch64-unknown-none --toolchain 1.99.0
./scripts/setup-qemu.ps1
npm ci --ignore-scripts
npm run check
cargo xtask test
```

`npm run check` проверяет форматирование, стиль кода, тесты на машине разработчика, модели и документацию. Сборки Cargo для AArch64 используют `-D warnings`: предупреждение останавливает компиляцию.

```powershell
cargo xtask routing test
cargo xtask asid-bench
cargo xtask debug
cargo run --locked -p repository-checks -- cost-l list --json
```

Программа проверки ядра требует **97 тестов в каждом профиле DEV/PROD**, обе обычные загрузки и **82 отрицательных контроля**. Контроли намеренно нарушают защиту и должны завершаться ожидаемым отказом. Пропущенные события, неожиданные паники, ошибки эмулятора и тайм-ауты завершают проверку ошибкой.

Сохранённый [прогон проверки выравнивания ELF](../../research/measurements/runs/1791171892998-issue70-elf-entry-alignment-1cd2f1cf8717.json) фиксирует эти числа для конкретных хешей исходников. Он не подтверждает более поздние версии или работу на физическом оборудовании. В `target/kernel/` сохраняются ELF-образы, хеши, события UART, настройки QEMU, размеры сборок, перечень участков unsafe и выборки измерений. Подробнее — в [руководстве по проверкам и GDB](docs/kernel/testing.md).

## Документация и исследования

| Каталог         | Содержимое                                                       |
| --------------- | ---------------------------------------------------------------- |
| `crates/`       | Реализация на Rust                                               |
| `docs/`         | Архитектура, контракты подсистем и принятые решения              |
| `research/`     | Исследования ОС с источниками и измерения с указанными границами |
| `schemas/`      | Машиночитаемые схемы данных                                      |
| `scripts/`      | Настройка среды и инструменты разработки                         |
| `translations/` | Переводы документации                                            |
| `assets/`       | Оформление проекта                                               |

Начните с [идеи проекта](docs/vision.md), [карты документации](docs/index.md) и [архитектурных решений](docs/architecture-decisions/). Подсистемы: [загрузка](docs/kernel/boot.md), [EL0](docs/kernel/el0.md), [планировщик](docs/kernel/scheduler.md), [процессы](docs/kernel/processes.md), [ожидание](docs/kernel/wait.md), [копирование памяти](docs/kernel/user-copy.md), [дескрипторы](docs/kernel/handles.md) и [домены](docs/kernel/domains.md).

[База исследований](../../research/) разбирает отказы, ограничения ABI, параллельное выполнение, особенности оборудования и устаревшие интерфейсы. Для каждого случая выясняется, что произошло, какие ограничения сохранились и нужно ли это поведение ядру, слою совместимости или никому.

## Участие и лицензия

Архитектурные изменения приветствуются при наличии обоснования. Для исключения совместимости нужно указать потребителя, причину размещения в ядре, срок существования, измеренную стоимость, способ удаления и защиту остальных потребителей от этой стоимости. Правила — в [CONTRIBUTING.md](CONTRIBUTING.md).

Лицензия проекта пока не выбрана. Сторонние материалы сохраняют свои лицензии.

## Проверка функций

Таблица из реестра разделяет реализацию и проверку. `BOUNDED_IMPLEMENTED` — реализовано с указанными ограничениями, `PLANNED` — запланировано. `VERIFIED` относится к записанным исходникам и среде, `STALE` означает, что свидетельства не покрывают текущий код, `UNKNOWN` — результат не установлен, `NOT_APPLICABLE` — среда неприменима. Готовность оценивается по контракту, на который ведёт ссылка.

<!-- feature-summary:start -->

| Каноническая функция                                                                                          | Реализация          | Граница доказательств                                                     |
| ------------------------------------------------------------------------------------------------------------- | ------------------- | ------------------------------------------------------------------------- |
| [kolvrt.arena](docs/architecture/component-arena.md#kolvrt-arena-scope)                                       | EXPERIMENTAL        | host-process: STALE; physical-arm64: UNKNOWN                              |
| [kolvrt.ci.performance](docs/ci/performance.md#ci-performance)                                                | EXPERIMENTAL        | github-actions: UNKNOWN                                                   |
| [kolvrt.compatibility.manifests](docs/architecture/compatibility-manifests.md#kolvrt-compatibility-manifests) | BOUNDED_IMPLEMENTED | host-process: STALE; physical-arm64: NOT_APPLICABLE                       |
| [kolvrt.cost-l.offline-queries](docs/research/cost-l-queries.md#kolvrt-cost-l-offline-queries)                | BOUNDED_IMPLEMENTED | host-process: STALE; physical-arm64: NOT_APPLICABLE                       |
| [kolvrt.dependencies.hygiene](docs/architecture/dependency-hygiene.md#kolvrt-dependency-hygiene)              | BOUNDED_IMPLEMENTED | host-process: STALE; physical-arm64: NOT_APPLICABLE                       |
| [kolvrt.docs.navigation](docs/knowledge-system.md#kolvrt-docs-navigation)                                     | BOUNDED_IMPLEMENTED | host-process: VERIFIED; physical-arm64: NOT_APPLICABLE                    |
| [kolvrt.handles.local](docs/kernel/handles.md#kolvrt-handles-local)                                           | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |
| [kolvrt.ipc.transport](docs/kernel/ipc.md#bounded-native-ipc)                                                 | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |
| [kolvrt.memory.user-copy](docs/kernel/user-copy.md#kolvrt-memory-user-copy)                                   | BOUNDED_IMPLEMENTED | qemu-arm64: VERIFIED; physical-arm64: UNKNOWN                             |
| [kolvrt.process.lifecycle](docs/kernel/processes.md#kolvrt-process-lifecycle)                                 | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |
| [kolvrt.research.host-survey](docs/research/host-survey.md#kolvrt-host-survey)                                | BOUNDED_IMPLEMENTED | host-process: STALE; windows-cim: UNKNOWN; physical-arm64: NOT_APPLICABLE |
| [kolvrt.security.capability-revocation](docs/kernel/capabilities.md#kolvrt-security-capability-revocation)    | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |
| [kolvrt.security.domains](docs/kernel/domains.md#kolvrt-domains-scope)                                        | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |
| [kolvrt.security.event-revocation](docs/kernel/capabilities.md#kolvrt-security-event-revocation)              | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |
| [kolvrt.security.verifier-time](docs/security/verifier-time.md#kolvrt-verifier-time)                          | BOUNDED_IMPLEMENTED | host-process: STALE; physical-arm64: NOT_APPLICABLE                       |
| [kolvrt.services.supervision](docs/kernel/supervision.md#isolated-el0-supervision)                            | EXPERIMENTAL        | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |

<!-- feature-summary:end -->

[Английский оригинал](../../README.md)
