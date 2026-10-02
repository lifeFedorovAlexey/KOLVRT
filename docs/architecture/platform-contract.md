# First platform contract

The design target is AArch64 EL1 with EL0 tasks, little-endian data, 4 KiB translation granules and two processors. CPU state and page-table permissions separate privileged code from tasks; writable pages are non-executable and executable pages are not writable. User faults terminate the task; privileged faults halt rather than resume unknown state. These are implementation obligations, not observations from a running kernel.

## Reproducible target

| Item                | Selected baseline                                               | Verification                                                        |
| ------------------- | --------------------------------------------------------------- | ------------------------------------------------------------------- |
| Compiler            | Rust 1.99.0, commit b940084d7eb6a299eb4bfeb8e34901bc051e7ac4    | Installed and executed                                              |
| Compiler backend    | LLVM 23.1.1 bundled with that compiler                          | Reported by rustc -vV                                               |
| Target              | aarch64-unknown-none                                            | Candidate no_std protocol checked successfully                      |
| Linker              | rust-lld bundled with the same pinned compiler                  | Kernel linking is not yet exercised                                 |
| Emulator design pin | QEMU 10.1.0, virt-10.1, cortex-a57, TCG, 2 CPUs, 256 MiB, GICv3 | Versioned source documentation read; emulator is not installed here |
| Optional devices    | No PCIe, disk, network or DMA consumer in the first slice       | Activate only with a device contract and tests                      |

The emulator pin is a reproducible design reference, not a recommendation for exposing an old emulator to hostile guest code. Before execution, verify the selected binary's provenance and compatibility or explicitly revise the pin. Do not silently substitute `virt`, `max`, host acceleration or another version. Emulator installation and real boot validation belong to implementation, and are recorded as not executed.

## Hardware-facing obligations

| Boundary            | Required contract before publishing tasks                                                                                                            |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Boot description    | Validate the complete device-tree extent, offsets, alignment, string termination and reserved regions before using any address                       |
| Memory              | Distinguish device and ordinary memory; complete required page-table publication and translation invalidation before reuse                           |
| Multiple processors | Bring up each processor with its own exception stack; acknowledge invalidation on all participating processors before freeing a mapped page          |
| Interrupts          | Initialize distributor, redistributors and CPU interfaces before enabling delivery; identify, service and deactivate the source in the correct order |
| Timer               | Read the architectural frequency, use a monotonic epoch and checked conversions; rearm or mask the source before completing a level interrupt        |
| Shutdown            | Revoke entry, quiesce interrupts and deferred callbacks, then release stacks and mappings; a timeout cannot stand in for quiescence                  |

No exact register sequence or barrier recipe is approved by this document. Implementation must attach the relevant architectural clauses to each unsafe boundary and test ordering on the actual backend. The finite host transition models assume their transitions are atomic; they are not a weak-memory model or a hardware proof.

## Source basis and limits

- [QEMU 10.1.0 board documentation](https://raw.githubusercontent.com/qemu/qemu/v10.1.0/docs/system/arm/virt.rst): selected CPU and GIC configuration, versioned board behavior and device-tree discovery. This supports a target configuration, not measured performance.
- [Device Tree specification v0.4, flattened format](https://raw.githubusercontent.com/devicetree-org/devicetree-specification/v0.4/source/chapter5-flattened-format.rst): binary structure and header bounds for the future parser.
- [Arm memory ordering guide 102336_0100_01_en](https://documentation-service.arm.com/static/62a304f231ea212bb662321d): sections 2, 5, 6, 8, 10 and 11 distinguish ordering, completion, scope and instruction synchronization. The guide motivates explicit obligations, not a universal barrier instruction.
- [Arm GICv3/v4 software overview](https://documentation-service.arm.com/static/5f1068720daa596235e7f6ef): interrupt states and software handling. The overview is not a substitute for register-level architecture clauses during implementation.
- [Rust bare-metal AArch64 target](https://doc.rust-lang.org/rustc/platform-support/aarch64-unknown-none.html): no_std target support. Actual compiler identity is pinned above because the online page can change.

The Arm timer tutorial endpoint was inaccessible during this review. Its text is not claimed as reviewed. Timer register programming, page-table instructions and real interrupt execution remain explicit implementation review gates; the design contract does not invent their details.

[Russian translation](../../translations/ru/docs/architecture/platform-contract.md)
