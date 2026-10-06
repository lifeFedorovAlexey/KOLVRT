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
| selftest-process                               | SYSTEM: kernel ELF/process/EXIT/fault containment                    | Only an external fault/EXIT actor is needed; scenario wiring is incomplete                  |
| selftest-ipc                                   | INTEGRATION/SYSTEM: actual native IPC through the public SDK         | External ABI client is permitted; production service and supervisor remain actual artifacts |
| selftest-service                               | E2E: actual service, client and supervisor                           | Duplicate ELF removed; restart/stale/shutdown steps remain incomplete                       |
| selftest-elf                                   | E2E: actual loader and application ELF                               | Uses ordinary counter-client                                                                |
| smoke-client                                   | E2E: ordinary applications and IPC                                   | Duplicate ELF removed; app-smoke selects counter-client                                     |
| selftest-counter-service / selftest-supervisor | Production implementation copies                                     | Removed; not test instruments                                                               |
| isolated_build / mutations.json                | Kernel source-copy mutation                                          | Removed; prior controls do not establish production execution                               |

Only external ABI/fault actors remain under tests/native-apps. They contain test inputs, public-call sequences and assertions; they implement neither the counter service, ordinary client nor supervisor. The fault actor contains its own illegal instruction. The ordinary SDK contains no test-only fault helper. Kernel/application source copies are not created.

## Commands and evidence limits

cargo xtask service-run and app-smoke build actual production ELF artifacts. The supervisor launches the service and ordinary client; the service continues after the client exits. The host observes continuation and then stops QEMU. This is neither graceful shutdown nor a zero-leaked-resources measurement.

cargo xtask selftest runs production component tests, the ordinary runtime scenario and an external ABI client in DEV/PROD. The ABI actor checks denied operations, Submit/Wait/Collect, a consumed receipt and three counter requests. The full command fails until mandatory restart/stale/reply/resource/shutdown scenarios are ready. Partial passes are not full acceptance.

cargo xtask native-controls currently reports the incomplete public-interface replacement. Source-copy controls were removed. scripts/native-closure-proof.cjs no longer copies sources or claims a physical-absence proof; that gate remains pending. Prior receipts retain their historical scope and are not relabeled as current-source evidence.

L1 exercises components, L2 external ABI actors, and L3 production QEMU system scenarios. None establishes L4 physical ARM: NOT_RUN/UNKNOWN. Filesystem, initramfs, storage, DMA, network, package manager, Linux ABI and dynamic linking remain outside scope.

Full Phase 3.7 is incomplete. Next gates: fault scenario wiring, actual restart/stale/reply/reclamation/shutdown scenarios, physical-absence verification without source copies, a current foundation matrix and exact-source EN/RU acceptance. Historical 144 tasks/140 controls are not claimed to pass after source mutation removal.

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
        "implementation_scope": "Single production ELF service/client/supervisor with original ELF loading and native IPC. Unit tests import production SDK methods; external ABI/fault actors contain no application implementation copies. Source-copy mutation controls are removed. Fault wiring, restart/stale/reply/resource/shutdown scenarios, physical absence and full source-bound acceptance remain incomplete.",
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
          "apps/native-apps/src/supervisor.rs",
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
          "tests/native-apps/Cargo.toml",
          "tests/native-apps/build.rs",
          "tests/native-apps/src/abi-client.rs",
          "tests/native-apps/src/abi_steps.rs",
          "tests/native-apps/src/fault-client.rs"
        ],
        "acceptance": [],
        "issues": [28],
        "adrs": ["adr.0026"],
        "limitations": [
          "Physical ARM NOT_RUN/UNKNOWN; no production trust, filesystem, disk durability, migration, generic spawn or stable ABI.",
          "Final exact-source receipts, physical-absence proof review and complete foundation regression acceptance are pending."
        ],
        "next_gate": "Complete actual production-interface fault/restart/stale/reply/reclamation/shutdown scenarios and physical-absence verification without source copies; then rerun required foundation and exact-source semantic/EN-RU gates. Do not close #28 while pending.",
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
