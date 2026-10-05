# Hardware and compatibility issue submissions

Document status: CURRENT
Evidence scope: repository issue-form definitions, local validation and authenticated GitHub render/sample validation on 2026-10-05; no hardware collection or sample issue publication.
Current reference: [Research program](../../research/compatibility/taxonomy.md)

## Choose a form

The five [issue forms](../../.github/ISSUE_TEMPLATE/) collect bounded requests and evidence for [program #44](https://github.com/lifeFedorovAlexey/KOLVRT/issues/44). Answers may be in English or Russian. Blank issues remain available; no labels, assignees or routing configuration are added.

| Form                           | Intended submission                                                   | Safe sample scope                                                                               |
| ------------------------------ | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Hardware Bring-up Request      | Concrete boot/device goal and bounded proposed test                   | boot_target; QEMU VirtIO block; device operation NOT RUN; DMA/IOMMU UNKNOWN                     |
| Driver Compatibility Research  | Pinned sources and recurring semantic dependencies                    | Linux v6.12 igb source; lexical observations only; configured/transitive behavior UNKNOWN       |
| COST-L Candidate               | Recurring semantics, history, native difference and removal questions | Existing COST-L-0001 candidate; allocation-context evidence; accepted native difference UNKNOWN |
| Hardware Test Result           | Exact executed context, result and limitations                        | Host collection only; NOT RUN for KOLVRT build; no boot, device or isolation guarantee claimed  |
| Compatibility Adapter Proposal | Concrete consumers, bounded lifetime, support and native alternative  | Proposed module; consumers UNKNOWN; costs UNKNOWN; decision deferred pending evidence           |

These are illustrative scopes, not submitted results. An upstream driver name or source match does not establish KOLVRT driver support. A host inventory is not a KOLVRT hardware test. QEMU, physical boot and physical device operation are separate scopes; PASS applies only to the stated executed guarantees. State exact commit/build/features/profile for an executed KOLVRT test and NOT RUN for a proposal or host-only collection.

## Review the exact public content

The [local host-survey framework](host-survey.md) provides allowlisted collection and synthetic validation for #53. Its default command prints help, and explicit collection never publishes. A research-host report is not a boot-target or physical KOLVRT result.

Public model names and bus vendor/device IDs can describe a device without identifying a machine. Do not submit raw inventories, unrestricted logs or dumps. Remove secrets, serials, MAC/IP/SSID, usernames, hostnames and private paths from every field and attachment locally. Review the exact sanitized text before submission. Screenshots and attachments need the same review. The required acknowledgement records the submitter's review; it does not automatically sanitize data or authorize additional collection/publication. UNKNOWN plus a reason is preferable to a guess.

## Review COST-L references and decisions

Assigned IDs follow COST-L-#### with four digits; COST-L-0000 is reserved. Verify each assigned ID against the [allocation ledger](../../research/cost-l/registry.json). Unallocated proposals use candidate plus an issue link. Form fields do not allocate IDs, confirm records or expand authority. The [registry contract](../architecture/compatibility-debt.md) checks allocated record identities and semantic relations separately.

GitHub form validations require answers and checkbox acknowledgements; these forms do not enforce a regex on free text or verify historical truth. Reviewers must reject malformed, reserved or unallocated stable references, ambiguous evidence scopes and fabricated costs before converting a submission into a record. Research classifications use the registry vocabulary; the form also allows unresolved classification with a reason. Owner, native decision, IOMMU guarantees and measured costs may be unknown at proposal time. Support declarations and executed observations remain distinct.

## Validation and availability

The YAML uses GitHub's [issue-form syntax](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms) and [form schema](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-githubs-form-schema). Local validation parses all five forms, checks identifiers, required fields, distinct dropdown options, acknowledgement controls, classification choices and repository links, and constructs a sample/preview for each. PR #60 merged the five forms into the default branch. Local checks establish structure, not GitHub's live rendering or semantic validity of future answers.

On 2026-10-05, authenticated browser validation for [issue #56](https://github.com/lifeFedorovAlexey/KOLVRT/issues/56) exercised the actual chooser and all five forms. Synthetic drafts verified editable fields, Markdown previews, dropdown choices and privacy acknowledgements. Empty titles, required text fields and unchecked acknowledgements produced GitHub validation errors; the optional upstream field in Hardware Bring-up remained optional. The chooser retained Blank issue and no labels were added. Malformed/reserved/unallocated COST-L text remained editable as documented; reviewer validation is still required. The [validation record](../../research/results/issue56-live-forms.json) identifies the exact form sources and observation limits. No sample issue, inventory, attachment or hardware result was published. Future changes to the forms require renewed validation; this observation does not certify future submissions or GitHub UI versions.

[Russian translation](../../translations/ru/docs/research/issue-submissions.md)
