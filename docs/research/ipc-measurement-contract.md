# IPC measurement contract

Document status: DESIGN BASELINE
Evidence scope: normative main-campaign measurement protocol; no execution result, verification state or acceptance receipt.
Current reference: [Native IPC](../kernel/ipc.md); [Benchmark methodology](../architecture/benchmarking.md)

<a name="kolvrt-arena-ipc-measurement-contract"></a>

## Workloads and authority

This contract specifies twelve cases through existing immutable native bootstrap grants. Cases 0–8 are transport request sizes 0, 8 and 256 bytes, each in same-CPU single-requester, cross-CPU single-requester and cross-CPU two-requester order. Case 9 is capacity-one saturation and drainage. Cases 10–11 execute the original production counter-service with its exact 16-byte Add/Get contract and one or two cross-CPU requesters.

Root CPU0 reaches the slot1 CPU0 responder for same-CPU transport or slot0 CPU1 for cross-CPU transport/service. Root and slot1 have separately minted SEND capabilities to slot0 for two-requester work. A numeric name sent to its owning process does not transfer authority between namespaces. No deployment input, grant expansion, forced kernel rendezvous or production service copy is permitted. Same-CPU contention, reverse CPU directions, same-CPU counter and production supervisor execution are outside this coverage.

The external transport responder validates the declared request bytes and returns an execution sequence. Successful zero-byte operations require the same independent server count/sequence ledger as nonempty operations. The production counter starts at zero; successful Add(1) results must be unique and consistent with the final Get and total successful additions across both clients. Rejections do not advance useful-success counts. Wrong results, identity mismatches, unexpected expiry, incomplete collection, abnormal exit or failed reclamation invalidate correctness. Source review and original ELF custody accompany observations; a host oracle is not proof against fabrication of the entire evidence chain.

## Timing and outcomes

Each attempt uses the unchanged SDK request policy: checked `start.ticks + start.frequency`, one counter-frequency interval. This is a client deadline, not a kernel synchronization timeout, accepted latency allowance or retry policy. Every offered attempt retains client/request identity, warmup/measured phase, raw start/end, outcome stage/status, partial execution-window and READ_WINDOW deltas, and useful result. Retain failures and unfinished attempts; no replacement, retry, selective omission or synthetic successful sample is permitted.

Three common CLOCK calls observe before Submit, immediately after Submit and after terminal Collect or admission refusal. The measured interval is the external userspace submit-to-collected-result/refusal envelope. It includes probe and scheduling effects and is not exact internal admission-to-terminal time. Checked differences and residuals must not underflow. CLOCK x2 is the client's partial execution window, not exclusive CPU or service cost. x3 is READ_WINDOW accounting, not IPC service. Queue residence, locks, capability-validation cost, dynamic copy counts, memory peaks, per-request switches and PMU cycles are unavailable without separate attributable evidence. DEV and PROD remain distinct classes; QEMU does not establish physical performance.

Capacity-one saturation uses ordinary feedback IPC. The responder waits for root's reply; root admits A and requires B to return Exhausted, then releases the responder, collects A and verifies a subsequent C. A's interval intentionally includes B's probes/refusal and the feedback-release RPC. Report A, rejected B and recovery C separately; A is not an independent ordinary round-trip. Expected Exhausted is a correctness witness with zero useful-throughput contribution. Two clients do not prove simultaneous admitted kernel work: report measured cross-client envelope intersections and queue refusals separately, including observed serialization.

## Metric families and observer pairing

The request-envelope family retains all offered measured intervals, including refusals with their exact outcomes. Successful latency and rejection latency are separately labeled conditional summaries, never truncated to matching lengths or relabeled as an all-success population. Publish counts, nearest-rank median/p95/p99, mean, sample variance and standard deviation when defined. Nearest rank is `sorted[ceil(p*n)-1]`. Empty or insufficient populations are explicitly unavailable/inconclusive; p99 from at most 32 successful samples selects the maximum and establishes no tail precision. Saturation's heterogeneous successful A/C summaries do not replace their separate distributions.

Throughput is a distinct metric family with one aggregate observation per boot and zero aggregate warmup observations. This does not eliminate request conditioning: retain four warmup and 32 measured offered requests, or three warmup and 33 measured offers for saturation. The common phase starts after both requesters finish warmups and before peer GO; it ends after peer DONE, or after the final root sample for one requester, before bulk raw export. GO/DONE control overhead is included. Rate is measured-phase useful successes times recorded frequency divided by common elapsed ticks. Retain numerator, denominator and all outcomes. Never invert median latency or sum overlapping client intervals. Zero elapsed time is invalid; no successful operations yields an observed zero rate, not a fabricated latency sample.

