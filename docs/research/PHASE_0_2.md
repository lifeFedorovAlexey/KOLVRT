# Phase 0.2 research backlog

Phase 1 does not start automatically. Tasks are ordered by dependency and require verifiable outputs.

Apply the [research decision filter](RESEARCH_METHOD.md) before starting each item. First establish the native workload, its correctness oracle and platform constraints. Deep historical replay, compatibility, routing and telemetry work become immediate priorities only when they resolve a concrete uncertainty blocking that deliverable; otherwise defer them. The table is an investigation backlog, not a mandatory feature list.

| Priority | Work and acceptance evidence |
|---|---|
| P0 | Deepen cases 07, 08 and 30: complete commit chains, release intervals, isolated failing/passing replay and negative controls. |
| P0 | Threat model: malicious application, adapter, driver, device and firmware; explicit trusted base and failure domains. |
| P0 | Executable routing model: conflicts, no downgrade, draining, generations and shared handles. |
| P0 | Shared ownership: file, lock, wait and credential consistency; counterexamples to invalid family splitting. |
| P0 | Platform contracts: pinned Arm and VirtIO specifications, invariants and Linux comparisons. |
| P0 | Kernel environment manifest: Rust, LLVM, linker, QEMU, machine, CPU and accelerator; no kernel implementation yet. |
| P1 | Placement: complete in-process and IPC host prototypes with identical request semantics. |
| P1 | Native ABI candidate: encoding, handles, errors, cancellation and deadlines; compatibility matrix and decoding fuzz corpus. |
| P1 | Memory model: copy-on-write, DMA leases, user-copy, pin quotas and retirement; boundary arguments and failing models. |
| P1 | Bug compatibility lifecycle: real consumer manifests, retirement records and a native-only dependency-checker prototype. |
| P1 | Telemetry: causal attribution, nested CPU time, lost events, unavailable states and observer cost with instrumentation on/off. |
| P1 | Provenance: pinned documentation revisions, hashes, retrieval records and unknown first versions. |
| P1 | Read primary commits before admitting arm64 set_fs removal or futex PI CVE-2014-3153 as cases. |
| P2 | Extend Linux coverage: io_uring lifetimes, signal restart, SysV IPC, TCP/netfilter, LSM, durability and EEVDF/RT. |
| P2 | Find evidence of a workaround outliving all supported affected hardware; age alone is not evidence. |
| P2 | Study one concrete failure, fix or migration for Starnix, Redox, seL4, Theseus, WSL1 and Asterinas. |
| P2 | Reassess actual consumers of obsolete sysctl, time32 and mandatory locking before implementing optional adapters. |

Preregister correctness oracles, sampling policy and artifacts for experiments. Native paths need not win. If evidence rejects an assumption, revise the decision and laws with a recorded reason.

Before discussing Phase 1, accept a threat model, platform and safety contracts, explained state boundaries, a minimal native ABI candidate and the scope of the first vertical slice. Phase 0.1 validation does not satisfy this gate.

[Russian translation](../../translations/ru/docs/research/PHASE_0_2.md)
