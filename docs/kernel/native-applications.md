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

The production supervisor now checks the actual lifecycle terminal outcome before replacement, retains accepted failure backoff and verifies fresh-instance readiness with a real GET(0). After a service RPC failure, the ordinary client requests a fresh binding, requires a new identity and initial value 0, rejects the old SEND and repeats business operations only in that new instance. An effect-unknown ADD is never retried against the same instance. UNIT tests import needs_replacement and fresh_binding; the integration actor imports recover_service rather than copying it.

The current external ELF inventory is explicit: selftest-abi-client calls the real native-userspace IPC API against the production service/supervisor; selftest-lifecycle-client calls public lifecycle/IPC APIs and imports native-apps::supervision::recover_service; selftest-lifecycle-peer exercises binding, stale SEND and fault containment. Each is a test client with no production counterpart. There is no copied production supervisor. The lifecycle SYSTEM scenario uses selftest-lifecycle-client as its root instead of native-supervisor, so production_supervisor_tested=false: that scenario does not execute the production supervisor. The real counter-service is used. The unused selftest-fault-client and dead native-*-negative features were removed; lifecycle-peer supplies the executed external fault scenario. Host UNIT tests in apps/native-apps/tests/supervision.rs import needs_replacement and fresh_binding without an ELF.

CI on exact 7d64fac stopped at user-context (shard 0), retirement (shard 1), shootdown (shard 2), and remote-tlbi (shard 3). Retirement compilation was repaired in 71753ae. That commit then failed static because the graph had a stale final manifest digest; c77f6e8 regenerated the exact graph and passed CI static. The four corruption-based controls are now migrated as mapped below. Other legacy controls remain subject to the complete matrix; the named passes do not establish a green foundation or full Phase 3.7 acceptance.

| Migrated control | Actual inputs / observations                                                                                              | Evidence limit                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| user-context     | valid_user_context rejects EL1 modes, AArch32 and masked IRQ; ordinary EL0 context preservation and timer switches        | No saved-frame corruption; validator + real round-trip checks        |
| user-root        | actual EL0 reads of kernel/foreign memory fault; private-memory isolation and clean reclaim                               | Real forbidden accesses; no root-table sabotage                      |
| shootdown        | live secondary reader keeps retirement pending; frame is not reused before ACK; release/reuse follows ACK                 | Observed retirement ordering; no forced missing ACK or timeout claim |
| user-retirement  | Registry::reclaim rejects a live process; state and retained frames remain unchanged; eventual reclaim is checked         | Real public ownership API; no extraction of private frame            |
| remote-tlbi      | acknowledged rejects a future generation; actual TLBI observer precedes ACK; access to retired mapping translation-faults | Instrumented test build; no skipped-TLBI mutation claim              |

These controls use ordinary kernel-tests builds and require every mapped test exactly once with status pass. Missing, duplicate, failed and unrelated observations are rejected. The task inventory is preserved. The four obsolete corruption features are removed. The ordinary suite now has 132 checks per profile. REMOTE_TLBI_COMPLETED is observation-only instrumentation in kernel-tests; this evidence is not byte-identical production-binary execution. The production supervisor crash-recovery acceptance remains incomplete.

The next obsolete controls are classified explicitly. completion-publication-state-inputs imports the production Ownership state machine and rejects completion while borrowed or not quiescent, rejects inspection before Done and verifies phase/generation retention. This UNIT evidence does not test a withheld remote CPU publication or its liveness/failure handling. process-exit calls the single production Table::complete with a duplicate completion and verifies the terminal record, state and live-slot count remain unchanged; the test-only Registry::repeat_completion wrapper is removed. process-rollback uses real zero-memory-budget creation failures and checks resource rollback. asid-reuse-invariants observes actual same-VA isolation, invalidation counters and pool exhaustion; it is not a skipped-invalidation negative control.

shootdown-invariants and remote-tlbi-invariants are positive invariant coverage; their previous control flags are CLI aliases. They do not establish missing-ACK or skipped-TLBI detection. The plan and archived counts distinguish ordinary, negative-input, invariant and remaining legacy-failure tasks. Withheld publication/missing ACK/skipped invalidation SYSTEM coverage and the production-supervisor crash-recovery E2E remain acceptance gaps.

The process transition family now uses explicit production-method negative inputs: Table::reclaim without quiescence preserves the completed record/live-slot count (UNIT); Registry::start for an already admitted process preserves state/frames; stale IDs fail start/data/receipt validation after actual slot reuse; Registry::reclaim for a live process preserves state/frames. No forced scheduler unlink is claimed. Obsolete process-contract features and early panic branches are removed.

