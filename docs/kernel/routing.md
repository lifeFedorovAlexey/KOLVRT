# Phase 2 routing execution contract

Document status: CURRENT
Evidence scope: bounded optional EL0 routing on the verified two-CPU QEMU foundation; native IPC/services remain incomplete.
Current reference: [Admission and evidence](../architecture-decisions/0015-el0-versioned-routing.md)

The bounded Versioned Routing & Translation slice runs in eight real EL0 processes on two CPUs. Different consumers concurrently select native, inclusive-v1, counted-v2 and the safe synthetic bug adapter. The native kernel depends only on kernel-core and does not decode legacy formats, select compatibility versions or dispatch to adapters. [ADR-0015](../architecture-decisions/0015-el0-versioned-routing.md) records placement, ownership, security and limits.

## Architecture and repository boundaries

```text
xtask: validate profile → build selected user image → validate ELF → select opaque boot payload
CPU0 supervisor: create private roots/stacks → release admission
CPU0 EL0 consumers: native / inclusive-v1 / counted-v2 / empty-first bug
CPU1 EL0 consumers: native / inclusive-v1 / counted-v2 / empty-first bug
                    ↓ private route + synchronous transaction
                    ↓ adapter converts to current native half-open Span
                    ↓ native SVC; current runqueue supplies task identity
EL1: own execution-accounting snapshot → checked window read → initialized register reply
EL0: shared native window reduction → sum/word count
Both CPUs: native root + completed TLBI → release Done
CPU0: acquire both Done → inspect bounded reports → drop charges → reclaim frames
```

```text
crates/kernel-core/src/execution.rs     native experimental operation contract
crates/kernel-core/src/window.rs        native checked algorithm and Backend boundary
crates/kernel/src/execution.rs          protected own observations and checked register reply
crates/kernel/src/scheduler/mod.rs          current-task attribution, SVC, admission
crates/kernel/src/memory/mod.rs         retained immutable RX image and guarded stacks
crates/routing/src/lib.rs               profile, versions, private binding and transactions
crates/routing/src/conformance.rs       behavior and rejection fixtures
crates/window-compat/src/lib.rs         optional legacy conversion and safe bug behavior
crates/routing-demo/                    separately linked no_std EL0 image
crates/xtask/src/routing_demo.rs         build, inspection, measurement and validation
```

```text
native kernel → kernel-core
EL0 demo → routing → kernel-core
                  → window-compat [optional, feature-selected] → kernel-core
host xtask → routing + kernel-core + evidence tools
native kernel ↛ routing or window-compat
```

The four-entry profile is a consumer template instantiated independently on each CPU. Runtime consumer identity is its native task ID, not the template slot or a caller-supplied selector. Each instance owns its private Rust Consumer and stack. This is not a global mutable route table.

## Native capability and authority

Timer handling records bounded own-task service intervals as u32 timer ticks, saturating at u32::MAX. Capture freezes eight observations once; repeated capture cannot replace that snapshot. The kernel accepts checked u32 half-open endpoints and returns at most eight own observations in initialized registers; EL0 performs the shared native reduction to sum/word count. No user memory is dereferenced, no foreign owner is accepted, and no raw root or privileged address is returned. Empty native spans return zero words; invalid widths, order or bounds fail before reading data.

Only privileged code can attest its kernel-owned scheduling observations and attribute the currently executing task. EL0 performs translation and route policy. The bootstrap grants each static task access only to its own observation capability for that lifetime. This is a bounded native authority contract, not a general handles/capabilities or security-domain implementation. Adapters cannot broaden it. All routes call the same backend; unsupported encoding or native rejection never triggers fallback.

Each adapter consumes one immutable owned record snapshot. v1 converts LE16 inclusive endpoints, including the all-ones empty sentinel; v2 converts checked BE32 start/count. The opt-in bug adapter maps zero count to one owned word. Its controlled demonstration differs from the corrected empty native result without bypassing bounds or exposing another task's data. It preserves a safe synthetic behavior, not an actual historical vulnerability.

