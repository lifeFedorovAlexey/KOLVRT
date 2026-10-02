# Primary-source ledger

Access date for all entries: 2026-10-02. URLs were read through web retrieval; no secondary articles support architectural conclusions. Summaries are paraphrases, not reproduced documentation. Search crawl timestamps are not publication dates. Living pages have no reproducible source commit unless explicitly stated; do not treat them as release-wide guarantees.

| ID  | Primary source                                                                                                        | Publisher, edition and date scope                             |
| --- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| K01 | [CE 1.2 overview](https://support.kaspersky.com/help/KCE/1.2/en-US/overview.htm)                                      | Kaspersky; CE 1.2; undated HTML                               |
| K02 | [CE architecture](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_architecture.htm)                         | Kaspersky; CE 1.2; undated HTML                               |
| K03 | [CE IPC control](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_ipc_control.htm)                           | Kaspersky; CE 1.2; undated HTML                               |
| K04 | [CE resource access](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_resource_acces_control.htm)            | Kaspersky; CE 1.2; undated HTML                               |
| K05 | [CE component specifications](https://support.kaspersky.com/help/KCE/1.2/en-US/ice.htm)                               | Kaspersky; CE 1.2; undated HTML                               |
| K06 | [CE IDL](https://support.kaspersky.com/help/KCE/1.2/en-US/ice_idl.htm)                                                | Kaspersky; CE 1.2; undated HTML                               |
| K07 | [CE guide](https://support.kaspersky.com/help/KCE/1.2/en-US/KasperskyOS-CE-en-US.pdf)                                 | Kaspersky; CE 1.2; copyright 2024; printed pp. 246–249        |
| K08 | [Developer FAQ](https://os.kaspersky.com/faq-general/)                                                                | Kaspersky; living/unversioned; date not stated                |
| K09 | [Historical security policies](https://support.kaspersky.com/kos/educationkitbeta0.1/en-us/security_policies.htm)     | Kaspersky; Education Kit Beta 0.1; reviewed 2020-05-19        |
| S01 | [seL4 capabilities](https://docs.sel4.systems/Tutorials/capabilities.html)                                            | seL4 project; living tutorial; version not pinned             |
| S02 | [seL4 IPC](https://docs.sel4.systems/Tutorials/ipc.html)                                                              | seL4 project; living tutorial; version not pinned             |
| S03 | [Verified configurations](https://docs.sel4.systems/projects/sel4/verified-configurations.html)                       | seL4 project; living configuration matrix                     |
| S04 | [Proof assumptions](https://sel4.systems/Verification/assumptions.html)                                               | seL4 project; living verification scope                       |
| F01 | [Zircon handles](https://fuchsia.dev/fuchsia-src/concepts/kernel/handles)                                             | Fuchsia project; living documentation                         |
| F02 | [Zircon rights](https://fuchsia.dev/fuchsia-src/concepts/kernel/rights)                                               | Fuchsia project; updated 2025-03-22                           |
| F03 | [Driver framework DFv2](https://fuchsia.dev/fuchsia-src/concepts/drivers/driver_framework)                            | Fuchsia project; living documentation                         |
| F04 | [IOMMU create](https://fuchsia.dev/reference/syscalls/iommu_create)                                                   | Fuchsia project; updated 2025-12-05                           |
| F05 | [BTI create](https://fuchsia.dev/reference/syscalls/bti_create)                                                       | Fuchsia project; updated 2025-03-04                           |
| F06 | [Starnix RFC-0082](https://fuchsia.dev/fuchsia-src/contribute/governance/rfcs/0082_starnix)                           | Fuchsia project; historical design RFC, 2021                  |
| F07 | [Starnix UAPI implementation](https://fuchsia.dev/fuchsia-src/concepts/starnix/kernel)                                | Fuchsia project; living implementation description            |
| R01 | [Redox goals and security design](https://raw.githubusercontent.com/redox-os/book/master/src/our-goals.md)            | Redox project book; mutable master, no commit pin             |
| L01 | [Linux LSM development](https://docs.kernel.org/security/lsm-development.html)                                        | Linux project; living documentation, not a kernel release pin |
| L02 | [Linux DMA guide](https://docs.kernel.org/core-api/dma-api-howto.html)                                                | Linux project; living documentation, not a kernel release pin |
| C01 | [Capsicum paper](https://www.cl.cam.ac.uk/research/security/capsicum/papers/2010usenix-security-capsicum-website.pdf) | Watson/Anderson/Laurie/Kennaway; USENIX Security 2010         |

## Claim locations and limitations

K01/K02 ground overview and privileged-mechanism facts; K03 grounds IPC checks; K04 grounds combined authority; K05/K06 ground interface generation; K07/K09 ground policy states; K08 grounds conditional DMA discussion. S01/S02 ground capabilities and IPC; S03/S04 ground proof limits. F01–F07 ground handles, driver hosts, IOMMU semantics and compatibility placement. R01 describes Redox's documented namespace/identity design and planned capability evolution, not completed universal object-capability enforcement. L01/L02 ground hooks and DMA address distinctions. C01 supplies a capability-research counterexample to equating capabilities with microkernel placement.

## Evidence unavailable

No foreign build/run, commercial kernel source audit, comparable privileged LOC, IPC benchmark, adversarial DMA run or general-purpose scalability proof was obtained. Guessed CE page URLs that failed retrieval are excluded. The Redox book site returned 403; the official project's raw book mirror was read instead. Older Education Kit terminology is explicitly historical. Public marketing promises are not converted into KOLVRT guarantees. Future experiments must pin source revisions and hardware configurations independently.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/sources.md)
