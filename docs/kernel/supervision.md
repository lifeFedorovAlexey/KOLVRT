# Isolated EL0 supervision

Document status: CURRENT
Evidence scope: accepted bounded Phase 3.6 after #126, current-source functional QEMU verification and Codex EN/RU semantic review; readiness and performance acceptance are separate.
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

Workload expiry and completion publication have distinct deadlines. Checkpoint has no workload deadline: its independent two-second coordination interval starts before local execution, and absence of workload expiry never removes the publication bound. Dispatch with a workload deadline retains strict BudgetExpired semantics and adds only the fixed coordination interval after that absolute deadline, never a second workload-duration join. Missing publication halts with CompletionPublicationTimeout; Registry ownership, namespaces and frames stay retained until both acquired completions and execution/root quiescence are confirmed. Pending-copy continuation drainage has its own existing two-second coordination bound and also halts without reclaim on failure. These counter-clock bounds assume timer/CPU progress; they are not physical real-time guarantees.

The deterministic checkpoint publication control holds CPU1 after native-root and execution-owner quiescence, before completion publication, and invokes Registry::checkpoint() with no workload deadline. Its exact witnesses are CheckpointPublicationHeld (quiescent=true) and CompletionPublicationTimeout. Before the fix this control reached only the external 30-second QEMU timeout. Positive kernel runs force pending publication on normal checkpoints and then release it; the separate zero-workload-deadline dispatch control remains intact. The WFI masked-condition race fix and its deterministic test remain intact.

Counter reads distinguish ticks_relaxed() for scheduler/coordination deadline polling from ticks_ordered() for observation/context ordering. Existing ticks() callers keep ordered semantics for measurements and accounting; trap-entry ISB instructions remain. Performance results are limited to measured QEMU scheduler workloads, with failures and variability retained; no physical-performance or absence-of-regression claim follows.

Lifecycle work runs at an explicit fixed-affinity two-CPU checkpoint. Each owner executes a bounded timer quantum, terminal transition, block or explicit lifecycle yield, restores its native root and releases execution ownership. CPU0 then acquires both completions before editing Registry or allocating/reclaiming frames. Pending copy transactions finish before namespaces return. Saved IPC wait identities, retry reasons and counters survive the barrier and are restored only to the same ProcessId; endpoint and source/mailbox ownership remain retained. A blocked process has no reclaim authorization merely because its root detached.

SGI-only entry cannot consume a checkpoint quantum before the first EL0 instruction; its deferred work drains while the armed timer still supplies progress. A scheduling cursor continues only for its matching live process generation. Death/stop clears blocked ownership on the exact process, closes admission and executes ASID retirement on the owning CPU. Old endpoint requests, terminal results and wake acknowledgements retain independent ownership until they drain. There is no elapsed grace-period reclamation.

This bounded checkpoint mechanism pauses healthy peers during lifecycle changes; it adds neither migration nor independent admission while a CPU executes user code. Death observation uses exact lifecycle queries; generic wait-any and autonomous persistent orchestration are excluded. The original continuous IPC path remains separately scoped and tested. Readiness, effects and failure policy are not inferred from a timeout or a stopped CPU.

## Readiness, restart and shutdown policy

The EL0 policy submits an exact endpoint readiness probe with an absolute 1/8-second counter deadline and recognizes READY only after a completed initialized response containing the expected marker. Only the service's bound receiver and accepted service token can commit/reply; a requester SEND handle cannot fabricate that response. An expired probe is not READY. These QEMU fixture durations are explicit policy parameters, not real-time guarantees.

A committed crashing service reports effect-unknown and an exact fault observation. The supervisor starts a fresh instance and checks rejection of its previous token and SEND handle. A silent service times out before commitment and is explicitly stopped. A crash storm receives three EL0-selected replacement attempts separated by 1/128-second counter-clock backoff; the supervisor then stops retrying despite remaining native authority credits. The healthy peer completes native probes before and after the bounded crash storm.

Under accepted load, the service sends a commit acknowledgement over its exact-instance feedback endpoint only after COMMIT succeeds. The supervisor validates and replies to that acknowledgement before cancellation; elapsed scheduler quanta cannot establish commitment. The supervisor observes cancellation after commitment as effect-unknown, stops admission, requests owner-local termination and waits for its exact terminal event. It never promises rollback or replays unknown effects. Final teardown closes every endpoint, finishes pending copies, drains source/mailbox ownership, retires namespaces/domains/ASIDs and releases private frames. The external ten-second test watchdog diagnoses a failed workload; it is not successful shutdown evidence or production policy. Unexpected supervisor failure, watchdog expiry or failed quiescence halts/quarantines this development fixture without reclaiming live resources; a deadline never authorizes freeing reachable state.

## Verification and next gate

