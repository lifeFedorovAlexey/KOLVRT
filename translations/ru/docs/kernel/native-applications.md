# Native ELF applications и persistent service

Document status: CURRENT
Evidence scope: экспериментальная реализация Phase 3.7; component tests и отдельно собранные ELF artifacts отделены от actual EL0/QEMU acceptance, которая ещё ожидается.
Current reference: [Принятая основа supervision](supervision.md)

<a name="native-elf-applications"></a>

## Native application slice

Issue #28 расширяет принятую основу #125/#126 отдельным no_std/no_main AArch64 ELF counter service, standalone client, isolated supervisor и real native IPC. Immutable original ELF bytes должны попадать в kernel ELF loader; host-extracted raw code не является acceptance. Kernel обеспечивает generic process/image/grant/resource mechanisms и не должен интерпретировать counter protocol или ожидаемое значение smoke application.

counter/1 имеет GET и checked ADD(delta). Initial value — zero; RESET отсутствует. State хранится по private userspace address и переживает requests одного instance. Overflow отвергает mutation. Fault/restart создаёт fresh ProcessId/domain/endpoint/handles и сбрасывает state в zero; disk/crash durability не заявляется. ADD(5), ADD(7), GET должны вернуть 5, 12, 12 через actual IPC.

Provisional lifecycle extension переиспользует existing exact-supervisor checkpoint. Immutable grants выбирают image format и не более одного SEND destination для client entry. Operation 6 использует client selector/token и exact destination instance token; caller-local client SEND handle создаётся только после acquired root/owner quiescence. Numeric tokens сами по себе не дают authority. Rebinding закрывает только previous handle, выданный той же entry. Sealing не восстанавливает unused bootstrap authority. Это bounded static-image development path, а не generic spawning, package policy или production image trust.

Первый ELF profile сохраняет принятое one-RX-segment image window; application state использует private writable stack pages. Это выбранный first-slice profile, а не архитектурный запрет будущих ELF data segments. Следующие требования должны заново выводить архитектуру, а не сохранять реализацию из-за усилий или совместимости тестов.

## Требуемые слои evidence

| Layer             | Evidence и ограничение                                                                                                                                       |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| L1 — COMPONENT    | Existing fast host/kernel-core tests, включая protocol/state, lifecycle/identity, quota/accounting, stale tokens и ELF parsing; не доказывает EL0 execution. |
| L2 — EL0 SELFTEST | Отдельные selftest-process, selftest-ipc, selftest-service и selftest-elf binaries через public native ABI; не доказывает full system boot.                  |
| L3 — QEMU SYSTEM  | DEV/PROD kernel + supervisor + persistent service + standalone ELF client с machine-readable operations, restart/binding и zero-leak shutdown results.       |
| L4 — HARDWARE     | Будущий physical ARM gate: NOT_RUN/UNKNOWN. QEMU не является hardware acceptance.                                                                            |

Host runner должен предоставить команды selftest и app-smoke, отвергать ordinary-boot-only evidence и обнаруживать precise broken ELF load, authorization, stale identity, readiness binding, reply/IPC и reclamation controls. Arbitrary panic не является valid witness. Отдельный persistent runtime mode должен жить дольше одного request; bounded acceptance может чисто завершаться после required observations.

Не реализовывать filesystem, initramfs, storage, DMA, network, package manager, Linux ABI, dynamic linker/shared libraries или stable ABI freeze. Native closure должна работать при физическом отсутствии routing, window-compat, migration-advisor, AI/Soul и package manager. Kernel external dependencies требуют отдельного architectural review.

## Сборка, выполнение и наблюдение

Отдельно скомпонованные бинарники используют собственный Cargo package и библиотеку системных вызовов, без связывания с kernel crate. Ядро получает полные ELF-файлы через неизменяемые boot inputs с именами по SHA-256 и вызывает настоящий ELF64 loader. Debug sections остаются в исходном файле; host не извлекает raw code window. Kernel boot передаёт непрозрачные слова отчёта и обеспечивает общие правила завершения процессов и владения ресурсами; разбор counter protocol и ожидаемое значение 12 принадлежат userspace и host acceptance runner.

`cargo xtask selftest` запускает L1-тесты native protocol/state/kernel-core, затем четыре отдельных клиентских ELF в DEV и PROD. selftest-process вызывает настоящий instruction fault вспомогательного процесса, пока основной клиент жив; selftest-ipc явно выполняет submit, wait, collect и проверяет отказ для использованного receipt, SEND и lifecycle authority; selftest-service проверяет сохранение состояния между вызовами и новый instance после restart; selftest-elf проверяет настоящий loader, native calls из EL0 и чистый выход. Kernel events независимо записывают terminal identities, CPU owners, блокировки и пробуждения. Тесты выполняются в полных QEMU-системах; разделение на слои определяет, что доказывает каждое наблюдение.

