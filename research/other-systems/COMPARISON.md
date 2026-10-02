# Other systems: ideas, limits and applicability

Research date: 2026-10-02. This is a documentary comparison; none of these systems was built or benchmarked locally. Results mean results described by primary sources. Missing failure reports do not establish an absence of failures. See the [source ledger](../sources/other-sources.json).

## Fuchsia / Zircon / Starnix

Starnix implements a Linux ABI in a Fuchsia userspace program while native Fuchsia retains its interfaces. Its scope includes process and file semantics, not just syscall numbers. [Overview](https://fuchsia.dev/fuchsia-src/concepts/starnix), [RFC-0082](https://fuchsia.dev/fuchsia-src/contribute/governance/rfcs/0082_starnix).

Apply separate ownership of compatibility semantics. This boundary does not remove state coupling or failures affecting consumers. The sources do not establish universal Linux coverage or a speed advantage. Study a concrete failure and syscall transport costs in Phase 0.2; do not import Zircon's object model without analyzing KOLVRT primitives.

## Redox OS

The official book describes a Rust microkernel, userspace drivers and services, and scheme IPC. It explicitly discusses service-interface revision and security-design evolution: Rust does not finish architectural design. [Official book mirror](https://github.com/redox-os/book/blob/master/src/our-goals.md).

Apply explicit resource protocols. The documented project structure is evidence, not locally verified hardware support. Current schemes and plans are not immutable ABI commitments. Changed plans alone do not prove security failure. Investigate scheme migration regressions and recovery costs; do not automatically adopt a file-like namespace model.

## seL4

seL4 publishes verified configurations and properties. Assurance depends on architecture and configuration; it does not automatically cover arbitrary drivers and applications. [Verified configurations](https://docs.sel4.systems/projects/sel4/verified-configurations.html).

Apply explicit proof boundaries, capability delegation and an invariant ledger. KOLVRT's compiler, hardware and driver assumptions require separate arguments. The source establishes neither a general seL4 failure nor proof of KOLVRT. Study trusted assumptions and proof-maintenance costs before selecting a verification plan; do not promise whole-kernel verification.

## Theseus OS

Theseus investigates Rust intralingual design, type-system resource management and runtime component replacement. [Book](https://www.theseus-os.com/Theseus/book/), [design principles](https://www.theseus-os.com/Theseus/book/design/idea.html).

Apply ownership and state structure as interface properties. Typed replacement alone does not prove semantic state conversion or isolation from unsafe code. The result here is a published design and implementation project, without verified performance numbers or a proof of arbitrary safe replacement. KOLVRT requires explicit quiescence and migration even with identical types. Study a concrete failure/recovery experiment.

## FreeBSD Linuxulator

The historical article describes executable ABI dispatch through sysentvec and includes old i386/2.6-era details. It is not a complete description of current ports. [Architecture article](https://docs.freebsd.org/en/articles/linux-emulation/).

The Q2 2023 report documents incorrect handling of absolute symlink targets by kern_alternate_path and moving alternate ABI roots into name lookup. This illustrates tension between isolated pathname translation and full path semantics. [Project report](https://www.freebsd.org/status/report-2023-04-2023-06/linuxulator/).

Apply executable compatibility binding. New native primitives, such as explicit lookup roots, need a general contract rather than a Linux-specific flag. Kernel-resident placement needs trusted-base and latency analysis. Current compatibility coverage requires a pinned conformance run.

## WSL1

Microsoft's 2016 overview describes Pico processes and kernel providers translating Linux syscalls into NT operations, with additional semantics when no direct mapping exists; fork is an example. It describes WSL1, not WSL2. [Overview](https://learn.microsoft.com/en-us/archive/blogs/wsl/windows-subsystem-for-linux-overview).

Apply explicit subsystem identity and an independent native host API. The developers describe unmodified ELF64 execution, but translation is not always one-to-one and providers remain privileged. An architecture transition alone does not establish total failure. Record inexpressible operations honestly and research filesystem/signal fidelity gaps and benchmark evidence separately.

## Asterinas

The book describes a Rust kernel targeting Linux ABI and OSTD as a foundation for safe OS development, while noting an early stage. [Book](https://asterinas.github.io/book/).

Apply a small reviewed foundation for safe clients. Linux ABI is a central Asterinas goal and an optional KOLVRT environment. The available project and interfaces are evidence; efficiency and security claims are not independently verified measurements. This source does not establish a concrete boundary failure. Examine OSTD soundness assumptions and actual fixes without importing Linux-oriented core design.

## KOLVRT conclusions

Adopt explicit semantic ownership, authority-aware boundaries, typed lifetimes, configuration-specific claims and performance evidence. Keep placement, process primitives, migration formats and verification scope open. [The placement decision](../../docs/architecture-decisions/0008-placement.md) remains proposed; philosophical similarity alone does not select a kernel architecture.

[Russian translation](../../translations/ru/research/other-systems/COMPARISON.md)
