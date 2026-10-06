# CI performance work

Document status: CURRENT
Evidence scope: experimental implementation for issue #109; a three-run historical CI wall baseline and local checks are distinct from hosted acceleration acceptance.
Current reference: [Knowledge system](../knowledge-system.md)

The new DEV/PROD checkpoint publication controls require both exact events: CPU1 publication held after quiescence and bounded CompletionPublicationTimeout. External QEMU timeout or arbitrary panic does not count; the zero-workload-deadline control remains separate.

<a name="ci-performance"></a>

## Supervisor integration and workload equivalence

The current source-bound plan contains 4 positive and 138 negative tasks, including the six supervisor controls. The shared runner requires the exact supervision rejection kind/error. The retained 130-task baseline remains historical; identical job/step topology is not evidence that the two inventories match. API-only reports leave check_inventory_id UNKNOWN and require separately reviewed equivalence for cross-inventory comparison.

## CI timing and optimization

[Issue #109](https://github.com/lifeFedorovAlexey/KOLVRT/issues/109) is in progress. The [retained baseline](../../research/measurements/ci-legacy-baseline.json) contains three successful Windows push runs with identical Git blob inventories for kernel, workspace, scripts, assets, manifests, lockfiles and the measured workflow. Documentation/evidence differ; build inputs do not. Execution spans are 2002, 1970 and 1829 seconds: median 1970 seconds (32 minutes 50 seconds). The substantially earlier 908-second run is excluded because its kernel workload differed. Runner load remains uncontrolled; these are descriptive wall observations, not paired speed inference or compiler CPU measurements.

A completed-workflow reporter retains every GitHub step outcome and wall time, exact measured SHA/run/attempt, the five most expensive steps, execution span and the declared DAG path. It checks out trusted default-branch code with read-only permissions and never executes triggering PR artifacts. The timing workflow becomes active when present on the default branch. Missing/API-failed observations stay unavailable or fail reporting. The retained kernel-130-v1 baseline remains historical. API job and step names cannot attest the source-bound task count: the supervision integration expands the current plan to 142 tasks without changing the shard topology. Automatic collection therefore leaves check_inventory_id UNKNOWN instead of treating that plan as kernel-130-v1. Automatic comparison is limited to the same source SHA and job/step graph. Cross-source/inventory comparison needs independently reviewed source-bound equivalence evidence, which the API-only collector does not supply; manually added inventory labels cannot enable it. Same-source repeat comparison still requires three distinct successful run IDs.

The shell wrapper records setup/download and command wall time. The optional xtask observer records individual Cargo build invocations and QEMU process lifetimes, including failed/timeout executions, in per-process JSONL files. Summaries retain separate nested categories; adding matrix, build and emulator totals would double-count work. Cargo invocation duration includes fingerprint/cache checks, not exclusive rustc CPU time. Exclusive compiler time and initial runner queue time remain UNKNOWN. DAG duration sums job durations along the longest declared dependency path, excluding runner queues; execution span includes waits between jobs. Cache outcomes are retained separately in each job.json.

## Cache and configuration boundaries

Cargo registry/git downloads and compiled host/AArch64 artifacts use versioned exact keys containing Windows OS, Rust 1.99.0, host/guest targets, workload/shard, Cargo.lock, manifests, crate sources, configuration, build assets and orchestration scripts. No broad restore prefix is used. Registry caching retains indexes and archives only: extracted source trees are reconstructed by Cargo from lockfile-checksummed archives. Git caching retains object databases only and runs git fsck --full; checkouts are reconstructed from pinned commits. DEV/PROD and feature/negative-control units retain Cargo's own configuration fingerprints; a fresh invocation is required for each task and its actual features/ELF hash are checked. Different jobs/shards never share a mutable build directory. A SHA-256 inventory verifies every restored compiled file before execution: additions, missing files, corruption and symlinks fail the job. Cache metadata is a consistency check within the GitHub cache trust boundary, not an authenticated artifact provenance service or a kernel TCB dependency.

Only the pinned QEMU archive is cached. Each restore verifies SHA-512, reconstructs the installation from that archive and checks the pinned binary SHA-256 and exact 10.1.0 version before recording provenance. Installed DLLs and binaries are reconstructed rather than trusted from an extracted-install cache. npm download caching preserves mandatory npm ci --ignore-scripts. The Ubuntu audit tool cache pins OS, Rust and cargo-deny 0.19.9, verifies its byte inventory before execution and checks its version; a miss builds from the locked package. No supply-chain checks are skipped.

The build-reuse experiment executes cold, warm, negative-feature, restored-positive and repeated clean builds under one target path. It compares named DEV/PROD ELF hashes and checks that the negative binary differs. It is not proof for all feature sets or cross-machine reproducibility. [sccache Rust documentation](https://github.com/mozilla/sccache/blob/main/docs/Rust.md) requires disabling incremental compilation and excludes crates invoking a system linker, including bin crates. Kernel binaries make that limitation material; sccache is not enabled without a separate measured benefit/correctness experiment. Review also follows the [Cargo build-cache contract](https://doc.rust-lang.org/cargo/reference/build-cache.html).

## Check inventory and parallel graph

| Original check                                                       | Current command/job              | Disposition                                                                                        |
| -------------------------------------------------------------------- | -------------------------------- | -------------------------------------------------------------------------------------------------- |
| Dependency policy and rejection fixtures                             | check:static                     | Preserved once.                                                                                    |
| Prettier, Taplo formatting, cargo fmt, Markdown and Taplo lint       | check:static                     | Preserved once.                                                                                    |
| Repository validation and PR-base impact/history                     | static                           | Preserved; actual PR base required.                                                                |
| Workspace Clippy, research tests, cargo test and native-state-models | check:host                       | Preserved; CI infrastructure rejection tests added.                                                |
| Exception registry subset                                            | Workspace cargo test in host     | Only the repeated identical subset command is removed; its tests remain.                           |
| Routing all-features and no-default-features tests                   | check:host                       | Both configurations preserved.                                                                     |
| DEV diagnostics/kernel-tests architecture lint                       | check:kernel-dev                 | Exact original target/features retained.                                                           |
| PROD release/no-default-features architecture lint                   | check:kernel-prod                | Exact original target/features retained.                                                           |
| Real kernel matrix and host failure propagation                      | matrix shards 0–3, then evidence | Current 4 positive and 138 negative tasks, partitioned once; historical baseline was 126 negative. |
| Real EL0 routing matrix and controls                                 | routing                          | Full specialized matrix retained.                                                                  |
| Eight paired real 16-bit ASID comparisons                            | asid                             | All sixteen boots and actual 16-bit assertions retained.                                           |

Local npm run check executes check:static then check:host. check:qemu retains all three expensive runners, while the named kernel lint commands are independently available. The CI static gate precedes parallel host, DEV lint, PROD lint, four kernel shards, routing and ASID jobs. fail-fast is disabled for shards. The evidence job independently generates the current source/task/configuration plan and rejects missing/duplicate tasks or shards, stale sources, wrong profiles/features, differing ELF hashes or QEMU arguments. Each task is marked executed. The final foundation status requires every mandatory job; failure, cancellation, omission or skip cannot become PASS.

Serial and sharded kernel execution share one task inventory and the original required failure predicates, including the parsed named IPC failure event and exact supervision-reject/error for supervision: controls. Partial shard receipts cannot substitute for full acceptance. All artifacts come from the current run and are named by exact head SHA, workload, shard and attempt. Cached paths exclude retained QEMU results; immutable historical receipts are preserved. The ASID receipt is copied into current-run target evidence only after execution, so host/lint jobs cannot publish an old checked-in ASID receipt as fresh evidence.

## Execution and remaining gates

Main pushes, PRs and manual runs use the same partitioned graph. The legacy serial job, dispatch selector and workload runner are removed; the final gate always requires the partitioned workloads. Historical timing baselines remain immutable comparison evidence. A feature-branch push does not duplicate the open PR synchronize run; local checks and manual dispatch remain available before opening a PR. New PR commits automatically cancel obsolete runs of the same workflow/PR. Different workflows cannot cancel one another; an active main run is preserved. Reports and raw evidence are retained for ninety days; checked-in baselines are permanent.

```text
npm run check
npm run check:kernel-dev
npm run check:kernel-prod
npm run check:qemu
cargo xtask matrix-plan
cargo xtask matrix-shard 0 4
node scripts/ci-build-reuse.cjs
node scripts/ci-timings.cjs OWNER/REPO RUN_ID target/ci-timings research/measurements/ci-legacy-baseline.json
```

The GitHub collector requires GITHUB_TOKEN with Actions read access. Four local shards must run sequentially in one checkout because that checkout has shared artifact paths; CI runs each shard on a separate runner. Aggregation validates all four receipts against a newly generated current plan.

Repeated ASID boots are independent counterbalanced statistical observations, not duplicate evidence consumers. Foundation, routing and ASID have different features, payloads or CPU configurations and cannot share one positive boot receipt. No retained-result replay is introduced; every observation remains executed. Further QEMU reuse requires a genuinely identical source/ELF/config/profile/feature tuple and explicit consumer semantics. Impact scheduling compares the exact kernel/build/runner source inventory against the actual base and analyzes ownership and prerequisite impact in both the accepted base and current knowledge graphs. CI/verification input changes, kernel authority, unknown files or an unavailable base require the full matrix. Only derived navigation or declared host-only units without kernel prerequisite impact may waive unchanged kernel inputs. No path == docs shortcut is used. A host-only run requires static/host success and explicit skips of all waived kernel jobs; an unexpected failed job is never hidden.

Warm PR feedback below ten minutes and cold CI below fifteen minutes remain acceptance targets until measured on hosted runners. Required remaining gates are cold/warm hosted runs, cache hit/miss and corruption review, comparison against the published baseline, meaningful attribution limits, and complete semantic/EN-RU review. The issue remains open.

[Russian translation](../../translations/ru/docs/ci/performance.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.ci.performance",
  "kind": "subsystem-contract",
  "summary": "CI timing and staged optimization without weakening kernel evidence.",
  "depends_on": ["doc.kolvrt.docs.knowledge", "adr.0006"],
  "units": [
    {
      "id": "kolvrt.ci.performance",
      "anchor": "ci-performance",
      "kind": "feature",
      "summary": "Measured legacy baseline, integrity-checked caches, parallel checks and complete four-shard QEMU matrix aggregation.",
      "tags": ["ci", "timing", "cache", "tooling"],
      "feature": {
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "Experimental timing, versioned integrity-checked caches, split local checks, four-way kernel matrix partitioning and exact-source aggregate validation; CI performance acceptance remains pending.",
        "sources": [
          ".github/workflows/kernel.yml",
          ".github/workflows/dependencies.yml",
          ".github/workflows/ci-timings.yml",
          ".github/workflows/ci-windows-workload.yml",
          "package.json",
          "scripts/setup-qemu.ps1",
          "scripts/run-timed.ps1",
          "scripts/ci-workload.ps1",
          "scripts/ci-timings.cjs",
          "scripts/ci-observations.cjs",
          "scripts/cache-integrity.cjs",
          "scripts/ci-gate.cjs",
          "scripts/ci-evidence.cjs",
          "scripts/ci-build-reuse.cjs",
          "scripts/tests/ci-timings.test.cjs",
          "scripts/tests/ci-infrastructure.test.cjs",
          "crates/xtask/src/main.rs",
          "crates/xtask/src/routing_demo.rs",
          "crates/xtask/src/matrix.rs",
          "crates/xtask/src/timing.rs",
          "scripts/ci-impact.cjs"
        ],
        "acceptance": [
          "research/measurements/ci-legacy-baseline.json",
          "research/measurements/ci-build-reuse.json"
        ],
        "issues": [109],
        "adrs": ["adr.0006"],
        "limitations": [
          "Hosted cold/warm acceleration, cache service behavior and semantic/EN-RU review remain pending. No compiler-exclusive CPU attribution or retained QEMU observation replay is claimed; conservative impact scheduling cannot prove semantic completeness."
        ],
        "next_gate": "Run the optimized graph on GitHub, compare cold/warm feedback with the three-run legacy baseline, review cache integrity and complete semantic/EN-RU review before issue closure.",
        "verification": [
          {
            "environment": "github-actions",
            "state": "UNKNOWN",
            "reason": "The reporting workflow has not run on GitHub; local fixture tests cannot establish runner measurements."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Begin issue #109 with an observational B0 reporting implementation; preserve every existing CI check.",
            "acceptance": []
          }
        ]
      }
    }
  ]
}
```
