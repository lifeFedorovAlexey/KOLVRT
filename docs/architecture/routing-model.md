# Routing model

There is no global compatibility switch. Execution profiles do not select semantics.
Resolve a route at launch or binding; dispatch uses the pinned result instead of
discovering a new implementation on every call.

## Resolution

The initial selection identity is `(consumer/state-domain binding, API family,
semantic route)`. The semantic route names its protocol and exact semantic version.
Consumer identity and state-domain identity remain distinct within the binding: two
consumers or domains cannot be merged merely to shorten the selector.

Rights, device scope, quotas and execution-profile restrictions are admission constraints,
not independent semantic selectors. Packages declare requirements and defaults; process,
driver, device and administrator policies narrow the admitted set by intersection.
Defaults never override a denial. Empty intersections fail binding; multiple admitted
routes require explicit selection or fail as ambiguous. Policy precedence cannot resolve
conflicts. Device identity scopes authority unless a reviewed protocol requires distinct
semantics; it does not automatically create another selector.

A binding records native operations, adapter chain, semantic version, artifact digest,
dependency closure, state domain, rights, limits, profile and generation. Unsupported
families do not silently fall back to compatibility. Routing cannot increase authority.

| Level                    | Initial role                                          |
| ------------------------ | ----------------------------------------------------- |
| Package or executable    | Declared behavior requirements and inherited defaults |
| Process                  | Personality, policy and bound handles                 |
| Driver or device         | Protocol and scoped hardware translation              |
| Subsystem or API family  | Public route selector                                 |
| Protocol and version     | Immutable semantic contract                           |
| Compatibility capability | Narrow deviation with proven state independence       |

Start with a finite family table, not a universal routing graph. Every new independent
selector, including a separately selectable compatibility capability, requires an ADR
with a named consumer workload, evidence that existing selectors cannot express it,
shared-state analysis and rejection tests. See [ADR-0002](../architecture-decisions/0002-stateful-routing.md).
An AArch64 compatibility environment does not promise execution of x86 or AArch32 instructions.

## Finite example and rejection obligations

The window workload has four finite semantic routes: `window.native/1.0.0`,
`window.inclusive/1.0.0`, `window.counted/2.0.0` and
`bug.window.empty-first/1.0.0`. The last is a synthetic bug-compatibility fixture.
For example, consumer A in domain A binds the window family to the native route;
consumer B in domain B binds it to the inclusive route. Each binding pins its own
artifact digest, dependency closure and generation. Dispatch reads that binding;
it does not choose another route based on a request's bytes or current defaults.

| Attempt                                                               | Required result                                                                                    |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| A requests an operation outside its intersected rights                | Deny without selecting a broader route                                                             |
| B accesses a device outside its bound scope                           | Deny even when the semantic route supports that device                                             |
| Either consumer exceeds its quota                                     | Deny admission without switching semantics or charging another domain                              |
| Package default permits a route that administrator policy denies      | Fail binding if no explicitly selected admitted route remains                                      |
| Required route artifact or dependency is unavailable                  | Fail binding; no substitute semantic contract                                                      |
| B imports A's handle under B's defaults                               | Preserve A's object identity and binding; reject incompatible import without an authorized gateway |
| Two routes split operations over one incompatible shared-state domain | Reject the split; separate consumer names do not prove independent state                           |

These are design rejection obligations. The finite window implementation checks route
availability, profile integrity, generation and busy rebinding. It does not implement
general device/quota policy intersections or handle gateways. Existing host domain
models cover identity-preserving transfer and attenuation; they do not prove a general
resolver or kernel enforcement. Future implementations must execute the corresponding
rejection cases before claiming these guarantees.

## Shared state

One process may use native memory, files and networking with an older synchronization
adapter, while another is entirely native and a third uses a required external contract. This
example is valid only if shared-memory identity and lifetime remain consistent.

File access, close, duplication, locks and event subscriptions can share one descriptor
graph. Credentials, namespaces and authorization form a security domain. Operations on
one stream share protocol state. These relationships constrain route granularity.

Transferred handles retain object identity and behavior binding; receiver defaults do
not reinterpret existing state. Cross-domain import requires a gateway that checks
rights and semantics. Unsupported combinations fail. Shared resources have one arbiter.

## Controlled migration

1. Check migration support, semantic equivalence and operator authority.
2. Stop admission across the entire affected state domain.
3. Drain requests, waiters, callbacks, locks and device I/O; timeout retains the old binding.
4. Snapshot or explicitly convert state and validate its invariants.
5. Publish the new binding generation atomically across all domain entry points.
6. Resume admission; retain old code until references and required grace periods end.

Failures before commitment preserve the old domain. External effects limit rollback.
If state cannot be converted or quiescence cannot be reached, restart is required.
Production does not offer experimental live switching. Authorization remains mandatory
on fast dispatch paths. Phase 0.2 must model conflicts, downgrades, handle transfer,
concurrent rebinding and unloading; cases 02, 03, 17, 18, 25, 26 and 30 motivate these tests.

[Russian translation](../../translations/ru/docs/architecture/routing-model.md)
