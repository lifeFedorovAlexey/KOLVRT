# Isolated EL0 supervision

Document status: CURRENT
Evidence scope: experimental Phase 3.6 development fixture and bounded lifecycle implementation; acceptance review, production bootstrap trust and physical ARM64 remain pending.
Current reference: [Lifecycle rendezvous proposal](../architecture-decisions/0026-el0-supervision.md)

ABI contract: native.lifecycle/1
Publication stage: EXPERIMENTAL
ABI-FREEZE: none

<a name="isolated-el0-supervision"></a>

## Bounded service lifecycle

The supervisor is a real isolated EL0 image. It selects startup order, checks dependencies, obtains readiness through native.request/1, observes exact service completion, selects fresh instances, limits restart attempts, applies counter-clock backoff and stops admission before shutdown. EL1 enforces provenance, finite image/placement/quota grants, private process creation, native IPC bindings and actual owner/root retirement. It does not interpret dependency, readiness, restart or discovery policy. No compatibility, routing, package manager or advisor enters the native dependency closure.

The development manifest selects one worker on CPU1 and a healthy peer on CPU0; the supervisor has its own CPU0 address space. The peer must complete an authenticated readiness probe before the dependent worker starts. A manifest entry whose readiness dependency is absent does not launch. This is a finite static workload, not a production package format or persistent service implementation.

## Bootstrap and authority lifetime

Trusted native bootstrap installs exactly one Scope, bound to the supervisor's exact ProcessId and immutable linked images, target CPUs, per-instance quotas and total instance credits. Every service has four handle slots, one endpoint, one queued request and two retained request charges; the supervisor has two endpoint slots for per-instance feedback. Each service receives only SEND authority to its own feedback endpoint, whose receiver belongs to the supervisor; memory is the private raw-image space. The worker has eight maximum instantiations and the peer has one. A third single-instance grant remains unused because its EL0 manifest dependency is absent; sealing extinguishes that grant and its later launch is rejected. The native authority ceiling and EL0 restart policy are independent. No supplied image pointer, manifest, principal, quota or placement field reaches the lifecycle entry. Numeric selectors and tokens alone confer no authority.

SVC 0xb0 uses initialized registers: x0 operation, x1 grant selector, x2 exact service token and x3 opaque initial service argument, and x4 version 1. Unsupported versions fail before command publication. Operations are initial launch 1, exact completion observation 2, replacement 3, stop admission and terminate 4, and irreversible bootstrap seal 5. Authorized checkpoint replies initialize x0 through x4: status, fresh token/event, caller-local SEND handle/detail, caller-local feedback RECEIVE handle/detail and process generation. Outside checkpoint mode the entry returns denial in x0 and preserves other caller registers. This encoding is experimental, has no ABI freeze and no public support promise. Unknown operations and malformed unused fields fail explicitly.

The native entry checks the executing Task against the installed supervisor identity. A child or reused namespace cannot become supervisor by supplying an identifier. Successful launch consumes an instance credit, issues a fresh nonwrapping token and fresh sender/receiver bindings. Replacement requires the exact current token and actual old completion; it preserves the granted image, placement and quotas. Old process identities, instance tokens and closed SEND handles cannot retarget the replacement. Failed publication retires the unpublished namespace and rolls back private frames; no runnable partial instance is published.

Seal destroys unused initial launch grants and prohibits repeated initial launch. Already-issued service capabilities separately retain their explicit remaining instance credits until Scope retirement. Sealing does not replenish credits; replacing the single-credit peer is rejected even while it remains healthy. Retirement disables the native entry; bootstrap cannot be reinstalled within this boot. Static linked-image and manifest binding provides test/development assurance only. Production authorization requires #38 to bind exact authorized images, manifest, grants and a current trust/freshness decision; no production trust chain, reproducibility or anti-rollback acceptance is claimed.

Completion observation returns event 0 for an admitted live instance, 1 for Exited(code), 2 for Faulted(class,address), 3 for explicit Terminated and 4 for BudgetExpired. Unknown/stale identity is an error, never fabricated live status. Forced stop is not reported as budget expiration.

## Execution and retirement

Lifecycle work runs at an explicit fixed-affinity two-CPU checkpoint. Each owner executes a bounded timer quantum, terminal transition, block or explicit lifecycle yield, restores its native root and releases execution ownership. CPU0 then acquires both completions before editing Registry or allocating/reclaiming frames. Pending copy transactions finish before namespaces return. Saved IPC wait identities, retry reasons and counters survive the barrier and are restored only to the same ProcessId; endpoint and source/mailbox ownership remain retained. A blocked process has no reclaim authorization merely because its root detached.

