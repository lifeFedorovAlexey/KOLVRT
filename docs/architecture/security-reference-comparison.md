# Security reference comparison

Documentary research, 2026-10-02. No winner scores or foreign benchmarks. KOLVRT cells are intended design, not implemented security. [Source versions and evidence limits](../../research/other-systems/kasperskyos/sources.md).

## Primary references

Linux: [LSM hooks](https://docs.kernel.org/security/lsm-development.html) and [DMA](https://docs.kernel.org/core-api/dma-api-howto.html). KasperskyOS CE 1.2: [architecture](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_architecture.htm), [IPC](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_ipc_control.htm), [resource authority](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_resource_acces_control.htm). seL4: [capabilities](https://docs.sel4.systems/Tutorials/capabilities.html), [IPC](https://docs.sel4.systems/Tutorials/ipc.html), [proof scope](https://docs.sel4.systems/projects/sel4/verified-configurations.html). Zircon: [handles](https://fuchsia.dev/fuchsia-src/concepts/kernel/handles), [driver hosts](https://fuchsia.dev/fuchsia-src/concepts/drivers/driver_framework), [Starnix RFC](https://fuchsia.dev/fuchsia-src/contribute/governance/rfcs/0082_starnix). Redox: [official book](https://raw.githubusercontent.com/redox-os/book/master/src/our-goals.md). These references ground the following mechanism cells; unknown means evidence was not established here.

## Mechanisms

| Property            | Linux                         | KasperskyOS CE 1.2   | seL4                         | Fuchsia/Zircon           | Redox documented design       | Proposed KOLVRT                     |
| ------------------- | ----------------------------- | -------------------- | ---------------------------- | ------------------------ | ----------------------------- | ----------------------------------- |
| Privileged size     | Not measured                  | Not measured         | Not measured                 | Not measured             | Not measured                  | Closure inventory; LOC not measured |
| Driver privilege    | Kernel modules possible       | Generally user mode  | User drivers                 | User driver hosts        | User services/drivers         | EL0 default; workload decision      |
| IPC                 | Subsystem-specific            | Rendezvous           | Endpoints                    | Channels/handles         | Schemes                       | Bounded frames; transport open      |
| Authority           | Credentials/LSM               | OCap plus policy     | Object capabilities          | Handle rights            | Namespaces, UID/GID           | Native grants, effect scope         |
| Default deny        | Policy/config dependent       | Explicit policy      | Grant/config dependent       | Grant/routing dependent  | Safe-default goal             | Missing native grant denies         |
| Capability model    | Not object-cap equivalence    | Handle rights        | CSpace/CNode                 | Process-bound handles    | Capability transition planned | Narrow types                        |
| Policy evaluation   | Operation hooks               | IPC security module  | Kernel authority/user config | Rights/services          | Scheme/identity checks        | Issuance and necessary live checks  |
| Isolation domains   | Processes; modules privileged | Processes            | VSpaces/CSpace               | Process/host             | Processes/namespaces          | Actual address spaces               |
| Failure containment | Boundary/config dependent     | Policy/TCB dependent | Assumption/config dependent  | Shared host shares risk  | Not reproduced here           | Explicit blast radius/TCB           |
| Compat isolation    | Mechanism-specific            | Unknown here         | Workload-specific            | Starnix userspace design | Relibc/ports                  | Separate trust boundary             |
| Runtime policy cost | Not measured                  | Not measured         | Not measured                 | Not measured             | Not measured                  | Experiment required                 |

## Limits and lessons

Driver hosts can co-locate drivers. Zircon's [stub IOMMU](https://fuchsia.dev/reference/syscalls/iommu_create) pins memory without hardware access protection. [seL4 DMA assumptions](https://sel4.systems/Verification/assumptions.html) are separate from CPU-memory proof. Redox's planned capability evolution is not completed enforcement evidence. [Starnix implementation notes](https://fuchsia.dev/fuchsia-src/concepts/starnix/kernel) describe UAPI/user-copy and conformance work, not KOLVRT's desired retention promise.

[Capsicum research](https://www.cl.cam.ac.uk/research/security/capsicum/papers/2010usenix-security-capsicum-website.pdf) supplies a counterexample to equating capabilities with microkernel placement. Recommend attribution, bounded delegation and explicit assumptions; do not infer superior speed/security from architectural labels. [Disposition register](../../research/other-systems/kasperskyos/kolvrt-lessons.md).

[Russian translation](../../translations/ru/docs/architecture/security-reference-comparison.md)
