# Reproducible kernel checks

Output framing and failure semantics follow the [output policy](../architecture/output-policy.md). Use cargo xtask run for the human console, run --prod for PROD and run --machine for framed evidence. cargo xtask test enables machine events in both profiles; console summaries hide JSON while raw logs retain it.

Install Rust 1.99.0 with rustfmt, clippy and target aarch64-unknown-none, Node.js 18 or later, and 7-Zip on Windows. First dependency/tool download requires network access. The Windows QEMU setup extracts the pinned installer into an ignored cache, checks SHA-512 and records provenance; it does not install a service. Other hosts may provide QEMU 10.1.0 via QEMU_AARCH64.

## Commands

```powershell
rustup toolchain install 1.99.0 --profile minimal --component rustfmt --component clippy
rustup target add aarch64-unknown-none --toolchain 1.99.0
./scripts/setup-qemu.ps1
npm ci --ignore-scripts
npm run check
cargo xtask test
```

The xtask matrix builds with locked dependencies, runs 53 real kernel tests in each profile and separately boots DEV and PROD without test features. Negative assertion, panic, second physical owner, retained-mapping release, secondary panic, retiring-frame release, missing acknowledgement and omitted remote TLBI images are child host commands: all eleven must exit nonzero with the expected evidence. The positive matrix fails if any control incorrectly succeeds. JSON events must contain the exact test set and final suite count. Missing results, emulator error or a 30-second timeout fail the command.

Artifacts under target/kernel include ELF, SHA-256 and feature/size build reports, full QEMU version/arguments, UART logs, structured results and unsafe inventory. ELF bytes include debug information; load_bytes counts PT_LOAD file payload; memory_bytes includes zero-filled storage. PROD uses release optimization and retains debug information for inspection; diagnostics and kernel-tests are absent in its boot image.

Each successful test run also validates raw measurement samples and recomputes their quantiles. Use `cargo xtask test --record LABEL` to preserve a run for later comparison; see [measurement history](../../research/measurements/README.md). The host allocator test checks linear scan work against the same implementation with its cursor disabled. These checks do not imply that every algorithm is globally fastest.

## Debugging

```text
cargo xtask debug
aarch64-none-elf-gdb target/kernel/dev-boot.elf
(gdb) target remote 127.0.0.1:1234
(gdb) break kernel_main
(gdb) continue
(gdb) info registers
(gdb) bt
```

Use a separately installed AArch64-capable GDB. QEMU pauses before entry with -S and binds its debug server to loopback. Rust symbols and frame pointers support inspection; assembly exception boundaries do not provide complete DWARF unwind metadata, so a backtrace across an exception is not guaranteed.

## Scope of evidence

Host tests check safe algorithms, malformed DTB rejection, descriptor encoding and concurrent lock publication. In-kernel tests check hardware-visible translation faults and timer IRQ, including complete SIMD restoration. Model checks remain Phase 0 design evidence. CI mirrors local checks in .github/workflows/kernel.yml; a configured workflow is not a completed remote CI run. See [unsafe report interpretation](unsafe.md) and [CPU boundary](smp.md).

## SMP controls

The same protocol runs with diagnostics on/off. The matrix requires 16 named SMP tests and actual CPU_OFF, not only an ONLINE flag. Additional flags are `--secondary-panic-control`, `--retirement-control`, `--shootdown-control` and `--remote-tlbi-control`. Each is expected to fail; the positive runner checks both nonzero status and the failure marker. The omitted-TLBI image must fail after CPU1 reads the mapping. `cargo test --locked` also rejects native dependency reversal. Standalone Phase 2 checks are `cargo test --locked -p routing --all-features` and `cargo test --locked -p routing --no-default-features`; neither connects to the kernel.

The [EL0 foundation](el0.md) adds 14 checks and actual non-test execution. Machine boot requires its EL0 result before terminal boot success. New negative controls are `--user-context-control`, `--user-root-control` and `--user-retirement-control`; they corrupt a saved context, omit a root switch and attempt release after forgetting a space guard. Workers must complete after their local peers fault, proving continued execution.

Run `cargo xtask routing test` for the separate [Phase 2 matrix](routing.md): real EL0 native/v1/v2/bug coexistence, feature-minimal PROD profiles, native-only consumers, stripped boot and three routing failure controls. The 53 native checks and eleven foundation controls remain unchanged. Host decoding tests are not substitutes for EL0 execution.
