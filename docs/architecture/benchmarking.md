# Migration benchmark methodology

Document status: DESIGN BASELINE
Evidence scope: measurement and reproducibility requirements; no new execution results are claimed.
Current reference: [ADR-0006](../architecture-decisions/0006-metrics.md)

This is a methodology, not measured results. Native execution is not presumed faster.
Artificial delays, unequal quotas and weaker safety for one path are prohibited. A
faster compatibility path is reported honestly and motivates native-path analysis.

## Experimental contract

Before running, specify the workload, correctness oracle, semantic differences, primary
metric, offered load and stopping rule. Compare equivalent useful results. Buffered
and durable writes do not provide the same guarantee; quantify semantic translation
separately rather than hiding that difference in a speed ratio.

Use identical inputs, resource limits, scheduling domains, memory policies and devices.
Reset fixtures between paired runs. Replay external effects in independent snapshots;
do not duplicate real writes, messages or payments. Alternate A/B and B/A or randomize
with a recorded seed. Do not contend on one device when measuring isolated latency.

Record exact kernel and adapter digests, route manifests, compiler/linker flags, emulator
version/machine/CPU/accelerator or hardware stepping/firmware, RAM, processor count,
affinity, frequency policy, temperature, interrupt placement, background load and probes.
Separate cold-start and steady-state experiments. Emulator throughput does not establish
silicon throughput. Apply processor affinity symmetrically.

## Sampling and statistics

Predeclare warm-up stabilization tolerance, windows and a maximum duration. Retain marked
warm-up samples separately. Failed stabilization invalidates a run; it is not permission
to discard inconvenient slow samples.

Select sample size from the desired uncertainty, observed dependence and tail coverage,
using a pilot before the main experiment. A fixed number of runs cannot guarantee a
reliable p99. Insufficient independent runs or tail observations produce an inconclusive
result. Do not alter the stopping rule after seeing the winner; sequential inference
needs its own declared method.

Report sample count, run duration, median, p95, p99, mean, sample variance and standard
deviation. Fix the quantile estimator, for example `sorted[ceil(p*n)-1]`, in advance.
Confidence intervals use independent paired runs or dependence-aware blocks rather than
treating correlated requests as independent. Show absolute differences, ratios and
uncertainty. A ratio to zero is undefined. Per-run quantile distributions and pooled
quantiles are different statistics; an average of p99 values is not a pooled p99.

Keep timeouts, failures, dropped and unfinished requests in accounting. Report censoring;
a censored tail may permit only a lower bound. Do not hide outliers. Exclusions need
predeclared invalid-run criteria, reasons and retained raw samples. Publish all workloads
and predeclared summary weights. Regression margins come from requirements, not observed
variance after the experiment.

## Measurements

| Measurement       | Definition                                                                          |
| ----------------- | ----------------------------------------------------------------------------------- |
| Wall latency      | Admission to terminal outcome; queue and service times separately                   |
| Throughput        | Successful useful units per wall second, with errors                                |
| Processor cost    | Exclusive execution time, attributable deferred work and separate cycle counts      |
| Memory            | Private, shared and pinned bytes plus peak use; no duplicate shared-page accounting |
| Allocation        | Count, bytes, peak live bytes, allocator and size distribution                      |
| Context switches  | Voluntary and involuntary, with scheduler scope                                     |
| Copies            | Count and bytes at declared instrumented boundaries                                 |
| Locks             | Acquisitions, contention count and wait-time distribution                           |
| Translation       | Adapter entries, conversions, serialized and deserialized bytes                     |
| Hardware counters | Supported events, encoding, multiplexing and sampling error                         |
| Observer cost     | Matched instrumentation-on/off runs and lost-event counts                           |

Do not subtract noisy observer cost and claim an exact corrected value. Report uncertainty.
Unavailable counters remain unavailable. Separate adapter cost, shared native service
cost and end-to-end cost. A run artifact contains its manifest, workload digest, oracle
result, raw samples, rejected-run log, analysis version and report. Implement a benchmark
comparison runner only when two real comparable paths exist. A standalone mechanism measurement may use one real useful path with an oracle and explicit scope; it establishes no comparative superiority. Case records specify workload-specific obligations.

Missing required data rejects the analysis; a configured resource budget makes both
baseline and candidate measurements mandatory for every selected observation. Optional
measurements use typed available/unavailable results with the specific cause and missing
observation counts. Budget assessments distinguish passed, failed and not evaluated:
an absent budget is `not_evaluated` with `budget_not_configured`, never a pass. Insufficient
tail samples cannot pass a configured budget. Reports expose global and per-consumer
assessments, including p99. A supported latency gain applies only to the evaluated
latency claim; it does not establish compliance with unconfigured resource budgets.

## Replaceable component comparisons

[KOLVRT Arena](component-arena.md) extends this methodology to replaceable components
through shared contracts and suites, PERF/SEC/REL/RES dimensions and admission gates.
Its roadmap covers reproducible comparisons and COST-L migration evidence. These requirements
preserve this document's statistical and fairness obligations. Experimental offline contract validation is available through cargo xtask arena; real measurement pipelines and records remain open under #93/#32.

[Russian translation](../../translations/ru/docs/architecture/benchmarking.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.architecture.benchmarking",
  "kind": "policy",
  "summary": "Benchmark methodology: statistics, comparison fairness, attribution and reproducibility."
}
```
