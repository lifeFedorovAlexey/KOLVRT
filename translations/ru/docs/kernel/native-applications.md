# Native ELF applications и постоянный сервис

Document status: CURRENT
Evidence scope: экспериментальная реализация Phase 3.7; прежние проверки копий service/supervisor и изменённых исходников не подтверждают production E2E. Полная acceptance не завершена.
Current reference: [Принятая основа supervision](supervision.md)

<a name="native-elf-applications"></a>

## Срез native-приложений

Обычные counter-service, counter-client и native-supervisor — отдельные no_std/no_main ELF в apps/native-apps. Kernel получает полные исходные ELF bytes и использует настоящий loader; counter protocol и ожидаемые значения проверяются приложениями и host harness. GET и checked ADD(delta) используют private userspace state; RESET отсутствует. ADD(5), ADD(7), GET дают 5, 12, 12. Нормальный сервис не содержит намеренного падения, test request counter или повреждения reply.

Finite client SEND binding использует существующий checkpoint supervision и точные instance tokens. Kernel предоставляет generic process/ELF/IPC/authority/lifetime механизмы. Обычная реализация не должна содержать callbacks приложения или тестовые вмешательства. Hardware, disk durability и stable ABI не заявляются.

## Классификация тестов и единственная реализация

| Прежняя проверка                               | Уровень и проверяемый production-код                               | Отдельный ELF и решение                                                               |
| ---------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| native-userspace tests                         | UNIT: Counter::apply, CounterRequest::decode, frame и reply codecs | Не нужен; apps/native-runtime/tests/counter.rs импортирует настоящие методы           |
| selftest-process                               | SYSTEM: kernel ELF/process/EXIT/fault containment                  | Внешний lifecycle-peer реально падает, а настоящий сервис продолжает отвечать         |
| selftest-ipc                                   | INTEGRATION/SYSTEM: настоящий native IPC через public SDK          | Внешний ABI-клиент допустим; production service и supervisor остаются настоящими      |
| selftest-service                               | SYSTEM: настоящий kernel/service и recover_service; test root      | Дублирующий ELF удалён; stop/replacement/stale/shutdown проверены через lifecycle ABI |
| selftest-elf                                   | E2E: настоящий loader и application ELF                            | Используется обычный counter-client                                                   |
| smoke-client                                   | E2E: обычные приложения и IPC                                      | Дублирующий ELF удалён; app-smoke выбирает counter-client                             |
| selftest-counter-service / selftest-supervisor | Копии production-реализаций                                        | Удалены; не считаются test instruments                                                |
| isolated_build / mutations.json                | Изменение копий kernel source                                      | Удалены; прежние controls не подтверждают production execution                        |

В tests/native-apps остаются только внешние ABI/fault actors. Они содержат тестовые входы, последовательности public calls и assertions; не реализуют counter service, обычный client или supervisor. Fault actor содержит собственную illegal instruction. Рабочий SDK не содержит test-only fault helper. Kernel/application source copies не создаются.

Production supervisor теперь проверяет настоящий terminal outcome через lifecycle API перед replacement, сохраняет принятую failure backoff и проверяет readiness новой instance реальным GET(0). После service RPC failure обычный client запрашивает новую binding, требует новую identity и initial value 0, отвергает старый SEND и повторяет бизнес-операции только в новой instance. Effect-unknown ADD не повторяется в прежней instance. needs_replacement и fresh_binding импортируются UNIT-тестами; recover_service импортируется integration actor, а не копируется.

Текущий список внешних ELF указан явно: selftest-abi-client вызывает настоящий native-userspace IPC API с production service/supervisor; selftest-lifecycle-client вызывает public lifecycle/IPC API и импортирует native-apps::supervision::recover_service; selftest-lifecycle-peer проверяет binding, stale SEND и fault containment. Каждый является тестовым клиентом без production-аналога. Копии production supervisor нет. Lifecycle SYSTEM-сценарий использует selftest-lifecycle-client как root вместо native-supervisor, поэтому production_supervisor_tested=false: production supervisor в этом сценарии не выполняется. Используется настоящий counter-service. Неиспользуемый selftest-fault-client и мёртвые native-*-negative features удалены; выполняемый внешний fault scenario предоставляет lifecycle-peer. Host UNIT-тесты apps/native-apps/tests/supervision.rs импортируют needs_replacement и fresh_binding без ELF.

