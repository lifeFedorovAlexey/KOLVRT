# Compatibility model

Document status: CURRENT
Document scope: software compatibility obligations; Phase 0 implementation claims are historical.
Status reference: [Implementation evidence](../documentation-policy.md)

Compat cannot issue missing consumer grants or expand effective authority. Service-private capabilities permit mediation only within native-authorized effects; [ADR-0013](../architecture-decisions/0013-security-boundaries.md) allows no compatibility waiver.

A compatibility module translates an external semantic contract into the current native
contract. It is a separately identifiable, replaceable, versioned and removable dependency. Such a module is justified only by a concrete KOLVRT consumer. Studying a foreign interface does not commit the project to supporting it.

| Manifest field                         | Obligation                                                     |
| -------------------------------------- | -------------------------------------------------------------- |
| Behavior identity and semantic version | Immutable definition of supported behavior                     |
| Implementation digest                  | Exact executable artifact, independent of the semantic version |
| Owner and source cases                 | Responsibility and engineering evidence                        |
| Input and native protocols             | Supported versions, encoding and errors                        |
| Required rights and resource limits    | Minimal grants and bounded memory, pins and queues             |
| State domain and dependencies          | Shared state and transitive module requirements                |
| Security boundary                      | Actual isolation or explicitly privileged execution            |
| Metrics and consumer identity          | Attribution, lifetime accounting and retention                 |
| Migration and removal                  | Admission control, draining, conversion and rollback limits    |

## Bounded support obligations

Every supported compatibility behavior has a registry record with required `OWNER`,
`NAMED CONSUMERS`, `CREATION REASON`, `MIGRATION TARGET`, `SUPPORT WINDOW` and
`REMOVAL CONDITION`. Name the responsible owner and exact consumer packages, identify
the dependency and evidence that motivated creation, and specify the target contract
or an explicit retirement path when migration is unavailable. The support window has
a start and finite deadline in a declared release or date domain; "until nobody needs
it" is not a deadline. The removal condition states default-distribution removal and
the separate runtime drain prerequisites.

Name dormant packages and offline recovery packages explicitly, with their owner and
support deadline. Zero recent calls cannot cancel their unexpired obligation. Conversely,
unknown hypothetical binaries and incomplete observation coverage do not renew an expired
window. Extensions require a recorded decision naming affected consumers, justification,
migration work and a new finite deadline. A successor module or semantic version does
not renew predecessor support; semantic-version publication guarantees are a separate
lifecycle decision.

At expiry, stop new unsupported bindings and request admissions, including on existing
bindings. Return an explicit unsupported/deprecated result with the migration target;
do not select another contract. Already admitted work retains its references and completes
or follows its declared cancellation/recovery contract. Removing the module from the
default distribution does not authorize freeing loaded code. Keep a registry tombstone
with behavior identity, last supported version, deadline, removal reason and migration
instructions so an old manifest receives a meaningful error.

An archived artifact may be available for explicit installation with stated security and
support limits and fresh admission authorization. Archival is not perpetual default
distribution or support renewal. Security-invalid behavior must never be restored.

These obligations refine [ADR-0007](../architecture-decisions/0007-bug-compat.md) and
LAW-008. They apply to supported software modules, not hypothetical foreign interfaces
or a promise to unload mandatory hardware workarounds.

Version selection uses an explicit compatibility matrix, not a highest-version heuristic.
A binding pins both semantic version and artifact digest. Versions can coexist only with
separate state or proven interoperability. Dependency cycles and unsupported operations
fail explicitly before use. A native-only build removes software compatibility modules
and passes the same native contract tests. Phase 0 had no kernel build; the merged
repository now has the bounded kernel implementation recorded in the status reference.

Legacy layouts, error conventions, syscall tables and historical state machines stay in
adapters. They cannot add application-version branches to the scheduler, memory manager,
filesystem core, hardware abstraction layer or native drivers. Adapters cannot add rights
or bypass protection. Fork, signals, credentials, descriptors and shared synchronization
may require one indivisible personality domain rather than independent call adapters.

