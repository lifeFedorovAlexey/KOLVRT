# ADR-0001 — Native authority and legacy isolation

Status: **Accepted**. Date: 2026-10-02.

## Context

Permanent external layouts can constrain internal change. Cases 06 and 28 are reference evidence for separating these contracts.

## Decision

The core depends only on native contracts. Versioned adapters own legacy behavior. A native-only build is required.

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

Before ABI release, revise the boundary through a decision record. Afterwards, preserve supported old contracts in adapters.

## Evidence

[Cases and sources](../research/CASE_INDEX.md); [other systems](../../research/other-systems/COMPARISON.md).

[Russian translation](../../translations/ru/docs/architecture-decisions/0001-native-authority.md)
