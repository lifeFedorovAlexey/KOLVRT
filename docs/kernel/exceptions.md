# Exception entry and recovery

The sixteen AArch64 vector slots dispatch synchronous faults, IRQ or fatal paths. Rust executes with a 16-byte-aligned stack. The 784-byte entry frame preserves all general registers, Q0–Q31, FPCR and FPSR. Pinned Rust runtime supports NEON; saving only general registers would corrupt interrupted code. A real timer test deliberately overwrites SIMD/FP state inside the IRQ and verifies restoration.

## Fault policy

PROD has no current-EL test recovery mechanism. Unexpected kernel exceptions mask IRQ, stop the timer, publish failure where UART is available and invoke PSCI shutdown. DEV additionally reports ESR, FAR and ELR. Kernel-test builds permit current-EL recovery only at an exact registered probe PC for BRK, data abort or instruction abort. Tests check ESR fault class and FAR for actual RO write, unmapped read and NX instruction fetch.

Lower-EL AArch64 synchronous/IRQ vectors use an 816-byte frame, extending the base with ELR, SPSR, SP_EL0 and TPIDR_EL0. The [EL0 scheduler](el0.md) records a user fault as a terminal process outcome and resumes another runnable process; it does not convert user register contents into privileged authority. Returning to the kernel restores its saved ABI and native root. Compile-time frame assertions and actual GPR/SIMD/FP/TLS/stack checks cover this path in both profiles.

Vectors precede memory protection tests. IRQ does not nest in this milestone. Each active CPU has its own permanent aligned stack and probe state. Stack overflow detection remains deferred. [Unsafe invariants](unsafe.md) describe assembly assumptions; [SMP scope](smp.md) defines the concurrency boundary.
