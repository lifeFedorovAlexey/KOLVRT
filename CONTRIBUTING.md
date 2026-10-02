# Project checks and formatting

Use the [output policy](docs/architecture/output-policy.md) for console messages, diagnostics and machine events. Human prose is not a machine ABI.

Use standard tool defaults: rustfmt for Rust, Prettier for Markdown/JSON/YAML, Taplo for TOML, Clippy for Rust diagnostics, and Markdownlint for document structure. Versions are pinned in `rust-toolchain.toml` and `package-lock.json`. Node.js 18 or later runs document tools; project research tools remain Rust. No Python is required.

Use tables for short, comparable values. Put long explanations, implementation boundaries and verification gaps in sections or lists so both the rendered document and its Markdown source remain readable.

```text
npm ci --ignore-scripts
npm run format
npm run check
```

`format` processes all supported repository files, including generated Markdown. It excludes downloaded tools, dependencies, build outputs and Cargo's generated lockfile. Original text evidence has no formatter and must not be rewritten. `check` fails on formatting, lint, test, model-evidence or documentation errors. Rust uses the standard formatter without custom layout rules. Prettier keeps paragraph wrapping; Markdownlint does not impose a line-length limit on prose or tables. Repeated headings are allowed in different sections.

VS Code workspace settings enable formatting on save and recommend the matching extensions. TOML uses Taplo in the project commands; save formatting is disabled until a compatible TOML extension is installed and selected. Other editors can use `.editorconfig` and the same commands. Formatting and linting never grant approval to a changed translation. Review English and Russian pairs, then record each changed pair using `repository-checks record-translation ru PATH`. Reformat the manifest afterwards. Research-record formatting can change its byte hash: verify that JSON values are unchanged before updating the bilingual catalog hash.

Before a commit, run `npm run check` and `git diff --check`. Keep commits focused with short English subjects. Do not commit downloaded toolchains, dependency directories, executables or temporary evidence copies. A host model test does not establish kernel correctness. Never suppress a check or regenerate evidence just to conceal a failure.

## Time, hardware values and behavioral settings

Pass time as a type, name hardware values, and separate behavioral settings from mechanics.

This applies to all magic numbers where a type or name explains units, purpose or a contract, not only to time. Use typed values and named constants when they improve understanding without adding runtime work or memory allocation. Keep obvious literals inline, such as zero initialization, a unit increment or a self-explanatory local calculation; do not introduce names merely to replace every number.

- **Time:** express durations with `Duration`, for example `Duration::from_millis(1)` instead of `1_000_000`. Keep conversion to hardware ticks at the hardware boundary.
- **Hardware values:** use named constants for registers and bit masks.
- **Other meaningful values:** name buffer sizes, capacity limits, retry counts and protocol codes when their purpose is not clear from the local expression. Prefer compile-time constants; do not add heap objects or runtime initialization just to name a value.
- **Behavioral settings:** define the scheduler quantum, intervals and timeouts next to the subsystem that chooses them. The mechanism accepts these settings rather than choosing behavioral values itself.

Desired API:

```rust
const TEST_TIMER_DELAY: Duration = Duration::from_millis(1);

cpu::timer(time::deadline_after(TEST_TIMER_DELAY));
```

## Implementation rules

These rules apply existing architectural obligations to code; they do not add laws.

1. Separate shared subsystem logic from hardware implementation. Keep ownership, accounting and policy independent of register access and architecture-specific instructions; pass platform data through explicit interfaces.
2. A public safe API must preserve resource ownership. Callers must not be able to change an allocation's identity, free another owner's allocation or release storage while a mapping retains it. Bounds and occupancy checks alone do not establish ownership.
3. Each synchronization primitive must document whether ordinary code, IRQ handlers and multiple CPUs may use it, along with lock order, permitted nesting and progress bounds. State unsupported contexts explicitly; test each supported context before claiming support.
4. Put test-platform limits in explicit platform configuration. RAM size, active CPU count, device layout and mapping limits must have a stated scope; shared subsystem logic must not silently adopt test-bench assumptions.
5. When capabilities change, update their documentation and checks in the same change. Link each affected law to the enforcing code and a check that detects the relevant violation; state missing checks and evidence limits explicitly.

