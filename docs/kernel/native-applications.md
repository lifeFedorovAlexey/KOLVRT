# Native ELF applications and persistent service

Document status: DESIGN BASELINE
Evidence scope: experimental Phase 3.7 implementation; component tests and separately built ELF artifacts are distinct from real EL0/QEMU acceptance, which remains pending.
Current reference: [Accepted supervision foundation](supervision.md)

<a name="native-elf-applications"></a>

## Native application slice

Issue #28 extends the accepted #125/#126 foundation with a separate no_std/no_main AArch64 ELF counter service, standalone client, isolated supervisor and real native IPC. Immutable original ELF bytes must reach the kernel ELF loader; host-extracted raw code is not acceptance. The kernel enforces generic process/image/grant/resource mechanisms and must not interpret the counter protocol or smoke application's expected value.

counter/1 has GET and checked ADD(delta). Initial value is zero; RESET is absent. State lives at a private userspace address and survives requests within one instance. Overflow rejects mutation. Fault/restart creates a fresh ProcessId/domain/endpoint/handles and resets state to zero; no disk/crash durability is claimed. ADD(5), ADD(7), GET must return 5, 12, 12 through actual IPC.

The provisional lifecycle extension reuses the existing exact-supervisor checkpoint. Immutable grants select image format and at most one SEND destination per client entry. Operation 6 uses client selector/token and exact destination instance token; it mints a caller-local client SEND handle only after acquired root/owner quiescence. Numeric tokens alone do not confer authority. Rebinding closes only the prior handle minted by that same entry. Sealing never restores unused bootstrap authority. This is a bounded static-image development path, not generic spawning, package policy or production image trust.

The first ELF profile retains the accepted one-RX-segment image window; application state uses private writable stack pages. This is a chosen first-slice profile, not an architectural ban on later ELF data segments. Later requirements must rederive the architecture rather than preserve this implementation by effort or test compatibility.

## Required evidence layers

| Layer             | Evidence and limit                                                                                                                                                |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| L1 — COMPONENT    | Existing fast host/kernel-core tests, including protocol/state, lifecycle/identity, quota/accounting, stale tokens and ELF parsing; does not prove EL0 execution. |
| L2 — EL0 SELFTEST | Separate selftest-process, selftest-ipc, selftest-service and selftest-elf binaries through public native ABI; does not prove full system boot.                   |
| L3 — QEMU SYSTEM  | DEV/PROD kernel + supervisor + persistent service + standalone ELF client with machine-readable operations, restart/binding and zero-leak shutdown results.       |
| L4 — HARDWARE     | Future physical ARM gate: NOT_RUN/UNKNOWN. QEMU is not hardware acceptance.                                                                                       |

The host runner must provide selftest and app-smoke commands, reject ordinary-boot-only evidence and detect precise broken ELF load, authorization, stale identity, readiness binding, reply/IPC and reclamation controls. Arbitrary panic is not a valid witness. A distinct persistent runtime mode must outlive one request; bounded acceptance may shut down cleanly after its required observations.

No filesystem, initramfs, storage, DMA, network, package manager, Linux ABI, dynamic linker/shared libraries or stable ABI freeze. The native closure must operate with routing, window-compat, migration-advisor, AI/Soul and package manager physically absent. Kernel external dependencies require separate architectural review.

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
