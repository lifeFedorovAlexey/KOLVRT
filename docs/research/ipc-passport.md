# IPC and production-service Arena pilot

Document status: CURRENT
Evidence scope: implementation in progress for issue #33; protocol and source changes do not establish successful execution or acceptance.
Current reference: [Native IPC](../kernel/ipc.md); [Arena measurement contract](../architecture/arena-measurement-contract.md)

<a name="kolvrt-arena-ipc-passport"></a>

## Useful operations and authority

The [pilot protocol](../../research/arena/ipc-query/protocol.json) names twelve cases. Nine transport cases combine 0/8/256 request bytes with same-CPU single requester, cross-CPU single requester and cross-CPU two requesters. A separate capacity-one scenario verifies rejection and drainage. Two cases exercise the unchanged production counter-service with its exact 16-byte Add/Get protocol, using one or two cross-CPU requesters.

Existing immutable grants suffice: root CPU0 sends to child slot1 CPU0 or service slot0 CPU1. Root and slot1 can hold separate authentic SEND capabilities to slot0. No handle integer is transferred as authority, no new deployment input or widened grant is introduced. Same-CPU contention, reverse directions and same-CPU counter are outside this selected coverage. The transport responder and clients are external ABI tools with no production counterpart; they do not copy service, supervisor or kernel implementation. Running an external root does not verify the production supervisor.

The responder checks payload and returns a sequence reflecting actual service execution. Zero-byte requests require sequence/ledger validation, not merely an empty successful reply. Counter results and the final state must agree with actual successful Add operations across both requesters. Queue saturation uses ordinary feedback IPC: the responder awaits root's reply while root fills the real queue, observes the next Submit refusal, releases the responder and verifies drainage. No timer delay or kernel test rendezvous establishes this order. The admitted A envelope deliberately includes nested B rejection probes and the feedback-release RPC before A Collect; it is a separate controlled workload, not an ordinary independent round-trip.

## Outcomes and measurements

Each offered attempt retains client/request identity, stage/status, endpoint timestamps and result. The one-second request deadline is the existing public SDK rpc policy: start plus counter frequency; it is not a latency guarantee or a new kernel coordination deadline. No failure causes a timeout increase or retry. Expected Exhausted proves queue refusal but contributes no useful success. Unexpected expiry, incomplete collection or wrong service effect rejects correctness. Concurrent clients alone do not prove temporal overlap. Timestamp intersections of admitted-outcome userspace envelopes are reported separately from queue refusals; envelope overlap does not prove simultaneous kernel-inflight requests.

Three common CLOCK calls bracket Submit, its intermediate result and completed Collect. The wall envelope is userspace submit-to-collected-result, not precise internal admission-to-terminal latency. x2 is the client's partial execution window, including transition edges and stalls; x3 covers READ_WINDOW, not IPC service. Queue residence, service CPU, capability/lock costs, PMU, dynamic copy counts, peak memory and per-request switches remain unavailable without attributable evidence. Throughput uses measured-phase useful successes only. The common interval starts after both warmup phases and before peer GO, and ends after peer DONE (or the last root sample for one requester), before bulk raw export. GO/DONE control overhead is included. Do not invert median latency or add overlapping durations.

Recorder ON adds exactly one volatile store of middle CLOCK ticks to actor-private memory before end CLOCK; OFF retains identical useful work, CLOCK calls and oracle. This is incremental recorder cost, not total probe cost, and is never subtracted as an exact correction. Keep DEV and PROD populations separate.

## Pilot, export and admission

The initial functional pilot offers 4 warmups plus 32 measured attempts per boot; saturation uses 3 plus 33 attempts. Two fresh OFF/ON and ON/OFF pairs across twelve cases and both profiles yield 96 boots. Retain every failed or unfinished attempt without replacement. Within-boot requests may correlate. A separate main plan must be frozen after reviewing the pilot; descriptive p95/p99 do not establish adequate tail precision.

The current actor bounds the pilot to 36 total observations because the existing stack is 16 KiB. Two requesters contribute 18 each, including two warmups and sixteen measured attempts. The report has a 128-word header and eight words per observation, totaling 416 words; the protocol fixes every field and reserved word. No stack, endpoint, report buffer or logger limit grows for the benchmark. Existing 2048-word reports are exported in bounded 64-word events and checked for complete identity-bound reconstruction. Data export is outside individual latency envelopes; its aggregate impact is disclosed.

The producer reuses CLOCK's ordinary build/execution path and shared Arena machinery. Existing schema, registry and correctness/security/loss gates remain authoritative. Success-conditioned populations with incomplete fixed sample plans may remain INELIGIBLE; no refusal is erased or replaced by a successful sample. Pilot data alone does not close #33. Main profiles, exact-source SEC/regression receipts, independent review and complete EN/RU checks remain required. QEMU does not establish physical ARM64 or Linux/seL4 superiority.

[Russian translation](../../translations/ru/docs/research/ipc-passport.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.research.ipc-passport",
  "kind": "subsystem-contract",
  "summary": "External IPC pilot with complete outcomes, real counter-service and bounded report export.",
  "units": [
    {
      "id": "kolvrt.arena.ipc-passport",
      "anchor": "kolvrt-arena-ipc-passport",
      "kind": "feature",
      "summary": "IPC transport and useful-service measurement through ordinary production interfaces.",
      "depends_on": [
        "kolvrt.ipc",
        "kolvrt.arena.clock-passport",
        "doc.kolvrt.arena.measurement-contract",
        "adr.0025",
        "adr.0026",
        "kolvrt.apps.native-elf"
      ],
      "feature": {
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "External functional IPC pilot covering three transport payloads, selected same/cross CPU placements, two requesters, capacity-one rejection and unchanged production counter-service; source implementation and acceptance are in progress.",
        "sources": [
          "tests/native-apps/Cargo.toml",
          "tests/native-apps/src/ipc-measure-root.rs",
          "tests/native-apps/src/ipc-measure-client.rs",
          "tests/native-apps/src/ipc-measure-peer.rs",
          "tests/native-apps/src/ipc_measure.rs",
          "crates/xtask/src/arena_ipc.rs",
          "crates/xtask/src/arena_common.rs",
          "crates/xtask/src/native_apps.rs",
          "crates/xtask/src/main.rs",
          "crates/kernel/src/native_boot.rs",
          "research/arena/ipc-query/protocol.json",
          "research/arena/ipc-query/input.json",
          "research/arena/ipc-query/resources.json",
          "research/arena/ipc-query/dev-environment.json",
          "research/arena/ipc-query/prod-environment.json",
          "tests/native-apps/src/ipc_measure_protocol.rs",
          "tests/native-apps/tests/ipc_measure_protocol.rs",
          "apps/native-runtime/src/lib.rs",
          "apps/native-apps/src/counter-service.rs"
        ],
        "acceptance": [],
        "issues": [33],
        "adrs": ["adr.0006", "adr.0025", "adr.0026"],
        "limitations": [
          "Pilot does not establish final main-campaign acceptance, adequate tail precision, physical performance or eligible records. No full Cartesian CPU/contention coverage or same-CPU counter claim."
        ],
        "next_gate": "Execute retained functional pilot, review actual outcome/stack/export behavior, freeze main sampling and admission profiles, obtain current-source SEC and regression evidence and independent EN/RU review before closing issue33.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "UNKNOWN",
            "reason": "Current IPC actor/runner changes have not completed a reviewed campaign."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical IPC campaign."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Introduce the external IPC producer without claiming execution or issue acceptance."
          }
        ]
      }
    }
  ]
}
```
