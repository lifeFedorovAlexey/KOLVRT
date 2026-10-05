# KOLVRT

## Kernel Outside Legacy, Versioned Routing & Translation

> **Legacy may run. It does not get to define the kernel.**

KOLVRT is an **ARM64-first operating-system kernel written in Rust**.

It starts from a deliberately uncomfortable question:

**What would a modern kernel look like if we stopped treating decades of historical behavior as permanent architectural law?**

KOLVRT is not a Linux fork.

It is not Linux rewritten in Rust.

It is not an attempt to reproduce Linux internals with safer syntax.

Linux and other mature operating systems are used as **evidence**: decades of bugs, regressions, hardware quirks, successful ideas, failed abstractions and compatibility decisions worth studying.

Then KOLVRT makes its own decision.

**The kernel stays clean. Compatibility adapts around it.**

---

## The idea

Traditional compatibility tends to accumulate inward:

```text
old software
     │
     ▼
special case
     │
     ▼
another special case
     │
     ▼
kernel
```

KOLVRT intends to push it outward:

```text
                         ┌───────────────┐
legacy software ───────► │ compat v1/v2  │ ──────┐
                         └───────────────┘       │
                                                 ▼
native software ─────────────────────────► Native API
                                                 │
                                                 ▼
                                          ┌────────────┐
                                          │   KOLVRT   │
                                          │   kernel   │
                                          └────────────┘
```

If old software depends on old behavior, that behavior belongs in a **versioned compatibility layer**.

If the kernel itself once exposed broken behavior and software started depending on it, the native behavior gets fixed. Compatibility with the mistake belongs outside the native core.

No permanent scar tissue just because somebody once depended on the wound.

---

## Why this exists

KOLVRT is built around several deliberately strict ideas:

- **Native behavior is the source of truth.**
- **Legacy semantics do not belong in the native kernel API.**
- **Compatibility is explicit, versioned and removable.**
- **Different applications, drivers or subsystems may use different routes at the same time.**
- **Compatibility cost must be measurable.**
- **Old behavior is preserved only where somebody actually needs it.**
- **Kernel bugs are fixed instead of promoted into eternal architecture.**
- **Safe Rust is preferred; `unsafe` is treated as an auditable boundary, not a convenience.**
- **ARM64 is Tier 1.**
- **External operating systems are references, not blueprints.**

Or, shorter:

> **Do not fix the kernel for broken assumptions. Translate the assumptions.**

---

## Current status

KOLVRT already boots as a native AArch64 kernel in QEMU.

| Area                          | Status                                                 |
| ----------------------------- | ------------------------------------------------------ |
| Rust `no_std` kernel          | ✅                                                     |
| AArch64 / ARM64               | ✅ Tier 1                                              |
| QEMU `virt` boot              | ✅                                                     |
| EL1 execution                 | ✅                                                     |
| Device Tree validation        | ✅                                                     |
| PL011 UART                    | ✅                                                     |
| Exception vectors             | ✅                                                     |
| Physical memory allocator     | ✅                                                     |
| Page tables / MMU             | ✅                                                     |
| W^X mappings                  | ✅                                                     |
| Kernel heap                   | ✅                                                     |
| GICv3                         | ✅ Both CPUs                                           |
| ARM physical timer IRQ        | ✅                                                     |
| DEV / PROD profiles           | ✅                                                     |
| Automated kernel test harness | ✅                                                     |
| Real in-kernel tests          | ✅ 84 checks in the exact-source DEV/PROD matrix       |
| Negative failure controls     | ✅                                                     |
| GDB debugging                 | ✅                                                     |
| SMP                           | ✅ Two-CPU QEMU foundation                             |
| EL0 / userspace               | ✅ Bounded isolated-process foundation                 |
| Scheduler                     | ✅ Timer-driven, fixed per-CPU affinity                |
| Process lifecycle             | ✅ Bounded create/start/exit/reclaim on two CPUs       |
| Own-process event wait        | ✅ Bounded block/wakeup path in `Registry::step()`     |
| Safe user copy                | ✅ Bounded Phase 3.2 synchronous snapshot boundary     |
| Runtime versioned routing     | ✅ Bounded EL0 vertical slice; native core independent |
| Migration advisor             | ✅ Host planner; production integrations remain open   |
| Linux compatibility           | ⏳ Not started                                         |

