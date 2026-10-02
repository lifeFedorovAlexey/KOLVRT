# Review of the initial kernel laws

[ADR-0013](../architecture-decisions/0013-security-boundaries.md) clarifies LAW-009: constrain consumer effects with no compatibility waiver. Privileged necessity makes LAW-040, LAW-041 and LAW-042 concrete; the law count is unchanged.

This records the initial consolidation into 17 obligations. The subsequent [coverage audit](law-audit.md) adds three independently justified laws; the active set now has 20.

The initial list was shaped by a requested range and incorrectly enforced that range in tooling. That constraint is removed. This review groups requirements by independent architectural obligations rather than selecting another target count. IDs are retained, not renumbered to disguise consolidation.

No protection requirement was removed merely to shorten the list. Language-specific mechanisms such as compiler lints and unsafe-block annotations belong in the [Rust safety policy](unsafe-policy.md); the corresponding language-independent boundary obligation remains in the laws. Case-specific examples remain in research records and the subsystem models.

| Previous ID | Current obligation | Reason                                                                                       |
| ----------- | ------------------ | -------------------------------------------------------------------------------------------- |
| LAW-001     | LAW-001            | Compatibility requirements must remain removable without freezing internal design.           |
| LAW-002     | LAW-001            | Compatibility requirements must remain removable without freezing internal design.           |
| LAW-003     | LAW-003            | A migration can involve only part of a workload.                                             |
| LAW-004     | LAW-004            | An implementation update and a semantic change are different events.                         |
| LAW-005     | LAW-005            | Call-level independence cannot be assumed from API names.                                    |
| LAW-006     | LAW-008            | Changing behavior or code does not erase existing state or consumers.                        |
| LAW-007     | LAW-008            | Changing behavior or code does not erase existing state or consumers.                        |
| LAW-008     | LAW-008            | Changing behavior or code does not erase existing state or consumers.                        |
| LAW-009     | LAW-009            | Compatibility mechanisms operate within the same protection contract.                        |
| LAW-010     | LAW-004            | An implementation update and a semantic change are different events.                         |
| LAW-011     | LAW-018            | Neither a convenient data layout nor an earlier check proves that later access is valid.     |
| LAW-012     | LAW-018            | Neither a convenient data layout nor an earlier check proves that later access is valid.     |
| LAW-013     | LAW-013            | Visibility, ownership and storage reclamation are distinct lifecycle properties.             |
| LAW-014     | LAW-013            | Visibility, ownership and storage reclamation are distinct lifecycle properties.             |
| LAW-015     | LAW-013            | Visibility, ownership and storage reclamation are distinct lifecycle properties.             |
| LAW-016     | LAW-013            | Visibility, ownership and storage reclamation are distinct lifecycle properties.             |
| LAW-017     | LAW-013            | Visibility, ownership and storage reclamation are distinct lifecycle properties.             |
| LAW-018     | LAW-018            | Neither a convenient data layout nor an earlier check proves that later access is valid.     |
| LAW-019     | LAW-018            | Neither a convenient data layout nor an earlier check proves that later access is valid.     |
| LAW-020     | LAW-020            | External agents outlive ordinary call scopes and observe memory independently.               |
| LAW-021     | LAW-020            | External agents outlive ordinary call scopes and observe memory independently.               |
| LAW-022     | LAW-018            | Neither a convenient data layout nor an earlier check proves that later access is valid.     |
| LAW-023     | LAW-018            | Neither a convenient data layout nor an earlier check proves that later access is valid.     |
| LAW-024     | LAW-009            | Compatibility mechanisms operate within the same protection contract.                        |
| LAW-025     | LAW-025            | Independent ledgers can each grant exclusive access or lose shared costs.                    |
| LAW-026     | LAW-026            | A successful intermediate step does not establish the final guarantee.                       |
| LAW-027     | LAW-027            | Relative priority has no useful meaning without its competition scope.                       |
| LAW-028     | LAW-013            | Visibility, ownership and storage reclamation are distinct lifecycle properties.             |
| LAW-029     | LAW-026            | A successful intermediate step does not establish the final guarantee.                       |
| LAW-030     | LAW-030            | Sequential waits do not implement atomic wait-any semantics.                                 |
| LAW-031     | LAW-031            | Hardware and firmware constraints are real but must not become unbounded generic exceptions. |
| LAW-032     | LAW-031            | Hardware and firmware constraints are real but must not become unbounded generic exceptions. |
| LAW-033     | LAW-031            | Hardware and firmware constraints are real but must not become unbounded generic exceptions. |
| LAW-034     | LAW-001            | Compatibility requirements must remain removable without freezing internal design.           |
| LAW-035     | LAW-035            | Instrumentation can alter timing but must not implement correctness.                         |
| LAW-036     | LAW-036            | A dependency label or favorable average cannot establish cost or correctness.                |
| LAW-037     | LAW-036            | A dependency label or favorable average cannot establish cost or correctness.                |
| LAW-038     | LAW-036            | A dependency label or favorable average cannot establish cost or correctness.                |
| LAW-039     | LAW-018            | Neither a convenient data layout nor an earlier check proves that later access is valid.     |
| LAW-040     | LAW-040            | A language choice or desired list length is not engineering evidence.                        |

[Russian translation](../../translations/ru/docs/architecture/law-review.md)
