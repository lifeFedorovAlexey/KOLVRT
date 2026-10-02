# Native model

## Boundaries

The core owns task and address-space lifetimes, scheduling mechanisms, memory ownership,
authorization, communication and waiting primitives, resource accounting and necessary
device abstractions. The initial service is isolated at EL0 under [decision 0008](../architecture-decisions/0008-placement.md). Placement of file services, networking, device drivers and compatibility adapters requires later workload-specific decisions.
A library boundary alone does not establish memory isolation.

Native consumers use native contracts directly. Legacy consumers enter a versioned
adapter and then the same native contracts. Firmware parsers and hardware workarounds
implement platform contracts. The core must not import compatibility types, layouts or
implementation dependencies. Adapters do not receive private memory-manager, scheduler
or filesystem objects. Dependency checks must include generated code and build features.

## Objects and authority

An object identity is distinct from its displayed integer name. A handle carries checked
identity, type and rights, with protection against reuse. Copying or delegating it cannot
add rights. Revocation specifies separately whether it stops new requests and how it
affects accepted requests. The first-slice [wire candidate](native-abi.md) is versioned and tested.

Creation publishes a fully initialized object or nothing. Its lifecycle is
`Constructing -> Live -> Retiring -> Dead`; upgrading a weak reference cannot resurrect
a retiring object. Requests retain required references until terminal completion.
Closing consumes a handle once even when late I/O reports an error. Cancellation does
not promise rollback: the outcome must state whether an external effect occurred.

Evidence: [process identity](../../research/cases/KOL-PATH-0019.json),
[close outcomes](../../research/cases/KOL-PATH-0001.json),
[reference resurrection](../../research/cases/KOL-PATH-0030.json).

## Memory and I/O

Read-only shared pages, writable owned buffers and device-access leases have distinct
contracts. Copy-on-write does not grant write authority. Device access retains pinned
memory until completion or a proven device reset. Cache coherence does not replace
ordering barriers. Foreign pointers require bounds, initialization, alignment, lifetime,
aliasing, mutation and fault-handling guarantees before becoming ordinary references.
Validation and execution use the same owned request snapshot or a proven immutable view.

Native and compatibility clients of a shared file lock or endpoint use one authoritative
arbiter. Zero-copy transfer retains backing-page ownership and permissions. Write
acceptance, completion, durable storage and close are separate outcomes. Completion
order is not implicitly submission order.

## Time and errors

External timestamps use signed 64-bit seconds, nanoseconds in 0..999999999 and a clock
domain. Conversions detect overflow. Waiting defaults to monotonic deadlines; conversion
from a relative duration happens once. Clock changes are never implicit.

Unsupported operations, denied authority, invalid requests, exhausted resources,
cancellation and I/O failures have distinct results. Legacy error-number conversion
belongs to adapters. No error path reports fabricated success.

## Platform contract

The initial AArch64 backend must cover boot descriptions, EL1 exceptions, memory mapping,
translation invalidation, processor startup and shutdown, interrupt handles, deadlines,
device registers and direct memory access. Multiprocessor translation invalidation and
interrupt context belong to the initial contract, not a later repair to a single-CPU model.

Pin exact QEMU version, machine, CPU, accelerator, memory, processor count, GICv3 and
transports before platform experiments. Discover devices from Device Tree rather than
assuming stable addresses. See [QEMU virt](https://www.qemu.org/docs/master/system/arm/virt.html).
Emulation does not prove silicon errata handling, real timing, device coherence or
hardware-counter costs. Core contract tests need a mock backend and eventually another
architecture implementation; portability is not proven merely by having an interface.

[Russian translation](../../translations/ru/docs/architecture/native-model.md)
