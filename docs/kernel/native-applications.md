# Native ELF applications and persistent service

Document status: CURRENT
Evidence scope: experimental Phase 3.7 implementation; prior copied-service/supervisor and source-mutant runs do not establish production E2E. Full acceptance is incomplete.
Current reference: [Accepted supervision foundation](supervision.md)

<a name="native-elf-applications"></a>

## Native application slice

Ordinary counter-service, counter-client and native-supervisor are separate no_std/no_main ELF artifacts under apps/native-apps. The kernel receives full original ELF bytes and uses its real loader; applications and the host harness verify the counter protocol and expected values. GET and checked ADD(delta) use private userspace state; RESET is absent. ADD(5), ADD(7), GET produce 5, 12, 12. The ordinary service contains no injected crash, test request counter or reply corruption.

Finite client SEND binding uses the existing supervision checkpoint and exact instance tokens. The kernel supplies generic process/ELF/IPC/authority/lifetime mechanisms. Ordinary implementation must contain no application callbacks or test interventions. Hardware, disk durability and stable ABI are not claimed.

## Test classification and single implementation

| Previous check                                 | Level and production code exercised                                  | Separate ELF and disposition                                                                |
| ---------------------------------------------- | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| native-userspace tests                         | UNIT: Counter::apply, CounterRequest::decode, frame and reply codecs | None; apps/native-runtime/tests/counter.rs imports actual methods                           |
| selftest-process                               | SYSTEM: kernel ELF/process/EXIT/fault containment                    | External lifecycle-peer actually faults while the real service continues replying           |
| selftest-ipc                                   | INTEGRATION/SYSTEM: actual native IPC through the public SDK         | External ABI client is permitted; production service and supervisor remain actual artifacts |
| selftest-service                               | SYSTEM: actual kernel/service and recover_service; test root         | Duplicate ELF removed; stop/replacement/stale/shutdown exercised through lifecycle ABI      |
| selftest-elf                                   | E2E: actual loader and application ELF                               | Uses ordinary counter-client                                                                |
| smoke-client                                   | E2E: ordinary applications and IPC                                   | Duplicate ELF removed; app-smoke selects counter-client                                     |
| selftest-counter-service / selftest-supervisor | Production implementation copies                                     | Removed; not test instruments                                                               |
| isolated_build / mutations.json                | Kernel source-copy mutation                                          | Removed; prior controls do not establish production execution                               |

Only external ABI/fault actors remain under tests/native-apps. They contain test inputs, public-call sequences and assertions; they implement neither the counter service, ordinary client nor supervisor. The fault actor contains its own illegal instruction. The ordinary SDK contains no test-only fault helper. Kernel/application source copies are not created.

The production supervisor now checks the actual lifecycle terminal outcome before replacement, retains accepted failure backoff and verifies fresh-instance readiness with a real GET(0). After a service RPC failure, the ordinary client requests a fresh binding, requires a new identity and initial value 0, rejects the old SEND and repeats business operations only in that new instance. An effect-unknown ADD is never retried against the same instance. UNIT tests import needs_replacement, fresh_binding and SessionPolicy::decode; the integration actor imports recover_service rather than copying it.

The current external ELF inventory is explicit: selftest-abi-client calls the real native-userspace IPC API against the production service/supervisor; selftest-lifecycle-client calls public lifecycle/IPC APIs and imports native-apps::supervision::recover_service; selftest-lifecycle-peer exercises binding, stale SEND and fault containment. Each is a test client with no production counterpart. There is no copied production supervisor. The lifecycle SYSTEM scenario uses selftest-lifecycle-client as its root instead of native-supervisor, so production_supervisor_tested=false: that scenario does not execute the production supervisor. The real counter-service is used. The unused selftest-fault-client and dead native-*-negative features were removed; lifecycle-peer supplies the executed external fault scenario. Host UNIT tests in apps/native-apps/tests/supervision.rs import needs_replacement and fresh_binding without an ELF.

CI on exact 7d64fac stopped at user-context (shard 0), retirement (shard 1), shootdown (shard 2), and remote-tlbi (shard 3). Retirement compilation was repaired in 71753ae. That commit then failed static because the graph had a stale final manifest digest; c77f6e8 regenerated the exact graph and passed CI static. The four corruption-based controls are now migrated as mapped below. Other legacy controls remain subject to the complete matrix; the named passes do not establish a green foundation or full Phase 3.7 acceptance.

