# Phase 1 native foundation review

The completed software foundation is a native EL1 kernel with one active CPU. SMP is explicitly deferred under the user's permitted preceding-level scope. Scheduler, userspace and compatibility are not implemented or automatically scheduled.

## Definition of Done evidence

| Requirement          | Evidence                                                            |
| -------------------- | ------------------------------------------------------------------- |
| Clean checkout build | Local Git candidate checkout; locked build and kernel matrix passed |
| QEMU AArch64 boot    | DEV and PROD boot images executed                                   |
| EL1                  | CurrentEL read and checked in kernel                                |
| UART                 | Real PL011 register identity and host-observed serial output        |
| Exceptions           | BRK, data-abort and instruction-abort probes                        |
| Physical allocator   | Reservation, allocation, free, reuse and exhaustion                 |
| MMU                  | Real translation, unmap, RO and NX faults                           |
| Kernel heap          | Real Box/Vec access and exhaustion                                  |
| Timer IRQ            | Physical timer delivery, masking and rearming                       |
| In-kernel harness    | 23 named tests per profile                                          |
| Failure propagation  | Four negative host commands exit 1                                  |
| DEV                  | Diagnostic core build and actual boot                               |
| PROD                 | Optimized same core; boot features empty                            |
| Native isolation     | Native dependency closure kernel/core/runtime only                  |
| Unsafe audit         | 48 lexical locations, one assembly file, three runtime artifacts    |
| Documentation        | English/Russian contracts and implementation review                 |
| CI/local command     | Local npm run check passed; remote CI configured, not dispatched    |

[Machine evidence](../../research/results/kernel-foundation.json) retains source hashes, ELF identities, exact emulator settings, measurements and clean-candidate identity. [Unsafe inventory](../../research/results/kernel-unsafe-audit.json) retains actual locations. Lexical count includes declarations and attributes; it is not a count of unsafe blocks or a safety proof.

## Repository and boot

```text
crates/
  kernel-core/      safe bounded algorithms and boot-description parser
  kernel/          arch/aarch64, platform, hal, memory, interrupt, sync, time, diagnostics
  xtask/           build, QEMU runner, failure controls, audit
  native-protocol-model/
  native-state-models/
  repository-checks/
docs/kernel/       boot, memory, exceptions, interrupts, time, smp, testing, unsafe
research/          cases, sources, results, requests, fixtures
scripts/           pinned QEMU setup
.github/workflows/ kernel checks
translations/ru/   mirrored documentation
```

ELF entry → EL1 → validated boot description → UART → vectors → physical discovery/reservations → allocator → page tables/MMU → heap → GICv3 → physical timer IRQ → memory validation → test harness or boot validation → PSCI shutdown. [Boot details](../kernel/boot.md) and [commands](../kernel/testing.md) describe exact assumptions.

## Results and sizes

44 host tests passed. Both DEV and PROD passed 23 real in-kernel tests. Separate boot images passed. Assertion, panic, second-owner and retained-mapping controls returned host exit 1. Phase 0 state-model evidence and negative controls also reproduced; these remain distinct from kernel execution.

| Boot image | ELF bytes | PT_LOAD file bytes | PT_LOAD memory bytes |
| ---------- | --------: | -----------------: | -------------------: |
| DEV        |   1606696 |              94559 |               913759 |
| PROD       |    261128 |              36100 |               851204 |

ELF retains debug information. Absolute source paths affect debug bytes and full ELF hash across checkouts; this establishes reproducible build and execution, not bit-for-bit binary identity. The figures identify the saved workspace artifacts; other checkout ELF sizes may differ slightly. Timing samples are TCG counter ticks, not hardware throughput or comparative algorithm superiority.

## Remaining architecture questions

SMP requires per-CPU state, affinity discovery and acknowledged remote retirement. A second platform and real hardware need independent contracts and tests. Growing heap workloads require allocator comparisons; targeted TLBI requires correctness and performance evidence. Complete exception-stack unwinding and stack-overflow handling remain future work. Private Frame identity is enforced by visibility, but a dedicated compile-fail fixture remains a verification gap. Project licensing still requires the owner's decision.

[Method review](../architecture/implementation-review.md) records alternatives, reliability constraints and historical failure checks before future adoption. A known failure mechanism is not permitted as temporary architecture.
