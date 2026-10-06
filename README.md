# KOLVRT

Checkpoint without a workload deadline now has an independent completion publication bound; timeout halts with resources retained. Performance and lifecycle acceptance require separate current-source review.

## Kernel Outside Legacy, Versioned Routing & Translation

KOLVRT is an experimental operating-system kernel written in Rust, with ARM64 as its primary platform.

> **Old software may run, but it does not define the kernel.**

The native API has its own contracts. Legacy behavior belongs in removable, versioned compatibility layers. Linux and other operating systems provide research evidence, not a design to reproduce.

## Current status

The kernel boots on QEMU `virt` and runs isolated EL0 processes on two CPUs. It is not yet a general-purpose operating system: stable userspace ABI, Linux compatibility and physical ARM64 validation remain open.

| Area                          | Implemented scope                                                                                           |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Boot and hardware foundations | EL1, Device Tree, PL011 UART, exceptions, GICv3 and timer interrupts                                        |
| Memory                        | Physical allocator, page tables, MMU, W^X and kernel heap                                                   |
| Two-CPU execution             | Boot, IPI, acknowledged TLB retirement and shutdown                                                         |
| Processes and scheduling      | Isolated address spaces, timer preemption, fixed CPU affinity, lifecycle and own-event wait                 |
| User-memory boundary          | Bounded copies, immutable request snapshots and fault recovery                                              |
| Handles and security          | Local references, attenuated transfer, scoped grants, revocation, quotas and retained notification outcomes |
| ELF and ASID                  | Bounded AArch64 loader, instruction-aligned entry validation and checked tag retirement/reuse               |
| Versioned routing             | Native/v1/v2 paths in isolated EL0 workloads; native core does not depend on adapters                       |
| Host tools                    | Migration planner, documentation queries and offline COST-L registry queries                                |
| Build quality                 | AArch64 builds reject warnings in DEV, PROD and negative-control configurations                             |

