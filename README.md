# KOLVRT

**Kernel Outside Legacy, Versioned Routing & Translation** is an ARM64-first operating-system research project, initially implemented in Rust. Phase 0 establishes the reviewed native design baseline and evidence for the first vertical slice. **There is no kernel implementation yet.**

The kernel must not adapt itself to legacy. Legacy compatibility must adapt itself to the kernel. KOLVRT defines its own object, authority, lifetime and execution model. External systems are references for investigating failure mechanisms, not product requirements or architectures to inherit. A feature must advance a concrete KOLVRT workload, correctness obligation or platform need; popularity and survival are not evidence of suitability.

- [Phase 0 review](docs/research/phase-0-review.md), [first native slice](docs/architecture/first-native-slice.md), [repository structure](docs/project-structure.md).
- [Vision](docs/vision.md), [kernel laws](docs/architecture/kernel-laws.md), [law review](docs/architecture/law-review.md).
- [Native model](docs/architecture/native-model.md), [compatibility](docs/architecture/compatibility-model.md), [routing](docs/architecture/routing-model.md).
- [Execution profiles](docs/architecture/execution-profiles.md), [Rust safety policy](docs/architecture/unsafe-policy.md).
- [Benchmarks](docs/architecture/benchmarking.md), [diagnostics](docs/architecture/diagnostics.md).
- [Case index](docs/research/case-index.md), [research records](docs/research/case-database.md).
- [Architecture decisions](docs/architecture-decisions/README.md), [other systems](docs/research/reference-systems.md).
- [Research method](docs/research/research-method.md), [coverage](docs/research/reference-coverage.md), [sources](docs/research/source-ledger.md).
- [Open questions](docs/research/open-questions.md), [Phase 0.2 backlog](docs/research/phase-0-2.md).
- [Phase 0.1 report](docs/research/phase-0-1-report.md), [documentation and translations](docs/documentation-policy.md).

Research date: **2026-10-02**. Unknown introducing commits and dates remain explicit. Documentary findings are not reproduced bugs. Kernel tests are future requirements, not completed runs.

## Checks

Install the pinned Rust toolchain through rustup and Node.js 18 or later, then run from the repository root:

```text
npm ci --ignore-scripts
npm run check
```

Dependency download is needed for the first build. The checker itself works offline and never downloads or executes exploit reproducers. See [formatting and contribution instructions](CONTRIBUTING.md) and [tooling instructions](crates/repository-checks/README.md) for individual commands and translation review.

Phase 1 does not start automatically. The project license remains undecided; third-party materials retain their original licenses.

[Russian translation](translations/ru/README.md)