## Commands and evidence limits

cargo xtask service-run and app-smoke build actual production ELF artifacts. The supervisor launches the service and ordinary client; the service continues after the client exits. The host observes continuation and then stops QEMU. This is neither graceful shutdown nor a zero-leaked-resources measurement.

cargo xtask selftest runs production component tests, the ordinary runtime scenario and an external ABI client in DEV/PROD. The ABI actor checks denied operations, Submit/Wait/Collect, a consumed receipt and three counter requests. It also runs lifecycle integration through the actual kernel and counter-service. An external client imports the single production recover_service method, stops the service through the public owner API, invokes recovery, verifies fresh identity, old token/SEND rejection, fresh state 0 and clean reclamation. A separate ABI actor actually faults; a subsequent real service response establishes containment. These passes do not test counter-service crashing inside the complete production supervisor loop.

cargo xtask native-controls checks invalid inputs to actual ELF/parser/policy methods, real denied/stale/binding/IPC calls and resource invariants. UNIT observation-fixture mutations test the same host oracle; they are not kernel execution failures. Source mutation is forbidden. scripts/native-closure-proof.cjs references the original implementation directories, verifies their physical identity, the absence of unrelated workspace crates and external native packages, and executes DEV/PROD runtime/lifecycle. Only build metadata is copied; there are no implementation copies. Host runner dependency closure is excluded from that scope. Prior receipts remain historical.

L1 exercises components, L2 external ABI actors, and L3 production QEMU system scenarios. None establishes L4 physical ARM: NOT_RUN/UNKNOWN. Filesystem, initramfs, storage, DMA, network, package manager, Linux ABI and dynamic linking remain outside scope.

Full Phase 3.7 is incomplete. Remaining gates: the complete production supervisor loop when counter-service itself crashes, a current foundation matrix without source mutation, final exact-source semantic/EN-RU acceptance and CI. Stop/replacement/stale/fresh binding/peer fault/shutdown have been exercised using the actual kernel/service and imported production recover_service. Historical 144 tasks/140 controls are not relabeled as current-source passes.

[Russian translation](../../translations/ru/docs/kernel/native-applications.md)

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
        "implementation_scope": "Single production ELF service/client/supervisor, original ELF loader and real native IPC. Unit tests import production codecs and recovery/binding policy. Integration imports the same production recover_service and verifies actual stop/replacement/stale identity/fresh binding/fault containment/reclamation. Source-copy mutations and application copies are absent. Native dependency proof references original implementation files. Full production-supervisor service-crash execution, current foundation controls and final source-bound acceptance remain incomplete.",
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
          "crates/kernel-core/src/domain.rs",
          "crates/kernel-core/src/elf.rs",
          "crates/kernel-core/src/handles.rs",
          "crates/kernel-core/src/ipc.rs",
          "crates/kernel-core/src/ipc/identity.rs",
          "crates/kernel-core/src/process.rs",
          "crates/kernel-core/src/wait.rs",
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
          "crates/kernel/src/supervision_workload.rs",
          "crates/kernel/src/sync/mod.rs",
          "crates/kernel/src/test_support/arch_probes.S",
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
          "tests/native-apps/Cargo.toml",
          "tests/native-apps/build.rs",
          "tests/native-apps/src/abi-client.rs",
          "tests/native-apps/src/abi_steps.rs",
          "tests/native-apps/src/lifecycle-client.rs",
          "tests/native-apps/src/lifecycle-peer.rs"
        ],
        "acceptance": [],
        "issues": [28],
        "adrs": ["adr.0026"],
        "limitations": [
          "Physical ARM NOT_RUN/UNKNOWN; no production trust, filesystem, disk durability, migration, generic spawn or stable ABI.",
          "Final exact-source receipts, physical-absence proof review and complete foundation regression acceptance are pending."
        ],
        "next_gate": "Complete exact-source semantic/EN-RU acceptance and a current foundation plan without source mutation; demonstrate the full production supervisor service-crash path or explicitly retain its evidence gap. No issue closure while required gates remain pending.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "UNKNOWN",
            "reason": "Phase 3.7 removes source-copy mutation builds and application implementation copies. Tests use the actual production code; replacement fault/restart/shutdown and related acceptance scenarios remain incomplete. Prior receipts retain their historical scope; partial passes are not full current-source acceptance."
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
