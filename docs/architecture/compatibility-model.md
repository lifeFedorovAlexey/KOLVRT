# Compatibility model

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

Version selection uses an explicit compatibility matrix, not a highest-version heuristic.
A binding pins both semantic version and artifact digest. Versions can coexist only with
separate state or proven interoperability. Dependency cycles and unsupported operations
fail explicitly before use. A native-only build removes software compatibility modules
and passes the same native contract tests. There is no kernel build yet in Phase 0.

Legacy layouts, error conventions, syscall tables and historical state machines stay in
adapters. They cannot add application-version branches to the scheduler, memory manager,
filesystem core, hardware abstraction layer or native drivers. Adapters cannot add rights
or bypass protection. Fork, signals, credentials, descriptors and shared synchronization
may require one indivisible personality domain rather than independent call adapters.

Hardware translation is separate. Firmware frontends produce validated native device
descriptions. A silicon workaround can affect code generation or platform operations;
it is not necessarily dynamically unloadable. Dropping a mandatory workaround drops
the affected target. [Cortex-A53 843419](../../research/cases/KOL-PATH-0011.json) is an example.

## Bug-compatibility lifecycle

1. Establish the cause, correct the native specification and add a regression test.
2. Demonstrate an actual consumer of the old behavior before adding a module.
3. Prove that the retained semantics does not weaken native security or ownership.
4. Assign a separate behavior identity, explicit opt-in and conformance fixture.
5. Provide migration instructions and equivalent-workload measurements.
6. Publish a support window before deprecation and account for affected consumers.
7. Remove the implementation only after declared dependencies, active bindings and
   in-flight state are gone and observation coverage is sufficient.

Dormant packages and offline recovery tools remain relevant even with no recent calls.
Keep a registry tombstone after implementation removal so an old manifest receives an
understandable error. The lifecycle is `Available -> Bound -> Draining -> Unbound -> Removable`.
Unloading bound or draining code is forbidden. Rollback is guaranteed only before an
irreversible effect; later failures require explicit recovery rather than blind retries.

[Russian translation](../../translations/ru/docs/architecture/compatibility-model.md)
