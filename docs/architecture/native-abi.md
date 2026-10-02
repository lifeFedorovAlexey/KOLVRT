# Native request ABI candidate v0

This is a testable encoding candidate for the [first slice](first-native-slice.md), not a stable public ABI. It is independent of compiler structure layout. All integers are unsigned little-endian unless stated otherwise. No pointers or padding bytes cross the protocol boundary. The caller's protection domain is established by entry context, never a message field.

## Request frame

| Offset | Bytes    | Meaning                                                               |
| ------ | -------- | --------------------------------------------------------------------- |
| 0      | 2        | Version, currently 0                                                  |
| 2      | 2        | Operation, 1 for bounded echo in the experiment                       |
| 4      | 4        | Total frame length, exactly 40 plus payload length                    |
| 8      | 8        | Caller-local handle: low 32 bits slot, high 32 bits generation        |
| 16     | 8        | Caller request ID, unique among outstanding requests on this endpoint |
| 24     | 8        | Absolute deadline in the boot-local monotonic tick domain             |
| 32     | 4        | Payload length, 0 through 256                                         |
| 36     | 4        | Reserved, zero                                                        |
| 40     | variable | Owned payload snapshot                                                |

The receiver validates exact length, supported version/operation, reserved fields and an unexpired deadline before admission. The tick frequency is negotiated in boot information; converting durations uses checked arithmetic. Counter overflow ends that clock epoch rather than reinterpreting old deadlines. The decoder accepts only a copied slice; real user-copy must additionally handle mapping changes and faults.

## Authority and ownership

Resolve a handle in the caller's table and validate its generation, object type, live state and required rights under one lock. The integer alone is not global authority. SEND is bit 0 and TRANSFER is bit 1; unknown rights fail. Transfer requires TRANSFER and a subset of held rights, creates a receiver-local handle, and preserves object and semantic binding. If receiver allocation fails, neither domain changes. Slots exhaust permanently instead of wrapping generations. Closing one handle does not close delegated handles or cancel already admitted requests.

Revocation is serialized with admission and stops future requests. It does not retroactively invalidate the memory ownership of accepted work. Strong immediate revocation, if later required, needs a distinct operation with a drain/reset contract. The slice does not implement it by freeing objects underneath requests.

## Completion contract

The response is a versioned frame with the same little-endian convention. The caller rejects an unexpected request ID, unknown status, nonzero reserved bytes, inconsistent length or payload on a non-completed result.

| Offset | Bytes    | Meaning                                                                                                                                    |
| ------ | -------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| 0      | 2        | Version, currently 0                                                                                                                       |
| 2      | 2        | Status: 0 completed, 1 invalid, 2 denied, 3 exhausted, 4 cancelled before effect, 5 effect unknown, 6 expired before effect, 7 unsupported |
| 4      | 4        | Total frame length, exactly 24 plus payload length                                                                                         |
| 8      | 8        | Original request ID                                                                                                                        |
| 16     | 4        | Payload length, 0 through 256; zero unless completed                                                                                       |
| 20     | 4        | Reserved, zero                                                                                                                             |
| 24     | variable | Response payload                                                                                                                           |

All observable bytes are initialized. A broken transport is an out-of-band failure, never a fabricated success frame. Loss after admission reports unknown effects unless absence was established. Duplicate outstanding IDs are rejected at endpoint admission; an ID becomes reusable only after the prior terminal result is consumed. The host experiment rejects a deliberately mismatched response ID. No exactly-once durable effect is promised across service crashes.

The cancellation linearization point competes with effect commitment. If cancellation wins, the effect cannot occur. If commitment wins, cancellation cannot report absence of effects. Completion, timeout and owner death use one terminal-result arbiter; retained references and charges are released once. Queue acceptance is not completion or durability.

## Verification boundary

The no_std [decoder](../../crates/native-protocol-model/src/lib.rs) checks encoding, snapshot ownership, rights attenuation and generation exhaustion. The [host models](../../crates/native-state-models/src/lib.rs) check lifetime, charge retention, wait registration, rebinding, startup and domain transfer. The candidate compiles for AArch64 without a host standard library. This does not verify exception entry, assembly calling conventions, page tables, real user-copy or an unbounded concurrent implementation.

[Russian translation](../../translations/ru/docs/architecture/native-abi.md)
