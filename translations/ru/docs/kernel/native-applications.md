# Native ELF applications и постоянный сервис

Document status: CURRENT
Evidence scope: экспериментальная реализация Phase 3.7; прежние проверки копий service/supervisor и изменённых исходников не подтверждают production E2E. Полная acceptance не завершена.
Current reference: [Принятая основа supervision](supervision.md)

<a name="native-elf-applications"></a>

## Срез native-приложений

Обычные counter-service, counter-client и native-supervisor — отдельные no_std/no_main ELF в apps/native-apps. Kernel получает полные исходные ELF bytes и использует настоящий loader; counter protocol и ожидаемые значения проверяются приложениями и host harness. GET и checked ADD(delta) используют private userspace state; RESET отсутствует. ADD(5), ADD(7), GET дают 5, 12, 12. Нормальный сервис не содержит намеренного падения, test request counter или повреждения reply.

Finite client SEND binding использует существующий checkpoint supervision и точные instance tokens. Kernel предоставляет generic process/ELF/IPC/authority/lifetime механизмы. Обычная реализация не должна содержать callbacks приложения или тестовые вмешательства. Hardware, disk durability и stable ABI не заявляются.

## Классификация тестов и единственная реализация

| Прежняя проверка                               | Уровень и проверяемый production-код                               | Отдельный ELF и решение                                                          |
| ---------------------------------------------- | ------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| native-userspace tests                         | UNIT: Counter::apply, CounterRequest::decode, frame и reply codecs | Не нужен; apps/native-runtime/tests/counter.rs импортирует настоящие методы      |
| selftest-process                               | SYSTEM: kernel ELF/process/EXIT/fault containment                  | Нужен только внешний fault/EXIT actor; scenario wiring ещё не завершён           |
| selftest-ipc                                   | INTEGRATION/SYSTEM: настоящий native IPC через public SDK          | Внешний ABI-клиент допустим; production service и supervisor остаются настоящими |
| selftest-service                               | E2E: настоящие service, client и supervisor                        | Отдельный дублирующий ELF удалён; restart/stale/shutdown steps ещё не завершены  |
| selftest-elf                                   | E2E: настоящий loader и application ELF                            | Используется обычный counter-client                                              |
| smoke-client                                   | E2E: обычные приложения и IPC                                      | Дублирующий ELF удалён; app-smoke выбирает counter-client                        |
| selftest-counter-service / selftest-supervisor | Копии production-реализаций                                        | Удалены; не считаются test instruments                                           |
| isolated_build / mutations.json                | Изменение копий kernel source                                      | Удалены; прежние controls не подтверждают production execution                   |

В tests/native-apps остаются только внешние ABI/fault actors. Они содержат тестовые входы, последовательности public calls и assertions; не реализуют counter service, обычный client или supervisor. Fault actor содержит собственную illegal instruction. Рабочий SDK не содержит test-only fault helper. Kernel/application source copies не создаются.

## Команды и пределы evidence

cargo xtask service-run и app-smoke собирают настоящие production ELF. Supervisor запускает сервис и обычный client; после выхода client сервис продолжает работать. Host наблюдает продолжение, затем останавливает QEMU. Это не graceful shutdown и не измерение zero leaked resources.

cargo xtask selftest запускает production component tests, обычный runtime scenario и внешний ABI-client в DEV/PROD. ABI actor проверяет denied operations, Submit/Wait/Collect, consumed receipt и три counter requests. Полная команда возвращает ошибку, пока обязательные restart/stale/reply/resource/shutdown scenarios не готовы. Частичные passes не выдаются за полную acceptance.

cargo xtask native-controls сейчас сообщает незавершённость public-interface replacement. Source-copy controls удалены. scripts/native-closure-proof.cjs больше не копирует исходники и не заявляет physical-absence proof; соответствующий gate остаётся pending. Старые receipts сохраняют исторический scope и не переименовываются в current-source evidence.

L1 проверяет компоненты, L2 — внешние ABI actors, L3 — production QEMU system scenarios. Ни один из этих уровней не доказывает L4 physical ARM: NOT_RUN/UNKNOWN. Filesystem, initramfs, storage, DMA, network, package manager, Linux ABI и dynamic linking остаются вне scope.

Полная Phase 3.7 не завершена. Следующие gates: fault scenario wiring, реальные restart/stale/reply/reclamation/shutdown scenarios, проверка physical absence без копий исходников, актуальная foundation matrix и exact-source EN/RU acceptance. Исторические 144 tasks/140 controls не считаются пройденными после удаления source mutations.

[English original](../../../../docs/kernel/native-applications.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.native-applications",
  "kind": "subsystem-contract",
  "summary": "Требования Phase 3.7 к standalone ELF/native IPC applications и persistent counter service.",
  "units": [
    {
      "id": "kolvrt.apps.native-elf",
      "anchor": "native-elf-applications",
      "kind": "feature",
      "summary": "Standalone native ELF applications, private persistent counter state и layered userspace acceptance.",
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
