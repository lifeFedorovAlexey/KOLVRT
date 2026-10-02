# Research that advances KOLVRT

Research serves KOLVRT's own architecture and intended workloads. External operating systems are reference material for investigating failure mechanisms and constraints. Their interfaces, feature lists and implementation choices are not requirements. Source names remain in evidence so claims can be checked; prominence in the corpus does not confer architectural authority.

## Selection bias and survivorship

The current corpus comes largely from one well-documented system. It is a convenience sample, not a representative comparison of all possible designs. Surviving projects expose maintenance history that abandoned projects may not preserve. More published defects can reflect visibility, use or better reporting rather than worse design. Missing reports do not prove safety; popularity, age and survival do not prove an architectural choice caused success.

For each proposed lesson, seek a failed or abandoned alternative, an opposing example, relevant specification constraints and a plausible competing explanation. Preserve unavailable evidence as unknown. Do not count cases as independent when they share one underlying mechanism. A repaired reference design is useful as a counterexample to an overbroad claim, not as a blueprint to copy. Record workloads, hardware, trust boundaries and historical constraints before transferring a conclusion.

## Decision filter

| Required question | Acceptable evidence |
|---|---|
| What KOLVRT need does this address? | Named native workload, correctness obligation or supported-platform constraint |
| Can the failure occur here? | Mapping to KOLVRT objects, ownership, authority or execution, with explicit assumptions |
| What is the smallest adequate solution? | Alternatives including omission, deferral and a simpler native design |
| What would disprove the choice? | Counterexample, failure oracle or measured acceptance criterion set before the experiment |
| What does it cost? | Implementation and maintenance effort, trusted code, latency, memory and added state complexity |
| Why do it now? | A dependency of the next agreed deliverable; otherwise an explicit backlog item |

Prioritize by dependency, risk reduction and demonstrated benefit, not feature counts or an arbitrary score. Do not implement compatibility merely because a reference has an interface. Do not build elaborate routing or telemetry before a workload requires it. Requirements describe needed guarantees; algorithm and placement choices stay open until evidence discriminates between alternatives.

## Reporting

Separate observed facts, causal inferences, design proposals and completed experiments. Publish negative results, failures, missing observations and costs with successes. Compare equivalent semantics and workloads; do not assume native paths must win. A failed assumption can remove a feature or revise a law. Keep that reason in the decision history.

The next research work remains bounded by the [Phase 0.2 backlog](PHASE_0_2.md). Research completeness, case counts and schema validation do not establish that a kernel implementation is ready to start.

[Russian translation](../../translations/ru/docs/research/RESEARCH_METHOD.md)
