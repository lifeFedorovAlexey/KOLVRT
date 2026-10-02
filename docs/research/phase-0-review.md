# Phase 0 design review

Date: 2026-10-02. Scope: a reviewable design baseline for the [first native slice](../architecture/first-native-slice.md). This is a self-review, not independent certification. Phase 1 requires a separate instruction; there is no kernel implementation.

## Acceptance evidence

| Obligation                             | Result and boundary                                                                                                                                                                    |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Native workload and oracle             | Two isolated tasks, a supervised bounded service and explicit terminal outcomes; no filesystem, network or foreign ABI requirement                                                     |
| Law audit                              | [20 active laws](../architecture/law-audit.md); three additions address progress, fault domains and subsystem lifecycle; count is an audit result, never a quota                       |
| Authority and request ABI              | [Version 0 candidate](../architecture/native-abi.md), owned snapshots, attenuated rights, generation retirement, validated response identity                                           |
| Lifetime, waiting, binding and startup | [Finite models](../../research/results/native-state-models.json): 535 reachable states and 1,334 transitions, six deliberately broken variants detected                                |
| Placement                              | [Accepted decision](../architecture-decisions/0008-placement.md); actual pipe exchange, observed child failure and recovery, malformed reply rejected; no sandbox or performance claim |
| Platform and trust                     | [Pinned platform contract](../architecture/platform-contract.md), [threat model](../architecture/threat-model.md), no_std candidate cross-compilation for AArch64                      |
| Repository quality                     | Automatic format/lint/test pipeline, English sources with reviewed Russian mirrors, case schema and link/hash checks                                                                   |

The model explores every reachable state of the represented finite transition systems without a depth cutoff. It does not represent weak memory, hardware, arbitrary queue sizes or all possible schedules of a future kernel. A test of round-robin choices assumes timer delivery and bounded non-preemptible work. Host process isolation demonstrates failure observation, not the eventual privilege boundary.

## Review findings resolved

The old law traceability check inadvertently prevented additions; it now checks justified additions as well as consolidation. The response protocol lacked an identity check; the candidate now encodes and checks the entire response header and rejects poisoned reserved fields. A retired handle slot prevented use of another free slot; allocation now skips retired generations and has a regression test. Research programs, data and reference documents were mixed; the repository layout now separates their responsibilities.

## Closure and remaining gates

The Phase 0 design baseline is complete when the committed tree passes the commands below. Completion does not mean every future subsystem has a finished design. [Open questions](open-questions.md) explicitly distinguish first-slice decisions from deferred features and implementation evidence. Historical cases retain unknown provenance and unreproduced failures; they are supporting evidence, not proofs of the native design.

Before accepting kernel code, review actual exception entry, page tables, fault-safe user-copy, timer/GIC register sequences and multiprocessor ordering. Before claiming the first slice works, execute isolation, exhaustion, cancellation and shutdown tests on the pinned platform. QEMU and real hardware have not been run. The project owner still chooses the license before public distribution.

## Reproduction

Run with the pinned toolchain and locked dependencies:

```text
npm ci --ignore-scripts
npm run check
cargo check --locked -p native-protocol-model --target aarch64-unknown-none
cargo test --locked --release -p native-state-models -p native-protocol-model
```

Install the AArch64 target with rustup if absent. Check mode compares evidence without rewriting it. Changes require review before recording a new result.

[Russian translation](../../translations/ru/docs/research/phase-0-review.md)
