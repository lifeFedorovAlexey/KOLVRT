# Documentation knowledge system

Document status: CURRENT
Evidence scope: deterministic offline navigation and the enrolled Phase 3 pilot; no kernel or hardware acceptance is inferred.
Current reference: [Documentation policy](documentation-policy.md)

<a name="kolvrt-docs-navigation"></a>

## Navigation feature

The offline tooling is bounded implemented: strict authored metadata, stable-ID/locale extraction, generated catalog/graph, explicit prerequisites, impact and deterministic context. The executed pilot verifies its named query and scope; broader semantic search quality remains unclaimed.

## Authority and ownership

Canonical English Markdown contains architectural meaning and authored navigation metadata. Russian mirrors are reviewed locales of the same entities. Existing COST-L and KOL-PATH JSON retain authority for their structured records. Catalog and graph are rebuildable navigation projections, never an independent architecture database. Code establishes actual behavior; accepted laws/ADRs establish obligations. A disagreement requires a defect or reviewed decision.

Only enrolled documents require knowledge metadata. [Enrollment](knowledge-enrollment.json) is a staged coverage gate; generated catalog lists unclassified legacy documents without inventing CURRENT state. No file is moved or removed for this migration.

## Metadata and stable identity

Use one literal HTML comment marker `knowledge`, followed immediately by a JSON fenced block, outside examples. The [closed schema](../schemas/knowledge.schema.json) uses the existing strict duplicate-key parser and JSON Schema Draft 2020-12. Required fields are schema_version, id, kind and summary. Tags/read_when, aliases, depends_on, typed relationships and selected units are optional. Units additionally require an explicit anchor. Existing Document status, Evidence scope and Current reference remain the only authored document-lifecycle declarations; title comes from the heading.

Semantic IDs use lowercase ASCII segments separated by dots, with digits/hyphens, at most 160 characters. A file move or heading rename preserves meaning/ID. A true replacement receives a new ID and scoped supersedes relationship. Aliases resolve directly to one canonical ID, cannot shadow IDs or form chains/cycles; IDs are never recycled. Retained REMOVED/SUPERSEDED feature units are tombstones. ADR, law, COST-L and case adapters preserve existing external IDs as aliases; they do not allocate new records.

Each feature has exactly one canonical unit. The section heading is its name; authored scope, limitations, sources, issues, ADRs, next gate, acceptance, verification and transition history live inside that unit. Locale copies preserve machine fields; translated summaries/read conditions are navigation prose. Existing mirrored explanatory prose carries the complete contract.

## Feature states and freshness

Implementation states are PLANNED, RESEARCH, EXPERIMENTAL, BOUNDED_IMPLEMENTED, IMPLEMENTED, SUPERSEDED and REMOVED. PLANNED and RESEARCH can alternate; either can become EXPERIMENTAL. Accepted bounded behavioral evidence permits EXPERIMENTAL to become BOUNDED_IMPLEMENTED; satisfying the declared complete contract permits IMPLEMENTED. Scope reductions require an explicit reviewed downgrade. Any nonterminal state can be superseded or removed with obligations resolved. Terminal identities cannot silently resurrect; a new meaning requires a new ID and explicit relation.

The initial UNRECORDED transition adopts previously reviewed reality without pretending this catalog delivered the feature. Later transitions are append-only, continuous and validated against the CI base. Events moving to implemented states carry actual acceptance paths. Code merged without acceptance stays EXPERIMENTAL. Removed sources need not exist, but their history and ID remain retained.

Verification is per environment: UNKNOWN, VERIFIED, FAILED, STALE or NOT_APPLICABLE with reasons. VERIFIED carries an exact receipt digest and source/revision/profile/platform scope. Historical receipts retain their original scope; current VERIFIED additionally requires matching declared source digests. Unknown hardware remains unknown. Relevant input changes require applicability review and STALE/new evidence as appropriate; unrelated documentation changes do not manufacture a behavioral regression. Readiness UNKNOWN/NOT_READY/READY is independent; READY requires separate acceptance. Roadmap phase orders guarantees, not status.

Every feature-affecting reviewed change MUST update the canonical status, scope, acceptance/evidence applicability, limitations and next gate in that same change, regenerate catalog/graph and affected public summaries, review affected complete EN/RU pairs and pass checks. Addition, partial/experimental delivery, bounds, removal, supersession, hardware verification and readiness all follow this contract. A feature PR is incomplete while public status is knowingly false. Issue closure requires scoped acceptance or an explicitly research-only disposition.

