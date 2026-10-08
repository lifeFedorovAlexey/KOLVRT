# IPC and production-service Arena passport

Document status: CURRENT
Evidence scope: 96 retained functional pilot boots; the separately preregistered 576-boot main campaign and current-source acceptance remain pending. Historical pilot evidence does not verify later writer changes.
Current reference: [Native IPC](../kernel/ipc.md); [Arena measurement contract](../architecture/arena-measurement-contract.md)

<a name="kolvrt-arena-ipc-passport"></a>

## Useful operations and authority

The [main protocol](../../research/arena/ipc-query/protocol.json) names twelve cases. Nine transport cases combine 0/8/256 request bytes with same-CPU single requester, cross-CPU single requester and cross-CPU two requesters. A separate capacity-one scenario verifies rejection and drainage. Two cases exercise the unchanged production counter-service with its exact 16-byte Add/Get protocol, using one or two cross-CPU requesters.

Existing immutable grants suffice: root CPU0 sends to child slot1 CPU0 or service slot0 CPU1. Root and slot1 can hold separate authentic SEND capabilities to slot0. No handle integer is transferred as authority, no new deployment input or widened grant is introduced. Same-CPU contention, reverse directions and same-CPU counter are outside this selected coverage. The transport responder and clients are external ABI tools with no production counterpart; they do not copy service, supervisor or kernel implementation. Running an external root does not verify the production supervisor.

The responder checks payload and returns a sequence reflecting actual service execution. Zero-byte requests require sequence/ledger validation, not merely an empty successful reply. Counter results and the final state must agree with actual successful Add operations across both requesters. Queue saturation uses ordinary feedback IPC: the responder awaits root's reply while root fills the real queue, observes the next Submit refusal, releases the responder and verifies drainage. No timer delay or kernel test rendezvous establishes this order. The admitted A envelope deliberately includes nested B rejection probes and the feedback-release RPC before A Collect; it is a separate controlled workload, not an ordinary independent round-trip.

## Outcomes and measurements

Each offered attempt retains client/request identity, stage/status, endpoint timestamps and result. The one-second request deadline is the existing public SDK rpc policy: start plus counter frequency; it is not a latency guarantee or a new kernel coordination deadline. No failure causes a timeout increase or retry. Expected Exhausted proves queue refusal but contributes no useful success. Unexpected expiry, incomplete collection or wrong service effect rejects correctness. Concurrent clients alone do not prove temporal overlap. Timestamp intersections of admitted-outcome userspace envelopes are reported separately from queue refusals; envelope overlap does not prove simultaneous kernel-inflight requests.

Three common CLOCK calls bracket Submit, its intermediate result and completed Collect. The wall envelope is userspace submit-to-collected-result, not precise internal admission-to-terminal latency. x2 is the client's partial execution window, including transition edges and stalls; x3 covers READ_WINDOW, not IPC service. Queue residence, service CPU, capability/lock costs, PMU, dynamic copy counts, peak memory and per-request switches remain unavailable without attributable evidence. Throughput uses measured-phase useful successes only. The common interval starts after both warmup phases and before peer GO, and ends after peer DONE (or the last root sample for one requester), before bulk raw export. GO/DONE control overhead is included. Do not invert median latency or add overlapping durations.

Recorder ON adds exactly one volatile store of middle CLOCK ticks to actor-private memory before end CLOCK; OFF retains identical offered work, CLOCK calls and oracle. This is incremental recorder cost, not total probe cost, and is never subtracted as an exact correction. Keep DEV and PROD populations separate.

## Retained pilot and main campaign

The [sealed pilot bundle](../../research/arena/runs/ipc-pilot-v1/README.txt) retains 96 of 96 successful planned functional boots and prior failed preparation attempts with their original provenance. It contains 3456 raw records: 3080 measured offers, 2637 useful successes and 443 measured Exhausted outcomes. The independent review recomputed counts, timing boundaries and cross-client envelope intersections. The archived source was recovered separately; the original dirty execution is not relabeled a clean run. Archive verification checks 646 original members plus definition/supplemental bytes without executing QEMU. This is functional pilot evidence, not qualifying performance or main-campaign acceptance.

The stable [measurement contract](ipc-measurement-contract.md) and main protocol fix twelve fresh pairs per case/profile, 576 boots: pair index 0–11, case 0–11, DEV then PROD, OFF/ON for even pairs and ON/OFF for odd pairs. Pilot slots do not count toward main execution. Offered populations stay at four warmups plus 32 measured requests, or three plus 33 for saturation. The design increases fresh boot repetitions without growing actor storage. It retains fixed finite-session conditioning and makes no steady-state stabilization or p99 precision claim.

The actor bounds each boot to 36 total observations because the existing stack is 16 KiB. Two requesters contribute 18 each, including two warmups and sixteen measured attempts. The report has a 128-word header and eight words per observation, totaling 416 words; the protocol fixes every field and reserved word. No stack, endpoint, report buffer or logger limit grows for the benchmark. Existing 2048-word reports are exported in bounded 64-word events and checked for complete identity-bound reconstruction. Data export is outside individual latency envelopes; its aggregate impact is disclosed.

The producer reuses shared build/execution, immutable artifact custody and the existing Arena assessor. All IPC profiles target the existing canonical feature `kolvrt.ipc.transport`. Forty-eight case/profile/family profiles separate successful latency from one achieved-throughput aggregate per boot. Throughput has no separate aggregate warmup observation; actual request conditioning remains retained. Case 9's pooled A/C latency is supplemental to separate A/B/C distributions. Matched observer arrays retain all offered measured requests and outcomes, without success truncation; differing outcome vectors make interpretation INCONCLUSIVE. Fixed success-conditioned sample shortfalls and refusals remain visible and may make structural admission INELIGIBLE.

