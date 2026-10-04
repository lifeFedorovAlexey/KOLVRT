# Unsafe boundary register

Run `cargo xtask audit` to generate target/kernel/unsafe-audit.json. It inventories first-party Rust locations with nearby invariant context, assembly SHA-256, register macro expansions, the native Cargo dependency closure and hashes of pinned core/alloc/compiler_builtins artifacts. Compiler runtime is explicitly trusted, not individually source-proven. Lexical inventory includes unsafe declarations and attributes; location count is not unsafe-block count.

## Invariants

### INV-ENTRY

**Necessity and owner:** Startup requires privileged assembly; primary CPU owns stack/BSS.

**Preconditions and verification:** EL1/EL2 input, aligned stack, IRQ masked, only CPU0. EL1 boot test and linker bounds.

### INV-VECTOR

**Necessity and owner:** Assembly preserves exception ABI outside ordinary Rust calls.

**Preconditions and verification:** All GPR/SIMD/FP state, 16-byte stack, 2 KiB vectors, no nested IRQ. A synchronous user-copy abort may interrupt a lower-EL trap: its EL1 register frame is restored while the outer lower-EL return state stays in saved Context. BRK/fault tests, deliberate IRQ SIMD corruption and precise user-copy recovery cover the supported paths.

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

**Preconditions and verification:** Acquire/Release, guard cannot duplicate, T:Send for Lock Sync, guard is not Send/Sync and cannot cross CPUs/threads; IRQ has no lock access. Host concurrent publication plus kernel locking tests.

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

### INV-USER-SPACE and INV-USER-RETIRE

CPU0 initializes checked exclusively allocated table/data/code/stack/image pages and retains Frame borrows plus persistent space charges. A forgotten guard cannot permit release. Roots are immutable while admitted; each is pinned to one CPU. Both CPUs restore native root and complete local TLBI before release completion. CPU0 acquires both completions before clearing charges. EL0 fixture writes its own tick word; kernel IRQ does not write fixture data; tag reads happen after all writers stop. Actual RO/NX/guard/foreign faults, peer survival, returned free count and forgotten-guard rejection cover these obligations.

### INV-USER-TTBR and INV-USER-IMAGE

Privileged root/barrier instructions require live aligned tables preserving kernel PC/SP. The ASID-zero fallback requires full local invalidation on each switch; fixed-affinity process roots use a hardware-width-checked ASID lease and local `TLBI ASIDE1` before retirement/reuse. The lease epoch rejects stale software retirement, and release requires the owning CPU's completed invalidation plus scheduler quiescence. Unsupported ASID encodings retain the full-flush baseline. Trusted immutable linker extents are copied into checked owned pages, cleaned to PoC and published with instruction-cache maintenance before launch. Root-switch omission and omitted reuse invalidation must fail on QEMU. No loader, migration, multi-CPU root residency or physical cache-coherency proof is claimed.

### INV-USER-CONTEXT and INV-RUNQUEUE

Lower-EL vectors use the per-CPU EL1 stack and complete aligned 816-byte frame; compile-time offsets match assembly. Kernel ABI/SP/TLS is restored on return. Each CPU exclusively mutates its UnsafeCell queue with IRQ masked; no reference spans user execution. Setup precedes release launch; acquire done precedes inspection. Per-CPU CAS admission and acquire terminal completion prevent stale polls from entering a reset batch. Per-process ownership CAS and fixed affinity reject duplicate/wrong-CPU execution. IRQ takes no locks and allocates nothing. User register expectations belong to the verifier, not authority logic. Actual GPR/SIMD/FP/TLS/stack checks, timer switching and corrupted-context rejection validate this scope. See the [EL0 contract](el0.md).

This register documents local proof obligations and observed tests. It does not establish hardware correctness, all malformed-firmware cases or SMP safety. Boot firmware, toolchain, emulator and generated instructions remain trust boundaries. The [SMP contract](smp.md) limits participation to two CPUs and explicitly retained readers. No unsafe is justified by performance alone; the [method gate](../architecture/implementation-review.md) applies before acceptance.

Phase 2 adds INV-DEMO-ENTRY and INV-DEMO-SVC in the separate EL0 image: a private aligned guarded stack, fixed RX entry and explicit GPR clobbers for nonpointer native calls. ELF extraction rejects malformed extents and writable globals before selection. Native SVC handlers use current-task state with IRQ masked, retain no user references and collect bounded reports only with machine-events. [ADR-0015](../architecture-decisions/0015-el0-versioned-routing.md) records authority/necessity and actual tests; native dependency closure stays unchanged.

