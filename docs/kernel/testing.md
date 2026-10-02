# Reproducible kernel checks

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

The xtask matrix builds with locked dependencies, runs 23 real kernel tests in each profile and separately boots DEV and PROD without test features. Negative assertion, panic, second physical owner and retained-mapping release images are child host commands: all four must exit nonzero with the expected evidence. The positive matrix fails if any control incorrectly succeeds. JSON events must contain the exact test set and final suite count. Missing results, emulator error or a 30-second timeout fail the command.

Artifacts under target/kernel include ELF, SHA-256 and feature/size build reports, full QEMU version/arguments, UART logs, structured results and unsafe inventory. ELF bytes include debug information; load_bytes counts PT_LOAD file payload; memory_bytes includes zero-filled storage. PROD uses release optimization and retains debug information for inspection; diagnostics and kernel-tests are absent in its boot image.

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