CI на exact 7d64fac остановился на user-context (shard 0), retirement (shard 1), shootdown (shard 2) и remote-tlbi (shard 3). Компиляция retirement исправлена в 71753ae. Затем static этого коммита упал из-за устаревшего final manifest digest в graph; c77f6e8 перегенерировал точный graph и прошёл CI static. Четыре corruption-based controls теперь перенесены согласно карте ниже. Остальные legacy controls ещё проверяются полной matrix; отдельные passes не подтверждают зелёный foundation или полную acceptance Phase 3.7.

| Перенесённый control | Настоящие входы / observations                                                                                                       | Предел evidence                                                                  |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| user-context         | valid_user_context отвергает режимы EL1, AArch32 и masked IRQ; обычные EL0 context preservation и timer switches                     | Нет повреждения saved frame; validator + реальные round-trip checks              |
| user-root            | настоящие EL0 reads kernel/foreign memory приводят к fault; private-memory isolation и clean reclaim                                 | Настоящие запрещённые accesses; нет sabotage root tables                         |
| shootdown            | живой secondary reader сохраняет pending retirement; frame не переиспользуется до ACK; release/reuse следуют за ACK                  | Наблюдаемый retirement ordering; нет намеренно потерянного ACK или timeout claim |
| user-retirement      | Registry::reclaim отвергает живой process; state и удержанные frames не меняются; окончательное reclamation проверено                | Настоящий public ownership API; private frame не извлекается                     |
| remote-tlbi          | acknowledged отвергает будущую generation; настоящий TLBI observer предшествует ACK; access к retired mapping даёт translation fault | Instrumented test build; нет skipped-TLBI mutation claim                         |

Эти controls используют обычную kernel-tests сборку и требуют каждую mapped test ровно один раз со status pass. Missing, duplicate, failed и unrelated observations отвергаются. Task inventory сохранён. Четыре старые corruption features удалены. Обычный suite теперь содержит 128 checks на profile. REMOTE_TLBI_COMPLETED — только observation instrumentation в kernel-tests; эта evidence не является byte-identical production-binary execution. Acceptance crash-recovery production supervisor остаётся незавершённой.

## Команды и пределы evidence

cargo xtask service-run и app-smoke собирают настоящие production ELF. Supervisor запускает сервис и обычный client; после выхода client сервис продолжает работать. Host наблюдает продолжение, затем останавливает QEMU. Это не graceful shutdown и не измерение zero leaked resources.

cargo xtask selftest запускает production component tests, обычный runtime scenario и внешний ABI-client в DEV/PROD. ABI actor проверяет denied operations, Submit/Wait/Collect, consumed receipt и три counter requests. Команда также выполняет lifecycle integration через настоящие kernel и counter-service. Внешний клиент импортирует единственный production recover_service, останавливает сервис через public owner API, вызывает recovery, проверяет новую identity, old token/SEND rejection, fresh state 0 и clean reclamation. Отдельный ABI actor действительно падает; последующий ответ настоящего сервиса подтверждает fault containment. Эти passes не являются проверкой аварийного завершения counter-service внутри полного production supervisor loop.

cargo xtask native-controls проверяет неверные входы настоящих ELF/parser/policy методов, реальные denied/stale/binding/IPC calls и resource invariants. UNIT изменения observation fixtures проверяют тот же host oracle; это не execution failures ядра. Source mutations запрещены. scripts/native-closure-proof.cjs подключает оригинальные implementation directories через directory references, проверяет их физическую identity, отсутствие несвязанных crates в workspace, отсутствие external native packages и выполняет DEV/PROD runtime/lifecycle. Копируется только build metadata; копий реализации нет. Host runner dependency closure исключена из этого scope. Старые receipts остаются историческими.

L1 проверяет компоненты, L2 — внешние ABI actors, L3 — production QEMU system scenarios. Ни один из этих уровней не доказывает L4 physical ARM: NOT_RUN/UNKNOWN. Filesystem, initramfs, storage, DMA, network, package manager, Linux ABI и dynamic linking остаются вне scope.

Полная Phase 3.7 не завершена. Оставшиеся gates: полный production supervisor loop при аварийном завершении именно counter-service, актуальная foundation matrix без source mutations, окончательная exact-source semantic/EN-RU acceptance и CI. Stop/replacement/stale/fresh binding/peer fault/shutdown уже проверены на настоящем kernel/service и импортированном production recover_service. Исторические 144 tasks/140 controls не переименовываются в current-source passes.

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
