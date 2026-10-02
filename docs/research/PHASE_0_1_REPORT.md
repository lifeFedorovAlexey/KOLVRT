# Phase 0.1 — результат и границы

Дата: 2026-10-02. Создан исследовательский repository foundation: 30 отдельных Linux
cases, JSON Schema, offline validator, 40 Kernel Laws, 8 ADR, architecture models и
сравнение 7 направлений OS design. **Kernel implementation отсутствует. Phase 1 не начата.**
Исходное задание сохранено в [phase-0-request.txt](phase-0-request.txt).

## Проверки

Выполненные checks для итогового дерева:

| Команда | Результат |
|---|---|
| `python tools/pathology/validate.py` | 30 cases: schema и consistency проходят |
| `python -m unittest discover -s tools/pathology -p "test_*.py" -v` | 18 tests проходят, включая negative mutations и CLI failure |
| `python tools/pathology/report.py --check` | Индекс соответствует JSON |
| `python tools/pathology/check_docs.py` | Local links, source ledger, 40 laws и 8 ADR проверены |
| `git diff --check` и staged equivalent | Whitespace errors отсутствуют |

Checks не запускают historical kernels, reproducers, benchmark workloads, Rust/Miri,
QEMU или real hardware. В частности, наличие `tests_required` — обязательство будущего
теста, а не отчёт PASS этого теста. Historical truth проверяется по sources вручную.

## Дерево содержимого

```text
D:\KOLVRT
├── README.md
├── .gitignore
├── docs
│   ├── vision.md
│   ├── architecture
│   │   ├── KERNEL_LAWS.md
│   │   ├── native-model.md
│   │   ├── compatibility-model.md
│   │   ├── routing-model.md
│   │   ├── execution-profiles.md
│   │   ├── unsafe-policy.md
│   │   ├── benchmarking.md
│   │   └── diagnostics.md
│   ├── adr
│   │   ├── README.md
│   │   └── 0001 … 0008 (8 отдельных ADR)
│   └── research
│       ├── CASE_INDEX.md
│       ├── OPEN_QUESTIONS.md
│       ├── PHASE_0_1_REPORT.md
│       ├── PHASE_0_2.md
│       └── phase-0-request.txt
├── research
│   ├── linux/COVERAGE.md
│   ├── other-systems/COMPARISON.md
│   ├── pathology
│   │   ├── README.md
│   │   └── KOL-PATH-0001 … KOL-PATH-0030.json
│   └── sources
│       ├── README.md
│       ├── linux-sources.json
│       └── other-sources.json
├── schemas/pathology
│   ├── README.md
│   └── case.schema.json
└── tools/pathology
    ├── requirements.txt
    ├── validate.py
    ├── test_validate.py
    ├── report.py
    └── check_docs.py
```

## Decisions

Полная [таблица всех cases и группировка](CASE_INDEX.md) генерируется из JSON.

| Decision | Cases |
|---|---:|
| NATIVE_FIX | 12 |
| COMPAT_ONLY | 6 |
| HARDWARE_TRANSLATION | 4 |
| ACCEPTED_TRADEOFF | 6 |
| RESEARCH_REQUIRED | 1 |
| NOT_APPLICABLE | 1 |

Обоснование: Linux не рассматривается как набор ошибок. DMA ordering, RCU и multiqueue
содержат реальные tradeoffs; internal driver API evolution и удаление некоторых ABI
показывают успешное устранение старых ограничений. Hardware quirk может требоваться
native target. Это не то же самое, что software compatibility dependency.

## Архитектурные результаты

- [40 законов](../architecture/KERNEL_LAWS.md): у каждого Rule, Rationale, Historical
  evidence, Prevents, Allowed exceptions, Enforcement, Testing.
- [ADR-0001](../adr/0001-native-authority.md): native authority и quarantine.
- [ADR-0002](../adr/0002-stateful-routing.md): routes по state domains.
- [ADR-0003](../adr/0003-profiles.md): DEV/STAGING/PROD одной codebase.
- [ADR-0004](../adr/0004-unsafe.md): safe Rust и audited boundaries.
- [ADR-0005](../adr/0005-platform.md): ARM64-first platform contracts.
- [ADR-0006](../adr/0006-metrics.md): dependency vector и честный A/B.
- [ADR-0007](../adr/0007-bug-compat.md): native fix, bug-compat lifecycle и removal.
- [ADR-0008](../adr/0008-placement.md): placement остаётся Proposed / RESEARCH_REQUIRED.

## Definition of Done: scope

Структура, vision, native/compat/profiles, unsafe policy, schema/validator, 30 cases с
источниками и decisions, laws, ADR, unresolved register и Phase 0.2 backlog созданы.
Benchmark numbers не выдуманы; runtime implementation не заявлена.

Это завершение документального среза Phase 0.1, не закрытие всех исторических вопросов.
Точные introducing commits многих старых API неизвестны; часть docs не pinned на immutable
revision; LKML thread case 30 недоступен; syzbot no-op bisection требует повторения.
Не найден доказанный case «workaround пережил исчезновение всего поддерживаемого hardware»;
он отдельно в backlog, а не выдан за установленный факт. Ограничения сравнения других
систем тоже сохранены, включая отсутствие конкретных failure postmortems для некоторых.

Спорные решения и критерии их закрытия: [OPEN_QUESTIONS](OPEN_QUESTIONS.md).
Следующая работа: [Phase 0.2 research backlog](PHASE_0_2.md). Начинать её или kernel Phase 1
автоматически этот отчёт не предписывает.
