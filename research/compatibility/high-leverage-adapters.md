# High-leverage compatibility adapter research

Document status: CURRENT
Document scope: initial observed source reach and a decision method; no working adapters or coverage percentages.
Status reference: [Retained observations](linux-driver-api-observations.json)

## Actual initial observation

The [pinned three-file sample](linux-driver-api-map.md) contains these observed lexical family references. Counts refer to source files within the chosen sample, not executed or compatible drivers.

| Family                                                       | Files with observed references | Meaning                                                                       |
| ------------------------------------------------------------ | ------------------------------ | ----------------------------------------------------------------------------- |
| Allocation, locking, deferred work                           | 3 each                         | Shared investigation candidates; semantics/configuration closure unresolved   |
| IRQ, DMA, PCI, USB                                           | 1 each                         | Device/transport authority and lifetime need independent contracts            |
| Power management, device model, sysfs, ioctl, network, block | 1 each                         | Specific source references; do not infer standalone reusable adapters         |
| Firmware                                                     | 0                              | No selected symbol observed; indirect requirements and broader corpus unknown |

The result supports reviewing shared allocation/context/deferred-work assumptions first. It does not show that three adapters enable these drivers. Low direct DMA reach is especially misleading for USB/VirtIO paths whose helpers and transport layers are outside this sample.

## Research method and stop conditions

1. Pin Linux revision, architectures/configurations, driver selection and exclusions. Include representative hardware/classes and disconfirming cases; report convenience-sample bias.
2. Build direct and transitive API/callback/state dependencies. Map actual observable semantic differences to reviewed COST-L causes; a family name alone is insufficient.
3. For each candidate adapter, prove required semantic closure, shared-state identity, supported operation/layout versions and the narrow native authorization/effect boundary.
4. Report potential reach only for consumers whose full prerequisites are established. Keep incomplete/conditional edges visible; denominator and corpus scope accompany any future proportion.
5. Compare hardware relevance, ARM64 applicability, security/TCB, complexity/API churn, measured-or-unknown runtime cost, maintenance, migration/support and native alternatives as separate dimensions.
6. Stop compatibility work when it expands authority, requires Linux semantics in native EL1, lacks bounded progress or cannot establish IRQ/DMA quiescence/reset. Reassess native rewrite, deferral or unsupported status before growing a second Linux kernel.

No weighted security/debt score or invented 10/20/40-family percentage is produced. Actual hardware popularity and enabled IOMMU/device containment remain UNKNOWN; source IDs alone cannot establish them. A source-backed hardware requirement may need native support without being software debt.

## Current decisions and remaining work

Allocation-context COST-L-0001 is a research candidate, not an implemented adapter proposal. Context-coupled locking/deferred work remain dossiers to split by actual state/progress assumptions. DMA ownership is a counterexample to blanket legacy criticism: the upstream guide already specifies map/unmap lifetime and barriers. Preserve the [seed dispositions](taxonomy.md).

Issue #57 owns the complete adapter-versus-native-rewrite rubric and worked acceptance decisions; this method is input, not a duplicate implementation. Issue #55 owns confirmed driver/debt graph edges. #45 remains open for configured/transitive and historical analysis. #59 considers host grouping/crash containment only after native service/device contracts are explicit; its prototype is not authorized here.

Benchmark contracts reuse #49/#50 and the existing migration advisor. Compare equal useful results, authority, outcomes and lifetime; retain raw provenance and honestly faster compatibility. Observed source counts are not costs. No Linux Driver Host, driver port, physical inventory or runtime compatibility is delivered.

[Russian translation](../../translations/ru/research/compatibility/high-leverage-adapters.md)
