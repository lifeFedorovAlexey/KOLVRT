# Source assurance and threat limits

## Documented versus demonstrated

KasperskyOS documentation explains mechanisms and product-specific policy assumptions. No commercial kernel source audit, reproduced attack containment, or KasperskyOS hardware benchmark was performed in this investigation. Public SDK examples and third-party ports must not be confused with publicly auditable kernel source. The [ledger](sources.md) identifies edition/date and unavailable evidence.

Kaspersky's FAQ presents DMA without IOMMU as dependent on trusted programming code; this does not cover a malicious device independently issuing transactions. seL4's assurance likewise depends on stated configuration and hardware assumptions. These distinctions inform [KOLVRT's threat model](../../../docs/security/threat-model.md), not an assurance equivalence claim.

## Questions for each mechanism

Assess protected resource/effect, enforcement location, TCB dependency, compromised-component reach, mutable decision state, runtime work and progress bounds. Identify whether capabilities can encode a stable grant or a live predicate remains necessary. Describe driver/compat effects, automatic rejection tests, compiler/static checks and the first concrete general-purpose workload. Where source material does not establish a property, record unknown instead of inferring safety from architectural vocabulary.

## Rejected claims

Neither safe Rust, typed IPC, capabilities, a policy engine nor microkernel placement eliminates zero-days. CPU isolation does not prove DMA containment, availability or side-channel resistance. Separate prevention, containment, detection and recovery; a detector cannot repair a trusted invariant failure. Policy authoring errors and insufficiently scoped grants are failures of the security argument even if every policy check executes.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/threat-model.md)
