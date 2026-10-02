# Case schema v1

[case.schema.json](case.schema.json) uses JSON Schema Draft 2020-12 with a closed structure: unknown properties are rejected. All declared required fields must be present. Unknown provenance is null with an explanation, never an invented date or hash.

The Rust [research checker](../../tools/research-checks/README.md) applies the schema with format validation and additionally checks unique keys and IDs, filenames, primary sources, evidence references, dates, decision/compatibility consistency and questions for unknown history. Phase 0.1 requires at least 30 research cases; this research deliverable count is unrelated to the number of kernel laws. Empty or missing databases fail.

Decisions are NATIVE_FIX, COMPAT_ONLY, HARDWARE_TRANSLATION, ACCEPTED_TRADEOFF, RESEARCH_REQUIRED or NOT_APPLICABLE. CONDITIONAL compatibility is not a promise to implement it.

An incompatible schema change requires a v2 migration decision. Preserve the distinctions between unknown, false and not applicable.

[Russian translation](../../translations/ru/schemas/pathology/README.md)