Hardware translation is separate. Firmware frontends produce validated native device
descriptions. A silicon workaround can affect code generation or platform operations;
it is not necessarily dynamically unloadable. Dropping a mandatory workaround drops
the affected target. [Cortex-A53 843419](../../research/cases/KOL-PATH-0011.json) is an example.

## Semantic-version lifecycle records

Maintain one lifecycle record per named semantic contract/version for both native public
contracts and compatibility adapters. Artifact digests and build revisions identify
implementations; changing them does not reset semantic support, create a new version
or extend a deadline. Link each record to its owner, migration and module-removal records.

| Field             | Requirement                                                                                                                                                                              |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `introduced`      | Recorded introduction date or release in a declared ordering domain.                                                                                                                     |
| `deprecated`      | Explicitly unset before deprecation; after deprecation, its actual date/release.                                                                                                         |
| `support_until`   | Finite deadline for promised support, mandatory on deprecation. Experimental/unpromised versions may explicitly leave it unset before deprecation and must state no long-term guarantee. |
| `known_consumers` | Named supported consumers, including declared dormant/offline packages and their obligations; unknown hypothetical programs do not renew support.                                        |
| `replacement`     | Named target contract/version and migration record, or an explicit no-replacement/end-of-service decision on deprecation. Before deprecation, explicitly pending is permitted.           |

The record also identifies contract/version, publication stage and support domain. PUBLIC
and STABLE native contracts retain their explicit provisional or ABI-FREEZE support
guarantees: an unset field cannot erase a promise. Deprecation must not shorten a promised
window. Experimental status is explicit absence of long-term support, not permission to
weaken running authority, ownership or outcomes.

Renewal requires reviewed named-consumer evidence, owner, migration plan and a new finite
deadline in the same domain; retain the prior record and decision. Missing evidence or
the existence of v2 is not renewal. Introducing v2 neither terminates v1 before its
promised deadline nor retains it indefinitely. At expiry, use explicit errors and the
tombstone; runtime reclamation still requires draining references and accepted work.
An ABI-FREEZE amendment cannot silently alter supported semantics or erase its support
promise. Independent versions may coexist only under compatible state/authority rules.

The current native request v0 is CANDIDATE, introduced by the first-slice decision on
2026-10-02, with deprecation unset, no declared long-term support deadline, the host echo
fixture as its known consumer and replacement explicitly pending. It has no ABI-FREEZE.
This documents its design status, not a production lifecycle registry. On any future
deprecation, even this unpromised candidate needs a recorded deadline and replacement
or end-of-service decision before removal. Version-creation checks in the host retirement
model reject successor-based renewal and premature removal; registry and negotiation
integration remain unimplemented. See [ADR-0007](../architecture-decisions/0007-bug-compat.md).

## Bug-compatibility lifecycle

1. Establish the cause, correct the native specification and add a regression test.
2. Demonstrate an actual consumer of the old behavior before adding a module.
3. Prove that the retained semantics does not weaken native security or ownership.
4. Assign a separate behavior identity, explicit opt-in and conformance fixture.
5. Provide migration instructions and equivalent-workload measurements.
6. Register finite support before deprecation, including named dormant/offline consumers;
   record any renewal with a new deadline.
7. End unsupported admission at expiry and retain a tombstone. Remove loaded code only
   after bindings, transitive runtime references and in-flight state have drained and
   callback/device sources and required grace periods are quiescent. A drain timeout
   retains the implementation; it does not silently extend support.

Declared dependencies protect their published support window, not an indefinite veto.
Support expiry and default-distribution removal are independent of the runtime lifecycle
`Available -> Bound -> Draining -> Unbound -> Removable`.
Unloading bound or draining code is forbidden. Rollback is guaranteed only before an
irreversible effect; later failures require explicit recovery rather than blind retries.

## Verification boundary

The host [retirement model](../../crates/native-state-models/src/retirement.rs) exhaustively
explores a support-deadline boundary with at most two binding references and two retained
requests. Negative controls detect deadline-triggered premature release, admission after
expiry and loss of the tombstone. The existing binding model checks drain-before-retirement.
Neither implements a production registry, deadline clock, archived installation, tombstone
error transport, callback/device quiescence or runtime unloading. Those require integration
checks before claiming implementation support.

[Russian translation](../../translations/ru/docs/architecture/compatibility-model.md)
