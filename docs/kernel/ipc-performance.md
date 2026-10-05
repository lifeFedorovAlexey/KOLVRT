# Phase 3.5 IPC performance passport

<a name="measurement-method"></a>

Document status: CURRENT
Evidence scope: reproducible QEMU regression baseline for the experimental bounded IPC implementation; not a hardware performance claim.
Current reference: [Native IPC contract](ipc.md)

## Measurement method

Run `cargo xtask ipc-bench` to build and execute both DEV and PROD AArch64 kernels under QEMU and replace the raw [baseline artifact](../../research/measurements/ipc-phase35-baseline.json). The runner records the exact LF-normalized source inventory and hashes used for each run, build metadata, machine events and every sample. It rejects an incomplete matrix or source changes during a run.

Each profile has 144 groups: four placements (CPU 0→0, 1→1, 0→1 and 1→0), four payload sizes (0, 8, 64 and 256 bytes), and nine scopes. The five operation scopes are submit, receive, reply, terminal collect and full round trip. Four additional scopes measure hot-ready receive, blocked receiver wake, blocked requester wait and full-queue rejection. Every group has four warmups and sixteen measured observations. Units are architectural counter ticks at the recorded frequency. Raw observations carry median, p95, p99, mean, population standard deviation, context-switch count and actor block counts.

The EL0 probes time each selected API operation or complete submit/wait/collect round trip using the native counter call. Values include probe entry/return, argument setup and result checks, plus actual blocking and wake scheduling when the actor blocks. There is no synthetic overhead subtraction. DEV diagnostic instrumentation and PROD optimized builds are separate populations and must not be compared as equivalent code paths.

The controlled path fixture verifies every warmup and measured operation with the owner-local condition-block counter observed in the existing clock probe. Hot receive requires a zero delta. A reverse IPC handshake holds the service until the main request is queued, establishing readiness independently of scheduling speed. Blocked receive and blocked requester wait each require a delta of exactly one. Finite computation over five milliseconds of boot-local time gives the peer an opportunity to block; the counter check rejects a run if that opportunity did not produce the required path. Full-queue rejection holds the service behind the handshake, fills all four queue slots, and requires explicit exhaustion and zero condition blocks for the extra submission. Failed admission is followed by successful drainage and zero final domain charges and frame leakage.

The clock's block-counter observation exists only in the measurement build. It neither grants authority nor changes IPC transitions. Generic receive remains a mixed-path population. Cross-CPU submit includes retained wake publication and notification; receive and round-trip envelopes include target resumption. These observations do not isolate the cost of the SGI handler alone.

## Ownership and copy baseline

The safe path currently copies four times: client user memory to initialized kernel request storage; kernel request storage to service user memory; service user response to initialized kernel result storage; and kernel result storage to client user memory. No zero-copy path is measured or claimed.

## Limits and follow-up

This is a QEMU regression baseline, not silicon latency or a speed target. It does not isolate individual instruction/exception overhead, model physical cache and interconnect behavior, or establish a universal workload distribution. Controlled path labels describe the verified operations, not all control-handshake operations in the group. Hardware measurement requires a named physical platform and matching firmware/tool provenance.

Before using the artifact as current evidence, rerun `cargo xtask ipc-bench` after the final implementation edits. A retained artifact with stale source hashes remains historical data and must not be described as verifying newer source.

[Russian translation](../../translations/ru/docs/kernel/ipc-performance.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.ipc-performance",
  "kind": "benchmark-evidence",
  "summary": "QEMU performance measurement method, copy count and limits for bounded native IPC.",
  "units": [
    {
      "id": "kolvrt.ipc.performance",
      "anchor": "measurement-method",
      "kind": "contract-section",
      "summary": "Raw baseline with verified hot, blocked and full-queue paths and explicit interpretation boundaries.",
      "depends_on": ["kolvrt.ipc"]
    }
  ]
}
```
