# Native model

This is a design model; the current kernel foundation does not implement the complete EL0 contract. Global authority alone does not justify EL1: require [privileged-necessity evidence](kernel-admission-policy.md). Service-private capabilities enable only native-authorized caller effects.

## Boundaries

The core provides irreducible privileged enforcement for task/address-space lifetimes,
scheduling, memory protection, authorization and bounded communication/waiting. Ownership,
authoritative state and resource coordination do not automatically belong in EL1: an EL0
service can be their arbiter using narrow existing primitives. Each new privileged
responsibility, including a device abstraction, requires the admission record in the
[kernel admission policy](kernel-admission-policy.md); only its demonstrated minimum
mechanism is admitted, while service policy defaults to EL0. The initial service is isolated at EL0 under [decision 0008](../architecture-decisions/0008-placement.md). Placement of file services, networking, device drivers and compatibility adapters requires later workload-specific decisions.
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

## Capability-type admission

A capability type defines an authority contract, not a typed ioctl or protocol opcode.
Before adding one, record an ADR-reviewed admission with the fields below. Driver-specific
convenience, a new command encoding or an implementation class does not establish new
authority. Conversely, do not merge unrelated authority models into a universal capability
with arbitrary opcodes/payloads. Operations and their authorized effects remain finite
and bounded; payload contents never mint rights or select an unchecked authority path.

| Record field                | Required justification                                                                                                                |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Named workload              | Actual consumer, resource and useful operation requiring authorization.                                                               |
| Authority distinction       | Specific difference in resource/operation authority, scope, delegation/attenuation, revocation or lifetime; not just a protocol name. |
| Existing bounded interfaces | Enumerate existing types/operations and explain precisely why they cannot safely express the requirement; if they can, reuse them.    |
| Issuer and scope            | Who grants authority, caller/object/effect limits, quota owner and denied cases; no compat-created missing rights.                    |
| Delegation                  | Permitted recipients, attenuation and nondelegable rights; transferred identity cannot be replaced by receiver defaults.              |
| Revocation and lifetime     | Admission linearization, accepted-work semantics, retained references, close/reuse and restart behavior.                              |
| Threat and failure          | Forgery, confused deputy, payload/opcode bypass, escalation, resource exhaustion and compromise containment.                          |
| Validation and limits       | Negative authority/delegation tests, actual execution scope, unsupported cases and remaining implementation gaps.                     |

Review against LAW-009 and LAW-013 under
[ADR-0013](../architecture-decisions/0013-security-boundaries.md). This gate does not
admit a universal object model or privileged implementation; placement has its own gate.

### Worked decision: candidate bounded echo endpoint

The named workload is the existing first-slice host echo fixture: a caller sends at most
256 bytes to one bound endpoint and optionally delegates its handle. The candidate has
one bounded SEND operation with SEND and TRANSFER rights. A second "echo-driver" type
for the same resource, effects, scope and lifetime is rejected: the existing endpoint
interface already expresses the requirement. A different wire layout belongs to protocol
versioning, not a new authority type. Arbitrary driver-command passthrough is also rejected:
it would introduce effects absent from the endpoint grant and cannot be justified by an opcode.

SEND does not authorize delegation; TRANSFER permits only a subset of held known rights.
Transfer preserves object and binding identity; receiver capacity failure or denied rights
must leave both domains unchanged. Closing the sender handle does not revoke an independently
delegated handle; generations prevent a stale sender handle from naming a replacement.
General descendant revocation is not implemented by this host fixture.

The [decoder and attenuation checks](../../crates/native-protocol-model/src/lib.rs)
reject every unsupported operation and unknown rights bit. The
[host domain tests](../../crates/native-state-models/src/lib.rs) check transactional
rejection, attenuation and preserved identity. A decoded request is not authorization:
caller-local lookup, required SEND rights and admission serialization still belong to
the executing endpoint. These are candidate/host checks, not a kernel capability issuer,
typed device authority, real IPC enforcement or proof of general revocation. Those need
separate workload decisions and integration tests before implementation claims.

## Common-object admission and deferral

Task, endpoint, address-space/memory lease and device primitives remain narrow.
The lifecycle sketch above expresses ownership obligations, not a universal base
class or executable state machine. Sharing checked arithmetic, generation validation
or bounded accounting does not imply interchangeable authority or destruction.

Before proposing a common object model, record at least two distinct concrete
consumer workloads, their current narrow interfaces and the exact proposed shared
mechanics. Compare authority, delegation/revocation, lifetime, effects and failures;
include counterexamples, rejection tests, implementation scope and the option to
defer. Reuse only demonstrated common mechanics while preserving resource-specific
guarantees. Review under LAW-009/LAW-013 and
[ADR-0013](../architecture-decisions/0013-security-boundaries.md); type admission
alone cannot approve a universal object abstraction.

| Dimension             | Bounded echo endpoint host workload                                                    | Private EL0 worker address space                                                                                           |
| --------------------- | -------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Existing interface    | Candidate SEND/TRANSFER, caller-local generation-checked handle and owned payload      | Frame allocation and UserSpace guards in the bounded EL0 boot workload                                                     |
| Authority             | SEND permits bounded endpoint effects; TRANSFER only attenuates known rights           | Page permissions isolate worker code/data/stack; allocation ownership grants no endpoint SEND right                        |
| Lifetime              | Closing one handle preserves delegated handles and admitted request references         | Roots/backing frames remain retained through execution and acquired CPU completion; release needs mapping/TLBI obligations |
| Failure               | Denied or exhausted admission is transactional; cancellation may leave effects unknown | Mapping/setup rejection preserves ownership; a worker fault terminates its execution while peers continue                  |
| Shared mechanics      | Checked sizes, bounded capacity and explicit ownership/error reporting                 | Checked extents, bounded capacity and explicit ownership/error reporting                                                   |
| Conflicting mechanics | Handle close does not cancel accepted work or revoke independent delegation            | Last reference alone is insufficient to prove mappings inactive or translation invalidation complete                       |

Evidence scope differs: endpoint behavior is a
[host model](../../crates/native-state-models/src/lib.rs), while the
[EL0 foundation](../kernel/el0.md) documents real bounded worker execution.
The [memory implementation](../../crates/kernel/src/memory/mod.rs) retains
resource-specific guards; these are not a general user-facing memory-lease API.

Counterexamples to a proposed `Object::close`/generic opcode interface: freeing a
space when one owner closes can leave an active translation, whereas cancelling an
endpoint on handle close breaks accepted-work and delegation semantics. A SEND grant
cannot become permission to map memory merely because both resources have integer
names. One generic error or teardown transition would erase these distinctions.

Decision: retain separate interfaces and defer a universal hierarchy. Existing bounded
helpers may be reused with their own validation; no new shared allocator, lifecycle
framework or speculative file/socket/driver hierarchy is admitted. Future consumers
may reopen the comparison with evidence. Document checks establish record consistency;
they do not prove abstraction safety or create runtime coverage for hypothetical resources.

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
