# ADR-0027 — Bounded native device observations

Status: **Accepted for the bounded Phase 4.0 contract**. Date: 2026-10-07.

Document status: CURRENT
Evidence scope: descriptor and observation-identity decision for one QEMU virt PL011; independent acceptance review and exact-source execution evidence remain separate.
Current reference: [Native device observations](../kernel/devices.md)

## Context

Issue #79 requires a native description before driver binding. Existing boot discovery reads the PL011 register region and the boot console already uses it. This existing use neither authorizes an EL0 driver nor justifies a universal device manager. [LAW-031](../architecture/kernel-laws.md#law-031) requires validated platform translation and rejects continuity inferred from identical replacement descriptions. [LAW-035](../architecture/kernel-laws.md#law-035) preserves enforcement across DEV/PROD. [ADR-0008](0008-placement.md) and the [admission policy](../architecture/kernel-admission-policy.md) keep service policy outside EL1 unless a concrete privileged invariant requires otherwise.

The pinned [QEMU 10.1.0 virt implementation](https://raw.githubusercontent.com/qemu/qemu/v10.1.0/hw/arm/virt.c) provides the selected topology and firmware description. The runtime parser validates actual supplied DTB bytes; constants in a host report do not substitute for discovery. The current immutable boot-input trust assumption remains explicit and does not establish cryptographic firmware authenticity.

## Decision

Keep one immutable PL011 descriptor with one checked MMIO region, one level-high GIC SPI, the validated interrupt-controller reference and the BootConsole reservation. Translate the supported root-level DTB subset once. Reject malformed cell widths, resource counts, ranges, overlap, controller references and unsupported interrupt encodings. Publish the private validated snapshot only after the complete discovery succeeds; later changes to public platform observations cannot rewrite that snapshot. Firmware identifiers describe topology and confer no authority.

Represent observation identity as a nonzero trusted-owner scope and a nonzero, monotonically increasing generation. The owner must allocate a scope unique among owners whose claims can meet and must not reset or reuse it while old claims survive. Firmware and untrusted claims cannot select the trusted owner. The boot adapter has one owner using scope 1 in its boot session and publishes once; this is not a machine-global allocator or persistent identity format.

The bounded Console state allows one current observation. An additional observation is rejected until the current identity is invalidated. Invalidation makes old claims stale; republishing identical descriptor bytes obtains a fresh generation. Generation exhaustion fails without wrapping or restoring a previous observation. Claim verification requires the current owner scope, exact generation and exact observed resource fields. Successful verification proves only that the claim matches an observation, never a grant.

The descriptor remains distinct from a future binding identity and explicit MMIO/IRQ grants. The boot console retains its reservation. Issue #80 must define binding, lifetime and any console handoff; #29 cannot claim this device by submitting matching descriptor bytes. These data contracts add no privileged syscall, mapping operation, driver policy or access authority.

## Alternatives

Considered alternatives are identity derived from physical observations, a generic device manager, and a separate test parser implementation.

## Why rejected

Deriving identity from MMIO addresses or compatible strings would revive old claims after a replacement and is rejected. A generic bus graph or Device mega-object would freeze unsupported policy and lifecycle obligations without a workload and is deferred. A separate copy of the parser for tests would fail to verify production behavior and is rejected.

## Consequences

The pure invalidation/republication methods express observation lifetime, not live hotplug, device reset, driver teardown or permission revocation. No hardware removal, DMA, arbitrary enumeration, grant transfer or automatic recovery is implemented by this decision. Future services must re-derive their scope allocation and binding architecture rather than inherit the boot singleton as a global service design.

## Compatibility impact

The internal descriptor and claim format creates no stable public ABI or Linux device compatibility layer. Existing boot-console ownership is preserved. Future binding and grants require an explicit contract rather than treating descriptor equality as compatible authority.

## Security impact

The descriptor is safe bounded data produced by the existing boot decoder. No new privileged mechanism is admitted; memory mappings and interrupt access remain separately enforced. An owner that reuses a scope outside the stated contract can alias identities, so future multi-owner integration must establish scope uniqueness. Structural validation does not defeat compromised firmware within the existing trusted boot boundary. DEV and PROD execute the same discovery checks.

## Performance impact

Validation occurs at discovery and observation transitions, not on the scheduler or IPC hot path. No timing, throughput, physical ARM64 or production trust improvement is claimed.

## Testing

Tests import the single production parser and identity implementation. They cover actual DTB discovery, truncation, forged or missing controller associations, duplicate identities/properties, invalid SPI/trigger, exact resource extents, overlap/overflow, altered claims, stale generations, scope separation and generation exhaustion. Production DEV/PROD boot must execute the real discovery path against QEMU's supplied DTB. Host fixtures alone do not prove guest execution or a working EL0 driver.

Independent Codex architecture/code and complete EN/RU semantic review accepted this bounded contract on 2026-10-07. Exact-source execution acceptance is recorded separately under the canonical device feature before issue closure. Historical Phase 3 receipts remain historical; this decision does not promote their verification or close later driver gates.

## Reversibility

The descriptor and identity types are internal and unfrozen. A replacement may narrow or revise their representation while preserving explicit observation/authority separation, stale-claim rejection and retained evidence. Future hardware or firmware support requires its own bounded contract review.

[Russian translation](../../translations/ru/docs/architecture-decisions/0027-device-observations.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0027",
  "kind": "adr",
  "summary": "Accepted bounded PL011 observations with owner-scoped identity and no implied device authority.",
  "aliases": ["ADR-0027"],
  "depends_on": ["adr.0008", "law.031", "law.035"]
}
```
