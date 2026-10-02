# Phase 0.1 results and limits

Date: 2026-10-02. The foundation contains 30 Linux cases, a JSON Schema, offline Rust checks, kernel laws, eight architecture decisions, architecture models and a comparison of seven operating-system approaches. **No kernel is implemented; Phase 1 has not started.** The [original request](phase-0-request.txt) is historical context; subsequent user corrections supersede its law-count quota and documentation-language choices.

## Validation scope

Run `cargo test --locked` and `cargo run --locked -p research-checks -- check` for the current tree. These check negative data mutations, schema and record consistency, the generated indexes, links, law structure, decision sections and reviewed translation revisions. `git diff --check` checks whitespace. Passing these checks establishes structural consistency only.

Historical kernels, exploit reproducers, kernel benchmarks, Miri kernel models, QEMU and real hardware have not run. A `tests_required` entry is a future obligation, not a passing test report. Source claims require manual review.

## Repository map

| Path | Purpose |
|---|---|
| `docs/architecture` | Language-independent laws and architecture policies |
| `docs/architecture-decisions` | Accepted and proposed decisions with alternatives and consequences |
| `docs/research` | Index, open questions, report and next research phase |
| `research/pathology` | Original case records |
| `research/sources` | Source ledgers and bilingual index text |
| `research/reference-analysis` | Coverage and method |
| `research/other-systems` | Comparison with other designs |
| `schemas/pathology` | Closed case schema |
| `tools/research-checks` | Rust validation and report generation |
| `translations/ru` | Complete Russian Markdown translations with mirrored paths |

## Findings

The [case index](CASE_INDEX.md) groups 12 NATIVE_FIX, 6 COMPAT_ONLY, 4 HARDWARE_TRANSLATION, 6 ACCEPTED_TRADEOFF, 1 RESEARCH_REQUIRED and 1 NOT_APPLICABLE decisions. DMA ordering, reclamation and multiple queues have real tradeoffs. Internal driver API evolution and ABI removal provide counterexamples to a universally frozen Linux design. Hardware translation can be necessary even without software compatibility.

The [law review](../architecture/LAW_REVIEW.md) consolidates overlapping obligations without a numeric quota or an implementation-language dependency. Rust-specific mechanisms remain in the safety policy. [Architecture decisions](../architecture-decisions/README.md) establish native authority, state-aware routes, shared profile semantics, audited boundaries, platform contracts, honest metrics and module retirement. Adapter and driver placement remains proposed.

## Remaining limits

Many introducing commits are unknown. Some live documentation lacks immutable pins. The LKML thread for case 30 was unavailable, and its no-op fix bisection needs replay. No proven case of a workaround outliving all supported affected hardware has been established. Several other-system comparisons still lack concrete failure postmortems. No benchmark numbers or implemented behavior are invented.

See [open questions](OPEN_QUESTIONS.md) and the [Phase 0.2 backlog](PHASE_0_2.md). This report does not authorize automatically starting that work or kernel implementation.

[Russian translation](../../translations/ru/docs/research/PHASE_0_1_REPORT.md)
