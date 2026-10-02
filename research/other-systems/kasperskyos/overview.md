# KasperskyOS security research

Status: documentary research, 2026-10-02; no foreign system was built or benchmarked. This addresses [issue #12](https://github.com/lifeFedorovAlexey/KOLVRT/issues/12) and supplies evidence for issues #5, #7 and #11. Published mechanisms, KOLVRT inferences and untested recommendations are distinguished. CE 1.2 is a versioned reference, not a claim about the latest commercial release. [Source ledger](sources.md).

The CE overview describes a separation microkernel and security monitor. [CE 1.2 overview](https://support.kaspersky.com/help/KCE/1.2/en-US/overview.htm).

## Investigation map

- [Privileged mechanisms](microkernel.md) and [security domains](security-domains.md).
- [IPC](ipc.md), [typed interfaces](typed-interfaces.md) and [policy versus capability issuance](security-policy.md).
- [Drivers and DMA](drivers.md), [TCB](tcb.md) and [source assurance limits](threat-model.md).
- [Performance experiments](performance-tradeoffs.md) and [conclusions, dispositions and implementation gates](kolvrt-lessons.md).
- [Comparison](../../../docs/architecture/security-reference-comparison.md), [KOLVRT threat model](../../../docs/security/threat-model.md) and [EL1 admission policy](../../../docs/architecture/kernel-admission-policy.md).

## Current KOLVRT scope

HEAD is a9371e7b81d6d6befc725572d9b9c697e998a14c, with an uncommitted two-CPU foundation. The [SMP contract](../../../docs/kernel/smp.md) and [retained results](../../results/kernel-smp.json) describe 39 tests per profile and eight negative controls. These are existing evidence, not tests rerun by this research. Kernel runtime has no scheduler, EL0 domains, capability syscalls or DMA isolation. Standalone synthetic routing is not integrated kernel compatibility. All future security-domain claims below are requirements, not implemented isolation.

The two user-confirmed decisions are no effective-authority expansion through compat and privileged necessity for EL1 admission. Other placement and policy choices remain research recommendations; unresolved fallback/support questions are not decided here.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/overview.md)
