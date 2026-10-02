# Research conclusions and decision register

Status: documentary research completed; general service isolation, hostile DMA and comparative performance remain unverified. [Sources](sources.md) and [comparison](../../../docs/architecture/security-reference-comparison.md) ground foreign-system facts. The [boundary ADR](../../../docs/architecture-decisions/0013-security-boundaries.md) records the two user-confirmed rules and subsequent narrow capability/object admission reviews.

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

Choose caller attribution for multi-hop services, policy-change linearization, delegation scope and whether a file service enforces scope or receives independently constrained object grants. Determine concrete IOMMU/SMMU topology, reset completeness and unsupported-device rules. Define bounded inspector authentication/redaction and trusted recovery capacity. Quantify mixed-domain scheduling and policy cost. The explicit fallback and finite-support policies now have reviewed dispositions in issues #3/#4/#8/#9. Concrete stable-contract support durations, authenticated archive admission and runtime policy integration remain open implementation questions.

## Later milestones, not automatically started

1. Confirm current SMP evidence and preserve its ownership/retirement constraints.
2. Extend the existing bounded scheduler/EL0 foundation through separately scoped dynamic process/loader and fault-safe user-copy tasks (#20–#22); static boot workers do not satisfy general service prerequisites.
3. Implement bounded IPC, caller-local grants and admission/revocation tests including unknown effects.
4. Demonstrate isolated service faults and scoped compat effective-authority cases.
5. Select one concrete device; establish actual DMA/IRQ/MMIO restriction and reset before an isolated driver claim.
6. Benchmark targeted dynamic checks and transport options; then implement authenticated inspection and recovery.

## Completion report

The research set covers microkernel, domains, IPC, IDL, policy/capability composition, drivers, TCB and source assumptions. It recommends native scoped authority and explicit boundaries; rejects zero-day immunity, scores and speculative frameworks; marks transport/driver/policy cost for experiments. Intended TCB, driver/compat trust, DEV/PROD semantics, revocation and recovery are in the [security model](../../../docs/security/threat-model.md). No kernel code, fake CLI or security engine is implemented. Repository validation is reported separately from architectural assurance.

## Validation outcome

The closure baseline is b030592. Issue #12 completes documentary research and adds
only offline artifact checks. The proposed EL1 boundary is CPU/MMU entry, protection,
bounded scheduling/admission and required IRQ machinery; EL0 services, adapters and
device drivers are separate proposed domains with native-scoped grants. An EL0 mediator
remains in the TCB for effects it alone authorizes; hostile DMA needs a verified target
boundary, or explicit trust/unsupported status. See [TCB](tcb.md), [drivers](drivers.md)
and the [threat model](../../../docs/security/threat-model.md).

LAW-009 clarification and privileged-necessity admission are recorded in ADR-0013;
default denial/resource authority use LAW-009/LAW-013, and containment uses LAW-042.
No extra law, universal policy engine, object hierarchy, immunity claim or score is added.
Issues #5/#7/#11 supply the completed admission reviews. Outstanding implementation
work follows dependencies: processes/loader/user-copy (#20–#22), handles/grants/domains
(#23–#25), IPC (#26), supervisor/service (#27–#28), scoped driver/storage/filesystem
(#29–#31), then measurements and reliability (#32–#36). Research does not start them.
Open questions remain caller attribution, live policy linearization, device topology/reset,
inspection authentication/redaction and recovery reserves; performance needs equivalent
workloads and recorded configurations under [the experiment protocol](performance-tradeoffs.md).

Closure validation on 2026-10-02: the complete repository check stages passed on an isolated b030592 snapshot plus this change: formatting (Prettier/Taplo/rustfmt), Markdownlint, workspace Clippy and tests, model evidence and research/document/translation validation. Two new offline security-artifact tests reject malformed domain records and unsupported execution claims. No new kernel, foreign-system or DMA experiment was run; future security scenarios remain not executed. Concurrent scheduler changes are excluded from this research commit.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/kolvrt-lessons.md)
