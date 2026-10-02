# Kernel admission policy

Research outcome and admission criteria, not an implemented admission service. [ADR-0013](../architecture-decisions/0013-security-boundaries.md) accepts two boundary rules. [System threat model](../security/threat-model.md) defines protected properties.

## Necessity before placement

Global ownership, authority or coordination alone does not justify EL1. Prove that an EL0 service using existing narrow privileged primitives cannot safely enforce the required invariant. Admit only the irreducible privileged mechanism. IPC cost, serialization, convenience and safe Rust do not establish privileged necessity. Benchmark IPC, batching and shared-memory alternatives before a performance-driven placement proposal.

## Admission record

| Field                  | Required evidence                                                             |
| ---------------------- | ----------------------------------------------------------------------------- |
| Consumer and invariant | Real workload, protected resource and failure consequence                     |
| Privileged operation   | Instruction, register or memory-protection action requiring privilege         |
| EL0 alternative        | Existing primitives considered and the specific invariant they cannot enforce |
| Minimal mechanism      | Smallest EL1 implementation and excluded service policy                       |
| Authority              | Native issuer, object/effect scope, delegation and revocation                 |
| Lifetime               | Ownership, interrupts, DMA completion and reset/timeout behavior              |
| TCB and unsafe         | Property-specific trust, dependencies and reviewed unsafe invariants          |
| Profiles               | Same enforcement in DEV/PROD; diagnostic removal cannot remove protection     |
| Validation             | Negative cases, measurements where relevant, hardware assumptions and gaps    |
| Reconsideration        | Owner and condition for moving policy or code out of EL1                      |

## Existing example and limits

AArch64 page-table publication and local privileged TLBI/barriers require a privileged mechanism. An EL0 service cannot execute the privileged instruction itself; this does not justify moving its allocation policy into EL1. The existing [two-CPU SMP contract](../kernel/smp.md) retains CPU0 physical ownership and requires acknowledged reader quiescence before reclamation. Its recorded tests cover that bounded implementation; they do not prove arbitrary EL0 address spaces or DMA containment.

A global file arbiter or authorization database may live in an EL0 service. Being authoritative does not establish an EL1 requirement. No new scheduler, policy engine, capability type or universal object model is admitted by this document.

## Research follow-up

[Scenario specifications](../../research/fixtures/security-boundaries.json) describe future checks and remain unexecuted. Hardware DMA admission needs verified IOMMU configuration or an explicitly trusted device/programming boundary. Inspectable evidence follows the [inspection model](../security/inspection-model.md); shape validation cannot prove minimum privilege.

[Russian translation](../../translations/ru/docs/architecture/kernel-admission-policy.md)
