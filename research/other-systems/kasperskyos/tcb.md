# Trusted computing base

## Evidence boundary

The seL4 verified-configuration page excludes device-address translation, debug interfaces and boot startup from its listed verified configurations. [Verified configurations](https://docs.sel4.systems/projects/sel4/verified-configurations.html). Its assumptions separately discuss DMA, hardware and machine-interface trust. [Proof assumptions](https://sel4.systems/Verification/assumptions.html). These are examples of explicit assurance scope, not evidence of equivalent KOLVRT proofs.

## KOLVRT TCB by property

| Property                                  | Components whose failure can invalidate it                                                                                                                    |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Kernel integrity and CPU memory isolation | EL1 entry/MMU/IRQ/ownership code, safe algorithm dependencies, generated instructions, compiler/runtime, boot assumptions, CPU/MMU                            |
| Caller-authorized file effects            | Above plus native grant issuer, caller attribution, namespace resolver and any service enforcing file scope, even at EL0                                      |
| Device-to-memory containment              | Above plus actual IOMMU/SMMU configuration, stream topology, invalidation, device/reset assumptions; trusted DMA mediator when hardware restriction is absent |
| Recovery without use-after-free           | Reference/charge arbiters, quiescence acknowledgements, restart supervisor and device completion/reset contract                                               |

A safe crate is not outside the TCB merely because it forbids unsafe. An EL0 driver may be outside kernel-integrity TCB on an adequately isolated platform yet remain trusted for data correctness and availability of its device. A broad compat service that enforces caller scope remains in that property's TCB until enforcement is independently constrained.

## Inventory and useful metrics

[Current unsafe register](../../../docs/kernel/unsafe.md) and cargo xtask audit inventory lexical first-party unsafe locations, assembly hashes, generated wrappers, compiler-runtime artifacts and native dependency tree. The current snapshot has 67 locations, not 67 unsafe blocks or a risk score; its freshness and test-feature scope must accompany any report. No comparable TCB LOC metric was computed. Measure compiled privileged closure separately from source LOC and safety annotations. Kernel manifests depend on kernel-core, not the standalone routing/window-compat crates; recheck features and generated code before relying on the boundary.

Record EL1 versus EL0 unsafe location counts, active privileged interfaces, per-domain grants, raw-device/DMA authority and source/compiler/configuration hashes. Distinguish a recorded zero from unavailable observations. An unsafe budget may trigger review, but does not prove safety or grant permission for extra unsafe. No numeric security score is justified.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/tcb.md)
