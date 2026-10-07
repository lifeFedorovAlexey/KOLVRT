# Clock-query Arena pilot

Document status: CURRENT
Evidence scope: proposed bounded issue #32 measurement protocol; implementation review and real execution receipts are pending.
Current reference: [Native CLOCK contract](../kernel/clock.md); [Arena measurement contract](../architecture/arena-measurement-contract.md)

<a name="kolvrt-arena-clock-passport"></a>

## Pilot and useful operation

This external ELF client exercises the single production `native::clock_snapshot()` implementation and the existing CLOCK syscall. No production service is copied; there is no added kernel feature, fault hook or timing policy. The generic native bootstrap receives the client as its root and passes the existing opaque `KOLVRT_NATIVE_ARGUMENT`: 1 selects recorder OFF, 2 selects recorder ON, and other values are invalid. This is a necessary standalone ABI measurement client with no production application counterpart.

Each iteration calls CLOCK three times: start snapshot, useful query, end snapshot. Both modes perform the same useful query and oracle. Frequency must remain nonzero and equal; timestamps and accumulated counters must be monotonic, and the useful query must lie inside the endpoint interval. Every READ_WINDOW delta must be zero. Checked arithmetic must reject underflow or overflow. The measured duration is the interval between the two endpoint kernel sampling points, including the useful query, userspace bookkeeping and parts of the endpoint calls. It is not pure syscall latency, a null trap, IPC latency or exclusive processor cost.

## Report and oracle

The report contains exactly 64 unsigned 64-bit words; integer precision must survive collection and import. Words 0–7 identify the magic `0x434c4b01`, version 1, mode, frequency, four warmups, 24 measured samples, 28 completed useful queries and zero accumulated READ_WINDOW delta. Words 8–35 contain all 28 wall deltas; words 36–63 contain their matching execution-window deltas. The first four entries in each array are explicitly retained warmups. The common oracle runs after the end snapshot and checks each sample before export; aggregate values cannot hide individual failures. The report magic is defined by the actor and checked by the host oracle.

The actor exports a successful schema-1 report only after the measured loop. A failed sample instead emits a partial schema-0 diagnostic after its interval and exits unsuccessfully; that diagnostic is never a performance result. REPORT calls and report-array stores after the end snapshot are outside the envelope. Missing, extra or malformed reports, unknown modes/versions, changed frequency/counts, failed useful-query checks, nonzero root exit, panic, timeout or missing reclamation reject the invocation. The host retains logs and failure accounting rather than converting partial execution to a successful run. Normal completion must release owners, restore frames and leave zero live processes/domains. Report export does not grant access to another process or change kernel report capacity.

## Matched recorder cost

ON records the intermediate useful-query snapshot to a fixed userspace buffer before the end snapshot. OFF omits only this additional recorder. Both keep the same three CLOCK calls, mandatory oracle, endpoint collection and report export. Retained volatile stores prevent removal of the ON recorder. Branch and storage cost belong to this named observer protocol.

The matched comparison estimates only the incremental intermediate-recorder cost. OFF is not instrumentation-free: endpoint CLOCK probes and ordinary result collection remain. Neither mode measures all timestamp-observer overhead. Preserve both raw series; do not subtract an exact corrected latency or infer syscall cost from their difference. The OFF series is the primary useful-operation measurement; ON provides matched observer evidence, not a second implementation or a superiority claim.

## Sampling and attribution

Before execution, fix three fresh pairs per profile in OFF/ON, ON/OFF, OFF/ON order. Every invocation has four marked warmups followed by 24 measured samples. DEV and PROD give twelve fresh boots total. There are no retries, historical observation reuse, selective outlier removal or success-dependent stopping. A failed campaign preserves all attempted and unfinished observations and does not publish successful campaign acceptance.

Independent guest boots are the repeat unit; samples within one boot may be correlated. The four warmups are fixed conditioning, not proof of stabilization. Retain pair identity and order; do not flatten 72 observations into independent trials. Use predeclared nearest-rank descriptive medians. Tail adequacy, p95/p99 inference, regression stability and comparative superiority remain INCONCLUSIVE with this limited campaign. No performance budget is invented.

Retain wall, execution-window and READ_WINDOW differences plus the checked unattributed residual. DEV exposes the existing kernel-collected partial execution-window attribution; PROD observes the same public ABI externally. Neither is exclusive CPU time. CLOCK service, IRQ service and host scheduling cannot be separately inferred from the residual. x3 is zero for this workload because no READ_WINDOW runs; it is not a zero-cost CLOCK assertion. x4 is not read. DEV/PROD have separate profiles and comparison classes even when a metric has SHARED visibility.

