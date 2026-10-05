# Time and measurements

The physical architectural counter supplies monotonic boot-epoch time. CNTFRQ supplies frequency; durations use Duration, not untyped nanosecond literals. Safe u128 conversion rounds deadlines up, rejects zero frequency and overflow; deadline addition is checked. No wall clock, scheduler policy or cycle-counter claim is introduced.

Timer fixture polling checks the published delivery counter before deciding expiration
and takes a final observation at the deadline boundary. Expiration without observed
delivery remains a failure; the 500 ms polling budget is unchanged. This avoids losing
a delivery when the emulator resumes after host descheduling. It does not establish
a physical interrupt-latency guarantee.

## Measurement contract

Kernel tests collect 64 uncontended-lock samples after eight warm-up iterations. Reports include raw unsorted samples, counter frequency, units, iterations and nearest-rank median, p95 and p99. Timer delivery has a deadline; IRQ counters use atomic publication.

TCG timer ticks measure emulator execution and have limited resolution. Zero deltas are retained. These samples verify the measurement path, not superiority over another OS or a fastest-algorithm claim. Workload comparisons need controlled alternatives, repeated runs and hardware evidence. [Review gate](../architecture/implementation-review.md) makes that distinction mandatory.
