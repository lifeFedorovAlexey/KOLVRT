# Architecture decisions

Records distinguish Phase 0 design requirements from the implemented kernel foundation in ADR-0010. Proposed records remain open.

- [ADR-0001 — Native authority and legacy isolation](0001-native-authority.md) — Accepted.
- [ADR-0002 — Routing by state domain](0002-stateful-routing.md) — Accepted.
- [ADR-0003 — One architecture across execution profiles](0003-profiles.md) — Accepted.
- [ADR-0004 — Safe Rust and audited boundaries](0004-unsafe.md) — Accepted.
- [ADR-0005 — ARM64-first platform contracts](0005-platform.md) — Accepted.
- [ADR-0006 — Dependency measurements and fair comparisons](0006-metrics.md) — Accepted.
- [ADR-0007 — Bug compatibility and module retirement](0007-bug-compat.md) — Accepted.
- [ADR-0008 — Service protection domains](0008-placement.md) — Accepted for the first native slice.
- [ADR-0009 — Native slice design baseline](0009-native-slice-baseline.md) — Accepted.
- [ADR-0010 — Native EL1 foundation](0010-kernel-foundation.md) — Accepted.
- [ADR-0011 — Performance and reliability method review](0011-method-review.md) — Accepted.

- [ADR-0012 — Multicore ownership and acknowledged retirement](0012-multicore-retirement.md) — Accepted for two-CPU QEMU.

- [ADR-0013 — Effective authority and privileged necessity](0013-security-boundaries.md) — Accepted for boundary rules only.

- [ADR-0014 — Fixed-affinity EL0 execution foundation](0014-el0-foundation.md) — Accepted for the bounded foundation.

- [ADR-0015 — Versioned routing in isolated EL0 consumers](0015-el0-versioned-routing.md) — Accepted for the bounded Phase 2 demonstration.

- [ADR-0016 — Scheduler responsibilities and enforced per-CPU ownership](0016-scheduler-ownership.md) — Accepted for bounded Phase 3.0.

- [ADR-0017 — Generation-safe native process lifecycle](0017-process-lifecycle.md) — Accepted for bounded Phase 3.1.

- [ADR-0018 — Bounded synchronous safe user copy](0018-safe-user-copy.md) — Accepted for bounded Phase 3.2.

- [ADR-0019 — Linear process-local handle namespaces](0019-process-local-handles.md) — Accepted for bounded Phase 3.3.

- [ADR-0020 — Attenuated handle transfer and retained targets](0020-handle-transfer-and-retention.md) — Accepted for bounded issue #23 implementation.

- [ADR-0021 — Fixed-affinity AArch64 ASID lifecycle](0021-asid-lifecycle.md) — Accepted for bounded process roots; migration remains unsupported.
- [ADR-0022 — Bounded native Event grants and revocation](0022-native-event-grants-and-revocation.md) — Accepted for the minimal Event authority slice.

- [ADR-0023 — One-process domains and scoped Event grants](0023-domains-and-scoped-grants.md) — Accepted for bounded Phase 3.4; full IPC remains separate.

[Russian translation](../../translations/ru/docs/architecture-decisions/README.md)

- [ADR-0023 — Documentation knowledge navigation and feature freshness](0023-documentation-knowledge.md) — Accepted for offline navigation.