SGI-only entry cannot consume a checkpoint quantum before the first EL0 instruction; its deferred work drains while the armed timer still supplies progress. A scheduling cursor continues only for its matching live process generation. Death/stop clears blocked ownership on the exact process, closes admission and executes ASID retirement on the owning CPU. Old endpoint requests, terminal results and wake acknowledgements retain independent ownership until they drain. There is no elapsed grace-period reclamation.

This bounded checkpoint mechanism pauses healthy peers during lifecycle changes; it adds neither migration nor independent admission while a CPU executes user code. Death observation uses exact lifecycle queries; generic wait-any and autonomous persistent orchestration are excluded. The original continuous IPC path remains separately scoped and tested. Readiness, effects and failure policy are not inferred from a timeout or a stopped CPU.

## Readiness, restart and shutdown policy

The EL0 policy submits an exact endpoint readiness probe with an absolute 1/8-second counter deadline and recognizes READY only after a completed initialized response containing the expected marker. Only the service's bound receiver and accepted service token can commit/reply; a requester SEND handle cannot fabricate that response. An expired probe is not READY. These QEMU fixture durations are explicit policy parameters, not real-time guarantees.

A committed crashing service reports effect-unknown and an exact fault observation. The supervisor starts a fresh instance and checks rejection of its previous token and SEND handle. A silent service times out before commitment and is explicitly stopped. A crash storm receives three EL0-selected replacement attempts separated by 1/128-second counter-clock backoff; the supervisor then stops retrying despite remaining native authority credits. The healthy peer completes native probes between worker failures.

Under accepted load, the service sends a commit acknowledgement over its exact-instance feedback endpoint only after COMMIT succeeds. The supervisor validates and replies to that acknowledgement before cancellation; elapsed scheduler quanta cannot establish commitment. The supervisor observes cancellation after commitment as effect-unknown, stops admission, requests owner-local termination and waits for its exact terminal event. It never promises rollback or replays unknown effects. Final teardown closes every endpoint, finishes pending copies, drains source/mailbox ownership, retires namespaces/domains/ASIDs and releases private frames. The external ten-second test watchdog diagnoses a failed workload; it is not successful shutdown evidence or production policy. Unexpected supervisor failure, watchdog expiry or failed quiescence halts/quarantines this development fixture without reclaiming live resources; a deadline never authorizes freeing reachable state.

## Verification and next gate

The later routing run 37389265393 failed in stripped PROD after initial EL0 validation while all kernel shards passed. Twenty-four exact-ELF replays did not reproduce the panic. Minimal failure identity now remains visible without machine-events/diagnostics: static process/IPC check names and panic source file/line, with no private data or addresses. A subsequent PROD boot latched CPU1 FAILED during the IPC deadline workload; bounded static source file/line is now retained before FAILED release-publication and read by the primary after acquire, without CPU1 UART or private values. This improves diagnosis; it does not establish a fix of that intermittent failure. Current-source verification remains STALE until the new stand run.

The retained [exact-source execution receipt](../../research/results/supervision-phase36.json) records 125 DEV and 125 PROD checks, both non-test boots and 136 rejected control runs. Its ten supervisor-specific runs are included in the 56 IPC/supervision controls. The latest source set verifies dependency denial, unpublished quota rollback and a commitment-aware shutdown handshake as well as actual Terminated distinct from BudgetExpired; earlier snapshots remain immutable. Technical QEMU verification does not complete proposed architecture or human EN/RU acceptance.

The real images check eight scenario groups: ordered normal startup; missing dependency and forged grant selector; irreversible seal, unused launch-grant extinction, instance-credit exhaustion, unsupported-version and repeated launch denial; forged readiness denial; crash, fresh restart and stale binding denial; startup timeout; bounded restart storm and healthy peer progress; shutdown under committed load. A bitmap is returned by EL0 control flow and checked together with actual process completion, released CPU owners, zero live domains/processes and restored physical page count. It does not replace source review.

Focused mutations remove supervisor provenance, stale service-token rejection retained IPC wait identity or commitment before the load acknowledgement. The false commit acknowledgement control must produce exact CommitNotProven in both profiles. Normal worker launch and the missing-dependency launch request use the same EL0 manifest policy routine; it requires every dependency bit, rejects the absent dependency before invoking native launch and initializes the denial result. Bypassing that EL0 guard must produce exact DependencyNotRejected in both profiles. The ASID mutation checks mandatory invalidation before its dependent same-VA observation, so a stale translation cannot mask the expected witness. Each must produce its named exact rejection in both DEV and PROD. Full existing kernel checks and controls remain mandatory. [Runner](../../crates/xtask/src/main.rs) supports `cargo xtask test --positive-only [--prod]` for iteration; that command does not run or replace the full matrix. Exact-source receipts are recorded after the full matrix. Semantic and complete EN/RU human review are required before issue #27 closes. #28 owns persistent service integration; #38 owns production bootstrap trust.

