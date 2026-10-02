# Unsafe boundary register

Run `cargo xtask audit` to generate target/kernel/unsafe-audit.json. It inventories first-party Rust locations with nearby invariant context, assembly SHA-256, register macro expansions, the native Cargo dependency closure and hashes of pinned core/alloc/compiler_builtins artifacts. Compiler runtime is explicitly trusted, not individually source-proven. Lexical inventory includes unsafe declarations and attributes; location count is not unsafe-block count.

## Invariants

### INV-ENTRY

**Necessity and owner:** Startup requires privileged assembly; primary CPU owns stack/BSS.

**Preconditions and verification:** EL1/EL2 input, aligned stack, IRQ masked, only CPU0. EL1 boot test and linker bounds.

### INV-VECTOR

**Necessity and owner:** Assembly preserves exception ABI outside ordinary Rust calls.

**Preconditions and verification:** All GPR/SIMD/FP state, 16-byte stack, 2 KiB vectors, no nesting. BRK/fault tests and deliberate IRQ SIMD corruption test.

### INV-REG

**Necessity and owner:** Rust cannot express privileged register reads.

**Preconditions and verification:** EL1 runtime, correct architectural encoding. EL, counter and MMU tests; compiler trusted.

### INV-FDT

**Necessity and owner:** Firmware bytes must become a slice.

**Preconditions and verification:** Pinned RAM start, immutable boot data, checked size ends before kernel. Safe parser validates extents, blocks, cells, properties; malformed/truncated host tests.

### INV-MMIO

**Necessity and owner:** Device access requires volatile raw pointers.

**Preconditions and verification:** Validated nonoverlapping platform regions, mapped device lifetime, checked width/alignment/overflow. UART identity and GIC delivery tests.

### INV-UART

**Necessity and owner:** Diagnostic output uses raw PL011 registers.

**Preconditions and verification:** Address published after validation, enforced CPU0-only writer, no IRQ logging, bounded polling; secondary failure publishes atomics. Real host-observed serial events.

### INV-FRAME

**Necessity and owner:** Zeroing and mapped RAM access require raw addresses.

**Preconditions and verification:** CPU0-only physical pool, non-transferable private linear Frame, initialized nonreserved RAM, borrowed Mapping/Retirement and remote reader acknowledgement before free. Exhaustion/reuse/map/permission tests and premature-release controls.

### INV-MMU

**Necessity and owner:** Installing translation tables requires system registers.

**Preconditions and verification:** Aligned initialized tables, current PC/SP mapped, section W^X, device attributes. Actual fault tests and ELF checks.

### INV-TLB

**Necessity and owner:** Ownership retirement requires architectural completion.

**Preconditions and verification:** CPU0 PTE writer releases the table lock before waiting. CPU1 acknowledges only after reader completion and local invalidation/barriers. Checked generation, one retirement; failure never permits release. Actual remote fault and omitted-TLBI control.

### INV-HEAP

**Necessity and owner:** GlobalAlloc is an unsafe pointer contract.

**Preconditions and verification:** Permanent reserved 64 KiB, locked disjoint live ranges, original Layout on free. Box/Vec and exhaustion tests.

### INV-LOCK

**Necessity and owner:** UnsafeCell and Sync implementation require exclusion.

**Preconditions and verification:** Acquire/Release, guard cannot duplicate, T:Send for Lock Sync, guard Sync follows T:Sync; IRQ has no lock access. Host concurrent publication plus kernel locking tests.

### INV-GIC

**Necessity and owner:** Device register construction assumes platform topology.

**Preconditions and verification:** CPU0 initializes distributor; each CPU matches GICR_TYPER to full MPIDR affinity in the validated bounded region. Unsupported VLPI stride fails; IRQ masked during local setup, bounded RWP checks. Both CPUs PPI and repeated bidirectional SGI tests.

### INV-IRQ

**Necessity and owner:** DAIF and GIC operations change execution state.

**Preconditions and verification:** Initialize before unmask, acknowledge exact ID, stop source before EOI, EOImode=0. Real IRQ tests.

### INV-TIMER

**Necessity and owner:** Counter comparator access is architectural.

**Preconditions and verification:** Checked boot-epoch deadline, frequency valid, PPI ready. Conversion host tests and real timer delivery.

### INV-PSCI

**Necessity and owner:** Firmware shutdown needs SMC.

**Preconditions and verification:** Pinned virt PSCI SMC contract, no returned ownership. QEMU completion and host timeout fallback.

### INV-PROBE

**Necessity and owner:** Negative tests deliberately fault or overwrite registers.

**Preconditions and verification:** Kernel-test feature only, exact PC/resume registration, bounded recovery, no PROD recovery. RO/NX/unmap and SIMD negative controls.

## Phase 1.1 boundaries

### INV-SECONDARY

**Necessity and owner:** PSCI entry needs assembly; CPU1 owns a separate permanent linker stack. No BSS/shared initialization. Actual EL1/MMU and stack-bound checks.

### INV-BOOT-PUBLISH

**Preconditions and verification:** CPU0 publishes READY, cleans linked RAM and root tables to PoC using the Cortex-A57 cache line, completes DSB SY before CPU_ON. CPU1 enables that root before acquiring shared publication. Both profile boots; coherent-platform pin does not prove arbitrary hardware coherence.

### INV-CPUON

**Preconditions and verification:** Validated DTB SMC conduit, CPU affinity and aligned native entry/root. SMCCC x0-x3 clobbers declared, PSCI result checked. CPU_OFF follows quiescence; CPU0 also checks AFFINITY_INFO.

### INV-PERCPU

**Preconditions and verification:** MPIDR matches the immutable affinity table. Stack pointer is observed only; mutable state and probes use per-CPU atomics. Distinct ID/stack and independent timer/fault tests.

### INV-GIC-AFFINITY and INV-SGI

**Preconditions and verification:** Bounded validated MMIO region, GICR_TYPER match, checked SGI target bits. Publication before SGI and exact acknowledged INTID for EOI. Repeated bidirectional IPIs and timers.

### INV-SHOOTDOWN and INV-REMOTE-READER

**Preconditions and verification:** Remote tests retain borrowed Mapping/Frame until completion or acknowledged retirement; raw submission is an unsafe test API. Reader admission is excluded during retirement. IRQ only publishes generation. Ordinary quiescence, local TLBI, DSB/ISB and release acknowledgement precede reclaim. Retained charge protects forgotten guards. Delayed-reader, premature-release, omitted-ACK and omitted-TLBI controls.

New assembly and unsafe Rust remain in target/kernel/unsafe-audit.json; machine counts are inventory, not proof.

## Limits

This register documents local proof obligations and observed tests. It does not establish hardware correctness, all malformed-firmware cases or SMP safety. Boot firmware, toolchain, emulator and generated instructions remain trust boundaries. The [SMP contract](smp.md) limits participation to two CPUs and explicitly retained readers. No unsafe is justified by performance alone; the [method gate](../architecture/implementation-review.md) applies before acceptance.