Implementation order is NOT architectural authority. Before using early functionality as a later milestone foundation, re-derive architecture from current Kernel Laws, accepted ADRs, invariants and intended authority/lifetime model. Refactor, replace or remove code that constrains, contradicts or prematurely freezes it. Existing implementation, effort, tests and rework cost are not architectural evidence.

## Sections, graph and context budgets

Explicit named anchors precede headings; ranges include the heading subtree through the next heading of equal/higher level, document status/evidence preamble and enclosing introductions. Fenced examples are excluded from heading parsing. Ranges are generated 1-based inclusive locations bound to normalized content hashes and locale. Stale/duplicate/missing anchors fail. Overlapping sections and shared prerequisites are deduplicated. Metadata is removed from displayed context.

Closed edge types are depends_on, related_to, supersedes, implemented_by, validated_by, motivates, contrasts_with, blocked_by, cost_l and source_of_truth. Prerequisites and supersession are acyclic; ordinary relations may cycle. Supersession must name scope. implemented_by/validated_by target repository files; other edges resolve knowledge IDs. cost_l targets debt entities. Current dependencies cannot use superseded authority. Do not treat every Markdown link as a prerequisite or infer semantic dependencies from lexical observations.

Discovery catalog contains compact navigation entries, aliases and unenrolled inventory. The graph contains generated locations, hashes, typed edges and full structured metadata, without Markdown bodies. Canonical source declarations alone own semantic relationships; backlinks are derived. No embedding service, cloud, database or vendor tokenizer is required. Budgets: 1 MiB per authored document, 8192 nodes, 4096 query bytes and 4 MiB rendered context. Review these input-safety limits if measured growth requires it.

## Commands and validation

```text
cargo xtask docs context "TASK_IN_RUSSIAN" --locale ru --budget-bytes 262144
cargo xtask docs generate
cargo xtask docs generate --check
cargo xtask docs find dma
cargo xtask docs show kolvrt.handles.identity
cargo xtask docs show kolvrt.handles.identity --locale ru
cargo xtask docs deps kolvrt.security.capability-revocation
cargo xtask docs related cost-l.0001
cargo xtask docs impact kolvrt.handles.identity
cargo xtask docs context "review Phase 3.4 capability revocation" --budget-bytes 131072
cargo xtask docs check-change origin/main
```

The xtask facade calls the existing repository-checks binary, sharing its library. Context ranks exact IDs/tags/read conditions/title/summary deterministically, selects up to three candidates with at least 60% of the top score, then includes their entire mandatory closure. Output explains selection and gaps, bytes and ceil(bytes/4) approximate token units. A budget violation is explicit and fails the command; no required dependency is silently dropped. Narrow the request instead. Related edges expand only when the task requires them. Impact follows declared reverse edges; it is documentation impact, not exhaustive code impact.

The normal repository check rejects schema/IDs/aliases/edges/cycles, missing enrolled metadata, stale outputs, broken sources/anchors, false feature transitions, receipt digest drift, outdated VERIFIED sources and incompatible EN/RU machine metadata. The CI base check additionally rejects rewritten feature history and implementation/build changes without digest-bound impact declarations. Explicit no-impact explanations can permit an unchanged canonical contract; reviewers still evaluate truth. External issue existence can be checked with authenticated GitHub tools; offline tooling only records IDs and makes no existence/closure inference. Network failures remain UNKNOWN.

## Migration and evidence limits

Stage A establishes schema/policy/checks; B migrates processes/user-copy/handles and bounded capability/domain acceptance; C adds law/ADR navigation and derived README pilot rows; D projects existing COST-L/cases; E seeds Linux mechanisms and maps hardware/security/benchmark/advisor entry points; F incrementally enrolls remaining historical docs. Each stage preserves human navigation, evidence and translations. The pilot measures actual selected context plus catalog overhead; it does not prove all future tasks retrieve sufficient sources.

Linux research uses the [mechanism template](../research/linux/mechanism-template.md); primary sources pin version/commit/path/configuration and review date. COST-L debt lifecycle, native decision, driver observations, compatibility modules and KOLVRT feature states remain distinct. Hardware research does not collect or publish personal inventory under this tooling. Reuse existing research issues and provenance gates.