Sources: [lifecycle mechanism](../../crates/kernel/src/supervision.rs), [EL0 images](../../crates/kernel/src/supervision_workload.S), [bootstrap fixture](../../crates/kernel/src/supervision_workload.rs), [process ownership](../../crates/kernel/src/process.rs), [checkpoint](../../crates/kernel/src/scheduler/mod.rs).

[Russian translation](../../translations/ru/docs/kernel/supervision.md)

## CI integration source boundary

The branch integrates reviewed main `096977f9a434398130b6d18ddbd6cbba20f2116f`, including the shared serial/four-shard CI task inventory. All ten supervisor control runs remain mandatory, bringing the plan to 140 tasks. The matrix runner recognizes exact supervision: failure events rather than any panic. Earlier execution receipts retain their original source scopes. The latest source-matching stand receipt records a complete Cortex-A57 matrix and eight successful 16-bit QEMU max ASID pairs. The earlier timer_rearm failure is retained and was never counted as a revocation witness. The current bounded run passed after the compiler-ordering correction; the intermittent timeout cause is not conclusively established, and general race-freedom is not claimed. Human architecture/EN-RU acceptance remains open.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.supervision",
  "kind": "subsystem-contract",
  "summary": "Isolated EL0 supervision with bounded exact-authority lifecycle rendezvous.",
  "units": [
    {
      "id": "kolvrt.services.supervision",
      "anchor": "isolated-el0-supervision",
      "kind": "feature",
      "summary": "EL0 readiness, finite restart and shutdown over privileged lifecycle mechanisms.",
      "tags": ["supervisor", "service", "lifecycle", "bootstrap", "el0"],
      "depends_on": [
        "kolvrt.ipc.transport",
        "kolvrt.process.lifecycle",
        "kolvrt.security.domains",
        "law.013",
        "law.025",
        "law.043",
        "adr.0026"
      ],
      "feature": {
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "Real isolated EL0 supervisor and static service images; exact-authority lifecycle rendezvous, ordered readiness, fresh replacement, finite backoff/restart policy and under-load shutdown.",
        "sources": [
          "crates/kernel-core/src/process.rs",
          "crates/kernel-core/tests/process_protocol.rs",
          "crates/kernel/Cargo.toml",
          "crates/kernel/src/main.rs",
          "crates/kernel/src/process.rs",
          "crates/kernel/src/scheduler/mod.rs",
          "crates/kernel/src/scheduler/task.rs",
          "crates/kernel/src/supervision.rs",
          "crates/kernel/src/supervision_workload.S",
          "crates/kernel/src/supervision_workload.rs",
          "crates/kernel/src/tests.rs",
          "crates/xtask/src/main.rs",
          "crates/xtask/src/matrix.rs",
          "crates/xtask/src/output.rs",
          "crates/xtask/src/timing.rs"
        ],
        "acceptance": [],
        "issues": [27],
        "adrs": ["adr.0026"],
        "limitations": [
          "Development static manifest/image assurance only; production trust remains #38.",
          "Fixed affinity and two-CPU lifecycle barrier; generic wait-any, migration, independent live admission and persistent services are excluded.",
          "Semantic architecture and complete EN/RU human review remain pending; QEMU is not physical ARM64 acceptance."
        ],
        "next_gate": "Complete architecture/code semantic and full EN/RU human review before bounded Phase 3.6 acceptance; #28 owns persistence and #38 production bootstrap trust.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "STALE",
            "reason": "Routing stand run 37389265393 failed in stripped PROD boot after initial EL0 validation; no panic identity was retained. Stripped failures now retain source location and process/IPC check name without data/address disclosure. Full current-source verification is pending; historical complete receipts remain immutable.",
            "receipt": "research/results/supervision-phase36.json",
            "receipt_sha256": "e23ec0cab4f9ed442cb9136ced9f0a435c903ef7ad91f2cbebc9623f3c97917c",
            "scope": "Real isolated EL0 supervisor and static worker/peer images on two fixed-affinity QEMU CPUs; ordered readiness, unused grant extinction, finite credits, version rejection, fresh restart, timeout, storm, truthful Terminated/effect-unknown and complete ownership/resource/source drainage. Not physical ARM64 or production trust."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical ARM64 receipt."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Implement isolated EL0 policy with a separately derived lifecycle barrier; acceptance review remains pending.",
            "acceptance": []
          }
        ]
      }
    }
  ]
}
```
