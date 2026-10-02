# Security domains

## Source finding

Kaspersky's developer FAQ describes isolated components, explicitly permitted interactions and solution-specific security objectives/TCB assumptions. Its marketing language is not an independent proof of containment. [Official FAQ](https://os.kaspersky.com/faq-general/).

## Proposed KOLVRT boundary

Define a domain by actual address-space enforcement, native grants, allowed resource/effect scope and failure contract. Crate, API family and semantic state domain are not necessarily protection domains. Keep service instances and shared address spaces visible: compromise of one co-resident component can affect the others.

The caller identity must come from trusted entry/transport context, never a consumer-supplied message field. A service may use its own storage capability for an authorized caller operation. Check the caller's effect scope and retain that attribution across nested requests. The service is in the TCB for any property it alone enforces; EL0 does not erase that obligation.

## Review and tests

Admission binds domain generation, rights, quotas and a pinned semantic route. State-dependent authorization needs synchronization with admission, not a stale cached verdict. CPU isolation cannot constrain hostile DMA without a separate hardware boundary. Model forged identity, cross-domain handle import, service crash, quota exhaustion and privilege reuse after restart. Runtime memory-fault evidence requires EL0, which is not implemented now. Creation/supervision cost and general-purpose deployment scale require future measurements.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/security-domains.md)
