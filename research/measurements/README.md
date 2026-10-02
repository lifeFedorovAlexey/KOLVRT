# Reproducible measurement history

Measurements are part of the kernel test run. Every successful cargo xtask test validates the real samples and writes target/kernel/measurement.json. An explicit record option additionally creates a new history file in research/measurements/runs; it refuses to overwrite an existing record.

## Commands

```text
cargo xtask test --record cpu0-baseline
cargo xtask compare research/measurements/runs/BASELINE.json research/measurements/runs/CANDIDATE.json
```

Use the actual recorded paths printed by the test command. Recording reruns the complete DEV/PROD matrix and four failure-propagation controls. Source changes during execution reject the record. Git revision, dirty state, normalized source hashes, ELF hashes/features/sizes, exact emulator configuration and raw samples identify the tested implementation.

## Tests and comparisons

The allocator test cursor_preserves_ownership_and_linear_scan_work exercises the actual pool with and without its cursor optimization. Both variants must allocate the same owned units and free them correctly. The cursor variant must inspect a linear number of units; the reference rescans the occupied prefix and exposes quadratic work. Exhaustion must inspect no additional units. Inspection counters exist only under cfg(test), so native DEV and PROD pay no runtime cost. This proves that particular optimization's work contract, not supremacy over free lists or buddy allocators.

Kernel lock measurements include scope, units, frequency, warm-up, iterations, raw samples and median/p95/p99. The host recomputes quantiles from samples and rejects inconsistent records. Compare requires matching emulator arguments/version, CPU scope, accelerator, compiler, target, features, workload, units, frequency, warm-up and iterations. It reports observed differences; it does not automatically accept a method or fail on an arbitrary timing threshold.

## Acceptance policy

Keep previous runs when changing an implementation. Collect repeated paired runs under comparable conditions; inspect reliability, exhaustion, concurrency and tail behavior as well as speed. Link a selected replacement to an ADR and its negative tests through the [method review](../../docs/architecture/implementation-review.md). A faster median alone does not justify adoption.

Current lock measurements are TCG timer observations with limited resolution, including zero deltas. They do not establish hardware throughput or that all selected kernel algorithms are fastest. Numerical regression budgets require a stable workload and reviewed justification; none is invented here. Formatting may change JSON whitespace, but never rewrite sample values to make a comparison pass.