Phase 3.0 confines scheduler UnsafeCell access to preparation, mutation and inspection in local.rs. Observed CPU, masked IRQ, phase, generation and a nonblocking permit precede every dereference; higher-ranked closures prevent borrowed results. Acquired Done plus released final access precede inspection/reset, and native-root/TLBI completion precedes reclamation. Per-CPU scope/lock tracking rejects both lock directions; guards cannot cross CPUs. [Scheduler contract](scheduler.md) lists actual rejection controls and remaining gaps. No reference or permit survives ERET or waits. Context expectations are now entirely post-quiescent bootstrap verification.

## Phase 3.1 ownership delta

[Process lifecycle](processes.md) adds no unsafe storage or Sync implementation: Registry owns linear Frames in a non-Send/non-Sync Rust value. Admission borrows owned spaces through synchronous dispatch; acquired Done and final permit release precede root unlink and copied completion. local.rs keeps one Sync and three storage dereference sites using exclusive permits. INV-USER-SPACE, INV-USER-RETIRE and INV-RUNQUEUE include process generation, rollback and unlink-before-reclaim. New unsafe is confined to trusted linker-image slicing, retained post-quiescent tag inspection and a pointer-free CPU1 contract probe, under INV-USER-IMAGE, INV-USER-RETIRE and INV-REMOTE-READER. [Updated inventory](../../research/results/kernel-phase31-unsafe-audit.json) records this scope; model/coordinator add no unsafe. See [ADR-0017](../architecture-decisions/0017-process-lifecycle.md).

## Phase 3.2 copy delta

### INV-USER-COPY

**Necessity and owner:** EL0 permissions and precise EL1 abort containment require privileged AT and unprivileged byte assembly. The indexed executing CPU owns the synchronous copy; the native memory/architecture boundary owns review. [Access](../../crates/kernel/src/user_copy.rs), [permission queries](../../crates/kernel/src/arch/aarch64/mod.rs), [loops](../../crates/kernel/src/arch/aarch64/entry.S) and [decision](../architecture-decisions/0018-safe-user-copy.md) define the boundary.

**Preconditions and failure:** Checked non-null aperture, overflow/size bounds, every page's EL0 permissions, exact retained process/queue generation and root, fixed affinity, exclusive scheduler scope and masked IRQ. Immutable private mappings and Registry admission borrows exclude mutation/retirement until return. Initialized disjoint kernel byte storage has no user Rust references or struct/padding serialization. Only exact LDTRB/STTRB data-abort PCs with active guard, valid in-range FAR and matching direction recover; unrelated EL1 faults remain fatal. Input failure publishes no snapshot; output failure reports its written prefix. No allocation/ordinary lock/yield or reference across ERET. PROD retains enforcement without optional counters.

**Tests and limits:** [Actual EL0 checks](../../crates/kernel/src/user_copy/testing.rs) exercise two CPUs, range/permission failures, cross-page/maximum copy, snapshot mutation, terminal/unlinked identity and stale generation. Disabled-recovery/live-reread controls fail in DEV/PROD. Recovery preserves saved outer EL0 return state and restores EL1 registers. Existing native-root/TLBI completion and reclamation remain mandatory. No mutable/shared mappings, asynchronous exit, migration, DMA or silicon proof is claimed.

### INV-USER-COPY-TEST

**Necessity and owner:** Kernel-test-only injection skips page preflight on eight bounded bytes: four valid bytes then unmapped guard. The fixture owns initialized scratch and retains current process/root. Production assembly and recovery return four completed bytes in both directions; input suffix stays poisoned, peers survive and reclaim. This tests hardware fault containment, not a supported unmap race. Trusted immutable fixture slicing uses INV-USER-IMAGE. No new unsafe Sync or shared mutable process storage.

## Phase 3.3 reference delta

[Handles](handles.md) add zero production unsafe sites. INV-RUNQUEUE already excludes concurrent namespace access during linear moves and requires acquired completion before quiescent extraction. Synchronous Retained borrows prevent close/retire while used. Three test-only lexical sites use INV-USER-IMAGE for immutable linked symbols and two bounded fixture slices; ownership, checked image lengths and two-CPU negative execution remain required. The [inventory](../../research/results/kernel-phase33-unsafe-audit.json) records 109 lexical locations versus 106 at Phase 3.2. No raw resource pointer, UnsafeCell, unsafe Sync or unchecked index is added.
