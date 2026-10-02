# Execution profiles

Development, staging and production use one architecture, codebase and native semantics.
Profiles control diagnostics and experimental operations, not authorization, ownership,
ordering, resource limits or correctness.

| Capability                            | Development              | Staging                          | Production                                  |
| ------------------------------------- | ------------------------ | -------------------------------- | ------------------------------------------- |
| Detailed tracing and route inspection | Authorized, bounded      | Bounded or sampled               | Explicitly enabled bounded counters only    |
| Diagnostic assertions                 | Full                     | Full or targeted within a budget | Optional diagnostics may be removed         |
| Mandatory request validation          | Always                   | Always                           | Always                                      |
| Fault injection                       | Explicit activation      | Isolated pre-release tests       | Absent                                      |
| Experimental live rebinding           | Quiescent domains only   | Controlled rehearsal             | Absent                                      |
| A/B experiments                       | Isolated workload copies | Representative fixtures          | Separate test environment or offline replay |
| Dependency information                | Full graph               | Full manifest                    | Bound manifest, no per-call graph traversal |

Staging uses production optimization, allocation policies and route manifests with
bounded observability. It is a configuration, not another kernel architecture. Diagnostic
timing can hide defects, so one contract suite must cover diagnostics on/off and both
debug and optimized builds, with and without software compatibility.

Compile-time elimination may remove tracing, optional counters and expensive diagnostic
scans. It must not remove bounds, permissions, handle validity, quotas, protocol validation
or hardware barriers merely because a build is called production. A proven redundant
check may be optimized away; the proof comes from invariants, not a profile name.

Progress, wakeups, lock release and lifetime must not depend on logging. Fault and
cancellation outcomes remain consistent. Fully stripped production builds cannot provide
dynamic measurements that were not collected: report unavailable rather than zero.
Offline manifests still describe declared dependencies. The bounded EL0 routing profile is implemented; it is not a global security or service policy.

The [Phase 2 profile matrix](../kernel/routing.md) distinguishes PROD evidence images from stripped deployment-shaped images; no DEV switching or A/B instrumentation is required for correctness.

[Russian translation](../../translations/ru/docs/architecture/execution-profiles.md)