| Migrated control | Actual inputs / observations                                                                                              | Evidence limit                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| user-context     | valid_user_context rejects EL1 modes, AArch32 and masked IRQ; ordinary EL0 context preservation and timer switches        | No saved-frame corruption; validator + real round-trip checks        |
| user-root        | actual EL0 reads of kernel/foreign memory fault; private-memory isolation and clean reclaim                               | Real forbidden accesses; no root-table sabotage                      |
| shootdown        | live secondary reader keeps retirement pending; frame is not reused before ACK; release/reuse follows ACK                 | Observed retirement ordering; no forced missing ACK or timeout claim |
| user-retirement  | Registry::reclaim rejects a live process; state and retained frames remain unchanged; eventual reclaim is checked         | Real public ownership API; no extraction of private frame            |
| remote-tlbi      | acknowledged rejects a future generation; actual TLBI observer precedes ACK; access to retired mapping translation-faults | Instrumented test build; no skipped-TLBI mutation claim              |

These controls use ordinary kernel-tests builds and require every mapped test exactly once with status pass. Missing, duplicate, failed and unrelated observations are rejected. The task inventory is preserved. The four obsolete corruption features are removed. The ordinary suite now has 147 checks per profile. REMOTE_TLBI_COMPLETED is observation-only instrumentation in kernel-tests; this evidence is not byte-identical production-binary execution. The new production crash/recovery/shutdown scenario is separate from these ordinary kernel checks.

The next obsolete controls are classified explicitly. completion-publication-state-inputs imports the production Ownership state machine and rejects completion while borrowed or not quiescent, rejects inspection before Done and verifies phase/generation retention. This UNIT evidence does not test a withheld remote CPU publication or its liveness/failure handling. process-exit calls the single production Table::complete with a duplicate completion and verifies the terminal record, state and live-slot count remain unchanged; the test-only Registry::repeat_completion wrapper is removed. process-rollback uses real zero-memory-budget creation failures and checks resource rollback. asid-reuse-invariants observes actual same-VA isolation, invalidation counters and pool exhaustion; it is not a skipped-invalidation negative control.

shootdown-invariants and remote-tlbi-invariants are positive invariant coverage; their previous control flags are CLI aliases. They do not establish missing-ACK or skipped-TLBI detection. The plan and archived counts distinguish ordinary, negative-input, invariant and remaining legacy-failure tasks. Withheld publication/missing ACK/skipped invalidation SYSTEM coverage remain outside this coverage; the separate production crash/recovery/shutdown E2E does not establish omitted-TLBI or missing-ACK detection.

The process transition family now uses explicit production-method negative inputs: Table::reclaim without quiescence preserves the completed record/live-slot count (UNIT); Registry::start for an already admitted process preserves state/frames; stale IDs fail start/data/receipt validation after actual slot reuse; Registry::reclaim for a live process preserves state/frames. No forced scheduler unlink is claimed. Obsolete process-contract features and early panic branches are removed.

The scheduler family now separates three levels of evidence. Registry::create rejects AArch32, privileged-mode and masked-user-IRQ input contexts before allocation; process count and available pages remain unchanged. Ownership UNIT tests call the one production protocol with foreign CPU, unmasked access, borrowed/reentrant access, stale generation, duplicate start, live reset and premature inspection; phase/generation remain retained. Adapter UNIT tests construct actual Local/Task/State fixture values and invoke the real methods with a lock held, a lock inside an ownership callback, stale task generation or an already running peer. These four fatal assertions execute in DEV and PROD; the host requires a fresh guest panic at the actual unique production assertion site, because PROD omits the diagnostic message. No implementation body is copied or mutated. The IRQ/SIMD compatibility flag now selects the real interrupt round-trip invariant; it does not establish skipped-restore detection. These tests do not establish complete hardware adapter fault coverage or current Phase 3.7 acceptance.

Exact d33ce92 CI passed static, native, host, kernel DEV/PROD, routing and ASID jobs, but all matrix shards failed: scheduler-user-irq (0), scheduler-context (1), scheduler-inner-lock (2), scheduler-aarch32 (3). The foundation aggregate failed and evidence was skipped. The scheduler migration addresses these obsolete controls and adjacent family controls; a full current matrix is still required.

The remaining no-op user-copy, handle, domain/capability and IPC mutation selectors are now removed. Controls use actual production APIs and existing real EL0 negative inputs/invariants, with new Namespace retirement, Mailbox invalid generation and Endpoint invalid consumer/token/duplicate-terminal UNIT inputs. Five fatal IPC adapter inputs passed through the matrix runner in DEV/PROD (ten executions): storage inside scheduler ownership, publication into READY, wrong-process wake, blocked unlink and dropping a closed endpoint with a retained reference. Public duplicate terminal rejection preserves outcome/accounting; no private second charge-release mutation is claimed. The current plan distinguishes negative inputs, invariants, mixed coverage and still-executed legacy failure/oracle checks. Measurement archival now retains the current complete matrix execution and coverage classes instead of requiring every IPC task to be a failure. ipc-controls selects every registered IPC task, including migrated positive/invariant checks.

