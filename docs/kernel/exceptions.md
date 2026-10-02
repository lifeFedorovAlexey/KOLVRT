# Exception entry and recovery

The sixteen AArch64 vector slots dispatch synchronous faults, IRQ or fatal paths. Rust executes with a 16-byte-aligned stack. The 784-byte entry frame preserves all general registers, Q0–Q31, FPCR and FPSR. Pinned Rust runtime supports NEON; saving only general registers would corrupt interrupted code. A real timer test deliberately overwrites SIMD/FP state inside the IRQ and verifies restoration.

## Fault policy

PROD has no test recovery mechanism. Unexpected exceptions mask IRQ, stop the timer, publish failure where UART is available and invoke PSCI shutdown. DEV additionally reports ESR, FAR and ELR. Kernel-test builds permit recovery only at an exact registered probe PC for BRK, current-EL data abort or instruction abort. Tests check ESR fault class and FAR for actual RO write, unmapped read and NX instruction fetch.

Vectors precede memory protection tests. IRQ does not nest in this milestone. Each active CPU has its own permanent aligned stack and probe state. Stack overflow detection remains deferred. [Unsafe invariants](unsafe.md) describe assembly assumptions; [SMP scope](smp.md) defines the concurrency boundary.
