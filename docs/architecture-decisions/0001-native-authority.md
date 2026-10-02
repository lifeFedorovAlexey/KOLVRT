# ADR-0001 — Native authority and legacy isolation

Status: **Accepted**. Date: 2026-10-02.

## Context

Permanent external layouts can constrain internal change. Cases 06 and 28 are reference evidence for separating these contracts.

## Decision

The core depends only on native contracts. Versioned adapters own legacy behavior. A native-only build is required.

Issue #4 clarifies publication under LAW-001 and LAW-004. Use EXPERIMENTAL, CANDIDATE,
PUBLIC and STABLE per named contract/version as specified in the
[native ABI lifecycle](../architecture/native-abi.md). PUBLIC has provisional support
and does not imply freeze. STABLE requires a separate accepted ABI-FREEZE ADR covering
encodings/operations, consumers, conformance evidence, negotiation and finite support/migration
policy. Candidate v0 remains unfrozen. This decision defines the gate; it is not an ABI-FREEZE.

## Alternatives

A core defined by a foreign ABI; a global compatibility mode; no compatibility at all.

## Why rejected

These respectively bind the core to a foreign ABI, exclude mixed consumers, or prevent practical migration.

## Consequences

Contracts, adapters and conformance fixtures are required. Some external semantics may be inexpressible. No foreign ABI is a product requirement without demonstrated consumer need.

## Compatibility impact

Old consumers receive explicitly supported behavior without silent fallback.

## Performance impact

Measure translation costs; native superiority is not assumed.

## Security impact

Adapters cannot bypass native rights. Their placement is a separate decision.

## Testing

Check the native-only dependency graph and build; apply the same native oracle with and without adapters.

## Reversibility

Before freeze, breaking replacement/removal is permitted with documented status and explicit version rejection while honoring separately declared support. After freeze, preserve covered supported contracts for the stated window. No stage relaxes running authority, safety or outcome guarantees.

## Evidence

[Cases and sources](../research/case-index.md); [other systems](../research/reference-systems.md).

[Russian translation](../../translations/ru/docs/architecture-decisions/0001-native-authority.md)
