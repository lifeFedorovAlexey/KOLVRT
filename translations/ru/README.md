# KOLVRT

**Kernel Outside Legacy, Versioned Routing & Translation** — исследовательский проект операционной системы с приоритетом ARM64 и первоначальной реализацией на Rust. Нулевой этап определяет проверенную собственную проектную основу и свидетельства для первого сквозного сценария. **Самостоятельное ядро EL1 теперь загружается и тестируется в QEMU; активен один CPU.**

Ядро не должно подстраиваться под наследуемые системы. Совместимость с ними должна подстраиваться под ядро. KOLVRT определяет собственную модель объектов, полномочий, времени жизни и исполнения. Внешние системы служат материалом для исследования механизмов отказа, а не требованиями продукта или архитектурами для наследования. Возможность должна продвигать конкретную нагрузку KOLVRT, обязательство правильности или потребность платформы; популярность и выживание не доказывают пригодность.

- [Ревью нулевого этапа](docs/research/phase-0-review.md), [первый собственный сценарий](docs/architecture/first-native-slice.md), [структура репозитория](docs/project-structure.md).
- [Загрузка ядра](docs/kernel/boot.md), [проверки ядра](docs/kernel/testing.md), [ревью методов](docs/architecture/implementation-review.md), [unsafe-инварианты](docs/kernel/unsafe.md).
- [Замысел](docs/vision.md), [законы ядра](docs/architecture/kernel-laws.md), [пересмотр законов](docs/architecture/law-review.md).
- [Собственная модель](docs/architecture/native-model.md), [совместимость](docs/architecture/compatibility-model.md), [маршрутизация](docs/architecture/routing-model.md).
- [Профили исполнения](docs/architecture/execution-profiles.md), [политика безопасности Rust](docs/architecture/unsafe-policy.md).
- [Измерения производительности](docs/architecture/benchmarking.md), [диагностика](docs/architecture/diagnostics.md).
- [Указатель случаев](docs/research/case-index.md), [исследовательские записи](docs/research/case-database.md).
- [Архитектурные решения](docs/architecture-decisions/README.md), [другие системы](docs/research/reference-systems.md).
- [Метод исследования](docs/research/research-method.md), [охват](docs/research/reference-coverage.md), [источники](docs/research/source-ledger.md).
- [Открытые вопросы](docs/research/open-questions.md), [план этапа 0.2](docs/research/phase-0-2.md).
- [Отчёт этапа 0.1](docs/research/phase-0-1-report.md), [документация и переводы](docs/documentation-policy.md).

Дата исследования: **2026-10-02**. Неизвестные вводящие изменения и даты обозначены явно. Документальные выводы не являются воспроизведёнными ошибками. Записи моделей Phase 0 остаются проектными свидетельствами; текущие запуски ядра генерируются xtask.

## Проверки

Установить закреплённый инструментарий Rust через rustup и Node.js 18 или новее, затем выполнить из корня репозитория:

```text
npm ci --ignore-scripts
npm run check
```

Для первой сборки требуется загрузить зависимости. Сама проверка работает без сети и никогда не загружает и не запускает средства воспроизведения уязвимостей. [Форматирование и участие](CONTRIBUTING.md), отдельные команды и порядок проверки переводов описаны в [инструкции инструментария](crates/repository-checks/README.md).

Запустите `cargo xtask test` после настройки платформы, описанной выше. SMP, scheduler, userspace и compatibility не начинаются автоматически. Лицензия проекта ещё не выбрана; сторонние материалы сохраняют исходные лицензии.

[Английский оригинал](../../README.md)