Bounded Phase 3.5 IPC is implemented and ready for its declared contract: isolated EL0 request/response, blocking/wakeup, cancellation, deadlines, death/shutdown arbitration and retained ownership. Supervision policy (#27) and persistent isolated services (#28) remain the next gates; physical ARM and stable ABI are separate scopes.

An earlier implementation does not settle later architecture. Before extending it, derive the design from current invariants and accepted decisions; refactor or remove code that constrains them.

Experimental [EL0 supervision](docs/kernel/supervision.md) implements a bounded static service workload with fresh lifecycle identities, authenticated readiness, finite restart/backoff and truthful shutdown. Human architecture and complete EN/RU acceptance remain pending for #27; #28 owns persistent integration and #38 owns production bootstrap trust. Physical ARM64 and stable ABI remain separate gates.

## Architecture

- Native semantics follow current contracts, independently of compatibility behavior.
- Compatibility is explicit, versioned and removable; each requirement names its consumer.
- Different consumers and API families may select different routes at the same time.
- Safe Rust is preferred. Each `unsafe` boundary names its invariant, owner and verification.
- Performance claims require measurements; hardware-specific behavior does not silently become a general rule.

The normative rules are the [Kernel Laws](docs/architecture/kernel-laws.md). See the [native model](docs/architecture/native-model.md), [compatibility model](docs/architecture/compatibility-model.md) and [unsafe audit](docs/kernel/unsafe.md).

## Compatibility and migration

An application may use native memory and networking APIs while routing one older API through `compat-v2`. Compatibility is scoped to that consumer and API family, rather than a system-wide mode. The [routing contract](docs/kernel/routing.md) describes the implemented EL0 slice and its limits.

DEV enables investigation, route inspection and fault injection. PROD uses release optimization and predetermined routes without experimental switching. Both profiles share the same architecture.

Compatibility cost is measured without artificial penalties: latency, CPU work, memory, copies and translations remain separate observations. QEMU results do not establish hardware performance.

The read-only [migration advisor](docs/architecture/migration-advisor.md) checks evidence and contracts, compares measurements and proposes rollback plans. Signed authorization does not install packages or change routes. Production integration, telemetry, execution, trusted key management, statistical validation and physical ARM64 trials remain open in [#14](https://github.com/lifeFedorovAlexey/KOLVRT/issues/14).

The [COST-L CLI](docs/research/cost-l-queries.md) provides `show`, `consumers`, `deps`, `list` and `top` with JSON, filters and bounded output. It queries validated declarations; it does not inspect a running system. Default queries validate reciprocal module manifests; `--manifests PATH` explicitly joins custom inventories. Production data/completeness and human acceptance remain open in [#47](https://github.com/lifeFedorovAlexey/KOLVRT/issues/47) and [#48](https://github.com/lifeFedorovAlexey/KOLVRT/issues/48).

## Build and checks

The [local host-survey framework](docs/research/host-survey.md) produces allowlisted research reports with explicit gaps and local review. These observations do not establish KOLVRT hardware support.

Run commands from the repository root. Requirements: Rust **1.99.0** with `rustfmt`, `clippy` and `aarch64-unknown-none`; Node.js **18+**; QEMU **10.1.0**. Automated Windows setup also needs 7-Zip.

```powershell
rustup toolchain install 1.99.0 --profile minimal --component rustfmt --component clippy
rustup target add aarch64-unknown-none --toolchain 1.99.0
./scripts/setup-qemu.ps1
npm ci --ignore-scripts
npm run check
cargo xtask test
```

`npm run check` covers formatting, linting, host tests, models and documentation. AArch64 Cargo builds use `-D warnings`; warnings fail compilation.

```powershell
cargo xtask routing test
cargo xtask asid-bench
cargo xtask debug
cargo run --locked -p repository-checks -- cost-l list --json
```

The kernel runner requires **125 checks per DEV/PROD profile**, both non-test boots and **138 negative controls**. Controls deliberately break enforcement and must fail with the expected evidence. Missing events, unexpected panics, emulator errors and timeouts fail the runner.

The retained [ELF-alignment run](research/measurements/runs/1791171892998-issue70-elf-entry-alignment-1cd2f1cf8717.json) records **97 checks per DEV/PROD profile and 82 negative controls** for its exact source hashes. It does not verify later revisions or physical hardware. Artifacts under `target/kernel/` retain ELF images, hashes, UART events, QEMU settings, size reports, unsafe inventory and measurement samples. See [testing and GDB instructions](docs/kernel/testing.md).

## Documentation and research

| Path            | Contents                                        |
| --------------- | ----------------------------------------------- |
| `crates/`       | Rust implementation                             |
| `docs/`         | Architecture, subsystem contracts and decisions |
| `research/`     | Source-backed OS cases and scoped measurements  |
| `schemas/`      | Machine-readable data schemas                   |
| `scripts/`      | Setup and development tools                     |
| `translations/` | Documentation translations                      |
| `assets/`       | Project branding                                |

Start with the [vision](docs/vision.md), [documentation map](docs/index.md) and [architecture decisions](docs/architecture-decisions/). Subsystem references: [boot](docs/kernel/boot.md), [EL0](docs/kernel/el0.md), [scheduler](docs/kernel/scheduler.md), [processes](docs/kernel/processes.md), [wait](docs/kernel/wait.md), [user copies](docs/kernel/user-copy.md), [handles](docs/kernel/handles.md) and [domains](docs/kernel/domains.md).

The [research collection](research/) examines failures, ABI constraints, concurrency, hardware quirks and obsolete interfaces. Each case asks what happened, which constraints still apply and whether the mechanism belongs in native behavior, compatibility or neither.

## Contributing and license

Architectural changes are welcome with evidence. A compatibility exception must explain who needs it, why it cannot live outside the core, its lifetime, measured cost, removal path and protection of native consumers. Follow [CONTRIBUTING.md](CONTRIBUTING.md).

A project license has not yet been selected. Third-party materials retain their respective licenses.

## Feature verification

This generated registry summary separates implementation from verification. `BOUNDED_IMPLEMENTED` means implemented within stated limits; `PLANNED` means planned. `VERIFIED` applies to the recorded sources and environment, `STALE` means that evidence no longer covers current sources, `UNKNOWN` means no established result, and `NOT_APPLICABLE` means the environment does not apply. Read the linked contract before claiming readiness.

<!-- feature-summary:start -->

| Canonical feature                                                                                             | Implementation      | Evidence limit                                                            |
| ------------------------------------------------------------------------------------------------------------- | ------------------- | ------------------------------------------------------------------------- |
| [kolvrt.arena](docs/architecture/component-arena.md#kolvrt-arena-scope)                                       | EXPERIMENTAL        | host-process: STALE; physical-arm64: UNKNOWN                              |
| [kolvrt.ci.performance](docs/ci/performance.md#ci-performance)                                                | EXPERIMENTAL        | github-actions: UNKNOWN                                                   |
| [kolvrt.compatibility.manifests](docs/architecture/compatibility-manifests.md#kolvrt-compatibility-manifests) | BOUNDED_IMPLEMENTED | host-process: STALE; physical-arm64: NOT_APPLICABLE                       |
| [kolvrt.cost-l.offline-queries](docs/research/cost-l-queries.md#kolvrt-cost-l-offline-queries)                | BOUNDED_IMPLEMENTED | host-process: STALE; physical-arm64: NOT_APPLICABLE                       |
| [kolvrt.dependencies.hygiene](docs/architecture/dependency-hygiene.md#kolvrt-dependency-hygiene)              | BOUNDED_IMPLEMENTED | host-process: STALE; physical-arm64: NOT_APPLICABLE                       |
| [kolvrt.docs.navigation](docs/knowledge-system.md#kolvrt-docs-navigation)                                     | BOUNDED_IMPLEMENTED | host-process: VERIFIED; physical-arm64: NOT_APPLICABLE                    |
| [kolvrt.handles.local](docs/kernel/handles.md#kolvrt-handles-local)                                           | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |
| [kolvrt.ipc.transport](docs/kernel/ipc.md#bounded-native-ipc)                                                 | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |
| [kolvrt.memory.user-copy](docs/kernel/user-copy.md#kolvrt-memory-user-copy)                                   | BOUNDED_IMPLEMENTED | qemu-arm64: VERIFIED; physical-arm64: UNKNOWN                             |
| [kolvrt.process.lifecycle](docs/kernel/processes.md#kolvrt-process-lifecycle)                                 | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |
| [kolvrt.research.host-survey](docs/research/host-survey.md#kolvrt-host-survey)                                | BOUNDED_IMPLEMENTED | host-process: STALE; windows-cim: UNKNOWN; physical-arm64: NOT_APPLICABLE |
| [kolvrt.security.capability-revocation](docs/kernel/capabilities.md#kolvrt-security-capability-revocation)    | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |
| [kolvrt.security.domains](docs/kernel/domains.md#kolvrt-domains-scope)                                        | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |
| [kolvrt.security.event-revocation](docs/kernel/capabilities.md#kolvrt-security-event-revocation)              | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |
| [kolvrt.security.verifier-time](docs/security/verifier-time.md#kolvrt-verifier-time)                          | BOUNDED_IMPLEMENTED | host-process: STALE; physical-arm64: NOT_APPLICABLE                       |
| [kolvrt.services.supervision](docs/kernel/supervision.md#isolated-el0-supervision)                            | EXPERIMENTAL        | qemu-arm64: STALE; physical-arm64: UNKNOWN                                |

<!-- feature-summary:end -->

[Russian translation](translations/ru/README.md)
