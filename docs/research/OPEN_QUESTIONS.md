# Спорные и unresolved решения

Эти вопросы не замаскированы реализацией. Phase 0.1 фиксирует contracts и evidence;
открытый выбор не даёт права заполнить working path заглушкой.

| ID | Вопрос | Текущее решение | Доказательство для закрытия |
|---|---|---|---|
| OQ-01 | Kernel/userspace placement adapters и drivers | ADR-0008 Proposed | Threat model + две реализации одного host workload, latency и containment |
| OQ-02 | Нативные fork/signals/process primitives | Spawn handle принят; fork открыт | Linux personality state graph и точные нужные primitives |
| OQ-03 | Mixed fd/locks/credentials domains | Независимый per-call split запрещён | Model-check interop и explicit gateway semantics |
| OQ-04 | USB persistence identity | Case 14 RESEARCH_REQUIRED; conservative reconnect | Device replacement/recovery model без false identity |
| OQ-05 | Shared accounting | Один identity; charge attribution открыта | Shared cache, DMA и deferred work ledger model |
| OQ-06 | RCU/epoch/refcount выбор | API lifetime requirement принят, алгоритм открыт | Contention model + bounded memory reclamation |
| OQ-07 | Native scheduler objectives | Visible domains приняты, алгоритм открыт | Fairness/latency objectives и adversarial workloads |
| OQ-08 | Arm barriers/MMU/GIC/timer/VirtIO specification | Linux evidence только исходная база | Arm и VirtIO primary spec clauses, litmus и pin версии |
| OQ-09 | QEMU/toolchain exact versions | Не выбраны | Reproducible environment manifest; EL1/GICv3/SMP capabilities |
| OQ-10 | Unsafe panic/unwind/allocation policy | Не выбрана | Error/context matrix; не unwind через недоказанную FFI/interrupt границу |
| OQ-11 | Compat consumers и retirement windows | Только evidence-driven | Реальный workload inventory; offline/dormant dependency accounting |
| OQ-12 | MOSTLY_NATIVE budgets | Нет глобальных arbitrary thresholds | Consumer migration SLO, coverage и multidimensional limits |
| OQ-13 | Исторические даты/commits | Null честно сохранён | Introducing/fix/revert chain и tag containment для каждого case |
| OQ-14 | Syzbot no-op fix bisection, LKML access | Case 30 causal claim ограничен | Local isolated replay и доступный maintainer thread |
| OQ-15 | Hardware support и лицензия | Нет обещания всех плат; license не выбрана | Product scope и явное решение владельца проекта |

Все case-specific вопросы автоматически перечислены в [CASE_INDEX](CASE_INDEX.md).
Сравнение других систем имеет отдельные ограничения: [COMPARISON](../../research/other-systems/COMPARISON.md).
Ни одно «unknown» не означает, что соответствующего Linux commit или проверки не существует.
