# Driver authority and DMA

## Source findings

Kaspersky's FAQ distinguishes hardware IOMMU containment from a trusted DMA-programming component on hardware without IOMMU. This is a conditional trust argument, not evidence that scanning eliminates vulnerabilities or contains malicious hardware. [Developer FAQ](https://os.kaspersky.com/faq-general/).

Zircon's stub IOMMU pins memory but explicitly provides no hardware memory-access protection. [IOMMU creation](https://fuchsia.dev/reference/syscalls/iommu_create). A BTI is associated with an IOMMU and hardware transaction identifier. [BTI creation](https://fuchsia.dev/reference/syscalls/bti_create). Linux's DMA API distinguishes CPU and device addresses; address mapping is not by itself an adversarial-device isolation claim. [DMA guide](https://docs.kernel.org/core-api/dma-api-howto.html).

## Proposed driver grant

For wifi0, independently authorize its MMIO range, IRQ source, device generation and accounted DMA leases. Deny arbitrary RAM/MMIO, unrelated IRQ/devices, kernel memory and debug access. MMIO must be scoped by register functionality where a region controls other devices or unrestricted DMA; range bounds alone may be insufficient.

| Platform condition                                              | Honest containment claim                                                                                       |
| --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Effective IOMMU/SMMU with verified stream identity and topology | Potential DMA restriction to granted pages; test bypasses, shared groups, peer transactions and reset behavior |
| No hardware DMA restriction, trusted programming mediator       | Depends on mediator and device correctness; cannot contain malicious DMA hardware                              |
| Untrusted direct DMA programming without restriction            | No memory containment; reject target/workload or explicitly include device and programming code in TCB         |
| Bounce buffers only                                             | Copy/lifetime mechanism; no restriction on arbitrary device DMA                                                |

## Revocation and recovery gate

Stop new submissions, retain pinned charges, drain or establish reset completion, revoke translations/interrupt routes with required barriers, then reclaim memory and publish a fresh device generation. A timeout retains quarantine or triggers fatal stop, never reuse. Test stale driver access, foreign MMIO/IRQ, ongoing DMA at crash, missing reset acknowledgement and device replacement. Actual target evidence is mandatory; future EL0 driver placement alone cannot establish these properties.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/drivers.md)
