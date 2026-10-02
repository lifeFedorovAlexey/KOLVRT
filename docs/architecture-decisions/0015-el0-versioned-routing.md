# ADR-0015 — Versioned routing in isolated EL0 consumers

Status: **Accepted for the bounded Phase 2 demonstration**. Date: 2026-10-02.

## Context

The authorized Phase 2 workload requires concurrent native and versioned adapters, removable compatibility, real translation and measurements. [SMP](../kernel/smp.md) and [EL0](../kernel/el0.md) now provide the required execution foundation. Existing routing groundwork has not been connected to native execution. [ADR-0013](0013-security-boundaries.md) rejects privileged placement based only on convenience, global ownership or speed.

## Decision

Keep routing, version decoding, legacy layouts, switching policy and adapter counters in a separately built, boot-selected EL0 image. Native kernel Cargo closure remains kernel-core only. The native capability exposes an immutable snapshot of the current task's protected timer-service history and checked half-open window read. The current runqueue establishes identity; no operation accepts another task ID, root, pointer or authority-bearing handle. Capture is idempotent for that task lifetime.

The privileged mechanism is the caller attribution and access to kernel-owned execution observations produced during timer handling. EL0 cannot independently attest those observations or read protected task storage. Arithmetic runs in EL0 using the shared safe native window algorithm; this is not justification to move a general reduction service, routing policy or authorization database into EL1. Allocation and static task creation remain the reviewed bootstrap scope. Before a general service/capability model, reconsider placement and replace implicit own-task access with its native grant contract.

The two ordinary adapters translate inclusive LE16 endpoints with an empty sentinel and BE32 start/count into the one current native span. A separately selected bug adapter preserves only the safe synthetic zero-count-as-one behavior, still subject to native bounds and caller scope. It does not restore a security defect or claim Linux compatibility.

Each EL0 consumer exclusively owns its route, generation and transaction state. A synchronous transaction pins the binding. DEV rebinding is allowed only after its guard ends; a forgotten guard keeps admission blocked. Different consumers execute on both CPUs. No shared mutable routing table, CPU0 writer assumption, route IRQ work or global compatibility lock is introduced. Referenced code is immutable and resident until native quiescence and frame reclamation; runtime unloading is unsupported.

The scheduler replaces its one-session guard with an atomic Idle → Admitted → Running → Done admission protocol. A CAS claim rejects stale secondary polls during reset. CPU0 acquires Done on both CPUs, observes native-root/TLBI completion and zero active execution, then resets or reclaims. There is no overlapping batch, migration or shared queue mutation. Four guarded stack pages and bounded opaque RX image pages extend the previous one-page fixture layout.

Profiles pin schema, native contract, generation, exact routes and implementation source identities. DEV emits a profile and an independently supplied expected digest; build-time and EL0 validation reject invalid or unsupported bindings. SHA-256 is integrity, not a signature. Source identities are not executable signatures: retained manifests separately record exact user image and kernel ELF hashes, compiler, features and emulator. Authentic boot/package signature infrastructure remains unimplemented; trusted build/boot selection is an explicit assumption.

## Privileged responsibility review

The [admission policy](../architecture/kernel-admission-policy.md) applies per responsibility; the [threat model](../security/threat-model.md) records scope and compromise consequences.

| Field                   | Bounded Phase 2 disposition                                                                                                                                                                                                                                                                                                |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Consumer and invariant  | Real two-CPU EL0 consumers query only their own protected timer observations. Forged width, bounds or owner selection must not disclose other task/kernel state.                                                                                                                                                           |
| Observation mechanism   | Current-task attribution, timer-handler history and checked register read of its frozen capture. EL0 cannot read protected task storage; an EL0 service still needs this narrow query. A read-only published mapping would need a new mapping/lifetime primitive. Arithmetic is already moved to EL0.                      |
| Clock/slices mechanism  | Native counter access and protected own-task slice lookup. Direct EL0 counter access is a future clock alternative after reviewing timer controls; it cannot replace protected task attribution. Only ticks/frequency and own slice count are exposed.                                                                     |
| Evidence mechanism      | With machine-events only, store one caller scalar in its bounded protected buffer, then emit through protected UART after both CPUs stop. A future service/host channel can replace it; this is test instrumentation, omitted from stripped PROD. No arbitrary memory or foreign report access is granted.                 |
| Bootstrap mechanism     | Existing page-table/cache/TLBI/context primitives publish retained private RX mappings and guarded stacks. Host/EL0 policy selects the image/profile; kernel parses neither ELF nor legacy data. CAS admission excludes stale reset polls. General loading/allocation policy is not admitted.                              |
| Authority               | Native bootstrap task creation grants own observations only. Caller identity comes from the protected runqueue, never user registers. No delegation; revocation is task termination. A reused static ID creates fresh state only after acquired quiescence; no user authority reference survives the prior batch.          |
| Lifetime and failure    | Indexed CPU mutates task state with IRQ masked; capture is immutable. Both CPUs restore native roots and finish TLBI before reset/reclamation. Timeout does not establish quiescence or authorize freeing executing storage. No DMA participation.                                                                         |
| TCB and unsafe          | Native scope/bounds/page/context enforcement and trusted boot/compiler are in the integrity TCB. EL0 adapters, arithmetic and profile policy are outside privileged enforcement. Privileged compromise is fatal; adapter faults are private-task faults. Entry and SVC invariants are inventoried.                         |
| Profiles and validation | DEV/PROD retain scope, initialized replies, width/bounds, page protections and reclamation. Real concurrent workloads, malformed calls, profile failures, adapter containment, accounting corruption and native removal checks cover the recorded scope. Silicon, hostile boot and general availability remain unverified. |
| Reconsideration         | Revisit capture/publication, direct counter access, scoped inspection transport and grant/revocation during the native IPC/authority slice. No speed or global-owner argument can enlarge this mechanism.                                                                                                                  |

