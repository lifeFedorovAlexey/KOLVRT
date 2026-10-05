# GICv3 and synchronization

Platform-validated DTB descriptors provide distributor and redistributor regions. Each CPU scans the bounded region and matches GICR_TYPER affinity to its MPIDR; unsupported VLPI stride or absent affinity fails. Initialization wakes it, configures nonsecure Group 1, level-triggered physical timer PPI 30, priority and system-register CPU interface while IRQ is masked. Register polling has a finite iteration limit.

## IRQ lifecycle

Acknowledge returns the real INTID. Timer service disables/deasserts the level source, increments an atomic delivery counter and issues EOI with EOImode=0, completing priority drop and deactivation. Architectural barriers complement volatile MMIO; volatile alone does not order device effects. Spurious IDs are ignored; unexpected implemented IDs take the fatal path. Tests verify masking, delivery and rearming, not a preinstalled boolean.

IRQ mask/unmask wrappers retain a compiler memory clobber, preventing compiler reordering of protected accesses across the DAIF transition. ISB synchronizes instruction execution after that transition; it does not provide hardware ordering of memory accesses. Hardware memory ordering requires an appropriate DMB or atomic ordering at the publication/ownership boundary. Timer probe timeouts remain failures after the existing 500 ms bound. Failure diagnostics read the local physical timer control/compare/counter, entry IRQ mask state and affinity-matched GICR enable/pending/active bits without acknowledging an interrupt or changing controller state. These observations distinguish source state from actual delivery; no pending bit substitutes for the delivery counter.

## Locking

The bounded TTAS lock waits through relaxed loads and uses Acquire CAS with Release unlock. Guard lifetimes protect UnsafeCell access; Send/Sync bounds follow the protected type. Host tests exercise publication with four concurrent threads. IRQ never takes this lock, allocates or writes UART. Thus interrupted code cannot deadlock against an IRQ that waits for its own lock. One million spin iterations is a failure bound, not a real-time guarantee or fairness proof. [Review](../architecture/implementation-review.md) records the alternatives.

## Multicore delivery

CPU0 initializes the distributor once. Each CPU initializes only its own redistributor, timer PPI and named coordination SGI. SGI target encoding preserves affinity levels and checks the supported target range. IRQ records actual delivery and pending TLB generation; ordinary code acknowledges retirement after readers finish and local TLBI completes. No IRQ takes table/heap locks or writes UART. Real tests cover both IPI directions, repeated delivery, independent timers and Acquire/Release shared-pair publication. See [SMP](smp.md).

Lower-EL timer delivery requests bounded deferred selection after source stop and EOI. The [EL0 switch](el0.md) runs with IRQ masked, takes no queue locks and allocates nothing; SGI delivery alone does not consume a timer slice.