[Russian translation](../translations/ru/docs/knowledge-system.md)

### Exact-source verification and implementation impact

VERIFIED requires every declared feature source to occur exactly once in the receipt's `source_files`, with matching LF-normalized `sha256_lf`. Duplicate/malformed digests, missing coverage or changed sources fail CI. Record STALE with a reason or supply new matching evidence before merging. Retained receipts remain immutable and digest checked in STALE; implementation scope and historical acceptance are independent of current verification. README summaries cannot publish an outdated VERIFIED after a successful check.

For each PR changing implementation or build inputs, author [the impact declaration](implementation-impact.json) against the exact reviewed base commit. Its [closed schema](../schemas/implementation-impact.schema.json) binds every changed file to before/after LF hashes (null for creation/deletion). The mandatory boundary includes all files under crates, scripts, .cargo and .github/workflows, root Cargo/package manifests and locks, build.rs/rust-toolchain.toml, plus every declared feature source. Newly added and deleted files count, including untracked files in local checks. Other implementation roots remain a human enrollment obligation; the checker cannot recognize arbitrary new functionality semantically.

Classify each file as new-feature, existing-feature or no-feature-impact with an explanation. New-feature requires a newly enrolled canonical feature owning that source. Changes to owned sources also require one feature disposition: semantic-change, evidence-change or no-impact. Semantic-change requires a substantive canonical update; whitespace touches do not qualify. Evidence-change requires changed evidence metadata. No-impact permits a source edit without a contract edit when its explanation is reviewed, but never bypasses exact-source verification. It is a review claim, not an automatic equivalence proof. Publish dispositions in the PR description and CI output; reviewers judge their reasons and EN/RU meaning. Rebase, source edits or base advancement invalidate the declaration and require renewed review. This file is a per-change review record, preserved by Git history, not a second feature registry.

Do not close an infrastructure issue while its own required semantic or locale review remains pending. Passing checks establishes mechanical consistency, not review completion.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.docs.knowledge",
  "kind": "policy",
  "summary": "Shared authored metadata and deterministic documentation navigation contract.",
  "units": [
    {
      "id": "kolvrt.docs.navigation",
      "anchor": "kolvrt-docs-navigation",
      "kind": "feature",
      "summary": "Offline stable-ID discovery, scoped extraction and graph validation.",
      "tags": ["documentation", "catalog", "navigation"],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "Offline metadata validation, Unicode EN/RU context, exact-source verification freshness, prerequisite closure and review-visible per-change implementation impact declarations.",
        "sources": [
          "crates/repository-checks/src/knowledge.rs",
          "crates/repository-checks/src/knowledge/impact.rs",
          "crates/repository-checks/tests/knowledge.rs",
          "schemas/implementation-impact.schema.json"
        ],
        "acceptance": ["research/results/documentation-knowledge-pilot.json"],
        "issues": [74],
        "adrs": ["adr.0024"],
        "limitations": [
          "No automatic semantic-completeness proof, no-impact truth proof, LLM search or all-doc migration; arbitrary implementation roots outside the declared CI boundary still require human enrollment review."
        ],
        "next_gate": "Review coverage on additional tasks before enrolling further domains or adding semantic search.",
        "verification": [
          {
            "environment": "host-process",
            "state": "VERIFIED",
            "reason": "Executed current-source deterministic EN/RU retrieval and reverse-impact pilots after the consolidated IPC/dependency integration; semantic sufficiency remains review judgment.",
            "receipt": "research/results/documentation-knowledge-pilot.json",
            "receipt_sha256": "a4bea0f88187a284b3290eb8af6ffbe987e87e0d79cc61bdc6293c71f702d760",
            "scope": "Named Phase 3.4 revocation queries and mandatory prerequisites, Unicode Russian retrieval, explicit planned gaps and reverse impact on this exact-source host tool; no kernel execution, universal retrieval quality or physical hardware claim."
          },
          {
            "environment": "physical-arm64",
            "state": "NOT_APPLICABLE",
            "reason": "Offline host navigation has no physical kernel verification claim."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Introduce the bounded offline navigation implementation.",
            "acceptance": []
          },
          {
            "from": "EXPERIMENTAL",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "Actual retrieval closure and explicit-gap pilot plus rejection tests.",
            "acceptance": [
              "research/results/documentation-knowledge-pilot.json"
            ]
          }
        ]
      }
    }
  ]
}
```