### Code and verification links

For changes at these boundaries, maintain the following links and gaps. Existing tests cover only their stated cases. New ownership guarantees need rejection tests for substitution, foreign release and retained mappings; synchronization guarantees need tests in the actual supported execution contexts.

#### LAW-013, LAW-025: ownership and accounting

**Implementation boundary:** [Physical, Frame and Mapping](crates/kernel/src/memory/mod.rs); [Pool](crates/kernel-core/src/memory.rs)

**Existing check and remaining gap:** `exhaustion_reuse_and_transactional_rejection` checks accounting and rejected invalid ranges. `physical_allocator`, `allocation_free`, `physical_exhaustion` and `invalid_mapping_rejection` in [kernel tests](crates/kernel/src/tests.rs) check allocation and mapping behavior. Frame identity is private and construction of a second physical owner is rejected. The xtask ownership and retained-mapping controls exercise second-owner rejection and refusal to free a frame after forgetting its Mapping guard. Missing: a dedicated compile-fail fixture for attempted private-field identity substitution; pool occupancy alone would not establish ownership.

#### LAW-041: contexts and progress

**Implementation boundary:** [Lock and Guard](crates/kernel/src/sync/mod.rs); [IRQ handling](crates/kernel/src/interrupt/mod.rs)

**Existing check and remaining gap:** `actual_concurrent_publication_and_exclusion` in [host lock tests](crates/xtask/tests/locking.rs) exercises the actual lock implementation with host threads. Kernel tests check `locking_exclusion`, `locking_release`, `interrupt_masking` and `irq_simd_context`. Missing: IRQ lock-use and lock-order checks; host threads do not establish multi-CPU kernel progress.

#### LAW-031: platform isolation and discovery

**Implementation boundary:** [Platform validation](crates/kernel/src/platform/mod.rs); [boot parser](crates/kernel-core/src/platform.rs); [QEMU runner](crates/xtask/src/main.rs)

**Existing check and remaining gap:** [Boot-description tests](crates/kernel-core/tests/boot_description.rs) reject truncation and malformed descriptions; `physical_discovery` checks the pinned platform. The runner and kernel share explicit platform configuration in crates/kernel/src/platform/config.rs. Missing: a second-platform contract test that detects assumptions leaking into shared logic. Historical LAW-034 was consolidated into LAW-001 in the [law review](docs/architecture/law-review.md); it is not an active hardware-limit law.

#### LAW-036, LAW-040: claims and evidence

**Implementation boundary:** [Audit and result validation](crates/xtask/src/main.rs); [scenario audit](docs/architecture/law-audit.md)

**Existing check and remaining gap:** `cargo xtask test` requires the named kernel tests and verifies failure propagation with negative controls; its run metadata states the emulator and active-CPU scope. `repository-checks check-docs` and `check-translations` detect broken references, incomplete obligations and stale reviewed pairs. Missing: automatic verification that every capability claim matches behavioral evidence; review claims against retained results.

Run host checks with `cargo test --locked`, kernel checks with `cargo xtask test`, and documentation checks with `cargo run --locked -p repository-checks -- check`. A passing formatter, source inventory or document checker does not prove these behavioral obligations.

Phase 1.1 adds [multicore checks](docs/kernel/smp.md): bidirectional/repeated IPI, actual shared-lock publication, delayed reader acknowledgement, remote translation fault, guarded retirement/reuse and quiescent CPU_OFF. The [decision](docs/architecture-decisions/0012-multicore-retirement.md) keeps physical ownership explicitly scoped and requires review before adding writers. Host tests do not certify physical weak-memory behavior.

The [EL0 checks](docs/kernel/el0.md) require real timer-driven processes on both CPUs, same-VA private data, full architectural context, contained faults and quiescent reclamation. Root/context/forgotten-user-guard negative controls must fail. The fixed-affinity scope does not justify future migration or shared queue mutation without review.

For [Phase 2 routing](docs/kernel/routing.md), run the real optional-image matrix and native-only/removal checks. Reject dependency reversal, legacy core types, unsupported profiles, unsafe switching and broadened caller effects. Keep evidence builds distinct from stripped PROD and preserve raw benchmark observations.

[Russian translation](translations/ru/CONTRIBUTING.md)
