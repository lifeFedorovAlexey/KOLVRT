# ADR-0004 — Safe Rust and audited boundaries

Status: **Accepted**. Date: 2026-10-02.

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

Use host models, Miri where applicable, concurrency tests, QEMU and hardware. Kernel checks have not run in this phase.

## Reversibility

Wrappers may be replaced; public safety guarantees cannot be silently weakened.

## Evidence

[Cases and sources](../research/CASE_INDEX.md); [other systems](../../research/other-systems/COMPARISON.md).

[Russian translation](../../translations/ru/docs/architecture-decisions/0004-unsafe.md)