The paired uncertainty report is conditional: twelve boot-pair statistics, median and sorted third-to-tenth interval with 96.14% nominal coverage under independent identically distributed continuous paired observations. Host drift can violate these assumptions. This is a marginal median-effect description, not simultaneous case coverage, a speedup decision or a guarantee of tail precision. No observed variance creates a performance allowance.

SEC maps six scoped native-request requirements to four source/runtime/negative-evidence methods. Full current matrix evidence must match the actual plan, all obligations and source inventory; the existing matrix validator checks configuration and donor lineage. Named checks need actual executed witnesses in both DEV and PROD, not a partial shard or reused-only assertion. Missing evidence cannot become PASS. Main execution, current-source SEC/regression receipts and independent acceptance remain pending; historical pilot success closes none of those gates. QEMU does not establish physical ARM64 or Linux/seL4 superiority.

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
        "implementation_scope": "Experimental external IPC producer with selected transport and original counter-service workloads, complete bounded report export, shared custody/admission and 48 profiles for a fixed 576-boot main plan. The retained 96-boot functional pilot is historical source-bound evidence; current main execution and SEC acceptance remain pending.",
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
          "apps/native-apps/src/counter-service.rs",
          "crates/xtask/src/arena_common/passport.rs",
          "crates/xtask/src/arena_ipc/passport.rs",
          "research/arena/profiles/ipc-query-0-dev-latency.json",
          "research/arena/profiles/ipc-query-0-dev-throughput.json",
          "research/arena/profiles/ipc-query-0-prod-latency.json",
          "research/arena/profiles/ipc-query-0-prod-throughput.json",
          "research/arena/profiles/ipc-query-1-dev-latency.json",
          "research/arena/profiles/ipc-query-1-dev-throughput.json",
          "research/arena/profiles/ipc-query-1-prod-latency.json",
          "research/arena/profiles/ipc-query-1-prod-throughput.json",
          "research/arena/profiles/ipc-query-2-dev-latency.json",
          "research/arena/profiles/ipc-query-2-dev-throughput.json",
          "research/arena/profiles/ipc-query-2-prod-latency.json",
          "research/arena/profiles/ipc-query-2-prod-throughput.json",
          "research/arena/profiles/ipc-query-3-dev-latency.json",
          "research/arena/profiles/ipc-query-3-dev-throughput.json",
          "research/arena/profiles/ipc-query-3-prod-latency.json",
          "research/arena/profiles/ipc-query-3-prod-throughput.json",
          "research/arena/profiles/ipc-query-4-dev-latency.json",
          "research/arena/profiles/ipc-query-4-dev-throughput.json",
          "research/arena/profiles/ipc-query-4-prod-latency.json",
          "research/arena/profiles/ipc-query-4-prod-throughput.json",
          "research/arena/profiles/ipc-query-5-dev-latency.json",
          "research/arena/profiles/ipc-query-5-dev-throughput.json",
          "research/arena/profiles/ipc-query-5-prod-latency.json",
          "research/arena/profiles/ipc-query-5-prod-throughput.json",
          "research/arena/profiles/ipc-query-6-dev-latency.json",
          "research/arena/profiles/ipc-query-6-dev-throughput.json",
          "research/arena/profiles/ipc-query-6-prod-latency.json",
          "research/arena/profiles/ipc-query-6-prod-throughput.json",
          "research/arena/profiles/ipc-query-7-dev-latency.json",
          "research/arena/profiles/ipc-query-7-dev-throughput.json",
          "research/arena/profiles/ipc-query-7-prod-latency.json",
          "research/arena/profiles/ipc-query-7-prod-throughput.json",
          "research/arena/profiles/ipc-query-8-dev-latency.json",
          "research/arena/profiles/ipc-query-8-dev-throughput.json",
          "research/arena/profiles/ipc-query-8-prod-latency.json",
          "research/arena/profiles/ipc-query-8-prod-throughput.json",
          "research/arena/profiles/ipc-query-9-dev-latency.json",
          "research/arena/profiles/ipc-query-9-dev-throughput.json",
          "research/arena/profiles/ipc-query-9-prod-latency.json",
          "research/arena/profiles/ipc-query-9-prod-throughput.json",
          "research/arena/profiles/ipc-query-10-dev-latency.json",
          "research/arena/profiles/ipc-query-10-dev-throughput.json",
          "research/arena/profiles/ipc-query-10-prod-latency.json",
          "research/arena/profiles/ipc-query-10-prod-throughput.json",
          "research/arena/profiles/ipc-query-11-dev-latency.json",
          "research/arena/profiles/ipc-query-11-dev-throughput.json",
          "research/arena/profiles/ipc-query-11-prod-latency.json",
          "research/arena/profiles/ipc-query-11-prod-throughput.json",
          "crates/xtask/src/arena_ipc/analysis.rs"
        ],
        "acceptance": [],
        "issues": [33],
        "adrs": ["adr.0006", "adr.0025", "adr.0026"],
        "limitations": [
          "Pilot does not establish final main-campaign acceptance, adequate tail precision, physical performance or eligible records. No full Cartesian CPU/contention coverage or same-CPU counter claim."
        ],
        "next_gate": "Execute the preregistered 576-boot main campaign without replacement; retain conditional paired uncertainty, complete outcomes and current-source SEC/full-matrix evidence, then independently review bounded acceptance. Historical pilot boots do not fill main slots.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "UNKNOWN",
            "reason": "The retained 96-boot pilot passed at its named source; subsequent shared writer/security admission changes and the main campaign have no accepted current-source receipt."
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