OFF and ON perform the same offered operations, three CLOCK calls and correctness checks. ON adds exactly one volatile store of intermediate CLOCK ticks to actor-private memory before end CLOCK. The matched observer arrays contain every offered measured envelope with request/phase identities and outcomes; they are not success-filtered or shortened to equal success counts. If outcome vectors differ, label the paired observer interpretation INCONCLUSIVE rather than interpreting conditional-population differences as recorder cost. Retain both complete populations regardless. Even with matching outcomes, the comparison concerns incremental recorder behavior under scheduling, not all timestamp instrumentation or isolated CPU cost. Do not subtract an exact corrected latency. OFF is the primary operation series.

## Fixed main campaign and uncertainty

After pilot review, the main campaign has twelve fresh pairs for each case/profile: 576 new boots. The order is pair index 0–11 outermost, case 0–11 next, DEV then PROD, with OFF/ON for even pair indices and ON/OFF for odd indices. This gives six pairs of each order per case/profile. Historical pilot runs do not fill main slots. The stopping rule is this fixed plan, not observed performance; retain every attempted failure and mark incomplete slots. Source changes during a campaign invalidate its source-bound applicability. An implementation revision does not inherently create a new comparison class; changes to configuration, functional guarantees, workload, security or methodology do.

Each boot offers 36 requests total. Ordinary cases retain four warmups and 32 measured offers; two requesters contribute two warmups and sixteen measured offers each. Saturation uses twelve three-operation cycles, the first as warmup. This remains a finite-session conditioning protocol, not proof of steady-state stabilization. No actor stack, endpoint or report capacity is enlarged. The report retains 128 header words plus eight words per observation, 416 words total, through the existing 2048-word storage and bounded 64-word event chunks. Bulk peer export and final reporting are outside individual timing intervals and the declared aggregate interval. Missing or malformed chunks invalidate completeness.

Fresh boots/pairs are the analysis units; requests within a boot may correlate. For each case/profile retain all twelve paired differences and log ON/OFF ratios of per-boot medians and achieved rates when defined. Report the median paired statistic and order-statistic interval from the third to tenth sorted paired values; exponentiate interval endpoints for ratios. Under independent, identically distributed continuous paired observations its coverage is `1 - 2*(1+12+66)/4096 = 0.96142578125`. This is a conservative marginal 95% interval for the median paired effect, not a mean-effect interval, simultaneous coverage across cases or guaranteed precision. Zero denominators, missing pairs and differing observer outcomes make the relevant ratio/interpretation unavailable or inconclusive; never replace pairs to obtain twelve usable values.

Twelve pairs are a bounded uncertainty design, not a performance allowance or power guarantee. Fresh boot does not establish independence or exchangeability under host drift. Publish pair index, order and all outcomes; disclose those assumptions and keep inference conditional when unsupported. Do not average per-run p99 and call it a pooled quantile. Pooled descriptions do not convert correlated requests into independent tail evidence. No regression margin, superiority decision or stopping adjustment may be invented after observing results.

## Admission and applicability

Use the existing Arena schema, versioned profiles, registry and assessor. Pin this stable contract, declared workload/input/resource/environment definitions and oracle separately from mutable acceptance documents. Exact implementation, image and source identities belong to run provenance. Functional, workload, security or methodology changes define new comparison classes. DEV/PROD and QEMU/hardware populations must not mix.

Actual SEC evidence must cover the declared native-request requirements for caller identity, authority attenuation, stale capabilities, isolation, bounded charging and reviewed revoke semantics. Required missing/failed correctness or SAR evidence prevents qualifying admission; expected capacity refusal cannot waive those gates. Structural admission is not authenticated execution, adequate statistical power or an eligible record. This contract supplies no campaign PASS, source-bound acceptance, hardware result or change to record eligibility. Current implementation and evidence are maintained separately in the [IPC passport](ipc-passport.md).

[Russian translation](../../translations/ru/docs/research/ipc-measurement-contract.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.research.ipc-measurement-contract",
  "kind": "subsystem-contract",
  "summary": "Fixed IPC measurement populations, useful-result oracles, observer pairing and bounded uncertainty.",
  "units": [
    {
      "id": "kolvrt.arena.ipc-measurement.contract",
      "anchor": "kolvrt-arena-ipc-measurement-contract",
      "kind": "contract-section",
      "summary": "Normative external IPC and production-counter measurement contract; no execution acceptance.",
      "depends_on": [
        "kolvrt.ipc.request",
        "kolvrt.clock.query.api",
        "kolvrt.apps.native-elf",
        "doc.kolvrt.architecture.benchmarking",
        "doc.kolvrt.arena.measurement-contract"
      ]
    }
  ]
}
```
