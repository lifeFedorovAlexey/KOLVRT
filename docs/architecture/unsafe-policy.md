# Rust implementation safety policy

Document status: CURRENT
Document scope: current safety obligations with historical Phase 0 verification limits.
Status reference: [Reviewed execution scope](../documentation-policy.md)

Phase 0 statements below describe the historical stage. Current checks are bounded by the [SMP contract](../kernel/smp.md). An unsafe inventory is not a risk score; safe Rust and EL0 do not themselves exclude a component from TCB. Trust is property-specific in the [system threat model](../security/threat-model.md).

This policy applies to the chosen Rust implementation. It is not a language-independent
kernel law. Architectural obligations are valid lifetimes, permissions, initialization,
ordering and reviewable proofs; another language must satisfy the same obligations using
its own mechanisms.

Safe Rust is the default. Unsafe is allowed only at a necessary hardware, foreign-interface
or memory boundary that available safe abstractions cannot express. Convenience is not
a justification; claimed speed requires measurement. Safe code alone does not prove
freedom from semantic races, deadlocks, exhaustion or invalid foreign-memory assumptions.

Keep each unsafe block minimal and attach a local `SAFETY:` comment with an invariant ID.
Core crates use `#![forbid(unsafe_code)]` except approved boundary crates, which use
`#![deny(unsafe_op_in_unsafe_fn)]`. Include dependencies, generated code, macro expansions,
assembly and unsafe trait implementations in the inventory.

| Invariant record     | Required information                                                    |
| -------------------- | ----------------------------------------------------------------------- |
| Identity and owner   | Stable ID, source locations and responsible reviewer                    |
| Necessity            | Why a safe implementation is unavailable at this boundary               |
| Preconditions        | Bounds, alignment, initialization, provenance, ownership and aliasing   |
| Temporal assumptions | Lifetimes, preemption, interrupt context, affinity and ordering         |
| External agents      | Devices, firmware, foreign callbacks and mutable user memory            |
| Guarantees           | Safe-caller contract including errors, cancellation and destruction     |
| Proof boundary       | Assumptions outside language checks and supporting evidence             |
| Tests and review     | Negative cases, failures, interleavings and independent boundary review |

Volatile access is not atomic and does not replace a barrier. A `Send` or `Sync`
implementation needs explicit multiprocessor and interrupt reasoning. Do not publish
`MaybeUninit` contents before observable bytes are initialized. Reused buffers reset
metadata. Destruction cannot free memory while a device may still access it. For the first slice, privileged panic halts and no unwinding crosses interrupts or foreign calls. Bounded allocation failure returns an explicit error; service death is observed by its supervisor. See the [threat model](threat-model.md).

Use Miri for supported host-side code, concurrency models for represented interleavings,
QEMU for integration and real hardware/litmus tests for ordering and silicon defects.
None proves the whole kernel. Phase 0 had no executed kernel checks; later retained
QEMU foundation, SMP and EL0 matrices cover only their recorded revisions and profiles.
No physical-hardware or complete unsafe proof follows. Pin compiler and reference
revision for each unsafe implementation and its tests.

Evidence: [Rust validity rules](https://doc.rust-lang.org/reference/behavior-considered-undefined.html),
[buffer initialization](../../research/cases/KOL-PATH-0008.json),
[device ordering](../../research/cases/KOL-PATH-0010.json),
[reclamation](../../research/cases/KOL-PATH-0022.json),
[reference upgrade](../../research/cases/KOL-PATH-0030.json).

[Russian translation](../../translations/ru/docs/architecture/unsafe-policy.md)
