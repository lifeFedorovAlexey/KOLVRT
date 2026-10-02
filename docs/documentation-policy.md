# Documentation and translations

## Status and evidence

Use `Document status`, `Document scope` and a local `Status reference` link on documents
whose milestones can be confused. These roles are distinct from ABI publication stages.

| Status          | Meaning                                                                                  |
| --------------- | ---------------------------------------------------------------------------------------- |
| CURRENT         | Current policy or implementation description within stated evidence limits.              |
| DESIGN BASELINE | Accepted intended behavior with explicit implementation and acceptance gaps.             |
| HISTORICAL      | Claims about a named earlier phase/revision, not current implementation.                 |
| SUPERSEDED      | Replaced for the stated scope; link the replacement and preserve unaffected obligations. |

Code establishes implemented behavior; accepted current ADRs/specifications establish
intended behavior. A discrepancy is a defect or requires a reviewed decision, not an
automatic CODE > ADR > DOC rule. Historical claims cannot override current decisions.
Supersession may be partial: a scheduler does not complete IPC, handles or cancellation.

## Reviewed status audit

Issue #6 reviews merged implementation through `d6bf7da`, excluding local Phase 2 changes.
[ADR-0010](architecture-decisions/0010-kernel-foundation.md) records original CPU0-only
execution; [ADR-0012](architecture-decisions/0012-multicore-retirement.md) and
[ADR-0014](architecture-decisions/0014-el0-foundation.md) record later bounded milestones.

| Statement/document                | Disposition                                                                                                                |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| No kernel/build exists            | Historical Phase 0 only; the merged repository contains a native AArch64 kernel.                                           |
| Kernel checks have not run        | Historical Phase 0 only; retained foundation/SMP/EL0 QEMU results exist.                                                   |
| CPU0-only execution               | Original Phase 1 scope; CPU0 allocator ownership is a separate still-valid restriction.                                    |
| First native slice                | DESIGN BASELINE; EL0/scheduling are partly realized, while IPC/handles/cancellation/service acceptance remains incomplete. |
| Compatibility and unsafe policies | CURRENT obligations with historical Phase 0 passages; no general loader or complete safety proof is claimed.               |
| Local Phase 2 routing             | Uncommitted code/results are not merged implementation or evidence for this reviewed revision.                             |

Retained [foundation results](../research/results/kernel-foundation.json) cover 23 tests
per profile and one active CPU; [SMP results](../research/results/kernel-smp.json) cover
39 tests and two active CPUs at recorded revision `a9371e7`; [EL0 results](../research/results/kernel-el0.json)
cover 53 tests at recorded revision `67e4cbc`. These DEV/PROD records use QEMU 10.1.0,
cortex-a57, TCG, 256 MiB and two configured CPUs. Source/artifact identities and negative
controls belong to each record, not automatically to current code. Physical ARM64,
general-purpose SMP and the complete native slice remain unverified. This change reruns
no kernel matrix and generates no new execution evidence.

Bounded checks validate metadata and local reference syntax for the audited documents;
ordinary link checks reject missing targets. They cannot prove that claims match behavior.
Review every capability claim against its actual revision, profile and platform.

Canonical Markdown is written entirely in English. Translations live under `translations/<language>/` and mirror the canonical repository paths, including filenames. Russian uses `translations/ru/`. Use descriptive directory names, such as `architecture-decisions`; avoid unexplained abbreviations in new directory names.

Translate the complete meaning: requirements, exceptions, uncertainty, examples and conclusions. Do not combine English prose with Russian prose. Preserve code, commands, identifiers, proper names and external URLs where translation would change their meaning. Russian navigation links stay within the Russian tree; links to shared data and source code resolve to canonical files.

Every canonical Markdown document requires a counterpart for each registered language. Keep the same heading hierarchy, lists, table shape, ordered link targets and law identifiers. Both documents link to each other. New languages are registered in `translations/manifest.json` and supply the complete mirrored tree.

The manifest records SHA-256 hashes of both reviewed texts, with CRLF normalized to LF. A change to either text invalidates the recorded pair. Checks never silently refresh hashes. After reviewing the complete pair, run the explicit recording command described in the [tooling guide](../crates/repository-checks/README.md). Structural and revision checks detect drift; they cannot prove linguistic equivalence. Human review remains required.

The original request is preserved as a historical text attachment. Research JSON retains its original source-analysis prose; it is not a Markdown translation. The bilingual case-index catalog provides reviewed English and Russian titles, subsystem labels and questions, tied to the research-record hash. A record change requires reviewing that catalog before regenerating the indexes.

## Phase 2 evidence update

The earlier audit explicitly excludes the then-local Phase 2 work. The now-verified [routing contract](kernel/routing.md) and [ADR-0015](architecture-decisions/0015-el0-versioned-routing.md) describe the bounded optional EL0 slice: eight independent consumers on two CPUs, native/v1/v2/safe-bug routes, pinned PROD profiles and native-core dependency isolation. [Execution results](../research/results/routing-phase2.json) and [physical source-removal evidence](../research/results/native-compat-removal.json) retain exact source/artifact scopes; the latter reruns the same 53-test DEV/PROD foundation and eleven negative controls without routing packages. These records establish only their QEMU scope. IPC, transferred authority, cancellation, general services, dynamic loading and silicon validation remain incomplete; the full first native slice is still a design baseline.

[Russian translation](../translations/ru/docs/documentation-policy.md)