## Ownership, switching and retirement

| State                             | Owner and mutability                                              | Publication/lifetime rule                                                                                        |
| --------------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Validated profile and user code   | Trusted build/boot selection, immutable                           | Exact image/profile hashes retained; no loader or runtime code replacement                                       |
| Binding and route counters        | One EL0 Consumer, exclusive mutable borrow                        | No shared references, IRQ access or CPU0 writer; different consumers run on different CPUs                       |
| Transaction and generation        | Private consumer, synchronous backend calls                       | Guard pins binding; DEV switch only after completion; forgotten guard blocks admission; generation cannot wrap   |
| Native history and frozen capture | Current task's indexed CPU, IRQ masked                            | No caller-selected owner; captured values immutable; no Rust reference survives ERET                             |
| Queue reset and completion        | CPU0 prepares; each CPU CAS-claims its admitted batch             | Idle → Admitted → Running → Done; acquire both terminal phases before reset; stale polls cannot claim idle state |
| Tables, image pages and stacks    | CPU0 Frame/UserSpace ownership; owning process uses private stack | Charges survive forgotten guards; both CPUs restore native root and complete TLBI before reclamation             |

State-free synchronous reduction permits DEV transaction-boundary switching. There is no state migration, live module unloading, shared-object rebinding or experimental PROD switching. A stateful capability requires its own transition proof or restart boundary. Four stack pages plus an unmapped guard and at most 128 KiB RX payload are explicit bootstrap limits. Timer quantum remains the typed native scheduler setting; payload execution has bounded slice/deadline admission. No IRQ allocation or new queue/global routing lock is used.

## DEV API and production profiles

```text
cargo xtask routing test
cargo xtask routing status
cargo xtask routing inspect 1
cargo xtask routing compare 1
cargo xtask routing top
cargo xtask routing profile native,inclusive,counted,bug target/routing-profile.bin
cargo xtask routing validate target/routing-profile.bin EXPECTED_SHA256
```

Inspection reads retained test observations; it is not a remotely exposed privileged inspector. Consumer::switch and Transaction::try_switch are the real DEV API exercised in EL0. Text labels remain understandable without color. LEGACY/COMPAT/MIXED/MOSTLY_NATIVE/NATIVE classification remains the reviewed multi-metric model; no invented scalar score is added. A zero admission denominator remains unknown. Report benchmark admissions separately from bound consumer traffic; native backend calls are not a second external native admission.

Profiles encode schema/native version, nonzero generation, explicit consumer entries and implementation source identities. Configure a separately built image with KOLVRT_ROUTING_PROFILE and independently trusted KOLVRT_ROUTING_DIGEST. Both build and EL0 reject integrity, identity, unsupported module and version errors. The digest is not a signature or operator authorization; build/boot authenticity is not implemented. Exact executable hashes are separate from semantic versions and source identities.

PROD contains only the feature-selected adapters and pinned profile. It has no rebind API, route experimentation or DEV counters. Evidence images explicitly add conformance and bounded reporting; the stripped image omits those fixtures and A/B code. User REPORT collection and its storage are compiled out of the native kernel without machine-events. Correctness checks, authority, bounds, page protection and lifetime remain. No runtime module unload is claimed; unused code is removed at a validated rebuild boundary.

## Measurements and verification

[Routing results](../../research/results/routing-phase2.json) preserve user/kernel artifact hashes, profile bytes/digests, actual events, native oracle, raw marked warmup and measurement samples. Six evidence configurations cover DEV/full, PROD/full, PROD/v1, PROD/v2, PROD/bug and PROD/native. A separate stripped PROD/v1 boot excludes diagnostic features. Three controls detect corrupted profiles, a contained adapter fault and corrupted accounting. [Native results](../../research/results/kernel-phase2.json) preserve the unchanged 53-test DEV/PROD foundation matrix and eleven existing controls. [Unsafe inventory](../../research/results/kernel-phase2-unsafe-audit.json) is an inventory rather than a proof.

