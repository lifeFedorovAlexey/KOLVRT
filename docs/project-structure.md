# Repository structure

The layout separates buildable programs, normative design, decision rationale and research data. It adds no empty kernel or driver directories before implementation exists.

```text
crates/
  native-protocol-model/   # no_std request/response candidate
  native-state-models/     # finite state models and host IPC experiment
  repository-checks/       # schema, links, laws, translations and reports
docs/
  architecture/           # contracts and reviewable specifications
  architecture-decisions/ # decision rationale, alternatives and consequences
  research/               # methods, indexes, reports and outstanding evidence
research/
  cases/                  # structured historical evidence
  sources/                # provenance ledger and bilingual catalog
  results/                # reproducible model output and law traceability
  requests/               # original request evidence
schemas/                  # data validation contracts
translations/ru/          # mirrored Markdown paths
```

## Rationale and alternatives

[Cargo recommends a flat crates directory](https://doc.rust-lang.org/cargo/reference/workspaces.html) for workspace packages. One root lockfile and target directory serve all three packages. Package names describe purpose; the model suffix prevents mistaking a prototype for a kernel implementation. A separate tools tree and executable crates buried in research data added locations without a different build lifecycle, so they were consolidated.

[Decision records](https://adr.github.io/) preserve why a choice was made and its tradeoffs; specifications describe the resulting contract. [Diataxis](https://diataxis.fr/) distinguishes documentation purposes. This project applies that distinction without inventing empty tutorial directories or claiming the framework mandates this exact tree. The contribution guide is a how-to; contracts are reference material; decisions explain choices; research reports record evidence.

Markdown uses descriptive lower-kebab-case names except conventional README.md and CONTRIBUTING.md. Stable case identifiers remain unchanged. A document belongs in one canonical location and links to related material; do not duplicate facts across indexes. Translations mirror canonical paths, while machine-readable records remain single-source. The translation manifest and generated case index enforce correspondence.

## Review checklist

New packages require a distinct responsibility or dependency boundary. New directories require existing content with a distinct role. Moves update links, checker paths, translation mappings and commands together. Formatting is automatic through the [contribution workflow](../CONTRIBUTING.md). Local caches and build outputs stay ignored.

[Russian translation](../translations/ru/docs/project-structure.md)
