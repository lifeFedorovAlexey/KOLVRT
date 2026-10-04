# Native scenario audit

The audit covers the lifecycle of the first native slice: construct, authorize, admit, execute, wait, cancel, revoke, fail, reclaim and shut down. It checks whether each scenario has an obligation, a design decision and a validation method. It does not prove completeness for every future OS feature. Law count is the consequence of this review, not an acceptance threshold.

## Coverage

| Law     | Native scenario or boundary                  | Evidence and remaining implementation test                                                                              |
| ------- | -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| LAW-001 | Core without external contracts              | Native-only protocol/models build; future kernel dependency audit                                                       |
| LAW-003 | Concurrent consumers with different bindings | Binding identity carried by domain transfer; compatibility routes are outside this slice                                |
| LAW-004 | Unknown version or operation                 | Decoder rejection tests; future loader resolves exact implementation identity                                           |
| LAW-005 | Handle crossing a domain                     | Domain transfer preserves object and binding; attenuation and failed-transfer tests                                     |
| LAW-008 | Changing a live binding                      | Exhaustive drain model and premature-retirement counterexample; live reload deferred                                    |
| LAW-009 | Unauthorized delegation                      | Exhaustive small rights combinations and caller-local lookup tests; real MMU separation pending                         |
| LAW-013 | Close racing execution and reuse             | Lifetime exploration, stale-generation checks, premature-free counterexample                                            |
| LAW-018 | Mutated, truncated or malformed request      | Phase 3.2 verifies bounded user-copy faults and immutable snapshot ownership; general malformed-message fuzzing remains |
| LAW-020 | Device retains memory after timeout          | Lease obligation retained; no DMA device enabled in first slice; device experiment required before activation           |
| LAW-025 | Competing requests exhaust capacity          | One-slot admission and charge invariant; lost-charge counterexample; kernel allocation-failure injection pending        |
| LAW-026 | Cancel races effect or service death         | Lifetime terminal states and host process failure; durable external effects deferred                                    |
| LAW-027 | Ready tasks compete                          | All nonempty three-task ready sets tested; timer and preemption assumptions explicit                                    |
| LAW-030 | Event occurs between probe and registration  | Finite wait graph and lost-wakeup counterexample; hardware interrupt interleavings pending                              |
| LAW-031 | Boot data or replaced device                 | Platform parser contract; hotplug outside slice; real descriptor parser tests before use                                |
| LAW-035 | Optimized build retains correctness          | Same model and decoder suites in debug/release; no runtime diagnostic dependency                                        |
| LAW-036 | Missing measurements                         | Recorded state counts and traces; no latency, sandbox or hardware performance claim                                     |
| LAW-040 | Reference evidence biases design             | Research decision filter, opposing cases and documented source gaps                                                     |
| LAW-041 | Progress depends on locks or cleanup         | Lock hierarchy, bounded pools, ready-set enumeration; real critical-section timing pending                              |
| LAW-042 | Service dies after acceptance                | Real child-process termination, surviving supervisor and fresh-instance success; hostile-process sandboxing pending     |
| LAW-043 | Stop while callbacks retain state            | Startup/shutdown exploration and premature-quiescence counterexample                                                    |

## Findings and corrections

The prior set missed independent progress assumptions, failure-domain recovery and cross-subsystem startup/shutdown ordering. Added obligations cover those gaps; earlier identities are preserved. The language-specific policy remains separate. No requirement was added because another project implements a feature.

The audit also found that the previous law-review checker required every active law to originate in the old list. That would have blocked legitimate additions despite claiming no quota. The checker now accepts explicitly justified additions while still requiring every previous obligation to be accounted for.

## Evidence limits

The [recorded experiment](../../research/results/native-state-models.json) contains finite reachable-state counts and shortest discovered negative-control traces. [Model source](../../crates/native-state-models/src/lib.rs) defines the exact transition domain. Two request actors, one queue credit and fixed finite lifecycle states are deliberate bounds. Exploration includes all reachable transitions in those models, without a depth cutoff; it does not establish arbitrary-thread linearizability, weak-memory safety or liveness under an unfair scheduler.

The placement experiment compares an actual local function call and an actual pipe-connected child process for the same payload operation. It kills a worker and observes a fresh one succeed. This is evidence of host crash containment, not performance superiority, a hostile-code sandbox or a KOLVRT process implementation. Future work must preserve these distinctions.

[Russian translation](../../translations/ru/docs/architecture/law-audit.md)
