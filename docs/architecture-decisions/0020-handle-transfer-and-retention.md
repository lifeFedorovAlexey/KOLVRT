# ADR-0020 — Attenuated handle transfer and retained targets

Status: **Accepted for bounded issue #23 implementation**. Date: 2026-10-03.

Document status: CURRENT  
Evidence scope: real kernel-local lookup, close, transfer and shared Event retention; two-CPU QEMU DEV/PROD.
Current reference: [Handle contract](../kernel/handles.md)

## Context

ADR-0019 established generation-safe identity and synchronous borrowed lookup. Issue #23 additionally requires rights checks, receiver-local transfer, accepted-work retention and close/lookup races on SMP. The kernel has fixed per-CPU scheduler ownership and no general allocator or object framework.

## Decision

- Keep eight caller-local entries, eight slot bits and 56 nonwrapping generation bits. Lookup checks owner, generation, live state, concrete primitive kind and required rights in one exclusive scheduler operation.
- Define only `SEND=1` and `TRANSFER=2`. Transfer requires TRANSFER, rejects unknown rights, applies subset attenuation, allocates a new receiver-local generation and preserves the same kernel-only TargetId. The receiver remains unchanged if validation or capacity fails.
- Permit an EL0 transfer to a namespace owned by the same fixed-affinity CPU. A cross-CPU transfer is performed only while the coordinator exclusively owns both process namespaces; a user request cannot access another CPU's scheduler storage.
- Keep the resource sum to Event and Completion. Event state lives in a fixed 512-slot atomic pool with nonwrapping generations and at most 1,024 live references per event. Each namespace entry and each accepted retained Event reference holds one count. Completion records are copied immutable values.
- A borrowed lookup prevents close/retire through Rust exclusivity. `Retained::retain` takes a bounded owned reference for work that outlives the call. The final reference releases the pool slot; close invalidates only that caller-local entry.
- Use the scheduler ownership permit as the table exclusion mechanism. Shared Event reference changes use bounded atomics. No nested table/object lock exists. Copy user input before borrowing namespace state; no lock or ownership permit spans allocation, blocking, wait or external calls.

## Alternatives

Global IDs, user-visible pointers, wrapping generations, unrestricted duplication, implicit authority, a global spinlock, dynamic reference-count allocation and a universal object/policy framework.

## Why rejected

Global selectors bypass caller attribution; user pointers expose kernel storage. Wrapping generations revive stale handles. Unrestricted duplication and implicit authority do not enforce attenuation. A global lock or allocating refcounts add blocking and failure paths to operations already serialized by CPU ownership. A universal object policy exceeds the two concrete target types and their bounded rights.

## Consequences

The 48-byte version-1 handle request supports lookup, close and transfer. Unknown types, rights, process generations, operation values and malformed extents fail closed. EL0 cannot select a source principal; it comes from the scheduler-owned task. Recipient slot and generation must match a bound receiver namespace on the caller's CPU. Closing the source leaves receiver handles and owned accepted Event references valid.

The fixed quotas bound memory and references. A full receiver produces no partial transfer. Exhausted generations are quarantined. A 512-slot global Event pool bounds shared objects; reference exhaustion is an explicit error. Completion handles retain completion data but do not retain a process or address space.

## Concurrency

No EL0 value becomes a pointer. TargetId stays kernel-only and is preserved across transfer. A failed rights check or receiver lookup changes neither namespace. Same-CPU table operations are serialized by the scheduler's owner permit. Cross-CPU coordinator transfers require both namespaces to be detached from execution. Concurrent Event close/lookup is safe because a recipient entry owns an independent atomic reference; clone and final release race through compare/exchange. Tests exercise the actual two-CPU EL0 lookup/close path and the production EL0 transfer decoder.

## Testing

Host tests cover generation exhaustion, forged/stale/foreign/wrong-kind handles, rights attenuation, escalation rejection, full receiver transactionality, binding preservation, close after delegation, owned retention after all handles close, concurrent signal/clone/release and quota exhaustion. QEMU runs the EL0 transfer request, source close, recipient lookup, cross-CPU delegated lookup/close, and focused enforcement mutations in DEV and PROD. Exact-source evidence is linked from the handle contract.

## Compatibility impact

The provisional 48-byte request has explicit words and status codes; it exposes no Rust layout, pointer, global object number or compatibility errno. Legacy fd/HANDLE conventions remain above this native boundary.

## Performance impact

Lookup, close and transfer use bounded table scans and no allocation. Event ownership uses bounded compare/exchange operations. QEMU timing receipts are regression observations, not hardware throughput claims.

## Security impact

EL0 cannot choose a source process or provide an object pointer. Rights checks gate the requested operation, transfer can only attenuate, and receiver publication occurs only after all validation. A closed source token cannot be looked up again; independent delegated and accepted-work references preserve lifetime.

## Reversibility

Changing the wire frame, rights bits, quotas, supported primitive set, affinity or retention guarantee requires a reviewed contract revision. New authority policy, revocation, IPC admission and domain budgets remain separate decisions.

[Russian translation](../../translations/ru/docs/architecture-decisions/0020-handle-transfer-and-retention.md)