Each benchmark uses the same frozen native data and equivalent nonempty useful result for all routes. Sixteen warmup samples and 128 measured samples per route are retained. Different consumers alternate forward/reverse route order. The KVR3 report retains wall, EL0 consumer and native-service counter deltas for each measured call. EL0 consumer ticks bracket Consumer::call_with(); native-service ticks measure only the READ_WINDOW function body around native_call. Their per-sample sum is an exclusive estimate for these two measured regions. It excludes shared SVC entry/return, the CLOCK service, scheduling, interrupts and other kernel work, so it is not total route CPU. The process result separately records cumulative READ_WINDOW service ticks, including calls outside measured samples. Source-boundary copies/conversions count logical translation work, not physical memory-copy instructions; resident pages are a kernel frame charge, not total physical memory. Timer preemptions remain recorded. The fixture has no installed-package identity or production catalog.

Nearest-rank median/p95/p99, mean and sample variance describe those observations. Warmup is fixed, not evidence of stabilization; eight concurrent fixtures are not independent hardware trials. Tail confidence and a fastest-path claim remain inconclusive. A faster compatibility median is retained for native-path investigation, with no artificial penalty. Emulator observations do not establish physical hardware throughput, side-channel protection or hard real-time progress.

Native-only builds run the same foundation suite. A separate pruned-workspace check builds without routing/adapter source packages. Cargo dependency closure checks all native features; source guards reject known compatibility imports, legacy input types and branches. Negative fixtures exercise those guards, malformed ELF bounds, missing/duplicate/foreign reports, each profile-byte mutation, provider denial and absent PROD switching. Generated code and arbitrary future identifiers still require review.

[Physical source-removal evidence](../../research/results/native-compat-removal.json) verifies 35 identical native/kernel-core/harness source files after deleting routing, window-compat and routing-demo packages; only workspace/tool dependency composition and the lockfile were pruned. The same 53 tests in each profile and eleven host controls pass. [Performance investigation #15](https://github.com/lifeFedorovAlexey/KOLVRT/issues/15) tracks eight lower compatibility-median observations in the retained DEV run, with uncertainty and follow-up in the [investigation record](../../research/results/routing-performance-investigation.json). This does not establish a faster hardware path.

## Preserved groundwork and next candidates

Preserved pure model: Route, semantic names, Profile schema/version/digest, immutable configuration, dependency rules, status definitions and native Span/Reduction. Preserved runtime candidates: Consumer/Transaction, dispatch and adapter conversion. Their backend boundary now calls the actual native capability from EL0; local providers remain conformance fixtures. No useful groundwork was rolled back. The old “no multi-CPU access” restriction becomes exclusive per-consumer ownership, while distinct consumers run concurrently. The old one-page image/stack and single boot session are replaced by bounded retained payloads and CAS admission.

No native → compatibility hook was connected. Routing runtime is connected outside native core. IPC, handles/capabilities, cancellation, services, security domains, arbitrary loaders, shared state-domain migration, signatures and dynamic unloading remain later work. Phase 3 candidates are the native IPC/authority slice, then a concrete service consumer; device/DMA or Linux personalities require independent contracts and evidence. Do not infer a global production security policy from this demonstration.

The later [Phase 3.0 scheduler](scheduler.md) separates runtime, architectural context and bootstrap verification, with checked generations and per-CPU storage permits. This changes no adapter binding or native observation semantics. [Regression results](../../research/results/routing-phase3-regression.json) cover the current scheduler sources; original Phase 2 measurements above remain historical. The native foundation now also tests repeated queue reuse without reboot.

## Phase 3.1 lifecycle regression

The optional image now uses the [native process lifecycle](processes.md) for owned creation, explicit start, exact completion and quiescent reclaim. Process and dispatch generations are distinct; adapter/profile generations remain EL0-private. [Current regression](../../research/results/routing-phase31-regression.json) verifies the existing route/profile/isolation behavior. No routing choice or legacy state enters Registry; earlier records retain their exact historical sources.

[Russian translation](../../translations/ru/docs/kernel/routing.md)
