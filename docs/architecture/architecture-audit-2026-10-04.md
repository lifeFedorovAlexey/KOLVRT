# Architecture audit — 2026-10-04

Document status: HISTORICAL MILESTONE

Evidence scope: architecture review of published main da16d9f; retained runtime receipts identify their own c5c440c source.
Current reference: [Project status](../../README.md)

## Scope and result

The review covers all twelve workspace crates, build and CI boundaries, accepted ADRs through ADR-0021, and the distinction between implementation, models and research evidence. This is an architecture audit and issue disposition, not a proof that every possible defect has been excluded.

The kernel dependency remains `kernel-core`; routing, compatibility, migration, AI and package policy are outside its dependency closure. Current enforcement is a bounded two-CPU, fixed-affinity foundation. General capabilities, domains, IPC, supervision, persistent services and device isolation are subsequent acceptance gates.

Existing implementation does not establish architectural authority. Before using an early implementation for a later milestone, derive the architecture from current invariants and accepted decisions, then refactor or remove incompatible constraints. See [admission policy](kernel-admission-policy.md).

## Coverage and disposition

| Area                          | Reviewed boundary                                                                                                | Remaining requirements                                                                           |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `kolvrt-kernel`               | EL1/EL0 traps, CPU-owned scheduler state, lifecycle, roots/ASIDs, copy, handles and bounded SMP retirement       | #24–#31; current foundation is not a general service or device runtime                           |
| `kernel-core`                 | Checked value models, identity/generations, rights attenuation, retained Event storage, copy geometry, ELF plans | #70 entry alignment; #26 must enforce its waiter contract                                        |
| `native-protocol-model`       | Candidate bounded request/response encoding and initialized outputs                                              | #26; model encoding is not actual transport or frozen ABI                                        |
| `native-state-models`         | Finite transition exploration, ownership, waits, fallback and retirement                                         | #34/#35; stated finite domains are not a kernel proof                                            |
| `routing`                     | Versioned selection, retained generation/identity and profile separation                                         | #47/#49/#50; selection does not grant native authority                                           |
| `window-compat`               | Owned snapshots and isolated synthetic legacy semantics                                                          | #45–#51; these are not Linux-driver support                                                      |
| `routing-demo`                | Bounded actual EL0 routing payload and evidence counters                                                         | #28/#33; no persistent service or exclusive adapter-only attribution claim                       |
| `migration-advisor`           | Independent solution checking, contracts, statistics, trust and separate authorization receipt                   | #14/#71; no deployment executor; library clock/ledger lifetime needs explicit hardening          |
| `migration-workbench`         | Alternating real host fixture processes, deadlines, rollback fixture and experimental signatures                 | #14/#50; host fixtures do not establish physical ARM64 performance or producer independence      |
| `host-process-metrics`        | Narrow Windows process-peak wrapper; unsupported targets preserve UNKNOWN                                        | #32/#50; process peak is not per-request allocation or energy                                    |
| `repository-checks`           | Closed schemas, duplicate-key rejection, bounded COST-L records, document links/status and translation hashes    | #45/#46/#47; structural consistency does not establish historical truth or translation meaning   |
| `xtask`, build scripts and CI | Pinned toolchain/platform, QEMU watchdogs, framed failure-first evidence and DEV/PROD matrices                   | #32–#36; unsupported hardware, soak and fuzz results cannot be inferred from short matrix passes |

## Findings and actions

- [#70](https://github.com/lifeFedorovAlexey/KOLVRT/issues/70): the ELF parser accepts byte-offset entry points without A64 instruction alignment. Today's fixed-entry loader independently rejects these offsets. Require parser rejection and retained negative coverage before broadening the profile.
- [#71](https://github.com/lifeFedorovAlexey/KOLVRT/issues/71): enforce verifier/session lifetime and the replay ledger's crash model. Constructor-time clock snapshots and create-once markers do not alone establish live freshness or crash durability.
- [#26](https://github.com/lifeFedorovAlexey/KOLVRT/issues/26): added an explicit single-waiter rejection or bounded multi-waiter admission criterion. Shared object retention alone does not prove wake delivery.
- [#37](https://github.com/lifeFedorovAlexey/KOLVRT/issues/37): corrected closed foundation #16–#23 and #6, and closed routing investigation #15; kept native integration gates open.
- [#44](https://github.com/lifeFedorovAlexey/KOLVRT/issues/44), [#46](https://github.com/lifeFedorovAlexey/KOLVRT/issues/46): corrected the obsolete absence claim. The schema, semantic validator and three stable COST-L records are present; full research/runtime delivery remains open.
- [#56](https://github.com/lifeFedorovAlexey/KOLVRT/issues/56): recorded the five forms delivered by PR #60. Live chooser/render/sample acceptance remains open.
- Corrected the [system threat model](../security/threat-model.md) in both languages: ADR-0019 is historical baseline; ADR-0020 adds bounded transfer and retention. General revocation/domains remain unimplemented.

Requirements already tracked in #24–#36 and #38–#59 retain their separate native, research, production and hardware gates. No duplicate blanket “finish the architecture” issue was created.

## Evidence and limits

The [kernel receipt](../../research/measurements/runs/1791123211326-docs-main-c5c440c-final-fd10fb19115e.json) records source c5c440c, 84 kernel checks in each DEV/PROD profile, 70 host negative controls and both non-test boots. The [routing receipt](../../research/results/routing-main-c5c440c-regression.json) retains six configurations and three controls. Documentation commit da16d9f preserves these source-bound results.

New local test/output changes observed during this audit are not part of that receipt. No rerun, physical device validation, power-loss experiment, soak duration, hardware inventory or successful future milestone is claimed by this report. Old ADR evidence counts remain historical facts.

[Russian translation](../../translations/ru/docs/architecture/architecture-audit-2026-10-04.md)
