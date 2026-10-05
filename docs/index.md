# Documentation map

This is navigation, not a feature-completion checklist. Use the [knowledge contract](knowledge-system.md) and [AI retrieval rules](ai-retrieval.md) for scoped context.

| Area                    | Entry and scope                                                                                                                                                                                      |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Architecture            | [Kernel Laws](architecture/kernel-laws.md), [native model](architecture/native-model.md)                                                                                                             |
| Kernel and processes    | [Process lifecycle](kernel/processes.md), [scheduler](kernel/scheduler.md)                                                                                                                           |
| Memory                  | [User-copy](kernel/user-copy.md), [MMU/memory](kernel/memory.md)                                                                                                                                     |
| Authority               | [Handles](kernel/handles.md), [capability scope](kernel/capabilities.md)                                                                                                                             |
| Drivers                 | [Platform contract](architecture/platform-contract.md); future device/service acceptance                                                                                                             |
| Storage and networking  | [Native slice baseline](architecture/first-native-slice.md); planned services                                                                                                                        |
| Security                | [Threat model](security/threat-model.md), [inspection model](security/inspection-model.md)                                                                                                           |
| Compatibility           | [Compatibility model](architecture/compatibility-model.md), [routing](kernel/routing.md)                                                                                                             |
| Linux research          | [Mechanism template](../research/linux/mechanism-template.md), [allocation](../research/linux/memory-allocation.md), [pinned driver observations](../research/compatibility/linux-driver-api-map.md) |
| COST-L                  | [Debt registry contract](architecture/compatibility-debt.md)                                                                                                                                         |
| Arena                   | [Arena design and roadmap](architecture/component-arena.md); PLANNED, no runner or measured passports                                                                                                |
| ADRs                    | [Decision index](architecture-decisions/README.md)                                                                                                                                                   |
| Benchmarks and evidence | [Benchmark method](architecture/benchmarking.md), [measurement history](../research/measurements/README.md)                                                                                          |
| Hardware research       | [Platform requirements](architecture/platform-contract.md), [research questions](research/open-questions.md); no physical verification implied                                                       |
| Migration               | [Host advisor](architecture/migration-advisor.md); production integrations remain separately gated                                                                                                   |
| Historical evidence     | [Historical architecture audit](architecture/architecture-audit-2026-10-04.md), [Phase 0 report](research/phase-0-1-report.md); retain its original milestone scope                                  |

[Russian translation](../translations/ru/docs/index.md)
