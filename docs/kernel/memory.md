# Memory ownership and mappings

The safe `kernel-core` bitmap pool has one owner, checked extents and a free count. The physical instance contains 65,536 units of 4 KiB, with 8 KiB bitmap metadata. Boot RAM through kernel end, DTB, stack, tables and firmware reservations are excluded before allocation. Allocated RAM is zeroed before publication. A private-address, non-copyable Frame carries ownership; Mapping borrows it. Release rejects retained dynamic mappings, including a forgotten mapping guard.

## Address model

TTBR0 covers a 39-bit identity address space; TTBR1 is disabled. RAM has 4 KiB pages: text is RO executable, rodata RO NX, other RAM RW NX. Dynamic data mappings occupy 0x80000000 through 0x801fffff, with 512 slots. Device regions use sparse 2 MiB Device-nGnRnE blocks and NX. Device block coverage can include adjacent registers; this is an EL1 platform aperture, not a userspace protection boundary.

Map rejects misalignment, W+X, executable dynamic data, kernel aliases, out-of-range addresses and occupied slots. Unmap retains a retiring frame charge, clears the descriptor, completes local TLBI and waits for CPU1 reader quiescence plus acknowledged local TLBI/barriers before ownership can return to the pool. Full invalidation has a measurable cost; targeted invalidation requires separate review. The privileged identity alias remains; ownership governs access after release.

## Heap

The [EL0 foundation](el0.md) additionally retains six-page UserSpace allocations. Borrowed frames plus persistent per-space charges protect table/code/data/stack lifetime even if a guard is forgotten. Roots are immutable while admitted, user aliases cannot access kernel mappings, and both CPUs restore the native root with local TLBI before charges can be removed. Native mapping mutation fails while the user batch is active; this is a scoped admission rule, not permanent CPU0 ownership for future workloads.

The heap permanently owns 16 aligned physical pages: 64 KiB, 64-byte allocation quantum, 128-byte bitmap. Allocation is bounded, checks alignment and returns null on exhaustion. A lock protects allocation metadata; IRQ handlers never enter the allocator. Deallocation requires the original live pointer and exact Layout, as required by GlobalAlloc.

Contiguous allocation can fail under fragmentation despite a positive free count. Quantization wastes up to 63 bytes per rounded allocation, with additional alignment fragmentation. This bounded foundation is not a claim to be the fastest allocator for future workloads. [Decision review](../architecture/implementation-review.md) compares alternatives. Tests exercise exhaustion, reuse, collision, actual translation and permission faults.

## Multicore retirement

Physical/Frame ownership cannot be transferred or shared; affinity checks enforce CPU0 allocation and PTE mutation. This is the present workload boundary, not a permanent allocation policy. Existing heap metadata locking supports ordinary code on both CPUs. New mappings fail while a retirement is pending. A forgotten Retirement guard retains its charge; release rejects it even after the PTE is cleared. Table locks are released before waiting. Timeout/failure is fatal and never allows reuse. The [SMP contract](smp.md) defines publication, reader assumptions and identity-alias limits.
