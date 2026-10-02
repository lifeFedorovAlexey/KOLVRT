# Security inspection artifact model

Research schema, not runtime CLI/API or stable ABI. [Offline domain schema](../../schemas/security-domain.schema.json) defines review records; limits bound the artifact, not permanent kernel budgets.

## Fields and limitations

Record ID, generation, privilege, implementation/evidence state, native issuer and object/effect scope for grants, service-private status, compat dependencies, relationships, device ownership, per-property TCB and source provenance. Right names are review identifiers, not admission of new capability types. Proposed records do not establish real grants. No fake CLI output is implemented.

A service-private capability inventory is not a caller authority inventory. Snapshot consistency, authorization, redaction, bounded enumeration and unavailable counters must be explicit. Do not expose payloads, usable handles, raw addresses or secrets. Later inspect commands require their own native grants.

## Future checks

The repository document gate validates the offline
[proposed example](../../research/fixtures/security-domain-review.json) against the
domain schema, and keeps the scenario specifications explicitly not executed.
Negative fixtures reject missing effect scope, unknown fields, excess grants and
unsupported DMA labels. This is artifact validation, not live inspection or a device test.

Reject unauthorized inspection and incomplete generations; test redaction and bounded truncation under concurrent revocation. Mandatory enforcement remains when diagnostic consumers disappear. Schema validation proves shape/bounds only, not authority, minimum privilege or hardware containment. [Scenario specifications](../../research/fixtures/security-boundaries.json) are marked not executed.

[Russian translation](../../translations/ru/docs/security/inspection-model.md)
