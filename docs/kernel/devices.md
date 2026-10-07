# Bounded native device observations

Document status: CURRENT
Evidence scope: Phase 4.0 descriptor contract; implementation and exact-source execution evidence are recorded separately below.
Current reference: [Device observation decision](../architecture-decisions/0027-device-observations.md)

<a name="kolvrt-devices-observations"></a>

## Device observations

The first device is the PL011 already discovered from the pinned QEMU virt boot DTB. Its immutable descriptor records one checked physical register region, one GIC SPI with level-high triggering, the interrupt controller reference and explicit boot-console reservation. Discovery validates the supported root-level topology, cell widths, exact resource counts, checked extents, non-overlap and controller association. Unsupported layouts fail explicitly; this is not a generic firmware bus mapper. DTB bytes belong to the existing trusted immutable boot-input boundary. Structural validation does not authenticate firmware cryptographically.

The descriptor describes hardware. It neither authorizes MMIO nor issues an IRQ capability. The current boot console retains ownership; an EL0 driver cannot acquire this device merely by presenting its descriptor. Phase 4 driver binding must separately justify a console handoff or select another device, validate grants and enforce their lifetime. Driver placement policy remains outside EL1. No new privileged entry point is required for these safe bounded data checks.

## Identity and lifetime

Device identity is an owner-scoped generation, separate from observed bytes and future binding identity. A trusted observation owner supplies a nonzero scope that it must not reuse while old claims can survive. Firmware cannot choose this scope. Generations increase without wrapping; exhaustion rejects further publication. Removing an observation invalidates its claims. Republishing identical physical observations produces a new generation and never restores an old claim or any authority. Claims from another scope, unknown or stale generations, and altered observations are rejected by the same production validation method.

The boot adapter has one owner and publishes one observation during boot. The bounded lifecycle methods specify identity invalidation; they do not implement hardware hotplug, driver shutdown or permission revocation. Scope allocation across future services, driver replacement and rebind protocols belong to their own reviewed contracts. Identity is not a persistent machine-global identifier or an external frozen ABI.

## Bounds and exclusions

This slice has one PL011 observation, one register region and one SPI. DMA, arbitrary buses, dynamic enumeration, live removal, EL0 MMIO mappings, IRQ delivery grants and a universal Device object are outside scope. Existing memory protection and boot mappings remain independently enforced. DEV and PROD use the same discovery checks. Physical ARM64 and runtime driver execution are not established by this descriptor contract.

## Validation

Host tests call the real discovery and identity methods with valid and invalid inputs. Rejections cover malformed resource and interrupt encodings, forbidden topology, overflow, overlap, controller mismatch, forged observations, stale identity and exhausted generation. Tests do not copy or corrupt production implementations. Real DEV and PROD boot must execute the discovery path against the QEMU-provided DTB; host fixtures alone do not establish that execution. Current-source receipts and independent architecture/EN-RU review are required before closing issue #79.

[Russian translation](../../translations/ru/docs/kernel/devices.md)

[Phase 4.0 acceptance](../../research/results/device-phase40-reviewed.json) records 98 kernel-core host tests and one doctest, 13 output-validator checks and the complete 144-obligation matrix: 32 actual executions and 112 uses of fresh same-run evidence. Both DEV/PROD suites passed 147 checks each; every suite and ordinary boot has exactly one verified PL011 observation with MMIO 0x09000000/0x1000 and IRQ 33. Independent architecture/code and complete EN/RU review are complete. READY applies only to the bounded issue #79 descriptor contract, not driver or physical-hardware readiness.

The runner now requires exactly one valid observation before accepting fresh machine-mode execution, including streaming native runtime. Missing, duplicate or malformed events fail; historical reader compatibility does not weaken this gate. [Initial acceptance](../../research/results/device-phase40.json) retains its earlier source scope.

## CLOCK pilot integration

The CLOCK passport reuses the fresh machine-mode executor and therefore must publish the same validated device observation before its own oracle can pass. Selecting a different external root does not bypass discovery or create a device grant. The shared-runner source change makes the previous receipt historical for that runner revision; descriptor implementation and accepted bounded scope remain unchanged.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.devices",
  "kind": "subsystem-contract",
  "summary": "Bounded native PL011 observations and owner-scoped nonwrapping identity, separate from device grants.",
  "units": [
    {
      "id": "kolvrt.devices.observations",
      "anchor": "kolvrt-devices-observations",
      "kind": "feature",
      "summary": "Checked boot-console descriptor and generation-safe observation claims.",
      "depends_on": ["law.031", "law.035", "adr.0008"],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "One root-level non-DMA PL011 discovered from the pinned boot DTB, immutable bounded observations, reserved boot-console ownership and owner-local identity validation.",
        "sources": [
          "crates/kernel-core/src/device.rs",
          "crates/kernel-core/src/lib.rs",
          "crates/kernel-core/src/platform.rs",
          "crates/kernel-core/tests/device_contract.rs",
          "crates/kernel-core/tests/boot_description.rs",
          "crates/kernel/src/main.rs",
          "crates/kernel/src/platform/mod.rs",
          "crates/xtask/src/output.rs",
          "crates/xtask/src/main.rs",
          "crates/xtask/src/native_apps.rs"
        ],
        "acceptance": [
          "research/results/device-phase40.json",
          "research/results/device-phase40-reviewed.json"
        ],
        "issues": [79],
        "adrs": ["adr.0027"],
        "limitations": [
          "No driver binding, MMIO/IRQ grants, DMA, hotplug, global scope allocator or physical ARM64 acceptance; boot console remains reserved."
        ],
        "next_gate": "Issue #80 must derive explicit binding/grants and any console handoff; no driver execution is claimed by this completed descriptor slice.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "STALE",
            "reason": "The CLOCK pilot changes shared execution/SDK sources; retained receipts remain historical until exact-source applicability is renewed. Production mechanism guarantees and historical acceptance are preserved.",
            "scope": "Pinned QEMU 10.1.0 virt/cortex-a57/TCG, two CPUs; bounded descriptor and observation identity only.",
            "receipt": "research/measurements/runs/1791406911825-phase40-device-reviewed-82cc5a1ec9cb.json",
            "receipt_sha256": "d1460f417412517edef54367766a2be65b0f9e8d5983f977d2e3a6921c902032"
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical device execution evidence."
          }
        ],
        "readiness": "READY",
        "roadmap_gate": "Phase 4.0",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Introduce the bounded descriptor implementation; execution acceptance remains separate."
          },
          {
            "from": "EXPERIMENTAL",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "Independent architecture/code and EN/RU review accepted the bounded contract; real DEV/PROD discovery and full 144-obligation matrix passed.",
            "acceptance": ["research/results/device-phase40.json"]
          }
        ],
        "readiness_acceptance": [
          "research/results/device-phase40-reviewed.json"
        ]
      }
    }
  ]
}
```