Both configured CPUs execute native EL1 code. The QEMU matrix checks secondary boot, per-CPU ownership, bidirectional IPI, acknowledged remote TLB retirement and multicore shutdown. Real hardware remains unverified.

---

## What happens next

The current development order is deliberate:

```text
Native ARM64 kernel foundation        ✅
        │
        ▼
SMP correctness foundation            ✅
        │
        ▼
EL0 + address spaces                  ✅
        │
        ▼
Scheduler + context switching        ✅
        │
        ▼
Versioned Routing & Translation      ✅ bounded EL0 slice
        │
        ▼
Process lifecycle + own-event wait  ✅ bounded Phase 3.1
        │
        ▼
Safe user-copy                      ✅ bounded Phase 3.2
        │
        ▼
Process-local handles               ✅ bounded Phase 3.3
        │
        ▼
Scoped grants and security domains  implemented (#24/#25)
IPC and services                    next (#26)
        │
        ▼
Compatibility personalities
        │
        ▼
Linux ABI compatibility where useful
```

Compatibility is not being wired into a single-CPU kernel and patched for SMP later.

The native execution model comes first. The bounded EL0/address-space and timer-scheduler foundation runs fixed-affinity processes on two CPUs. Phase 2 routing executes in an optional isolated EL0 image; native core remains independent. Phase 3.1 adds kernel-owned process creation, start, completion and reclaim, plus a limited own-process event latch used by the preemptible step path. These are trusted-bootstrap mechanisms, not public EL0 process or wait APIs. Phase 3.2 adds bounded safe user-copy, and Phase 3.3 adds process-local handles and receiver-local transfer. Phase 3.4 adds scoped Event grants, revocation, resource budgets and [security domains](docs/kernel/domains.md). General IPC, cancellation and persistent services remain future work; the full native slice is incomplete. See the [process lifecycle](docs/kernel/processes.md), [wait contract](docs/kernel/wait.md), [handles](docs/kernel/handles.md) and [scheduler](docs/kernel/scheduler.md). Issue [#24](https://github.com/lifeFedorovAlexey/KOLVRT/issues/24) records the scoped grants, attenuation and revocation gate; [#26](https://github.com/lifeFedorovAlexey/KOLVRT/issues/26) is the next IPC gate.

---

## Versioned Routing & Translation

The long-term model is **not**:

```text
SYSTEM = NATIVE
```

or:

```text
SYSTEM = COMPAT
```

Routing is intended to be granular.

For example:

```text
Browser
├── memory          → native
├── filesystem      → native
├── networking      → native
└── old_sync_api    → compat-v2

Database
└── everything      → native

Old driver
└── device API      → compat-v1

New driver
└── device API      → native
```

Native and compatibility consumers can coexist.

Compatibility does not become a property of the entire operating system.

---

## Compatibility has a price. Measure it

KOLVRT does not intend to make compatibility artificially slow.

That would be cheating.

Instead, diagnostic builds are intended to expose the **real cost** of translation. The following layout and numbers are illustrative only; they are not measurements:

```text
Component: example-driver

Route: COMPAT v2

Calls                  1,842,991
Translations             291,440
Extra copies              18,202

                COMPAT       NATIVE
median latency   14.2 µs      9.1 µs
p99              31.8 µs     19.7 µs
CPU               3.8 %       2.9 %
memory           18.4 MB      14.1 MB
```

If native is faster, the developer can see exactly what migration buys.

If compatibility is faster, **that is a native-path performance bug worth investigating**.

No fake penalties.

No marketing benchmarks.

No hiding inconvenient numbers.

### Migration advisor

The read-only host-side [migration advisor](docs/architecture/migration-advisor.md) models capability-based dependency alternatives, verifies evidence and contract receipts, analyzes paired measurements, and prepares rollback proposals. A separate signed authorization gate is implemented, but it does not deploy packages or mutate routes. Production catalog/installed-state collection, complete OS telemetry, real application contract and rollback executors, production key custody, validated dependence/power analysis, and physical ARM64 A/B evidence remain open under [issue #14](https://github.com/lifeFedorovAlexey/KOLVRT/issues/14).

---

## DEV and PROD are different jobs

KOLVRT is designed around two execution profiles.

### DEV / DIAGNOSTIC

Built for investigation:

- invariant checks
- detailed panic evidence
- tracing
- compatibility accounting
- route inspection
- A/B measurements
- fault injection
- unsafe-boundary auditing
- performance counters

### PROD

Built to execute the already validated configuration:

- release optimization
- no experimental route switching
- no unnecessary diagnostic instrumentation
- only required compatibility modules
- predetermined routing
- minimal runtime overhead

They are **not separate kernels**.

The same architecture must survive both modes.

---

## Linux is a reference, not a religion

KOLVRT maintains a structured research base of real operating-system failure mechanisms.

The project studies:

- Linux kernel regressions
- ABI constraints
- historical behavior that became compatibility requirements
- memory-safety failures
- concurrency failures
- driver-model complexity
- hardware quirks
- security fixes
- syzkaller / syzbot findings
- deprecated and obsolete interfaces
- design decisions from other operating systems

Every interesting case should eventually answer:

```text
What happened?
Why did it happen?
Was it actually a mistake?
What constraints existed at the time?
Do those constraints still exist?
What would KOLVRT do?
Does it belong in native behavior?
Does it belong in compatibility?
Does it belong anywhere?
```

Survival is evidence that something worked.

It is **not evidence that we should copy it**.

---

## Kernel laws

KOLVRT architecture is constrained by explicit **Kernel Laws**.

They exist to prevent architecture from slowly degrading into a collection of reasonable exceptions.

Examples of the philosophy:

```text
Native semantics must not depend on compatibility semantics.

Compatibility must be removable without breaking native execution.

Unsafe code must have an explicit invariant.

Observed legacy behavior does not automatically become native specification.

Hardware quirks must not silently become generic architecture.

A compatibility requirement must identify its consumer.

Performance claims require measurements.
```

The complete and normative rules live in:

[`docs/architecture/kernel-laws.md`](docs/architecture/kernel-laws.md)

---

## Unsafe means explain yourself

Kernel development inevitably crosses boundaries Rust cannot prove.

KOLVRT does not pretend otherwise.

Instead, every necessary `unsafe` boundary is expected to answer:

```text
Why is unsafe required here?
What invariant makes this correct?
Who establishes that invariant?
Who may invalidate it?
How is it tested?
```

The project generates an unsafe inventory as part of kernel verification.

See:

[`docs/kernel/unsafe.md`](docs/kernel/unsafe.md)

---

## Quick start

### Requirements

Current development baseline:

- Rust **1.99.0**
- target `aarch64-unknown-none`
- `rustfmt`
- `clippy`
- Node.js **18+**
- QEMU **10.1.0**
- 7-Zip on Windows for the automated QEMU setup

### Windows setup

```powershell
rustup toolchain install 1.99.0 --profile minimal --component rustfmt --component clippy
rustup target add aarch64-unknown-none --toolchain 1.99.0

./scripts/setup-qemu.ps1

npm ci --ignore-scripts
npm run check
cargo xtask test
```

### Run the verification matrix

```bash
cargo xtask test
```

The test command builds and runs real AArch64 kernel images under QEMU.

A missing event, panic, fatal exception, emulator error or timeout fails the host command.

### Debug with GDB

```bash
cargo xtask debug
```

Then:

```text
aarch64-none-elf-gdb target/kernel/dev-boot.elf

(gdb) target remote 127.0.0.1:1234
(gdb) break kernel_main
(gdb) continue
(gdb) info registers
(gdb) bt
```

---

## Tests are supposed to fail when the kernel is wrong

The latest retained exact-source integration matrix records **96 kernel checks per DEV and PROD
profile** and 80 negative host controls. It includes process-local handle transfer, the bounded
preemption/event-wait paths, the ELF loader and ASID lifecycle. The [receipt](research/measurements/runs/1791130278634-phase3-4-revocation-integrated-f3be261c515b.json)
binds the tested source, artifacts and QEMU TCG scope; it is not evidence of physical ARM64 behavior
or later source changes. Historical milestone counts remain in their original decision records.

It also runs negative controls that are required to fail correctly.

That distinction matters.

A green test suite that cannot detect a deliberately introduced failure is decoration.

KOLVRT tests exercise real kernel behavior including allocation, mapping, memory access, unmapping, freeing, exception paths and timer delivery.

Generated artifacts include:

- ELF images
- SHA-256 hashes
- UART logs
- structured test events
- QEMU arguments and version
- build size reports
- feature reports
- unsafe inventory
- measurement samples

---

## Repository map

```text
.
├── crates/          Rust implementation
├── docs/            architecture and kernel documentation
├── research/        source-backed OS research and measurements
├── schemas/         machine-readable research schemas
├── scripts/         development/bootstrap tooling
├── translations/    translated project documentation
└── assets/          project branding
```

Start here:

- [`Vision`](docs/vision.md)
- [`Kernel Laws`](docs/architecture/kernel-laws.md)
- [`Native model`](docs/architecture/native-model.md)
- [`Compatibility model`](docs/architecture/compatibility-model.md)
- [`Routing model`](docs/architecture/routing-model.md)
- [`Kernel boot`](docs/kernel/boot.md)
- [`Testing`](docs/kernel/testing.md)
- [`SMP boundary`](docs/kernel/smp.md)
- [`Unsafe boundaries`](docs/kernel/unsafe.md)
- [`Architecture decisions`](docs/architecture-decisions/)
- [`Research`](research/)

Russian documentation:

[`translations/ru/`](translations/ru/)

---

## Contributing

KOLVRT is still early.

That means architectural changes are welcome.

Architectural hand-waving is not.

Before proposing a kernel-level compatibility exception, be prepared to answer:

1. Who requires this behavior?
2. Why can it not live outside the native core?
3. What lifetime does the exception have?
4. How is its use measured?
5. How is it removed?
6. What prevents native consumers from paying for it?

For formatting, checks and contribution rules see:

[`CONTRIBUTING.md`](CONTRIBUTING.md)

---

## Experimental means experimental

KOLVRT is an operating-system research project under active development.

It is not currently a general-purpose production operating system.

There is no stable userspace ABI yet.

There is no Linux compatibility promise yet.

There is no claim of broad hardware support yet.

Claims are expected to follow evidence.

If something is not implemented, the documentation should say so.

If something is not measured, it should not have a benchmark number.

If something is not proven SMP-safe, it should not be called SMP-safe.

---

## License

A project license has not yet been selected.

Third-party materials retain their respective licenses.

---

## Final rule

> **Compatibility is allowed. Legacy is quarantined. Native stays clean.**

Or, in the less formal project dialect:

> **Do not spit into the kernel—you will drink from it yourself.**

[EL0 foundation contract](docs/kernel/el0.md) records ownership, retirement, tests and limits; [ADR-0014](docs/architecture-decisions/0014-el0-foundation.md) reviews the mechanisms.

The [Phase 2 contract](docs/kernel/routing.md) records the actual EL0 routes, profile validation, measurements, authority boundary and limits. The full IPC/service native slice remains incomplete.

[Russian translation](translations/ru/README.md)

## Phase 3.2 safe user-copy

The [user-copy boundary](docs/kernel/user-copy.md) supports bounded current-process byte copies, immutable input snapshots and precise fault recovery on both CPUs. Its DEV/PROD suite has 69 checks and 57 failure controls. [ADR-0018](docs/architecture-decisions/0018-safe-user-copy.md) preserves synchronous mapping/lifetime exclusion and separates memory validity from authority.

## Phase 3.3 process-local handles

[Handles](docs/kernel/handles.md) provide bounded caller-local opaque references, generation/type/live/rights checks, receiver-local transfer with rights attenuation, retained Event targets and deterministic exit/fault cleanup. EL0 transfer currently targets a namespace on the same CPU; cross-CPU delegation is prepared by the coordinator and exercised under concurrent EL0 close/lookup. [ADR-0019](docs/architecture-decisions/0019-process-local-handles.md) records the original identity/lifetime decision; [ADR-0020](docs/architecture-decisions/0020-handle-transfer-and-retention.md) records the transfer and retention contract. The original Phase 3.3 [matrix](research/results/kernel-phase33.json) records 72 checks; the issue #23 [exact-source matrix](research/measurements/runs/1791022558822-issue23-transfer-bf3f9b298688.json) records 73 DEV/PROD checks and 69 negative controls. [Phase 3.4](docs/kernel/domains.md) adds scoped grants and domains; general IPC remains future work.

## Canonical feature status

This derived pilot summary reports implementation scope independently of roadmap ordering. Read the canonical contract and historical receipt scope before claiming verification.

[Documentation map](docs/index.md)

<!-- feature-summary:start -->

| Canonical feature                                                                                          | Implementation      | Evidence limit                                         |
| ---------------------------------------------------------------------------------------------------------- | ------------------- | ------------------------------------------------------ |
| [kolvrt.arena](docs/architecture/component-arena.md#kolvrt-arena-scope)                                    | PLANNED             | host-process: UNKNOWN; physical-arm64: UNKNOWN         |
| [kolvrt.cost-l.offline-queries](docs/research/cost-l-queries.md#kolvrt-cost-l-offline-queries)             | BOUNDED_IMPLEMENTED | host-process: VERIFIED; physical-arm64: NOT_APPLICABLE |
| [kolvrt.docs.navigation](docs/knowledge-system.md#kolvrt-docs-navigation)                                  | BOUNDED_IMPLEMENTED | host-process: VERIFIED; physical-arm64: NOT_APPLICABLE |
| [kolvrt.handles.local](docs/kernel/handles.md#kolvrt-handles-local)                                        | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN             |
| [kolvrt.memory.user-copy](docs/kernel/user-copy.md#kolvrt-memory-user-copy)                                | BOUNDED_IMPLEMENTED | qemu-arm64: VERIFIED; physical-arm64: UNKNOWN          |
| [kolvrt.process.lifecycle](docs/kernel/processes.md#kolvrt-process-lifecycle)                              | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN             |
| [kolvrt.security.capability-revocation](docs/kernel/capabilities.md#kolvrt-security-capability-revocation) | BOUNDED_IMPLEMENTED | qemu-arm64: VERIFIED; physical-arm64: UNKNOWN          |
| [kolvrt.security.domains](docs/kernel/domains.md#kolvrt-domains-scope)                                     | BOUNDED_IMPLEMENTED | qemu-arm64: VERIFIED; physical-arm64: UNKNOWN          |
| [kolvrt.security.event-revocation](docs/kernel/capabilities.md#kolvrt-security-event-revocation)           | BOUNDED_IMPLEMENTED | qemu-arm64: STALE; physical-arm64: UNKNOWN             |

<!-- feature-summary:end -->
