# GICv3 and synchronization

Platform-validated DTB descriptors provide distributor and redistributor regions. CPU0 uses the first redistributor frame for the pinned affinity-zero topology. Initialization wakes it, configures nonsecure Group 1, level-triggered physical timer PPI 30, priority and system-register CPU interface while IRQ is masked. Register polling has a finite iteration limit.

## IRQ lifecycle

Acknowledge returns the real INTID. Timer service disables/deasserts the level source, increments an atomic delivery counter and issues EOI with EOImode=0, completing priority drop and deactivation. Architectural barriers complement volatile MMIO; volatile alone does not order device effects. Spurious IDs are ignored; unexpected implemented IDs take the fatal path. Tests verify masking, delivery and rearming, not a preinstalled boolean.

## Locking

The bounded TTAS lock waits through relaxed loads and uses Acquire CAS with Release unlock. Guard lifetimes protect UnsafeCell access; Send/Sync bounds follow the protected type. Host tests exercise publication with four concurrent threads. IRQ never takes this lock, allocates or writes UART. Thus interrupted code cannot deadlock against an IRQ that waits for its own lock. One million spin iterations is a failure bound, not a real-time guarantee or fairness proof. [Review](../architecture/implementation-review.md) records the alternatives.