Exact 0439387 CI passed scheduler and failed next at user-copy-recovery (0), handle-generation (1), handle-owner (2), handle-type (3). Static/native/host/kernel DEV/PROD/routing/ASID passed; evidence was skipped and foundation failed. The broader migration addresses this scope and adjacent families; a full current 144-task execution is running and is not yet a claimed pass.

Phase 3.7 removes the arbitrary two-second coordination cutoff from the actual SMP wait and scheduler completion/copy-drain paths at the maintainer’s explicit request. Acquired completion, root detachment, owner release and transport drainage remain mandatory; published secondary failure still halts with resources retained. A silent stalled CPU has no finite in-kernel detector in this foundation; the host watchdog diagnoses tests and cannot establish completion or reclaim. Workload expiry and the canonical 1/128-second supervisor restart backoff remain separate policies. Historical #126 timeout evidence is not current-source acceptance.

### Production supervisor crash-recovery SYSTEM scenario

cargo xtask crash-recovery (also mandatory in selftest) builds the actual native-supervisor, counter-service and counter-client. tests/system/native-crash-recovery.cjs verifies the original ELF digests and selected kernel, then uses external QEMU GDB hardware breakpoints at the service's public IPC SVC. After the readiness response and actual client delivery, before the first ADD COMMIT, the harness changes only CPU1's EL0 PC to unmapped zero. Source files and ELF bytes remain unchanged; no test entry point, copied implementation or separate test ELF is needed.

The oracle requires actual lower-EL instruction-abort completion of the old service, a fresh service ProcessId generation, successful exit of the same production client and the production supervisor's final continued counter result 12 with the fresh token. The real client checks old SEND rejection, initial value 0 and ADD(5)/ADD(7)/GET after recovery through the actual supervisor main loop. The ordinary supervisor now finishes its session after the actual recovered client succeeds: it observes owner-local service termination and exits normally. The same guest then releases ownership, restores frames and reports zero live processes/domains before poweroff. DEV/PROD executions passed locally. The host oracle separately rejects ordinary success without fault, wrong CPU/EL, stopped service, stale replacement, unrelated panic, wrong client completion, forged report signature and invalid event order. A captured observation fixture is a HOST oracle input, not SYSTEM evidence.

This is selected post-binding, pre-commit crash coverage. It establishes normal production-session shutdown/reclamation after this selected recovery. It does not establish startup-crash recovery, post-commit replay, performance or physical ARM. The older lifecycle test root still has production_supervisor_tested=false; this new separate scenario reports true without rewriting that older scope.

Exact CI 17ac5e9 passed all 144 tasks and complete foundation. Later 66b497a failed shard 2 inside the actual supervision cancellation fixture (AlreadyTerminal, coverage 127); the other shards and ordinary workloads passed. Its load and commit-ack requests accidentally used the short readiness-probe deadline, permitting expiry to win before the requested Cancel. Those test inputs now use the maximum valid absolute deadline (zero is invalid/Expired by the native ABI); readiness deadlines and canonical 1/128 restart backoff remain intact. The test still requires actual commitment acknowledgement, successful explicit cancellation and EffectUnknown, and does not accept an expiry as cancellation. Production IPC state transitions and timing policy are unchanged. New exact-source full CI/acceptance remains pending.

Earlier b037833 CI failed matrix shard 1 in supervision_workload during initial readiness, not in the observable process-stale guards. The other three shards, static, host, native, routing, ASID and kernel DEV/PROD passed; native includes the actual production crash-recovery SYSTEM scenario. Probe-stage/timestamp diagnostics now preserve the accepted readiness deadline to investigate admission expiry before making a repair. Final CI and current-source acceptance remain pending.

Latest completed cc50a5a CI failed only matrix shard 0 at process_quantum_return_and_peer_progress; three other shards and static/host/native/routing/ASID/kernel DEV/PROD passed. Readiness did not fail in that run. The quantum fixture now seeds x20 consistently with its submitted TPIDR and retains every execution/context/resource check, with detailed failure-bit diagnostics. Full new-source CI and retained acceptance remain required.

## Commands and evidence limits