`cargo xtask app-smoke` отдельно запускает standalone counter client и требует все machine fields: readiness supervisor, ненулевой instance, загрузку клиента как ELF, успешный выход, повторные requests, итог 12, restart, отказ старых bindings, завершение запросов через новую binding и нулевые live processes/domains с восстановленными frames. Успешного обычного boot недостаточно. Supervisor и client требуют точный stale-handle error после replacement; timeout не подтверждает отказ старой binding.

`cargo xtask service-run` наблюдает отдельный persistent runtime в обоих профилях. Guest lifecycle не имеет workload deadline или fixture shutdown: после выхода клиента supervisor закрывает bootstrap authority и продолжает опрашивать живой сервис. Host завершает наблюдение остановкой QEMU после непрозрачного userspace witness продолжения. Эта внешняя остановка явно записывается и не подтверждает отсутствие утечек при shutdown. Полученный kernel ELF можно запустить напрямую с записанными QEMU arguments для продолжения runtime.

`cargo xtask native-controls` изолирует семь отказов в каждом профиле: повреждённый исходный ELF, расширенная immutable SEND authority, принятый stale instance, binding к неверному feedback endpoint, испорченное значение reply, сломанный native Submit и пропущенный reclaim root. Acceptance требует выбранный loader error, application assertion/exit либо реальные счётчики удержанных процессов и frames; произвольный panic не подтверждает control. Controls выбираются отдельными build features и отсутствуют в обычном runtime.

`node scripts/native-closure-proof.cjs` создаёт новый native-only workspace, физически исключает несвязанные component crates и host routing implementation, удаляет только их workspace/path-dependency declarations, сохраняет выбранные версии зависимостей, проверяет kernel dependency closure и выполняет полный selftest вместе с app smoke в DEV и PROD. Digests исходных kernel/app/model sources сравниваются с копиями; proof directory сохраняется для проверки.

Все команды принимают `--prod` для выбора только PROD. Без этого флага выполняются DEV и PROD. Target receipts остаются изменяемыми рабочими результатами до сохранения проверенных source-bound копий в research/results.

Для непрерывной host-сессии используйте `cargo xtask service-run --live` (DEV) или `cargo xtask service-run --live --prod`. Команда запускает тот же неограниченный guest runtime и оставляет QEMU работающим до остановки host-сессии; clean-shutdown receipt при этом не создаётся.

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
        "adr.0026"
      ],
      "feature": {
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "Separately built original AArch64 ELF supervisor, counter service, client and four EL0 selftest binaries; private checked counter state, exact-instance finite SEND rebinding, observed blocking/wakeup across CPU owners, bounded clean shutdown and separate persistent runtime. Host selftest runs L1 models and DEV/PROD QEMU with seven precise controls. Acceptance receipts and complete foundation regression review are being finalized.",
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
          "apps/native-apps/src/selftest-elf.rs",
          "apps/native-apps/src/selftest-ipc.rs",
          "apps/native-apps/src/selftest-process.rs",
          "apps/native-apps/src/selftest-service.rs",
          "apps/native-apps/src/smoke-client.rs",
          "apps/native-apps/src/supervisor.rs",
          "apps/native-runtime/Cargo.toml",
          "apps/native-runtime/src/lib.rs",
          "crates/kernel/Cargo.toml",
          "crates/kernel/build.rs",
          "crates/kernel/src/ipc/native.rs",
          "crates/kernel/src/main.rs",
          "crates/kernel/src/memory/mod.rs",
          "crates/kernel/src/native_boot.rs",
          "crates/kernel/src/platform/config.rs",
          "crates/kernel/src/process.rs",
          "crates/kernel/src/scheduler/local.rs",
          "crates/kernel/src/scheduler/mod.rs",
          "crates/kernel/src/supervision.rs",
          "crates/kernel/src/supervision_workload.rs",
          "crates/xtask/src/main.rs",
          "crates/xtask/src/native_apps.rs",
          "package.json",
          "scripts/ci-gate.cjs",
          "scripts/ci-workload.ps1",
          "scripts/native-closure-proof.cjs",
          "scripts/tests/ci-infrastructure.test.cjs"
        ],
        "acceptance": [],
        "issues": [28],
        "adrs": ["adr.0026"],
        "limitations": [
          "Physical ARM NOT_RUN/UNKNOWN; no production trust, filesystem, disk durability, migration, generic spawn or stable ABI.",
          "Final exact-source receipts, physical-absence proof review and complete foundation regression acceptance are pending."
        ],
        "next_gate": "Finalize precise DEV/PROD controls, source-bound receipts, native closure proof and full code/EN-RU semantic review; do not close #28 while those gates are pending.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "UNKNOWN",
            "reason": "No integrated current-source EL0 execution receipt yet."
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