The [new acceptance review](../architecture/supervision-phase36-acceptance-review.md) is against merged main ddd526cc5a522d97029d0324e6e619f5fd6ec6e5 after #126. Codex re-reviewed complete architecture/code and EN/RU semantic requirements, including mandatory publication bounds for checkpoint(None), the separate copy-drain bound and relaxed/ordered counter reads. Functional implementation is BOUNDED_IMPLEMENTED. Readiness remains NOT_READY without introducing a new numeric performance gate or denying the functional foundation for #28.

The [fresh current-source receipt](../../research/results/supervision-phase36-main126.json) independently checks CI run 37439023443 on final #126 head cceb17e74a7a249d76d0f10905de23205f69b1dd: every source digest matches, actual ELF/result hashes and all 142 tasks were checked — 125 checks per profile, both ordinary boots and 138 controls. All ten supervision controls remain. New DEV/PROD publication controls require both CheckpointPublicationHeld with quiescent=true and CompletionPublicationTimeout; an unrelated panic is not counted.

The [old receipt](../../research/results/supervision-phase36-current.json) and [pre-#126 proposal](../architecture/supervision-phase36-pre126-review.md) retain only their historical 9cf87bc/692b024 scope. They do not verify current sources.

[Original performance observations](../../research/results/checkpoint-publication-bound.json) show median +1.57% with tagged ASIDs and +6.64% with ASID-zero. Sequential before/after runs under uncontrolled host load combine lifecycle correction and counter refinement; ISB attribution and absence of regression are unproven. The strict original PROD IPC benchmark also failed blocked_requester_wait with block_delta=0. That failure is retained and the control is not weakened. Performance acceptance is not claimed; this is a separate open scope, not a new universal percentage threshold.

Eight real EL0 scenario groups check ordered startup, dependency/authority denial, finite grants/seal, forged readiness, crash/fresh restart/stale binding, startup timeout, bounded storm and committed-load shutdown. Bitmap 255 is supplemented by actual completion, released CPU owners, zero live processes/domains and restored physical pages. Host tests do not replace EL0; QEMU is not physical ARM. Earlier timer/stripped-PROD failures remain historical; universal race-freedom is not claimed. #28 owns persistent service integration and #38 production bootstrap trust.

Sources: [lifecycle mechanism](../../crates/kernel/src/supervision.rs), [EL0 images](../../crates/kernel/src/supervision_workload.S), [bootstrap fixture](../../crates/kernel/src/supervision_workload.rs), [process ownership](../../crates/kernel/src/process.rs), [checkpoint](../../crates/kernel/src/scheduler/mod.rs).

[Russian translation](../../translations/ru/docs/kernel/supervision.md)

## CI integration source boundary

Current main after #126 contains 142 tasks and 138 controls from one shared serial/four-shard inventory. Publication failure fabricates no success: resources remain retained until acquired quiescence. Fresh verification is scoped to this exact source set; historical inventories are not rewritten. Performance and readiness remain separate from functional acceptance.

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
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "Real isolated EL0 supervisor and static service images; exact-authority lifecycle rendezvous with mandatory bounded completion publication independent of workload expiry, bounded copy drainage, ordered readiness, fresh replacement, finite backoff/restart policy and under-load shutdown.",
        "sources": [
          "crates/kernel-core/src/process.rs",
          "crates/kernel-core/tests/process_protocol.rs",
          "crates/kernel/Cargo.toml",
          "crates/kernel/src/main.rs",
          "crates/kernel/src/process.rs",
          "crates/kernel/src/arch/aarch64/mod.rs",
          "crates/kernel/src/smp.rs",
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
        "acceptance": ["research/results/supervision-phase36-main126.json"],
        "issues": [27],
        "adrs": ["adr.0026"],
        "limitations": [
          "Development static manifest/image assurance only; production trust remains #38.",
          "Fixed affinity and two-CPU lifecycle barrier; generic wait-any, migration, independent live admission and persistent services are excluded.",
          "Codex semantic/EN-RU review completed after #126; performance attribution/admissibility and physical ARM/production acceptance remain open."
        ],
        "next_gate": "Phase 3.7/#28 persistent ELF integration over the accepted bounded functional foundation; performance readiness and production trust remain separate review scopes.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "STALE",
            "reason": "Phase 3.7 immutable ELF grants/client SEND binding or workspace inputs changed; matching-source verification must be refreshed. Historical receipts remain immutable.",
            "receipt": "research/results/supervision-phase36-main126.json",
            "receipt_sha256": "52a48b8246f516b7d393c1cecb6a7a0a58bac5a9af31822cda15390747c2907f",
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
          },
          {
            "from": "EXPERIMENTAL",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "Post-#126 code/architecture and full EN/RU semantic review with independent current-source 142-task functional verification; readiness/performance acceptance remain separate.",
            "acceptance": ["research/results/supervision-phase36-main126.json"]
          }
        ]
      }
    }
  ]
}
```
