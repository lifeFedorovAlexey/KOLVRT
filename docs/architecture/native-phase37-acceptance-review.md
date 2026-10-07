# Phase 3.7 native application acceptance review

Document status: HISTORICAL MILESTONE
Evidence scope: Codex source/code/architecture/EN-RU review at the maintainer's request over implementation 7f8dd80f6a322a4654994fc1aa3ed44beaefdd10, reviewed base bf54960925559d8925c24b6d40e557e98af08f41 and independently checked current CI artifacts. No independent human sign-off, production trust, physical ARM or performance acceptance is claimed.
Current reference: [Native applications](../kernel/native-applications.md)

## Decision and scope

Accept the bounded first native application slice for issue #28: original standalone ELF supervisor, counter service and client, private EL0 state across requests, native IPC, actual fresh-instance recovery and normal session reclamation. This is bounded implementation acceptance, separate from production readiness. Native interfaces remain EXPERIMENTAL and unfrozen; hardware remains UNKNOWN. Filesystem, disk durability, generic spawn, packages, networking and later milestones are excluded.

The complete source-bound CI run [37557806495](https://github.com/lifeFedorovAlexey/KOLVRT/actions/runs/37557806495) passed all four shards, all 144 planned/executed tasks, evidence aggregation, native, host, routing, ASID and both profile lints. Dependency hygiene passed separately. The [receipt](../../research/results/native-phase37-current.json) retains eight original artifact payloads with byte digests, 151 matching source inputs and exact checkout provenance. Seven workflow/tooling inputs are identified separately from the guest plan; their validation is not described as guest compilation. Original payloads use .receipt suffixes to preserve their JSON bytes and u64 values.

## Requirement-to-evidence review

| Issue #28 requirement                              | Actual implementation and evidence                                                                                                                                                                                                                                                                  | Conclusion                                                                                                                    |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Persistent service and retained state              | Counter::apply is the single SDK implementation; the real service loops over Receive/Commit/Reply. Actual client ADD(5), ADD(7), GET returns 5,12,12; both runtime profiles verify a GET after client exit.                                                                                         | State survives requests in one private instance; persistent mode has no kernel bootstrap watchdog.                            |
| Real standalone ELF and private processes          | Separate no_std/no_main artifacts; host passes original ELF bytes. Kernel Registry/ELF loader creates root, service and client with separate spaces; actual ELF publications and IPC/native calls are observed.                                                                                     | Host raw-image extraction and kernel application callbacks are absent.                                                        |
| Fast component methods                             | Host tests import actual Counter/parser/codecs, Table/Domain/Endpoint/ELF methods, application policy and shared bootstrap Grant/validate_grants called by Kernel Scope.                                                                                                                            | No second framework or copied implementation; grants validate authority shape, not image trust or dependency policy.          |
| External userspace selftests                       | Three separate ABI/lifecycle actors call public native SDK interfaces. Lifecycle actor imports recover_service; its test root does not execute production supervisor.                                                                                                                               | Necessary external ELF tools only; that older scope remains production_supervisor_tested=false.                               |
| DEV/PROD full SYSTEM and smoke                     | Ordinary completed sessions and separate app-smoke load the actual supervisor/service/client, verify three useful operations, client success and normal root exit.                                                                                                                                  | Both profiles passed; ordinary boot alone is insufficient.                                                                    |
| Actual service crash containment/recovery          | External GDB faults CPU1 EL0 before first client ADD COMMIT after binding. Same production supervisor queries real fault, applies canonical 1/128 backoff, creates fresh identity/binding and checks initial GET(0); same client rejects old SEND and completes business operations.                | Selected real crash/recovery is proved; no startup-crash or post-commit replay guarantee.                                     |
| Old endpoint/handle/token and authority            | Actual stale lifecycle/SEND and forbidden child binding/reply calls return precise errors. Wrong target and wrong endpoint kind are rejected; malformed parser/ELF inputs call real methods.                                                                                                        | Invalid inputs and real state refusals, not production sabotage or arbitrary-panic detection.                                 |
| Shutdown and zero retained resources               | FinishAfterClient observes successful client exit, exact service Terminated and normal root exit. Same crashed/recovered guest completes generic acquired retirement, restores physical frames and reports zero live processes/domains. Charges/endpoint references retain domains by construction. | Zero retained charges/frames/endpoints follows actual lifetime/resource invariants; forced VM stop is not this evidence.      |
| Cancel/revocation/under-load outcomes              | Full actual IPC/supervision matrix preserves real commit acknowledgement, explicit cancellation, EffectUnknown, denied/revoked/stale authority, source/ack drainage and reclamation checks.                                                                                                         | Expiry is not cancellation, elapsed time is not commitment or reclaim authorization.                                          |
| Test layers and exact negative coverage            | Four ordinary, ten legacy-failure, twenty-one invariant, ninety-three negative-input and sixteen mixed tasks. Host oracles reject wrong/missing fault, completion, identity, results and retained resources.                                                                                        | 140 control slots are not all mutation controls; positive TLBI/IRQ/ASID invariants do not prove omitted-operation detection.  |
| Physical dependency absence                        | Corrected caller-workspace runner actually creates DEV/PROD runtime/lifecycle/smoke receipts inside restricted workspace. Original implementation directory identities match; unrelated packages are absent, external native package count and implementation copies are zero.                      | Earlier proof that escaped to original cwd is invalid/historical; current proof is inspected, not inferred from green status. |
| Documentation, source/evidence and excluded claims | Complete changed canonical/Russian contracts and summaries preserve authority, outcomes, profile/scope boundaries, counts and uncertainty; historical receipts and append-only transitions remain.                                                                                                  | Source/semantic review is Codex review, not invented human approval; ABI and hardware exclusions remain explicit.             |

## Timing and test integrity

Production coordination requires actual acquired completion, detached roots, released owners and copy/source/ack drainage. There is no arbitrary completion-publication/copy-drain timeout; published CPU FAILED halts with resources retained. Silent stalls require external diagnosis and never permit reclaim. ELF bootstrap watchdogs are external; canonical supervisor backoff and assembly readiness /8 remain unchanged. The quantum fixture initializes its compared x20/TPIDR input before any IRQ and retains real execution/growth, exact outcomes, eight-quantum, context and resource assertions.

Historical 3873bc8 observed worker Submit expiry before admission: 127.22 ms against its 125 ms absolute policy. No request reached the service; this does not establish slow reply or a scheduler-performance defect. CLOCK residency attribution is retained, and the current complete run passed without increasing /8. This bounded functional acceptance is not a real-time guarantee, universal race-freedom claim or inference that all historical timing failures were fixed.

The external crash observer distinguishes unique receiver/service-token replies from repeated pre-SVC stops and requires a new COMMIT token. Ordinary W00 completion without the selected fault remains rejected. Host tests cover duplicate-hit accounting; the actual fault/completion/replacement/client/shutdown ledger remains strict. No retry-until-green, hidden guest delay or source-copy mutation is used.

## Semantic and locale review

Reviewed production SDK/application behavior, original ELF selection/loader, bootstrap grants and caller provenance, lifecycle binding/retirement, acquired scheduler/SMP/IPC drainage, current guard-input classification, workspace/tool resolution and actual public ABI actors. Read the complete changed EN/RU native, supervision, process, IPC, scheduler, SMP, CI, law/ADR and README pairs against those sources. Both locales preserve same limits, field meanings, source-specific history and exclusions. Translation hash recording and repository checks establish mechanical consistency; this authored source comparison supplies the separate semantic review.

## Remaining scope

No physical ARM, production image authorization/trust (#38), disk/crash durability, generic creation/migration, arbitrary startup/post-commit crash recovery or performance admissibility is accepted. Missing-publication/missing-ACK/skipped-TLBI/restore mutation equivalence is not claimed. Those broader scopes do not redefine the executed first-slice results. Issue #28 can use this bounded acceptance and exact-source evidence; merge/PR state is separately inspectable.

[Russian translation](../../translations/ru/docs/architecture/native-phase37-acceptance-review.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.native-applications.acceptance-review",
  "kind": "security-analysis",
  "summary": "Source-grounded bounded Phase 3.7 acceptance and full semantic EN/RU review, separate from production readiness and physical ARM.",
  "relationships": [
    {
      "type": "related_to",
      "to": "kolvrt.apps.native-elf",
      "scope": "First native ELF/persistent counter, selected actual production crash/recovery/shutdown and source-matched QEMU evidence; no physical ARM or stable ABI acceptance."
    }
  ]
}
```
