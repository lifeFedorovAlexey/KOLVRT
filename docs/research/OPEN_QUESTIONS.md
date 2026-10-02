# Open questions

Phase 0.1 records contracts and evidence. Open decisions do not permit placeholders in working paths.

| ID | Question | Evidence needed |
|---|---|---|
| OQ-01 | Adapter and driver placement | ADR-0008 remains proposed; compare threats, latency and fault containment in two host implementations. |
| OQ-02 | Native process primitives | Spawn handles are accepted; define isolation, creation and notification needs from native workloads. Fork or foreign signal semantics require a demonstrated consumer before expanding the contract. |
| OQ-03 | Shared state domains | Independent per-call splitting is forbidden; model-check descriptor, lock and credential gateways. |
| OQ-04 | USB persistence identity | Case 14 remains RESEARCH_REQUIRED; model replacement and recovery without false identity. |
| OQ-05 | Shared accounting | Keep one object identity; define charge attribution for shared cache, DMA and deferred work. |
| OQ-06 | Memory reclamation | Lifetime obligations are accepted; choose RCU, epochs or reference counts after contention and bounded-reclamation models. |
| OQ-07 | Scheduler objectives | Visible domains are accepted; specify fairness, latency and adversarial workloads before choosing an algorithm. |
| OQ-08 | Platform specifications | Pin Arm and VirtIO clauses for barriers, MMU, GIC, timers and SMP; Linux evidence is only a starting point. |
| OQ-09 | Kernel build environment | Pin kernel Rust/LLVM/linker/QEMU, machine and CPU capabilities. The pinned host checker does not settle this choice. |
| OQ-10 | Panic, unwinding and allocation | Define an error/context matrix; never unwind through unproven foreign or interrupt boundaries. |
| OQ-11 | Compatibility retirement | Inventory real workloads and offline or dormant dependencies before setting retirement windows. |
| OQ-12 | MOSTLY_NATIVE budgets | No global arbitrary threshold; require consumer migration objectives, coverage and multidimensional limits. |
| OQ-13 | Historical provenance | Preserve unknown values; establish introducing, fixing and reverting commits and containing tags per case. |
| OQ-14 | Syzbot causality and LKML | Case 30 needs isolated replay of the no-op fix bisection and access to the maintainer discussion. |
| OQ-15 | Hardware scope and license | No promise to support every board; product scope and a project-owner licensing decision are required. |

Case-specific questions appear in the [case index](CASE_INDEX.md). [Other-system findings](../../research/other-systems/COMPARISON.md) retain their own limitations. Unknown does not mean that a Linux commit or test does not exist.

[Russian translation](../../translations/ru/docs/research/OPEN_QUESTIONS.md)
