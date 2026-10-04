# Phase 1.1 completion and Phase 2 boundary

Document status: HISTORICAL MILESTONE
Evidence scope: Phase 1.1 review revision; Phase 2 integration was still future work when this report was written.
Current reference: [Current scheduler contract](../kernel/scheduler.md); [Phase 2 routing contract](../kernel/routing.md)

Phase 1 remains the completed historical native foundation. Phase 1.1 adds real two-CPU execution, PSCI startup, affinity-correct GICv3, SGI/IPI, acknowledged reader/TLB retirement and orderly CPU_OFF. [SMP contract](../kernel/smp.md) contains the architecture/boot/shootdown diagrams, ownership table and limits. [ADR-0012](../architecture-decisions/0012-multicore-retirement.md) records alternatives; existing Kernel Laws need no changes.

## Evidence

The [machine record](../../research/results/kernel-smp.json) contains exact source hashes, 39 named tests per DEV/PROD image, real non-test boots, raw timer samples and artifact sizes. Eight negative commands must exit nonzero, including secondary panic, premature release, missing acknowledgement and omitted remote TLBI. Host checks include actual DTB CPU/PSCI rejection, native dependency closure and the existing concurrent lock test. CI is configured; no remote CI execution is claimed. QEMU evidence does not certify physical hardware or arbitrary weak-memory interleavings.

The Phase 1 [record](../../research/results/kernel-foundation.json) remains unchanged. Its single-CPU timings are not comparable to the new SMP workload. The current report's size table distinguishes ELF/debug size from PT_LOAD file/memory size; the second permanent stack accounts for 256 KiB of added memory. Unsafe inventory records lexical locations and assembly hash, not an unsafe-block proof.

| Non-test boot image | Phase 1 bytes | Phase 1.1 bytes | Delta bytes |
| ------------------- | ------------: | --------------: | ----------: |
| DEV ELF             |       1611120 |         1721552 |     +110432 |
| DEV PT_LOAD file    |         95571 |          116703 |      +21132 |
| DEV PT_LOAD memory  |        914771 |         1202047 |     +287276 |
| PROD ELF            |        266368 |          322504 |      +56136 |
| PROD PT_LOAD file   |         37484 |           47268 |       +9784 |
| PROD PT_LOAD memory |        852588 |         1128516 |     +275928 |

The [current unsafe inventory](../../research/results/kernel-smp-unsafe-audit.json) contains 67 lexical locations versus 48 in Phase 1, including declarations and test-only probes. Added boundaries cover secondary startup/publication, PSCI, affinity/SGI, per-CPU state and remote reader retirement. Each boundary is linked in [unsafe review](../kernel/unsafe.md). The reproducible final matrix is labelled `phase-1-1`; earlier development runs remain historical evidence. The earlier `smp-foundation` record predates the corrected negative-control count and is not the completion record.

The full repository check passed before concurrent research edits. The subsequent check finds an unfinished link from `research/other-systems/kasperskyos/microkernel.md` to `docs/architecture/kernel-admission-policy.md`; the user confirmed another task is writing those documents. That work is preserved. Final SMP source hashes remain unchanged, and isolated routing tests/Clippy pass after the constant cleanup. The current combined workspace documentation check is therefore not claimed green.

## Preserved Phase 2 files

| Files                                                                            | Classification and current boundary                                                         |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `crates/kernel-core/src/window.rs`, its `lib.rs` declaration                     | Pure bounded native reduction and host oracle; no kernel call/hook                          |
| `crates/routing/Cargo.toml`, `build.rs`                                          | Isolated dependency features and source-identity generation; no kernel dependency           |
| `crates/routing/src/lib.rs`: Route, Error, Input, Work, Status, Profile          | Types, version names, profile schema/integrity model and descriptions                       |
| `crates/routing/src/lib.rs`: Consumer, Transaction, dispatch, classify, counters | Standalone runtime candidates; retained, not connected or accepted as multicore integration |
| `crates/routing/src/conformance.rs`                                              | Standalone contracts for versions, validation, isolation and switching; host-only evidence  |
| `crates/window-compat/Cargo.toml`, `src/lib.rs`                                  | Standalone optional adapter candidates; no kernel dispatch                                  |
| Root `Cargo.toml`, `Cargo.lock`                                                  | Workspace registration/locked dependencies; registration is not a kernel dependency         |

No Phase 2 files were deleted or rolled back. Necessary formatting, named-layout constants and standalone build checks preserve usable groundwork. The native executable's normal/build dependency closure is still exactly kernel and kernel-core; the dependency regression rejects routing/adapters in that closure.

## Runtime work deferred

No kernel hooks, shared routing registry, live kernel switching, compatibility dispatch, production routing profile or translation benchmark was added. The standalone Consumer uses exclusive borrowing, a busy transaction flag and local counter arrays. That is an ownership candidate, not proof of consumer migration across CPUs, shared publication, remote draining or accounting accuracy in a concurrent kernel. No Phase 2 completion or measured overhead is claimed.

## Assumptions found and integration prerequisites

| Finding                                                                             | Required before Phase 2 integration                                                                                            |
| ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Consumer mutation/counters assume one exclusive owner                               | Define owner transfer, state-domain synchronization and per-CPU aggregation; avoid a global mode/temporary global lock         |
| Transaction drain covers synchronous borrow scope only                              | Pin route generation and adapter lifetime across all admitted work and CPUs; reject unprovable live changes                    |
| Tick accounting is manually charged, without a kernel clock boundary                | Implement bounded real measurement, overflow/loss handling and instrumentation-free PROD                                       |
| Profile identity hashes source files, not full executable closure                   | Separate immutable semantic identity from exact built artifact identity and trusted profile authenticity                       |
| Routing build script reads adapter source even with compatibility features disabled | Remove that build-time source requirement before proving complete adapter-source removal; kernel itself has no such dependency |
| Four demonstration consumers and one family                                         | Keep bounds explicit; extend only for a demonstrated workload and shared-state review                                          |
| Compatibility adapters are privileged library candidates                            | Review placement/threat model; no EL0 isolation or safety bypass is implied                                                    |
| Physical ownership/PTE mutation is CPU0-scoped in Phase 1.1                         | Future routing must not treat CPU0 as permanent sole writer; use reviewed SMP ownership/publication primitives                 |

At the time of this review, the next milestone was Phase 2 integration after review and a corresponding ADR. The later Phase 2 and Phase 3.0 evidence is recorded separately in the [documentation status policy](../documentation-policy.md); this report remains evidence for Phase 1.1 only.

[Russian translation](../../translations/ru/docs/research/phase-1-1-review.md)
