# Independent IPC pilot analysis

Campaign SHA256 `fa96a21f80a02ed69a6eab5f4c9d74f2166f1c07aaebfe20898a57b4456eec7b`. Counts: `{'boots': 96, 'passed': 96, 'raw_records': 3456, 'measured_offered': 3080, 'measured_successes': 2637, 'measured_exhausted': 443, 'all_phase_exhausted': 467}`. Source mismatches: `[]`. Validation findings: `[]`.

|Case|Profile|Measured successes /4boots|Exhausted /4boots|Overlap boots|ON/OFF p50 /2pairs|ON/OFF throughput|Phase max/min|
|---|---|---|---|---|---|---|---|
|0|dev|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[0.9956, 1.0189]|[1.0122, 0.9691]|1.032|
|0|prod|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[1.0038, 0.9692]|[1.0475, 1.0541]|1.106|
|1|dev|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[1.0025, 1.008]|[1.0074, 1.022]|1.033|
|1|prod|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[0.9573, 1.0319]|[1.1049, 0.9834]|1.105|
|2|dev|[21, 26, 24, 26]|[11, 6, 8, 6]|4/4|[0.974, 0.9613]|[1.1171, 1.0683]|1.215|
|2|prod|[16, 19, 20, 18]|[16, 13, 12, 14]|3/4|[1.1754, 0.7903]|[1.0024, 1.1994]|1.248|
|3|dev|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[0.9876, 0.9985]|[1.0077, 1.0031]|1.018|
|3|prod|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[1.0188, 1.0211]|[0.9661, 0.9776]|1.079|
|4|dev|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[0.9871, 1.0079]|[1.0456, 0.9742]|1.046|
|4|prod|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[0.999, 1.0173]|[1.0064, 0.9866]|1.019|
|5|dev|[23, 22, 19, 28]|[9, 10, 13, 4]|4/4|[0.756, 0.6727]|[0.8318, 0.8178]|1.237|
|5|prod|[20, 17, 19, 20]|[12, 15, 13, 12]|4/4|[1.0766, 0.6579]|[0.8885, 1.4164]|1.491|
|6|dev|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[1.0097, 1.0391]|[1.0015, 0.9694]|1.037|
|6|prod|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[0.9855, 0.9889]|[1.1158, 1.0886]|1.116|
|7|dev|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[1.0069, 1.0195]|[1.0108, 0.9912]|1.029|
|7|prod|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[0.9609, 0.9632]|[1.0713, 1.057]|1.153|
|8|dev|[19, 24, 20, 26]|[13, 8, 12, 6]|4/4|[1.1053, 1.0329]|[1.1208, 0.8886]|1.164|
|8|prod|[18, 17, 18, 20]|[14, 15, 14, 12]|4/4|[1.1009, 0.8348]|[0.9265, 1.0519]|1.248|
|9|dev|[22, 22, 22, 22]|[11, 11, 11, 11]|0/4|[0.9961, 0.9681]|[1.0145, 0.9938]|1.020|
|9|prod|[22, 22, 22, 22]|[11, 11, 11, 11]|0/4|[1.0826, 0.997]|[0.9002, 1.0176]|1.111|
|10|dev|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[1.0018, 0.9731]|[0.9536, 1.0364]|1.092|
|10|prod|[32, 32, 32, 32]|[0, 0, 0, 0]|0/4|[0.9993, 1.0103]|[1.0096, 1.0059]|1.073|
|11|dev|[23, 29, 23, 20]|[9, 3, 9, 12]|4/4|[0.9877, 0.9658]|[0.905, 1.0117]|1.530|
|11|prod|[18, 17, 19, 20]|[14, 15, 13, 12]|4/4|[1.0212, 1.0299]|[0.8664, 0.9171]|1.134|

## Interpretation

Successconditional observer ratios compare different selected populations under contention; not isolated recorderCPU cost.
Expected Exhausted is zero useful throughput contribution, not missing silently discarded sample.
Userspace envelope overlap does not prove simultaneous admitted kernel work.
Saturation A includes nestedBprobes/rejection and feedbackrelease; A/C heterogeneous.
No p99 precision, physical/PMU/exclusiveCPU/DEVPROD equivalence or speedupclaim.

## Finite main recommendation

**pairs_per_case_profile:** 12

**total_boots:** 576

**population:** Unchanged36 total observations:4warm+32measured; saturation3+33. No stack/buffer expansion.

**order:** Alternate OFF/ON and ON/OFF,6pairs each; pair rounds across24 case/profile groups in fixed case/profile order.

**stopping:** Exactly576 planned boot slots. No replacement/retry, outcome-dependent stopping or selective omissions. Failed/incomplete boots remain in accounting and invalidate unavailable paired metrics.

**primary:** Per-boot OFF success-latency nearest-rank p50, achieved throughput and full outcome counts. Saturation A/B/C reported separately; pooled successful quantiles supplemental.

**uncertainty:** Per case/profile report all12 paired differences and log ON/OFF ratios of boot p50 and rates, their median and order-statistic interval [3rd,10th]; exponentiate loginterval for ratios. Under independent identically distributed continuous paired observations coverage=1-2*(1+12+66)/4096=0.96142578125. Conservative marginal95% interval for MEDIAN paired effect, not mean or simultaneous24groupconfidence.

**sample_size_reason:** Two pilotpairs cannot support stable uncertainty and contention ratios vary substantially. Twelve pairs enable central-rank3rd..10th interval with >=95% nominal coverage; eightpairs require full min..max for >=95%. Resource choice for bounded descriptive uncertainty, not proof of power for a chosen performance allowance.

**assumptions:** Freshboots do not guarantee independence/exchangeability under hostdrift. Publish pairindex/order tables. If drift/assumptions unsupported, mark intervals conditional/descriptive and conclusions inconclusive. No superiority/regression gate or post-hoc margin.

**warmup:** Preserve fixed warmup regime. Pilot does not demonstrate steady-state stabilization. Label finite-session measurements, not stabilized steady-state; a stabilization claim needs separately preregistered work.

**tails:** <=32successful samples perboot makes p99 the maximum. Report descriptive quantiles and inadequate-tail flag; moreboots do not justify treating requests as independent.

