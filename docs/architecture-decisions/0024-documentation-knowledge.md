# ADR-0024 — Documentation knowledge navigation and feature freshness

Status: **Accepted for bounded offline documentation infrastructure**. Date: 2026-10-04.

Document status: CURRENT
Evidence scope: host navigation contracts only; no kernel feature acceptance.
Current reference: [Knowledge contract](../knowledge-system.md)

## Context

File/heading navigation cannot select small authoritative context or enforce feature freshness. Existing document lifecycle, scoped ADRs, JSON research and EN/RU review must survive migration.

## Decision

Use authored fenced JSON metadata in canonical Markdown with stable semantic document/section IDs. Reuse strict JSON Schema validation and existing Rust repository checks. Generate compact catalog and typed graph; retrieve mandatory prerequisites and scoped section excerpts deterministically. Keep implementation, verification, readiness, decision disposition and roadmap independent. Apply the same-reviewed-change freshness contract and architecture re-derivation rule in the [documentation policy](../documentation-policy.md).

## Alternatives

YAML frontmatter, hand-maintained graph/feature registries, filename-only IDs, treating every link as dependency, separate AI prose, embeddings/cloud/database-first infrastructure.

## Why rejected

JSON reuses the existing parser and duplicate-key discipline. Duplicate registries drift; path identity breaks moves/locales; arbitrary transitive links inflate context. Unmeasured external infrastructure adds cost without evidence of need. Markdown remains shared architectural meaning.

## Consequences

Enrollment is staged and visible. Enrolled files require current metadata fields and mirrored machine identity. Historical documents and counts remain retained. Feature transitions are explicit and append-only; initial adoption is not a new implementation claim. A context budget cannot discard required authority.

## Compatibility impact

No native ABI, Linux semantics, kernel authority, COST-L allocation/lifecycle or migration-advisor protocol changes. Existing record schemas retain their own truth and are projected only for navigation.

## Performance impact

Measure catalog bytes, deterministic query context, overlap deduplication and missing sources in the real pilot. UTF-8 bytes/4 is approximate budgeting, not vendor tokenization or a semantic-quality result. Bounded offline traversal requires no LLM service.

## Security impact

Reject out-of-repository paths, stale ranges, duplicate IDs, unknown/cyclic mandatory links and QEMU receipts advertised as physical verification. Research instructions remain data; catalog entries cannot authorize kernel behavior or deployment.

## Testing

Focused rejection fixtures and the actual Phase 3 context test live in [knowledge tests](../../crates/repository-checks/tests/knowledge.rs). Run repository checks, host tests and CLI commands; CI validates declared feature impact against its base revision. Structural success does not establish behavioral or linguistic truth.

## Reversibility

Metadata encoding and host tooling are revisable against current invariants. Preserve semantic identities through aliases/tombstones, retain original evidence and translations, and review supersession scope. Implementation order and sunk cost cannot freeze this format.

[Russian translation](../../translations/ru/docs/architecture-decisions/0024-documentation-knowledge.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0024",
  "kind": "adr",
  "summary": "Bounded shared documentation navigation and feature freshness contract.",
  "aliases": ["ADR-0024"]
}
```
