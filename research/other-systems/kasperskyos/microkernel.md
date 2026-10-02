# Privileged mechanisms

## Source finding

CE 1.2 describes rendezvous IPC, memory/thread managers, I/O resource management, ROMFS/ELF startup and optional debugging. Ordinary drivers generally execute with application-level privilege; HAL includes low-level device support. [Architecture](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_architecture.htm).

## KOLVRT inference

A microkernel label does not prove that every privileged subsystem is necessary. Kernel integrity depends on entry code, MMU changes, IRQ routing, memory ownership enforcement and authority validation. A compromised member of that boundary can invalidate all service isolation. Global bookkeeping can belong to an EL0 arbiter; only unavoidable privileged enforcement belongs in EL1.

KOLVRT should review each responsibility through the [admission policy](../../../docs/architecture/kernel-admission-policy.md). Do not copy ROMFS, ELF loading, object manager, futex ABI or the three-syscall interface merely because they are in this source. Narrow resource interfaces suffice until workloads demonstrate common semantics.

## Costs and enforcement

Separating services adds request validation, scheduling transitions and failure supervision. Static dependency checks can reject adapter imports into the native core; they cannot prove minimum privilege. A placement record needs a concrete invariant, an EL0 alternative, threat assumptions and negative tests. The first real EL0 workload must reject unauthorized memory/entry operations and contain a service fault. General-purpose scaling, DMA placement and multi-service scheduling remain unmeasured.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/microkernel.md)
