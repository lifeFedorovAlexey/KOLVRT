# Native ELF applications and persistent service

Document status: CURRENT
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

## Build, execution and observation

The separately linked binaries use their own Cargo package and the syscall library, without linking the kernel crate. The kernel receives full ELF files through immutable SHA-256-named boot inputs and invokes its real ELF64 loader. Debug sections remain in the original file; the host never extracts a raw code window. Kernel boot forwards opaque report words and enforces generic process completion and resource ownership; counter decoding and expected value 12 belong to userspace and the host acceptance runner.

`cargo xtask selftest` runs the L1 native protocol/state/kernel-core tests, then four independent client ELF binaries in DEV and PROD. selftest-process contains an auxiliary real instruction fault while the main client remains alive; selftest-ipc explicitly submits, waits, collects, rejects a consumed receipt and denied SEND/lifecycle authority; selftest-service checks repeated state and fresh restart; selftest-elf exercises actual loader, EL0 native calls and clean exit. Kernel events independently record terminal identities, owner CPUs, blocking and wakeups. These tests execute on full QEMU systems; the layer distinction describes what each observation proves.

`cargo xtask app-smoke` separately runs the standalone counter client and requires all machine fields: supervisor readiness, nonzero instance, actual client ELF load, successful exit, repeated requests, final 12, restart, rejected old bindings, completed fresh binding and zero live processes/domains/restored frames. Successful ordinary boot is insufficient. Both supervisor and client require the exact stale-handle error after replacement; timeout is not stale-binding acceptance.

`cargo xtask service-run` observes the separate persistent runtime in both profiles. Guest lifecycle has no workload deadline or fixture shutdown: after the client exits, the supervisor seals bootstrap authority and continues probing the live service. The host ends its observation by stopping QEMU after an opaque userspace continuation witness. This external stop is recorded explicitly and makes no zero-leak shutdown claim. The resulting kernel ELF may be run directly with the recorded QEMU arguments to continue the runtime.

`cargo xtask native-controls` isolates seven failures in each profile: malformed original ELF, widened immutable SEND authority, accepted stale instance, binding to a wrong feedback endpoint, corrupt reply value, broken native Submit and skipped root reclamation. Acceptance requires the selected loader error, application assertion/exit, or actual retained-process/frame counts; an unrelated panic cannot satisfy a control. Controls are separate build features and are absent from normal runtime selection.

`node scripts/native-closure-proof.cjs` creates a fresh native-only workspace, physically omits unrelated component crates and the host routing implementation, removes only their workspace/path-dependency declarations, preserves selected dependency versions, checks the kernel dependency closure and executes the full selftest plus app smoke in DEV and PROD. Original kernel/app/model source digests are compared with the copied files; the proof directory is retained for inspection.

All commands accept `--prod` to select only PROD. Without it they run DEV and PROD. Target receipts are mutable working results until reviewed source-bound copies are retained under research/results.

For an ongoing host session, use `cargo xtask service-run --live` (DEV) or `cargo xtask service-run --live --prod`. This starts the same unbounded guest runtime and leaves QEMU running until the host session is stopped; it does not manufacture a clean-shutdown receipt.

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
