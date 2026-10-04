# Kernel admission policy

Research outcome and admission criteria, not an implemented admission service. [ADR-0013](../architecture-decisions/0013-security-boundaries.md) accepts two boundary rules. [System threat model](../security/threat-model.md) defines protected properties.

## Necessity before placement

Global ownership, authority or coordination alone does not justify EL1. Prove that an EL0 service using existing narrow privileged primitives cannot safely enforce the required invariant. Admit only the irreducible privileged mechanism. IPC cost, serialization, convenience and safe Rust do not establish privileged necessity. Benchmark IPC, batching and shared-memory alternatives before a performance-driven placement proposal.

## Architecture before implementation order

Later-stage functionality may be implemented early. Before building a later milestone on an existing implementation, re-derive the required architecture from current invariants and accepted decisions. Implementation order, effort spent, compatibility with current tests and avoiding rework do not establish architectural authority. If an early implementation constrains, contradicts or prematurely freezes that architecture, refactor or remove it. Preserve the required guarantees and meaningful negative checks; a test's current representation is not itself a design requirement.

## Admission record

Require a record for every proposed new privileged responsibility, not merely for a
whole component. Review it before implementation through an ADR and updated threat model.
"Global owner", "authoritative database", "coordination" and measured speed are insufficient
answers to the EL0-alternative field. If existing primitives let an EL0 service enforce
the invariant, reject the proposed EL1 addition. A missing necessity argument blocks
admission even when benchmarks favor EL1. This is a design review gate, not a runtime API.

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

### Worked current-boundary record: mapping retirement

This records the existing bounded MMU/SMP boundary, not a new privileged admission or
permission to move allocation policy. Source boundaries are
[mapping ownership](../../crates/kernel/src/memory/mod.rs),
[AArch64 operations](../../crates/kernel/src/arch/aarch64/mod.rs) and
[SMP coordination](../../crates/kernel/src/smp.rs). Retained
[SMP evidence](../../research/results/kernel-smp.json) has its own source/build scope;
it is historical execution evidence, not a rerun of the current working tree.

| Field                  | Current boundary and review disposition                                                                                                                                                                                                                                                                                         |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Consumer and invariant | Two-CPU QEMU mapping/read/retire workload: a frame cannot be reused while any participating CPU retains a mapping or stale translation. Violations allow cross-owner reads or corruption.                                                                                                                                       |
| Privileged operation   | Protected page-table publication and privileged local translation invalidation with barriers on participating CPUs.                                                                                                                                                                                                             |
| EL0 alternative        | EL0 can select allocations and coordinate ownership through existing mapping/retirement primitives. EL0 alone cannot safely publish protected tables or perform privileged invalidation. Existing narrow primitives already cover this workload: no new EL1 responsibility is justified.                                        |
| Minimal mechanism      | Validate/retain the mapping and frame, publish the protected change, invalidate translations, acknowledge reader quiescence and gate reuse. Allocation strategy, file arbitration and application policy are excluded.                                                                                                          |
| Authority              | CPU0 owns the current physical allocator; internal guards retain frame identity. No general EL0 mapping-grant issuer or delegation/revocation API is claimed. A future interface must independently validate caller/object/effect scope before publication.                                                                     |
| Lifetime               | Retain backing storage through local invalidation and remote acknowledgement; delayed readers block reclamation. Missing acknowledgement is fatal within this bounded contract. DMA leases and device reset are outside it.                                                                                                     |
| TCB and unsafe         | Page-table writer, architecture invalidation, acknowledgement protocol, frame ownership and boot/compiler/platform assumptions are trusted for memory isolation. Corruption of those mechanisms is a fatal kernel failure, not a recoverable service crash.                                                                     |
| Profiles               | Permission, ownership, ordering and acknowledgement gates remain required in DEV and PROD; diagnostics cannot substitute for them.                                                                                                                                                                                              |
| Validation             | Retained checks cover delayed acknowledgement, blocked retiring-frame reuse, a real remote translation fault and safe reuse; negative controls include premature release, missing acknowledgement and omitted remote TLBI. No physical ARM64, arbitrary writers, DMA containment or general EL0 mapping service is established. |
| Reconsideration        | Kernel memory/architecture maintainers own review. Additional writers, migration, device access or a new mapping API require a new invariant/authority analysis and ADR; move policy out when existing narrow primitives suffice.                                                                                               |

## Performance proposals

Retain equivalent-workload latency, throughput and resource results under the
[benchmark methodology](benchmarking.md), including raw observations, digests, platform,
load and uncertainty. Keep authority, isolation, outcome semantics, quotas and accounting
equal across alternatives; a weaker guarantee is not a speedup for the same contract.
Evaluate applicable bounded IPC improvements, owned/shared-memory protocols and batching.
Record why each considered alternative is suitable or unsuitable; do not implement every
optimization mechanically. Shared memory requires permission, lifetime, ordering,
revocation and cancellation analysis; batching must retain bounds and truthful outcomes.

Even expensive IPC after optimization does not authorize EL1 service policy. Separate the
minimum privileged primitive from the service implementation. Revising a protection
boundary requires an explicit ADR and threat/failure analysis; there is no performance
exception hidden in code. This issue adds no benchmark result or kernel mechanism.

## Research follow-up

[Scenario specifications](../../research/fixtures/security-boundaries.json) describe future checks and remain unexecuted. Hardware DMA admission needs verified IOMMU configuration or an explicitly trusted device/programming boundary. Inspectable evidence follows the [inspection model](../security/inspection-model.md); shape validation cannot prove minimum privilege.

[Russian translation](../../translations/ru/docs/architecture/kernel-admission-policy.md)
