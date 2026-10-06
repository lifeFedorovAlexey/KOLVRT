# Native ELF applications и persistent service

Document status: DESIGN BASELINE
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
        "implementation_scope": "Provisional immutable ELF lifecycle grants and exact-instance client SEND binding; standalone Rust counter-service ELF and native syscall/protocol component library. Real integrated EL0 execution and complete selftests remain pending.",
        "sources": [
          "apps/native-runtime/src/lib.rs",
          "apps/native-runtime/Cargo.toml",
          "apps/native-apps/src/counter-service.rs",
          "apps/native-apps/Cargo.toml",
          "apps/native-apps/build.rs",
          "apps/native-apps/linker.ld",
          "crates/kernel/src/supervision.rs",
          "crates/kernel/src/process.rs",
          "crates/kernel/src/supervision_workload.rs"
        ],
        "acceptance": [],
        "issues": [28],
        "adrs": ["adr.0026"],
        "limitations": [
          "Standalone client/supervisor, four userspace selftests, host runner, exact-source DEV/PROD QEMU and negative controls remain incomplete; no physical ARM, production trust or stable ABI claim."
        ],
        "next_gate": "Integrate actual standalone ELF supervisor/client/service, real native IPC and all four selftest binaries; verify DEV/PROD and precise controls with exact-source receipts.",
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
