# KOLVRT

**Kernel Outside Legacy, Versioned Routing & Translation** — исследовательский проект
ARM64-first kernel на Rust. Phase 0.1: архитектура и Linux archaeology, **ядра пока нет**.

> The kernel must not adapt itself to legacy. Legacy compatibility must adapt itself to the kernel.

Это не Linux fork и не перенос внутренних Linux subsystems на Rust. Linux используется
как источник проверяемого инженерного опыта, включая успешные redesign и необходимые компромиссы.

- [Vision](docs/vision.md), [40 Kernel Laws](docs/architecture/KERNEL_LAWS.md).
- [Native model](docs/architecture/native-model.md), [compatibility](docs/architecture/compatibility-model.md), [routing](docs/architecture/routing-model.md).
- [Execution profiles](docs/architecture/execution-profiles.md), [unsafe policy](docs/architecture/unsafe-policy.md).
- [Benchmark methodology](docs/architecture/benchmarking.md), [diagnostics and score](docs/architecture/diagnostics.md).
- [30 cases: таблица и решения](docs/research/CASE_INDEX.md), [первичные JSON](research/pathology/).
- [ADR index](docs/adr/README.md), [другие ОС](research/other-systems/COMPARISON.md).
- [Методология и покрытие](research/linux/COVERAGE.md), [источники](research/sources/README.md).
- [Unresolved decisions](docs/research/OPEN_QUESTIONS.md), [Phase 0.2 backlog](docs/research/PHASE_0_2.md).
- [Отчёт Phase 0.1 и границы проверок](docs/research/PHASE_0_1_REPORT.md).

Дата исследования: **2026-10-02**. Неизвестные introducing commits и даты помечены явно.
Документальные наблюдения не равны воспроизведённым багам. Будущие kernel tests описаны
как требования; выполнены только проверки исследовательских данных и tooling.

## Проверка

Python 3.11+ и jsonschema 4.26.0:

```text
python -m pip install -r tools/pathology/requirements.txt
python tools/pathology/validate.py
python -m unittest discover -s tools/pathology -p "test_*.py" -v
python tools/pathology/report.py --check
python tools/pathology/check_docs.py
```

Валидатор работает без сети. При изменении JSON обновить индекс:
`python tools/pathology/report.py`. Installation command нужен только если зависимости
ещё не установлены. Tooling не загружает и не запускает exploit reproducers.

Phase 1 автоматически не начинается. Лицензия будущего проекта ещё не выбрана;
никакие права на Linux source этим репозиторием не переоформляются.
