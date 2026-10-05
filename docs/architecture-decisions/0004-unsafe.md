# ADR-0004 — Safe Rust and audited boundaries

Status: **Accepted**. Date: 2026-10-02.

Document status: CURRENT
Evidence scope: unsafe-boundary obligations; the original test-status claim applies only to Phase 0.
Current reference: [Later verification evidence](../documentation-policy.md)

## Context

Dirty Pipe, DMA ordering and reclamation require explicit lifetime arguments (cases 08, 10, 22).

## Decision

Use safe Rust by default. Restrict unsafe code to boundary crates with an invariant inventory and SAFETY comments.

## Alternatives

Allow unsafe code wherever convenient; forbid it even for memory-mapped hardware access.

## Why rejected

The former disperses the audit surface; the latter prevents implementing hardware boundaries.

## Consequences

Review and complementary tests are required. Safe Rust does not prove semantic correctness.

## Compatibility impact

Legacy parsing has no exemption from the safety policy.

## Performance impact

Safe abstractions may have overhead. Optimization requires justification and measurements.

## Security impact

Dependencies, generated code, Send/Sync and assembly belong in the trusted computing base inventory.

## Testing

Use host models, Miri where applicable, concurrency tests, QEMU and hardware. Kernel
checks had not run in Phase 0; later retained QEMU matrices have their own scope and
do not certify all unsafe boundaries or physical hardware.

## Reversibility

Wrappers may be replaced; public safety guarantees cannot be silently weakened.

## Evidence

[Cases and sources](../research/case-index.md); [other systems](../research/reference-systems.md).

[Russian translation](../../translations/ru/docs/architecture-decisions/0004-unsafe.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0004",
  "kind": "adr",
  "aliases": ["ADR-0004"],
  "summary": "Scoped architecture decision; acceptance is intended authority, not proof of all implementation.",
  "tags": ["architecture", "decision"]
}
```
