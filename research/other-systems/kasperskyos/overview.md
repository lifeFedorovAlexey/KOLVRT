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

Reviewed implementation baseline: b030592. The committed [SMP contract](../../../docs/kernel/smp.md), [EL0 foundation](../../../docs/kernel/el0.md) and [Phase 2 routing contract](../../../docs/kernel/routing.md) distinguish their retained execution evidence. Eight fixed-affinity EL0 workers run in private address spaces on two CPUs; the optional routing payload performs bounded synchronous own-task observations and EL0 adapter arithmetic. This research does not rerun those kernel matrices. General native IPC, caller-local capability syscalls, dynamic services, hostile-device DMA containment and general-purpose availability remain unimplemented or unverified. Existing bounded task-fault containment is not evidence for those larger guarantees.

The two user-confirmed decisions are no effective-authority expansion through compat and privileged necessity for EL1 admission. Other placement and policy choices remain research recommendations. Issues #3, #4, #8 and #9 now record bounded support, ABI publication and explicit fallback decisions; deployment-specific support promises and runtime enforcement still require evidence.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/overview.md)
