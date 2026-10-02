# Schema v1

[case.schema.json](case.schema.json) — JSON Schema Draft 2020-12, закрытая структура:
unknown properties запрещены. Все поля обязательны; неизвестные provenance values
представляются null с объяснением, а не вымышленными датами и hashes.

Структурные проверки выполняет jsonschema. [validate.py](../../tools/pathology/validate.py)
дополнительно проверяет ID uniqueness, filename, primary source, evidence references,
даты, decision/compatibility consistency и наличие вопросов при неизвестной истории.
`--minimum 30` — default для Phase 0.1, меньший порог допустим для isolated tooling tests.
Пустая/несуществующая база не проходит проверку.

Enum decisions: NATIVE_FIX, COMPAT_ONLY, HARDWARE_TRANSLATION, ACCEPTED_TRADEOFF,
RESEARCH_REQUIRED, NOT_APPLICABLE. Compatible behavior может требоваться только условно;
CONDITIONAL не является обещанием его реализации.

При несовместимом изменении schema создать v2 migration ADR; не переписывать старые
значения так, чтобы исчезло различие между unknown, false и not applicable.