## Alternatives

Link adapters into EL1; add compatibility hooks to native dispatch; use one shared global route table; finish a full IPC/service stack before the bounded demonstration; retain host-only dispatch.

## Why rejected

Adapters and route selection require no privileged instruction and would enlarge the integrity TCB. Shared locking adds synchronization without shared state. The pure synchronous own-task observation needs neither cross-task IPC nor transferred capabilities. Host-only calls cannot prove real EL0 dispatch, SMP execution or containment.

## Consequences

Phase 2 is a real static vertical slice, not a package resolver, arbitrary ELF loader, personality service or completed native IPC slice. PROD fixes the profile at build/launch and has no rebind methods, route search or A/B code. Evidence builds add explicitly named conformance/reporting features; a stripped PROD image is separately built and booted. Native tests remain unchanged and run without the optional payload. See the [implemented contract](../kernel/routing.md).

## Compatibility impact

Old and new adapters coexist per consumer. Missing modules, invalid encodings and backend rejection fail explicitly. No silent downgrade, fallback or native semantic branch depends on adapter identity. Removal is a rebuild boundary; zero observed calls alone is not an unload authorization.

## Performance impact

Retain equivalent nonempty workloads, immutable native snapshots, marked warmup and raw timer samples. Consumers alternate route order to expose order effects. Observed median/p95/p99 are empirical TCG latency values; 128 samples per route do not certify tails or a statistically established winner. Source-boundary snapshots/conversions are logical work counts, not retired-instruction or physical-copy counters. Exclusive CPU attribution and saturation throughput remain unavailable. Investigate observed faster compatibility medians without adding artificial penalties.

## Security impact

Adapters cannot increase native-authorized effects: only the current task's bounded frozen observations can be read, with width/bounds validation repeated in EL1. No user pointers, writable kernel aliases, IRQ allocations or interrupted locks are added. Reports contain test observations rather than raw addresses or payloads. DEV reporting is bounded and absent from stripped builds. This establishes neither general grant/revocation policy, inspection authentication, DMA containment nor hostile boot resistance.

## Testing

QEMU runs native/v1/v2/bug consumers together, DEV and PROD profiles, feature-minimal adapter builds, native-only operation, transaction switching and rejection, accounting, profile versions/identity/integrity and controlled bug behavior. Negative controls corrupt a profile, fault one adapter and corrupt an accounting report. Host checks cover malformed images, report identity/chunks, every profile-byte mutation, provider denial and dependency direction. Native foundation tests and their existing failure controls are unchanged. Dependency audit and source guards reject core imports and compatibility conditionals; guards are bounded enforcement, not a proof against arbitrary generated code.

## Reversibility

The optional image can be omitted without changing native tests. A future service model may replace the experimental native call numbers and own-task snapshot interface. Stateful adapters, live shared publication, code unloading, migration or delegated authority require a new lifetime/security review and evidence.

## Evidence

[Kernel Laws](../architecture/kernel-laws.md) remain unchanged: LAW-001, LAW-003, LAW-004, LAW-005, LAW-008, LAW-009, LAW-013, LAW-018, LAW-025, LAW-035, LAW-036 and LAW-041 apply. Relevant failure mechanisms include [shared identity](../../research/cases/KOL-PATH-0002.json), [unsafe old behavior](../../research/cases/KOL-PATH-0007.json), [check/use mutation](../../research/cases/KOL-PATH-0018.json) and [retiring references](../../research/cases/KOL-PATH-0030.json). [Results](../../research/results/routing-phase2.json) retain execution observations and exact source inventories.

[Russian translation](../../translations/ru/docs/architecture-decisions/0015-el0-versioned-routing.md)
