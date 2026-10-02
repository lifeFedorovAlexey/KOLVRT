# ADR-0007 — Bug compatibility and module retirement

Status: **Accepted**. Date: 2026-10-02.

## Context

An error can become a dependency, but security defects must not be preserved (cases 01, 07, 08, 23, 29).

## Decision

Fix native behavior and add regression checks. Preserve safe old semantics only in an explicit optional module backed by consumer evidence. Retire it using declared and observed dependencies.

## Alternatives

Keep the native bug forever; break all consumers immediately; generate an adapter for every bug.

## Why rejected

These undermine native correctness, prevent migration, or create unsupported maintenance debt.

## Consequences

Maintain a registry, retirement records, migration windows and accounting for dormant or offline consumers.

## Compatibility impact

Reject requests incompatible with security. Old semantics do not grant a right to exploit a defect.

## Performance impact

Lifecycle accounting has costs outside the hot path; runtime metrics depend on the profile.

## Security impact

Never restore unsafe behavior or unload referenced code.

## Testing

Check migration and native-only operation. Zero observed consumers with incomplete coverage does not permit removal.

## Reversibility

A supported module may return as a new artifact before support expires; the native bug must not return.

## Evidence

[Cases and sources](../research/case-index.md); [other systems](../research/reference-systems.md).

[Russian translation](../../translations/ru/docs/architecture-decisions/0007-bug-compat.md)