## Passport and security scope

The frozen [protocol artifact](../../research/arena/clock-query/protocol.json) defines the versioned workload semantics and recorder comparison. Profiles pin its bytes independently of this feature's evolving acceptance metadata; the normative CLOCK document supplies the separately pinned API contract. Any change to the protocol requires a profile revision and applicability review.

Use the existing Arena profile/run schemas, registry and xtask evidence pipeline. The [DEV profile](../../research/arena/profiles/clock-query-dev.json) and [PROD profile](../../research/arena/profiles/clock-query-prod.json) begin PROPOSED. Each pair keeps source/image/config/toolchain/QEMU identities, frozen profile/registry snapshots, marked warmups, raw observations and artifact digests; an immutable campaign manifest links the three pairs. No second registry or universal score is introduced. A standalone mechanism needs no fabricated alternative implementation. Existing admission remains at most STRUCTURALLY_ADMISSIBLE; `record_eligible=false` is preserved and contribution ownership stays UNATTRIBUTED without separate accepted evidence.

The security target covers CLOCK's read-only current-process observation boundary, the external oracle and retained-result admission. Trusted boot input, timer operation and existing process isolation are assumptions, not newly proven properties. Custom SFRs require truthful complete observations, no inferred authority or cross-process selector, and normal bounded termination/reclamation. Required SAR evidence combines actual execution, source-bound review and negative host inputs. An omitted operation reported as an incomplete useful-query count, broken oracle, missing/duplicate/truncated observations, mandatory SFR failure or missing SAR must reject the record. These controls mutate external inputs or evidence, never kernel implementation. They do not prove protection against a malicious kernel or a producer able to forge the entire custody chain.

No clock authorization grant is invented. PMU, physical hardware, resource peaks, reliability campaigns, fuzz coverage, vulnerability analysis, attack-potential levels and certification are not established by this pilot. PERF and SEC remain separate dimensions; other dimensions have explicit scope gaps. A producer PASS label cannot replace evidence or independent applicability review.

## Current evidence

No real pilot execution is accepted in this document yet. Source and complete EN/RU review, host rejection checks, exact-source DEV/PROD runs and retained reports must precede acceptance. This provisional protocol does not close issue #32 or advance physical-platform readiness.

[Russian translation](../../translations/ru/docs/research/clock-passport.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.research.clock-passport",
  "kind": "subsystem-contract",
  "summary": "Bounded external CLOCK envelope protocol and separate observer/security evidence.",
  "units": [
    {
      "id": "kolvrt.arena.clock-passport",
      "anchor": "kolvrt-arena-clock-passport",
      "kind": "feature",
      "summary": "Real mechanism passport pilot with partial accounting and matched recorder protocol.",
      "depends_on": [
        "kolvrt.clock.query.api",
        "doc.kolvrt.arena.measurement-contract",
        "adr.0006",
        "law.036",
        "law.044"
      ],
      "feature": {
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "Bounded external CLOCK ABI actor and Arena passport pipeline with matched intermediate-recorder observations; actual execution acceptance pending.",
        "sources": [
          "apps/native-runtime/src/lib.rs",
          "tests/native-apps/Cargo.toml",
          "tests/native-apps/src/clock-client.rs",
          "crates/xtask/src/arena_clock.rs",
          "crates/xtask/src/arena_clock/passport.rs",
          "crates/xtask/src/native_apps.rs",
          "crates/xtask/src/main.rs",
          "crates/kernel/src/scheduler/mod.rs",
          "crates/kernel/src/scheduler/task.rs",
          "crates/kernel/src/arch/aarch64/entry.S",
          "research/arena/profiles/clock-query-dev.json",
          "research/arena/profiles/clock-query-prod.json",
          "research/arena/clock-query/protocol.json",
          "research/arena/clock-query/input.json",
          "research/arena/clock-query/resources.json",
          "research/arena/clock-query/dev-environment.json",
          "research/arena/clock-query/prod-environment.json"
        ],
        "acceptance": [],
        "issues": [32],
        "adrs": ["adr.0006", "adr.0015"],
        "limitations": [
          "No pure syscall latency, full timestamp observer cost, exclusive CPU attribution, physical evidence, adequate tails or eligible Arena record."
        ],
        "next_gate": "Review profile, oracle and complete EN/RU pair; execute twelve fresh DEV/PROD guest boots and retain exact-source passport evidence.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "UNKNOWN",
            "reason": "Real pilot campaign pending."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical benchmark campaign."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Introduce the external pilot protocol and proposed profiles without execution acceptance."
          }
        ]
      }
    }
  ]
}
```
