# Threat and failure model

The first native slice protects task memory, object identity, authority, bounded resource use and truthful outcomes. Its trust base consists of boot input selection, privileged memory/exception/interrupt code, object and quota arbiters, the scheduler and the supervisor's grant policy. Safe language use does not remove this trust base.

## Boundaries

| Actor or failure             | Capability                                                                   | Required response                                                                                                                             |
| ---------------------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Malicious task               | Submit malformed bytes, guess handles, race close/revoke, exhaust its budget | Validate an owned snapshot; resolve only in the caller's table; serialize admission and rights; reject without partial allocation             |
| Faulty service               | Crash, omit replies, send malformed or duplicated replies                    | Isolated address space; bounded pending work; validate response identity; one terminal result; no automatic replay after unknown effects      |
| Confused deputy              | Present a legitimate handle for an operation exceeding delegated rights      | Check operation rights at admission; transfer can only attenuate rights and preserves object/binding identity                                 |
| Hostile input mutation       | Change user memory between checks                                            | Fault-safe bounded copy first, then validate and execute that snapshot; a host decoder alone does not implement user-copy                     |
| Resource exhaustion          | Fill queues, retain handles or starve cleanup                                | Charge before publication; retain charges until release; reserve supervisor cleanup capacity separately                                       |
| Privileged invariant failure | Corrupt trusted state, fail an internal assertion                            | Stop the affected kernel execution and report a fatal failure; do not continue or restart privileged state as if repaired                     |
| Untrusted firmware or device | Supply invalid descriptions or perform unauthorized DMA                      | Validate descriptions; device access requires isolation and a lease contract before enabling such a device; no DMA devices in the first slice |

## Recovery and resource policy

Task faults terminate the task and revoke future access from its domain. Accepted requests retain references until a terminal outcome; a dead service produces an effect-unknown failure unless absence of effects is established. Restart creates a new instance and generation with no inherited authority. Host process separation in the experiment demonstrates crash containment only: both processes run with the host user's permissions and are not a security sandbox.

Boot constructs dependencies before publishing admission. Shutdown reverses this order: stop admission, finish or fail requests, quiesce interrupt/callback sources, then reclaim mappings. Timeout does not authorize freeing memory still reachable by a device or privileged reader. Fail closed and keep that resource quarantined until reset or global stop establishes safety.

Allocation failure before admission is recoverable and leaves the old state unchanged. The first slice uses bounded pools; cleanup cannot require fresh unbounded allocation. Kernel panic policy is abort/halt, with no unwinding across exception, interrupt or foreign boundaries. Service crashes are recovered by the supervisor, not by catching an arbitrary privileged panic.

## Assumptions and exclusions

The selected emulator, compiler and trusted boot input are assumed to implement their contracts. Physical attacks, hostile firmware execution, microarchitectural side channels and compromised host administration are outside the first-slice assurance claim. These are explicit deployment exclusions, not universal safety claims. Debug logs must exclude raw user payload and privileged addresses by default. Future hardware deployment requires revisiting these assumptions, DMA containment, boot authenticity and device reset evidence.

[Russian translation](../../translations/ru/docs/architecture/threat-model.md)
