# Native kernel boot

Console and event presentation follow the [output policy](../architecture/output-policy.md). Default images show completed milestones with units and no raw JSON. Structured evidence is explicitly enabled; the runner retains it in log/result files.

The current implementation boots a native EL1 kernel, with two active CPUs, without userspace, scheduler or compatibility paths. [Foundation decision](../architecture-decisions/0010-kernel-foundation.md) defines the accepted scope.

QEMU 10.1.0 uses `virt-10.1,gic-version=3,virtualization=on,its=off,dtb-randomness=off`, Cortex-A57, TCG, two configured CPUs and 256 MiB RAM. Both CPU0 and CPU1 execute native EL1 code; CPU1 starts through PSCI CPU_ON with a separate stack. Platform discovery is isolated in `crates/kernel/src/platform`; architecture registers and vectors live in `arch/aarch64`. Directory names describe responsibilities and do not import another operating system.

## Sequence

```text
ELF entry → mask IRQ → EL2 to EL1h if necessary → stack and BSS
→ immutable DTB validation → PL011 UART → exception vectors
→ reserved-memory discovery → physical pool → page tables and MMU
→ heap → GICv3 CPU0 interface → physical timer IRQ
→ memory validation → kernel tests or boot validation → PSCI SYSTEM_OFF
```

The ELF starts at 0x40200000, leaving up to 2 MiB at RAM base 0x40000000 for QEMU's DTB. The 256 KiB stack is aligned; the vector table is aligned to 2 KiB. ELF PT_LOAD segments and page permissions enforce W^X. QEMU loads ELF directly; no Linux boot protocol, syscall table or Linux source is linked.

After UART initialization, both profiles print the project's ASCII emblem and KOLVRT name. The original user-supplied PNG is retained at assets/branding/logo.png; assets/branding/boot-logo.txt is compiled as immutable text. Boot performs no image decoding or heap allocation for the banner. Human-readable milestones follow the emblem. Versioned machine records appear only when explicitly enabled.

## Verification

Run `cargo xtask test`. Both DEV and PROD boot images exercise real allocation, mapping, memory access, unmapping, freeing and timer delivery before publishing a structured boot event. Missing events, panic, fatal exception or timeout fail the host command. See [testing](testing.md) and [unsafe boundaries](unsafe.md).

The [SMP protocol](smp.md) adds cleaned release/acquire publication, secondary root activation, affinity-matched local interrupts and acknowledged retirement. Both boot profiles require a real secondary IPI and confirmed CPU_OFF before reporting boot success.

In the boot event, `active_cpus` counts CPUs that participated in boot validation; it is not an online-CPU snapshot at event emission. `secondary_shutdown_verified: true` is emitted only after `smp::shutdown()` observes both the secondary's quiescent state and PSCI AFFINITY_INFO returning OFF. Thus two participating CPUs and verified secondary shutdown describe successive stages. The earlier `secondary_off` field in retained Phase 1.1 records has the same meaning; historical measurements are not rewritten.
