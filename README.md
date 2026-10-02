# KOLVRT

**Kernel Outside Legacy, Versioned Routing & Translation** is an ARM64-first operating-system research project, initially implemented in Rust. Phase 0.1 establishes native contracts, architecture and evidence for design decisions. **There is no kernel implementation yet.**

The kernel must not adapt itself to legacy. Legacy compatibility must adapt itself to the kernel. KOLVRT defines its own object, authority, lifetime and execution model. External systems are references for investigating failure mechanisms, not product requirements or architectures to inherit. A feature must advance a concrete KOLVRT workload, correctness obligation or platform need; popularity and survival are not evidence of suitability.

- [Vision](docs/vision.md), [kernel laws](docs/architecture/KERNEL_LAWS.md), [law review](docs/architecture/LAW_REVIEW.md).
- [Native model](docs/architecture/native-model.md), [compatibility](docs/architecture/compatibility-model.md), [routing](docs/architecture/routing-model.md).
- [Execution profiles](docs/architecture/execution-profiles.md), [Rust safety policy](docs/architecture/unsafe-policy.md).
- [Benchmarks](docs/architecture/benchmarking.md), [diagnostics](docs/architecture/diagnostics.md).
- [Case index](docs/research/CASE_INDEX.md), [research records](research/pathology/README.md).
- [Architecture decisions](docs/architecture-decisions/README.md), [other systems](research/other-systems/COMPARISON.md).
- [Research method](docs/research/RESEARCH_METHOD.md), [coverage](research/reference-analysis/COVERAGE.md), [sources](research/sources/README.md).
- [Open questions](docs/research/OPEN_QUESTIONS.md), [Phase 0.2 backlog](docs/research/PHASE_0_2.md).
- [Phase 0.1 report](docs/research/PHASE_0_1_REPORT.md), [documentation and translations](DOCUMENTATION.md).

Research date: **2026-10-02**. Unknown introducing commits and dates remain explicit. Documentary findings are not reproduced bugs. Kernel tests are future requirements, not completed runs.

## Checks

Install the pinned Rust toolchain through rustup, then run from the repository root:

```text
cargo test --locked
cargo run --locked -p research-checks -- check
```

Dependency download is needed for the first build. The checker itself works offline and never downloads or executes exploit reproducers. See [tooling instructions](tools/research-checks/README.md) for individual commands and translation review.

Phase 1 does not start automatically. The project license remains undecided; third-party materials retain their original licenses.

[Russian translation](translations/ru/README.md)
