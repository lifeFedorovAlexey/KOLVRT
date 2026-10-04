# Карта документации

Это навигация, не список завершённых функций. Для контекста в нужных границах используйте [контракт знаний](knowledge-system.md) и [правила AI retrieval](ai-retrieval.md).

| Область               | Вход и границы                                                                                                                                                                                             |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Архитектура           | [Kernel Laws](architecture/kernel-laws.md), [native model](architecture/native-model.md)                                                                                                                   |
| Ядро и процессы       | [Жизненный цикл](kernel/processes.md), [планировщик](kernel/scheduler.md)                                                                                                                                  |
| Память                | [User-copy](kernel/user-copy.md), [MMU/память](kernel/memory.md)                                                                                                                                           |
| Полномочия            | [Handles](kernel/handles.md), [границы capabilities](kernel/capabilities.md)                                                                                                                               |
| Драйверы              | [Контракт платформы](architecture/platform-contract.md); будущая проверка устройств/служб                                                                                                                  |
| Хранение и сеть       | [Базис нативного среза](architecture/first-native-slice.md); планируемые службы                                                                                                                            |
| Безопасность          | [Модель угроз](security/threat-model.md), [инспекция](security/inspection-model.md)                                                                                                                        |
| Совместимость         | [Модель совместимости](architecture/compatibility-model.md), [routing](kernel/routing.md)                                                                                                                  |
| Linux research        | [Шаблон механизма](../research/linux/mechanism-template.md), [allocation](../research/linux/memory-allocation.md), [фиксированные наблюдения драйверов](../research/compatibility/linux-driver-api-map.md) |
| COST-L                | [Контракт реестра](architecture/compatibility-debt.md)                                                                                                                                                     |
| ADRs                  | [Индекс решений](architecture-decisions/README.md)                                                                                                                                                         |
| Benchmarks и evidence | [Метод измерений](architecture/benchmarking.md), [история](../research/measurements/README.md)                                                                                                             |
| Hardware research     | [Требования платформы](architecture/platform-contract.md), [вопросы](research/open-questions.md); аппаратная проверка не подразумевается                                                                   |
| Миграция              | [Host advisor](architecture/migration-advisor.md); production integration проверяется отдельно                                                                                                             |
| Historical evidence   | [Отчёт Phase 0](research/phase-0-1-report.md); сохраняет исходные границы этапа                                                                                                                            |

[Английский оригинал](../../../docs/index.md)
