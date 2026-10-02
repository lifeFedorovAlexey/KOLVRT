# Execution profiles

DEV, STAGING и PROD используют одну native semantics, один graph boundaries и одну
codebase. Профиль изменяет instrumentation и допуск экспериментальных операций,
но не правильность, authorization, memory ordering или resource limits.

| Возможность | DEV / DIAGNOSTIC | STAGING | PROD |
|---|---|---|---|
| Detailed tracing, live routing view | Да, права и quotas | Bounded/sampled | Только отдельно разрешённые bounded counters |
| Invariant assertions | Полные диагностические | Полные/целевые с budget | Необязательные diagnostic assertions устраняются |
| Mandatory request validation | Всегда | Всегда | Всегда |
| Fault injection | Явно активируемая | Для изолированного pre-release теста | Не включена |
| Experimental live rebind | Только quiescent domains | Controlled migration rehearsal | Отсутствует |
| A/B | Изолированные copies workload | Representative fixtures | Offline replay/отдельный стенд |
| Dependency graph | Полный | Полный manifest | Bound manifest, без hot-path graph traversal |
| CPU/memory overhead budgets | Измеряются | Release gate | Минимальные обязательные механизмы |

STAGING принят как конфигурация сборки/развёртывания: production optimizer, allocator,
route manifests и compiler flags плюс дозированная observability. Он выявляет ошибки,
скрытые debug timing. Отдельный staging kernel architecture запрещён.

Compile-time elimination допустим для tracing, counters и expensive diagnostic scans.
Не допустимо удалять bounds checks, permission checks, reference validity, DMA barriers,
quota enforcement или protocol validation как «fake checks». Доказанная compiler
elimination redundant checks допустима; отсутствие потребности подтверждается invariant,
а не названием PROD. Fault/cancel behavior совпадает во всех profiles.

Instrumentation не может быть нужна для progress: lock release, wakeup, lifetime и
ownership не зависят от logging. Каждый backend должен проходить semantic contract suite
с diagnostics ON/OFF и optimization debug/release. Native-only и compat-enabled suites
должны проверять один native oracle. На Phase 0.1 это требования к будущему CI.

В полностью stripped PROD динамические доли calls/CPU неизвестны. Diagnostic tool обязан
показать `unavailable`, а не ноль. Static manifest dependency graph остаётся доступным
offline. Нельзя одновременно обещать полную online телеметрию и нулевую instrumentation.
