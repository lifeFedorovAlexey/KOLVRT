# KOLVRT vision

KOLVRT means **Kernel Outside Legacy, Versioned Routing & Translation**. It is a new
ARM64-first operating-system project, initially implemented in Rust. Its architectural
requirements do not depend on that implementation language. Phase 0 establishes
contracts and evidence before kernel implementation.

The kernel must not adapt itself to legacy. Legacy compatibility must adapt itself to
the kernel. The current native specification is authoritative. Versioned adapters may
preserve required older semantics, but cannot weaken memory safety, authorization,
resource accounting or ownership. Behavior that cannot be expressed safely is rejected.

Compatibility is not a system-wide mode. Packages, processes, drivers, devices and API
families may use different routes concurrently. Shared state determines which operations
must remain bound together. A module has an identity, semantic version, implementation
digest, owner, consumers, resource limits, measurements and a removal contract.

The initial target is AArch64, 64-bit execution at EL1, MMU, SMP, GICv3, the Arm Generic
Timer, Device Tree, UART and VirtIO on QEMU virt. PCIe is a possible transport extension;
ACPI and x86_64 are later considerations. Platform details must not require rewriting
core subsystems for another architecture.

Software compatibility is optional in a native build. A workaround for faulty hardware
may still be mandatory on an affected native target. Removing that workaround also
removes support for that target unless another correct implementation exists.

KOLVRT requirements come from its intended workloads, native correctness and target hardware. External failure histories help identify risks, but do not select the architecture. Account for survivorship bias: visible surviving projects overrepresent some choices, while abandoned approaches and unreported failures are missing. Seek disconfirming cases, alternative mechanisms and conditions of applicability. Prefer the smallest design that satisfies a measurable KOLVRT need; defer features with no demonstrated consumer. See the [research method](research/RESEARCH_METHOD.md).

A KOLVRT bug must be corrected in the native specification and implementation with a
regression test. A demonstrated need for safe old behavior may justify a separate
bug-compatibility module, a migration plan and eventual removal. No recent calls does
not prove the absence of dormant or offline consumers.

Phase 0 delivers research records, sources, requirements, architecture decisions and
test obligations. It does not deliver a boot stub, a kernel, drivers, compatibility implementations
or invented performance results. Normative requirements describe future implementation
obligations; they do not claim that execution tests already pass. The [next research phase](research/PHASE_0_2.md)
does not authorize starting Phase 1 automatically.

[Russian translation](../translations/ru/docs/vision.md)
