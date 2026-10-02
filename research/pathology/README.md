# Engineering case database

Each record is a `KOL-PATH-NNNN.json` file. Never reuse retired IDs; supersede records while preserving Git history. See the [schema](../../schemas/pathology/case.schema.json) and [case index](../../docs/research/CASE_INDEX.md).

The database includes defects, necessary hardware constraints, successful redesigns and accepted tradeoffs. `category` may contain several classifications; `kolvrt_decision` contains one decision. NATIVE_FIX means preventive native design, not an implemented KOLVRT fix. COMPAT_ONLY does not authorize an unused module. HARDWARE_TRANSLATION may be necessary for a native target.

ANALYZED_WITH_OPEN_QUESTIONS means a completed documentary analysis with stated limits, not a reproduced bug or complete Git history. MEDIUM confidence includes unverified applicability to a future KOLVRT. HIGH requires additional historical verification or relevant reproduction; schema validation cannot raise confidence.

`first_known_version` may identify a confirmed change or alternative rather than the original introduction; its text says which. Unknown introduction is null with an open question. Source dates are not research dates. `linux_current_solution` describes the solution supported by the read source, not every subsequent Linux release.

## Research questions

| Question | Fields |
|---|---|
| What happened? | observable_behavior, root_cause |
| When? | historical_context, linux_versions, first_known_version, sources |
| Why? | original_reason |
| Constraints then | constraints_then |
| Constraints now | constraints_now |
| Classification | category, root_cause |
| Consequences of change | change_consequences, compatibility_dependency |
| Linux response | linux_current_solution |
| Improvement potential | improvement_assessment, remaining_problem |
| KOLVRT decision | kolvrt_decision |
| Native semantics | kolvrt_native_semantics |
| Compatibility need | compatibility_required, compatibility_scope |
| Is it needed at all? | kolvrt_decision, migration_strategy |

`evidence` links historical fields to source IDs. Impact assessments are engineering analysis, not measured KOLVRT results or maintainer quotations. Design decisions, required tests and benchmarks are project requirements. Explain an inapplicable benchmark instead of leaving an empty list.

When changing a record, review sources, locators, provenance, decision, confidence and migration/testing implications. Validate and refresh the bilingual index as described in the [tooling guide](../../tools/research-checks/README.md). Checks do not establish source truth or replace review. Original JSON research prose is retained; English and Russian Markdown use reviewed index text.

[Russian translation](../../translations/ru/research/pathology/README.md)
