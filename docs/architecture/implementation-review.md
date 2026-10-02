# Implementation decision gate

Performance tests alone do not justify an algorithm. Before accepting a new method, define its workload, correctness invariants, ownership, concurrency, failure outcome and resource limits; compare plausible alternatives; identify relevant historical failure mechanisms; then inspect the implementation and measure the real path. Prefer the fastest reviewed method that satisfies those reliability constraints. A speed claim needs comparative evidence, not familiarity with another OS.

This gate implements LAW-040, LAW-036 and LAW-041. A conflict with a Kernel Law stops that decision and requires an ADR; neither a temporary workaround nor a planned future fix grants an exception. No new law is added merely to meet a count.

## Current choices

### Bitmap physical pool with free count and single-page cursor

**Alternatives and cost:** Free-list gives constant-time single-page allocation but complicates aligned contiguous ownership; buddy improves splitting/coalescing but adds order metadata. Bitmap uses 8 KiB, bounded scans, O(1) exhaustion check; cursor avoids repeated scanning of an allocated prefix.

**Reliability review and acceptance boundary:** Fixed 256 MiB workload, one owner, reservations, zeroing, checked extents, reuse and exhaustion tests. Contiguous fragmentation is explicit. No measured fastest-allocator claim.

### Fixed 64 KiB heap with 64-byte quantum

**Alternatives and cost:** Slabs improve small-object density but require size classes; segregated fit adds splitting/coalescing invariants. Current bitmap has 128-byte metadata and finite scan bounds.

**Reliability review and acceptance boundary:** Exact Layout on free; null exhaustion; real Box/Vec tests. Quantization and fixed capacity are visible costs. Reassess for a concrete growing workload.

### Static 4 KiB RAM page tables

**Alternatives and cost:** 2 MiB blocks reduce table cost but cannot express section W^X boundaries; dynamic tables save unused memory but add fallible bootstrap/rollback.

**Reliability review and acceptance boundary:** Static table storage is 133 pages (532 KiB); no table allocation in map. Real RO/NX/unmap faults verify permissions.

### Acknowledged full local TLBI after mapping changes

**Alternatives and cost:** Per-VA invalidation reduces unrelated translation loss.

**Reliability review and acceptance boundary:** Conservative completed ordering and explicit remote reader acknowledgement before reuse; bounded two-CPU workload. Targeted optimization requires its own negative tests and comparative measurements.

### TTAS Acquire/Release lock

**Alternatives and cost:** TAS repeatedly writes the held cache line; ticket/MCS add queue storage and fairness protocol.

**Reliability review and acceptance boundary:** Relaxed read waiting reduces attempted writes; guard bounds reviewed; four-thread host publication check. IRQ has no lock dependency. Fairness and multicore kernel performance are unproven.

### Complete SIMD/FP exception frame

**Alternatives and cost:** A soft-float target could avoid this save cost, but requires a distinct compiler/runtime ABI audit.

**Reliability review and acceptance boundary:** The pinned target permits NEON. Omitting state is a correctness failure; real IRQ clobber control validates restoration. Frame cost is 784 bytes.

The accepted foundation is a bounded implementation, not a declaration that every chosen algorithm is globally fastest. Current TCG timing validates observation, not algorithm ranking. Before changing capacities or enabling concurrency, repeat this review with the affected workload and comparison data.

## Historical failure checks

[Dirty COW](../../research/cases/KOL-PATH-0007.json) separates authority to write from a transient fault outcome. KOLVRT has no COW or forced-write bypass; descriptors and real faults check rights. Future COW must review ownership afresh rather than inheriting a workaround.

Initialization, retirement, device ordering and progress are treated as independent obligations: zero frames before publication; retain ownership until unmap/TLBI completes; do not equate volatile with architectural barriers; never allocate or acquire interrupted locks inside IRQ. Missing these obligations is rejected even if the happy-path benchmark improves. The existing [case database](../research/case-database.md) remains evidence for mechanisms, not a list of features to reproduce.

## Implementation acceptance

Inspect generated features and PT_LOAD permissions, boundary arithmetic, alias lifetime, register ABI, IRQ source/EOI ordering, failure propagation and DEV/PROD semantic equivalence. Keep raw measurements and negative controls. Automated unsafe inventory locates boundaries; it does not prove them. Record material choices in [ADR-0010](../architecture-decisions/0010-kernel-foundation.md) and policy in [ADR-0011](../architecture-decisions/0011-method-review.md). Scheduler, userspace and compatibility remain outside the authorized milestone.

The [multicore decision](../architecture-decisions/0012-multicore-retirement.md) reviews PSCI publication, CPU ownership, affinity-correct interrupts, ordinary-code acknowledgement and quiescent shutdown. Real SMP tests supplement host lock evidence; no new global lock or speculative reclamation algorithm is needed.

The later [EL0 decision](../architecture-decisions/0014-el0-foundation.md) accepts fixed-affinity queues, immutable roots, full local flushes and complete context preservation. Pure selection is separated from privileged switching. User faults terminate only their process; ownership charges survive forgotten guards. IPC and compatibility remain deferred.

The later [Phase 2 decision](../architecture-decisions/0015-el0-versioned-routing.md) keeps routing and legacy conversion in EL0, admits only current-task access to protected execution observations, and replaces one-session admission with a CAS protocol. Allocation, bounds, native root/TLBI completion and charges remain mandatory. No fastest-path or general authority-policy claim follows.

The later [scheduler ownership decision](../architecture-decisions/0016-scheduler-ownership.md) rejects a global scheduler lock and comment-only shared-mutable assumptions. It separates immutable setup, owned mutation and copied completion, with nonblocking CPU/IRQ/generation/phase checks and actual DEV/PROD rejection controls. It adds no EL1 service policy and makes no speed claim; future dynamic lifecycle must preserve the checked boundary.

## Phase 3.1 process review

The [process decision](../architecture-decisions/0017-process-lifecycle.md) compares retained generations with reused integers, process identity with session identity, linear spaces with raw self-references and conservative retirement with speculative concurrent reclaim. No unsafe process storage/global lock or supervisor/authority/routing policy is added. [Lifecycle](../kernel/processes.md) records rollback, ordering, optional bounds and limits; the DEV/PROD matrix and the physical compatibility-removal matrix pass with 65 tests per profile and 53 negative controls. No performance ranking follows.

[Russian translation](../../translations/ru/docs/architecture/implementation-review.md)