The ordinary supervisor accepts application SessionPolicy argument 1 (Persistent) or 2 (FinishAfterClient), rejecting unknown configuration. These are real application lifetime policies, not test implementations. cargo xtask service-run selects Persistent and observes the service after client exit; host VM stop measures no reclamation. cargo xtask app-smoke selects FinishAfterClient, verifies actual client success, owner-local service termination and normal supervisor exit, then requires acquired owner release, restored frames and zero live processes/domains from generic kernel retirement. No arbitrary watchdog remains in the immutable ELF kernel bootstrap; test hangs are bounded by the external runner.

cargo xtask selftest runs production component tests, ordinary completed sessions, the persistent runtime scenario and an external ABI client in DEV/PROD. The ABI actor checks denied operations, Submit/Wait/Collect, a consumed receipt and three counter requests. It also runs lifecycle integration through the actual kernel and counter-service. An external client imports the single production recover_service method, stops the service through the public owner API, invokes recovery, verifies fresh identity, old token/SEND rejection, fresh state 0 and clean reclamation. A separate ABI actor actually faults; a subsequent real service response establishes containment. That older lifecycle scope does not execute the production supervisor loop; the separate external crash scenario now tests the actual supervisor recovery and normal shutdown.

cargo xtask native-controls checks invalid inputs to actual ELF/parser/policy methods, real denied/stale/binding/IPC calls and resource invariants. UNIT observation-fixture mutations test the same host oracle; they are not kernel execution failures. Source mutation is forbidden. scripts/native-closure-proof.cjs references the original implementation directories, verifies their physical identity, the absence of unrelated workspace crates and external native packages, and executes DEV/PROD runtime/lifecycle. Only build metadata is copied; there are no implementation copies. Host runner dependency closure is excluded from that scope. Prior receipts remain historical.

L1 exercises components, L2 external ABI actors, and L3 production QEMU system scenarios. None establishes L4 physical ARM: NOT_RUN/UNKNOWN. Filesystem, initramfs, storage, DMA, network, package manager, Linux ABI and dynamic linking remain outside scope.

Full Phase 3.7 acceptance remains incomplete pending the current complete foundation and final exact-source semantic/EN-RU review. The actual production supervisor crash path is now exercised by the separate external post-binding pre-commit SYSTEM scenario. Lifecycle stop/replacement/stale/fresh binding/peer fault/reclamation retain their separate test-root scope. Normal production-session shutdown/reclamation is now executed; startup-crash and post-commit replay limits remain explicit. Historical receipts are not relabeled as current-source passes.

[Russian translation](../../translations/ru/docs/kernel/native-applications.md)

