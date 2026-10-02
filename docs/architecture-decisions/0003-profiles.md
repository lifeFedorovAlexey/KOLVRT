# ADR-0003 — One architecture across execution profiles

Status: **Accepted**. Date: 2026-10-02.

## Context

Debug instrumentation changes timing; release builds must retain safety (cases 10, 18, 30).

## Decision

STAGING uses production configuration with bounded diagnostics. DEV, STAGING and PROD share one native contract.

## Alternatives

A separate production kernel; debug-only testing; full tracing everywhere.

## Why rejected

Architectures diverge, debug timing can mask failures, and unrestricted tracing exceeds cost budgets.

## Consequences

The test matrix covers optimizer and instrumentation combinations. Fully stripped PROD cannot provide dynamic counters.

## Compatibility impact

The profile does not select a compatibility route.

## Performance impact

Optional diagnostics can be removed at compile time; mandatory checks remain.

## Security impact

PROD excludes fault injection and live experiments, but retains validation.

## Testing

Run semantic tests across profiles and measure observer overhead.

## Reversibility

Budgets and defaults may change without changing API semantics.

## Evidence

[Cases and sources](../research/case-index.md); [other systems](../research/reference-systems.md).

[Russian translation](../../translations/ru/docs/architecture-decisions/0003-profiles.md)
