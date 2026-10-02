# Research conclusions and decision register

Status: documentary research completed; executable isolation, DMA and comparative performance remain unverified. [Sources](sources.md) and [comparison](../../../docs/architecture/security-reference-comparison.md) ground foreign-system facts. The [boundary ADR](../../../docs/architecture-decisions/0013-security-boundaries.md) records only the two user-confirmed rules.

## Mechanism dispositions

| Mechanism                                   | Disposition          | Reason and next evidence                                                             |
| ------------------------------------------- | -------------------- | ------------------------------------------------------------------------------------ |
| Narrow privilege boundary                   | Applicable           | Privileged necessity, not global coordination or convenience; issue #5               |
| Default-deny scoped grants                  | Applicable           | Explicit native authorization; missing permissions reject before effects             |
| Typed IPC and checked response              | Applicable           | Narrow frames and owned snapshots; actual EL0 user-copy/fault tests still needed     |
| Capability-first issuance                   | Partially applicable | Stable grants help; quotas, revocation, time and reassignment require live semantics |
| User-space drivers/services/compat          | Requires experiment  | Preserve resource-specific TCB and measure actual failure containment; DMA first     |
| Policy separated from business logic        | Applicable principle | Reviewable native decisions; no universal language required                          |
| Generated PSL-style global policy engine    | Not applicable now   | No workload proves the complexity/cost necessary; not a permanent ban                |
| Unified kernel object hierarchy             | Not applicable now   | Reuse narrow mechanics only; issue #11 requires multiple real workloads              |
| Full formal assurance by analogy with seL4  | Rejected claim       | No KOLVRT proof or equivalent hardware/configuration argument exists                 |
| Diagnostics graph and restart orchestration | Requires experiment  | Schema now; authenticated bounded inspection and recovery later                      |

## Laws and ADR disposition

LAW-009 is clarified as no effective consumer authority expansion; service-private capabilities are permitted only within native-authorized caller effects. Existing protection/quota and delegation obligations remain. EL1 admission becomes an explicit policy under the boundary ADR, without allocating a slogan-only law ID.

Default Deny and Explicit Resource Authority strengthen existing authority/admission obligations; Compromise Must Have a Boundary is already covered by LAW-042 and the threat model. No extra laws are added merely to reach a count. Driver placement, a universal revocation tree and dynamic-policy implementation ADRs are deferred until concrete workloads exist. The admission template and authority examples support issues #5 and #7 without implementing them.

## Remaining questions

Choose caller attribution for multi-hop services, policy-change linearization, delegation scope and whether a file service enforces scope or receives independently constrained object grants. Determine concrete IOMMU/SMMU topology, reset completeness and unsupported-device rules. Define bounded inspector authentication/redaction and trusted recovery capacity. Quantify mixed-domain scheduling and policy cost. Fallback timing, STABLE support duration and archived compat admission remain the user's unresolved issue-review questions.

## Later milestones, not automatically started

1. Confirm current SMP evidence and preserve its ownership/retirement constraints.
2. Separately authorize scheduler, EL0 address spaces and fault-safe user-copy for the first native slice.
3. Implement bounded IPC, caller-local grants and admission/revocation tests including unknown effects.
4. Demonstrate isolated service faults and scoped compat effective-authority cases.
5. Select one concrete device; establish actual DMA/IRQ/MMIO restriction and reset before an isolated driver claim.
6. Benchmark targeted dynamic checks and transport options; then implement authenticated inspection and recovery.

## Completion report

The research set covers microkernel, domains, IPC, IDL, policy/capability composition, drivers, TCB and source assumptions. It recommends native scoped authority and explicit boundaries; rejects zero-day immunity, scores and speculative frameworks; marks transport/driver/policy cost for experiments. Intended TCB, driver/compat trust, DEV/PROD semantics, revocation and recovery are in the [security model](../../../docs/security/threat-model.md). No kernel code, fake CLI or security engine is implemented. Repository validation is reported separately from architectural assurance.

## Validation outcome

As of 2026-10-02: Prettier, Markdownlint (148 files), documents/translations and research database checks, cargo test --locked (50 tests), native state model reproduction and git diff --check passed. Full npm run check stopped at cargo fmt in concurrently modified crates/kernel/src/arch/aarch64/mod.rs and crates/kernel/src/memory/mod.rs. Separate lint stopped at Clippy dead_code for USER_ACCESS, NOT_GLOBAL and user_descriptor in kernel page.rs included by xtask/locking. Those kernel changes are outside this research and were not repaired here. This validates the working tree, not hardware isolation; new security scenarios were not executed. Research is uncommitted; full-check acceptance remains open.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/kolvrt-lessons.md)