The fast COMPONENT layer additionally imports the actual shared kernel_core::supervision::validate_grants used by Kernel Scope::install: empty images, zero/excess instance credits and self/missing SEND bindings are rejected. The single production Grant type replaces the earlier kernel-local definition and inline validation; no model/copy is built for these tests. Separate actual ELF, lifecycle, domain/quota and application protocol methods retain their own input tests.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.native-applications",
  "kind": "subsystem-contract",
  "summary": "Phase 3.7 standalone ELF/native IPC application and persistent counter service requirements.",
  "units": [
    {
      "id": "kolvrt.apps.native-elf",
      "anchor": "native-elf-applications",
      "kind": "feature",
      "summary": "Standalone native ELF applications, private persistent counter state and layered userspace acceptance.",
      "depends_on": [
        "kolvrt.services.supervision",
        "kolvrt.ipc.transport",
        "kolvrt.process.lifecycle",
        "law.009",
        "law.013",
        "law.018",
        "law.025",
        "law.041",
        "law.043",
        "adr.0026",
        "law.044"
      ],
      "feature": {
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "Original standalone no_std ELF supervisor/service/client loaded by the actual kernel ELF loader. One production SDK/service/supervisor implementation; external ABI actors only. Actual persistent requests and post-binding pre-first-COMMIT fault/recovery/fresh binding followed by normal production-session shutdown and acquired reclamation. SessionPolicy selects persistent or finish-after-client lifetime through an opaque application bootstrap argument; no kernel watchdog or application-specific callbacks.",
        "sources": [
          ".github/workflows/ci-windows-workload.yml",
          ".github/workflows/kernel.yml",
          "Cargo.lock",
          "Cargo.toml",
          "apps/native-apps/Cargo.toml",
          "apps/native-apps/build.rs",
          "apps/native-apps/linker.ld",
          "apps/native-apps/src/client.rs",
          "apps/native-apps/src/counter-client.rs",
          "apps/native-apps/src/counter-service.rs",
          "apps/native-apps/src/lib.rs",
          "apps/native-apps/src/supervision.rs",
          "apps/native-apps/src/supervisor.rs",
          "apps/native-apps/tests/supervision.rs",
          "apps/native-runtime/Cargo.toml",
          "apps/native-runtime/src/lib.rs",
          "apps/native-runtime/tests/counter.rs",
          "crates/kernel-core/Cargo.toml",
          "crates/kernel-core/src/domain.rs",
          "crates/kernel-core/src/elf.rs",
          "crates/kernel-core/src/handles.rs",
          "crates/kernel-core/src/ipc.rs",
          "crates/kernel-core/src/ipc/identity.rs",
          "crates/kernel-core/src/lib.rs",
          "crates/kernel-core/src/process.rs",
          "crates/kernel-core/src/supervision.rs",
          "crates/kernel-core/src/wait.rs",
          "crates/kernel-core/tests/bootstrap_grants.rs",
          "crates/kernel/Cargo.toml",
          "crates/kernel/build.rs",
          "crates/kernel/src/arch/aarch64/context.rs",
          "crates/kernel/src/arch/aarch64/entry.S",
          "crates/kernel/src/arch/aarch64/mod.rs",
          "crates/kernel/src/asid.rs",
          "crates/kernel/src/boot_workload.rs",
          "crates/kernel/src/handles/testing.rs",
          "crates/kernel/src/ipc/deferred.rs",
          "crates/kernel/src/ipc/native.rs",
          "crates/kernel/src/ipc_benchmark.rs",
          "crates/kernel/src/ipc_workload.rs",
          "crates/kernel/src/main.rs",
          "crates/kernel/src/memory/mod.rs",
          "crates/kernel/src/native_boot.rs",
          "crates/kernel/src/platform/config.rs",
          "crates/kernel/src/process.rs",
          "crates/kernel/src/process_workload.rs",
          "crates/kernel/src/scheduler/local.rs",
          "crates/kernel/src/scheduler/mod.rs",
          "crates/kernel/src/scheduler/task.rs",
          "crates/kernel/src/security.rs",
          "crates/kernel/src/security/testing.rs",
          "crates/kernel/src/smp.rs",
          "crates/kernel/src/supervision.rs",
          "crates/kernel/src/supervision_workload.S",
          "crates/kernel/src/supervision_workload.rs",
          "crates/kernel/src/sync/mod.rs",
          "crates/kernel/src/test_support/arch_probes.S",
          "crates/kernel/src/test_support/ipc_inputs.rs",
          "crates/kernel/src/test_support/scheduler_inputs.rs",
          "crates/kernel/src/tests.rs",
          "crates/kernel/src/user_copy.rs",
          "crates/kernel/src/user_copy/testing.rs",
          "crates/xtask/src/main.rs",
          "crates/xtask/src/matrix.rs",
          "crates/xtask/src/native_apps.rs",
          "package.json",
          "scripts/ci-gate.cjs",
          "scripts/ci-workload.ps1",
          "scripts/native-closure-proof.cjs",
          "scripts/tests/ci-infrastructure.test.cjs",
          "tests/fixtures/native-lifecycle-observations.json",
          "tests/fixtures/native-production-crash-observations.json",
          "tests/fixtures/native-production-crash-shutdown-observations.json",
          "tests/fixtures/native-production-session-observations.json",
          "tests/native-apps/Cargo.toml",
          "tests/native-apps/build.rs",
          "tests/native-apps/src/abi-client.rs",
          "tests/native-apps/src/abi_steps.rs",
          "tests/native-apps/src/lifecycle-client.rs",
          "tests/native-apps/src/lifecycle-peer.rs",
          "tests/system/native-crash-recovery.cjs",
          "tests/system/native-crash-recovery.test.cjs"
        ],
        "acceptance": [],
        "issues": [28],
        "adrs": ["adr.0026"],
        "limitations": [
          "Physical ARM NOT_RUN/UNKNOWN; no production trust, filesystem, disk durability, migration, generic spawn or stable ABI.",
          "Final exact-source receipts, physical-absence proof review and complete foundation regression acceptance are pending."
        ],
        "next_gate": "Complete current-source full foundation, retained exact-source receipt and semantic/EN-RU acceptance. Selected production crash/recovery/normal shutdown executes; startup and post-commit replay are outside this slice. No issue closure while required gates remain pending.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "UNKNOWN",
            "reason": "Phase 3.7 changes actual application session lifetime, external production crash/recovery/shutdown oracle, generic bootstrap and quantum fixture inputs. Prior receipts retain their earlier source scope; new full exact-source evidence and semantic review remain required."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical ARM execution; NOT_RUN."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Begin Phase 3.7 on merged current main after #125/#126; register the partial scope without claiming application/QEMU acceptance.",
            "acceptance": []
          }
        ]
      }
    }
  ]
}
```
