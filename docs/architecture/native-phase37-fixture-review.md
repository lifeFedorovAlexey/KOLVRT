# Phase 3.7 committed-crash fixture and acceptance renewal

Document status: HISTORICAL MILESTONE
Evidence scope: Codex source/code/architecture and complete EN/RU review at the maintainer's request for implementation 43af402b185afab82177713ed3076595d7e648a6, base bf54960925559d8925c24b6d40e557e98af08f41 and independently inspected current CI. No independent human sign-off, physical ARM or performance acceptance is claimed.
Current reference: [Native applications](../kernel/native-applications.md)

## Decision and exact correction

Renew the [earlier bounded first-slice acceptance](native-phase37-acceptance-review.md) for the corrected source. The production SDK, ELF applications, loader, IPC transitions, scheduler and recovery behavior are unchanged by this correction. ABI remains experimental; production readiness and image trust remain separate.

The [333726b failure](../../research/results/native-phase37-333726b-failure.json) is preserved: stage 1002, Submit operation 1, Expired=16, coverage 15. Source identifies payload operation 2, the committed-crash request, not a readiness probe. Its short readiness deadline allowed expiry before admission, so the test could not reach the actual COMMIT/fault it intended to check. Native DEV/PROD passed independently in that failed full run; those results did not authorize claiming a green foundation.

The assembly fixture now selects the maximum valid absolute deadline for operation 2, as it already did for the separate Cancel scenario. This is a non-expiry input to the committed-fault test, not a new production timeout or an accepted Expired result. Operation 1 readiness retains /8, including the deliberate silent-service expiry case. Actual COMMIT, EffectUnknown, Faulted completion, fresh identity, stale rejection, bitmap 255 and acquired resource reclamation remain mandatory. No production special case, delay, retry-until-green or source-copy mutation is added.

## Executed evidence and review

[Full CI 37571776305](https://github.com/lifeFedorovAlexey/KOLVRT/actions/runs/37571776305) passed all four shards and every mandatory workload, evidence and foundation at 43af402b185afab82177713ed3076595d7e648a6. Dependency hygiene passed separately. The [new receipt](../../research/results/native-phase37-43af402.json) independently verifies 144/144 executed tasks, matching build/run ELF identities, 151 source inputs and eight immutable original artifact payloads. Guest plan inputs are distinguished from seven authenticated workflow/tooling checkout inputs. Native DEV/PROD again prove persistent requests and actual production crash/recovery/fresh binding/counter 12 followed by normal shutdown with zero leaks. The corrected restricted-workspace execution proof passed.

Three local DEV+PROD ordinary repeats passed all 147 checks/profile. npm run check, docs generate/pilot, complete reviewed EN/RU recording, source-bound impact/check-change against the named base and diff checks passed. Reviewed the changed deadline selection against the distinct readiness, committed-crash and cancellation assertions and the accepted IPC expiry semantics. Both locales retain the same inputs, exact outcomes, historical failures and exclusions. The earlier source/code/architecture review remains applicable to unchanged production code; this review covers the new fixture delta and evidence freshness.

## Preserved limits

Coverage remains four ordinary, ten legacy-failure, twenty-one invariant, ninety-three negative-input and sixteen mixed tasks. Positive TLBI/IRQ/ASID/shootdown coverage does not prove omitted-operation detection. This renewal does not claim that every historical finite-readiness expiry is repaired, universal latency, startup-crash/post-commit replay, disk durability, image trust, frozen ABI, physical ARM or performance admissibility. Earlier passing and failing receipts retain their exact original scope.

[Russian translation](../../translations/ru/docs/architecture/native-phase37-fixture-review.md)

## Attribution correction on 2026-10-07

The earlier statement in this historical review that stage 1002 uniquely identifies committed-crash payload 2 was too strong. The stage persists through the subsequent readiness probe payload 1 before sup_event 5; both inputs used /8 at 333726b. The retained old receipt cannot distinguish these sites, so the cause of that particular expiry remains uncertain. Original text and receipts are preserved for traceability; changing the payload 2 input alone does not prove that the cause of the 333726b failure was repaired.

The new [#133](https://github.com/lifeFedorovAlexey/KOLVRT/issues/133) failure at 038d960 in [CI 37576487522](https://github.com/lifeFedorovAlexey/KOLVRT/actions/runs/37576487522) is unambiguously replacement readiness: payload 2 uses the maximum deadline, while payload 1 inherits stage 1002 and /8. Submit rejects expiry before admission after 131.395008 ms with an EL0 residency increase of 1.499088 ms; the remaining 129.895920 ms is unattributed. The native job passed; earlier bounded acceptance and passing source-bound observations retain their scopes. Master #37 closure awaits regression disposition. No runtime repair, timeout increase or successful retry is claimed here.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.native-applications.fixture-review",
  "kind": "security-analysis",
  "summary": "Source-grounded committed-crash test input correction and bounded Phase 3.7 evidence renewal.",
  "relationships": [
    {
      "type": "related_to",
      "to": "kolvrt.apps.native-elf",
      "scope": "Renewed exact-source full CI after fixture correction; unchanged production code, no hardware/performance acceptance."
    }
  ]
}
```
