# Kernel component map

Document status: CURRENT
Evidence scope: source-derived logical responsibilities and selected interactions; no runtime verification or per-component performance acceptance.
Current reference: [Native model](native-model.md); [Component Arena](component-arena.md)

This version 1 view describes current source responsibilities and selected interactions. It owns the grouping below; the linked production contracts remain authoritative. It does not establish runtime verification, viewer implementation acceptance or component performance. The [native model](native-model.md) and [Component Arena](component-arena.md) define the architectural and measurement boundaries.

## Counting boundary

The view contains 13 functional EL1 groups and two production EL0 service nodes. This is a declared logical counting boundary, not an intrinsic count of kernel components, source files or features. A group is not a new ABI, an isolated address space, a separately replaceable binary or a grant of authority. Test actors are not production nodes.

Sources intentionally overlap: scheduler dispatch participates in scheduling, waiting and CLOCK observations; boot and platform code also publish device identity. Membership does not imply exclusive ownership or additive code size or cost. EL0 supervisor policy is separate from the EL1 lifecycle enforcement grouped under processes.

## Interaction semantics

The selected edges are source-backed interactions, not an exhaustive dependency graph or a runtime call trace. `call` describes dispatch or invocation; `data` describes transferred or exposed information; `authority` describes an existing authorization boundary; `lifetime` describes retention, completion or safe-reuse obligations. Direction follows the described interaction, not necessarily a direct Rust function call. EL0 edges abstract the SDK and exception-entry path. An authority edge creates no new permission. Cycles are legitimate.

## Metrics and freshness

Unmeasured node costs, edge frequencies and edge weights are UNKNOWN or NOT_MEASURED, never zero. CLOCK evidence describes a complete userspace envelope with partial execution-window observations; it cannot allocate that envelope among these groups. IPC path measurements likewise do not establish additive per-node costs. Coverage is not speed. DEV, PROD, QEMU and physical-machine evidence retain their distinct scopes.

The same canonical machine model drives terminal and interactive views. Both must preserve node identities, layers, members, edge kinds, evidence and unknown values. Referential validation establishes that IDs and paths resolve; it is not runtime verification. `source_files` binds the reviewed source snapshot with LF-normalized SHA-256 values. A mismatch makes the snapshot STALE; a renderer must not silently present it as current. `reviewed_base` identifies the reviewed base revision, not a claim that subsequent working-tree changes are committed there. Re-review source changes and update this view when responsibilities or interactions change; change the model version when the declared grouping contract changes.

## Logical groups

<a name="kolvrt-kernel-view-boot"></a>

### Boot and platform (EL1)

Initialize the selected platform and admit the initial runtime.

<a name="kolvrt-kernel-view-memory"></a>

### Memory and address spaces (EL1)

Own physical frames, mappings and ASID lifetimes.

<a name="kolvrt-kernel-view-interrupts"></a>

### Exceptions and interrupts (EL1)

Save architectural context and dispatch synchronous exceptions and interrupts.

<a name="kolvrt-kernel-view-smp"></a>

### SMP and per-CPU state (EL1)

Coordinate secondary CPUs, IPIs and acquired invalidation acknowledgements.

<a name="kolvrt-kernel-view-scheduler"></a>

### Scheduling and task ownership (EL1)

Own runnable task state, dispatch and completion publication.

<a name="kolvrt-kernel-view-process"></a>

### Processes, images and lifecycle (EL1)

Admit images and enforce process creation, completion, reclamation and lifecycle mechanisms.

<a name="kolvrt-kernel-view-user-copy"></a>

### User-copy boundary (EL1)

Validate user ranges and copy through the protected user-memory boundary.

<a name="kolvrt-kernel-view-authority"></a>

### Names, rights and resource authority (EL1)

Resolve local handles, enforce rights and account domain resources.

<a name="kolvrt-kernel-view-wait"></a>

### Wait, wake and synchronization (EL1)

Maintain wait identities and coordinate wakeups with task state.

<a name="kolvrt-kernel-view-ipc"></a>

### Bounded IPC (EL1)

Enforce bounded requests, payload transfer, completion and cancellation.

<a name="kolvrt-kernel-view-clock"></a>

### Time and own-task observations (EL1)

Expose time and already-accounted current-task execution windows.

<a name="kolvrt-kernel-view-devices"></a>

### Immutable device identity (EL1)

Validate immutable platform device descriptors without granting device authority.

<a name="kolvrt-kernel-view-diagnostics"></a>

### Diagnostics and boot console (EL1)

Emit bounded diagnostics through the reserved boot console.

<a name="kolvrt-kernel-view-supervisor"></a>

### Native supervisor (EL0)

Apply production service supervision policy through public lifecycle and IPC interfaces.

<a name="kolvrt-kernel-view-counter-service"></a>

### Counter service (EL0)

Execute production counter requests and publish replies.

## Canonical machine model

The machine identifiers and labels below are shared by both language editions.

<!-- arena-architecture -->

```json
{
  "schema_version": 1,
  "version": "1",
  "reviewed_base": "8df8f2d",
  "source_files": {
    "apps/native-apps/src/counter-service.rs": "635fca7cd71c649dd20a773e214ed49f3752ccc0a0f5b9241d82c39ebcb9ee1c",
    "apps/native-apps/src/supervision.rs": "deababd624234c547c23d5d4d2e1034b401bebaaeb515bea34095a052669dda6",
    "apps/native-apps/src/supervisor.rs": "7d65f92ce1283bbecb2dfd5e5a88b13cbeae289fa5ddd2a4b8e44b6fe1cb6c99",
    "apps/native-runtime/src/lib.rs": "27a2e5a3c0a1bd0f8836661cb6ea50d185fa62e4292cb9c639e5152724bfa2c1",
    "crates/kernel-core/src/device.rs": "8e4e073e258ebc0376d169b3b638a5bb2820dc8b2fe2162335a4d895e60e7f66",
    "crates/kernel-core/src/domain.rs": "dad82df93e0beaf252017dd6ee5cb02fe2bb3b815ba64f1bf911e2d77de78788",
    "crates/kernel-core/src/elf.rs": "faa97534ec072f53c773012af327345dde9a35cece294ac1e436dd7399a79e58",
    "crates/kernel-core/src/execution.rs": "1b66287b7111d0fd319f8439e051bcc8658ef11fd8d156002afed213bf52933c",
    "crates/kernel-core/src/handles.rs": "6effe77cfae8e5b64f35e7d2eeb4a4052d2dbe01666204bad06cd64a9af43cd2",
    "crates/kernel-core/src/ipc.rs": "9cd0ca58b7504db9487d6862a811502032ecb36f3cbb3cad3e3b15441dad97f4",
    "crates/kernel-core/src/memory.rs": "0505ba3442c15da22f87b77b6365698588d3ca8c0db65d67e4bf130e232ceb58",
    "crates/kernel-core/src/platform.rs": "4423248c501cef4d50bc229fc6c97fa13eafbdea3632d037eb2b6a78f3fcd1a9",
    "crates/kernel-core/src/process.rs": "1067a28c3b51d65994a4ae5fe43e5e4d2f3303cc8c24c6709c22b52531312983",
    "crates/kernel-core/src/scheduling/ownership.rs": "41376235734055e9824655e6c5565b3224364f8796146dd917c53f1e784936ea",
    "crates/kernel-core/src/supervision.rs": "96d0ccd94f13e4d1e47720e01c308b3c9e8dd37c216ed05078fc0c4ecefb7ca6",
    "crates/kernel-core/src/time.rs": "308484a5d494f89849224ec0687368f8420712215739418249e30e49a104c403",
    "crates/kernel-core/src/user_copy.rs": "3fec5bdf29e979923cc3dcda503e1fcea2627cb17291090a045344a6bc0909da",
    "crates/kernel-core/src/wait.rs": "c9b1a7f133f9eb2794b8396abec06f791d3fd6b0c850207ee6402f8b910f7332",
    "crates/kernel-core/src/window.rs": "9c27386bd51db4c9db01151926a023bb7fbe5f8668c4d08bef753b257971f92f",
    "crates/kernel/src/arch/aarch64/entry.S": "5df5e13ad91b815d43d31838238d13ce91032e818bd40e7ffcb411e819c1ac1c",
    "crates/kernel/src/arch/aarch64/mod.rs": "e54606a21a0152921041ddab5fef0ede92fbcc6f436026acac4770b1455e5e2e",
    "crates/kernel/src/asid.rs": "7216e67ee636e5df1679728ee0911b90fd77bc0a76371dd090b300d8d205cd7e",
    "crates/kernel/src/diagnostics/mod.rs": "b03821bb2ff3a75672c6f5033eaf5182dc10acf45105db0ac806b15435fc19d4",
    "crates/kernel/src/execution.rs": "a22679e860a69e7dfde7845fe0b882ec6039701a20272901c1b1d957aa108d1f",
    "crates/kernel/src/hal/mod.rs": "4cd1c37db0bc76b990e919f2c2673d84385266793642d23519f417d8a841e731",
    "crates/kernel/src/handles.rs": "53f114a1ba01ac43f3e8e5e3af85bf5e2ce47b08049b241578e42c9d7c539db2",
    "crates/kernel/src/interrupt/mod.rs": "1ed289dca58d3e0d2bdc107279d871c61d7e9152f24602a380b0886a527d1a2c",
    "crates/kernel/src/ipc.rs": "4d8942e9ad2836b10b2bc62e98a6e35ce17befae305fe1276f5d9bd35af1e933",
    "crates/kernel/src/ipc/deferred.rs": "2dd4e5bddd94a02e05d743508c4e453fe175d990e5550f40ab21d70138d91ba4",
    "crates/kernel/src/ipc/native.rs": "bcf6931bd66b9648110bd35be16d6358eda9478a901328c49c7ce42b089d87ac",
    "crates/kernel/src/ipc/storage.rs": "f4b296d6e9bb07efd78fffbd6aca55c2f740460b22395860585f8e42fe1b9991",
    "crates/kernel/src/main.rs": "669b1dd09ebf4c69c5412d593ae751f690e661b95a7034bfd1c2bcc469846fac",
    "crates/kernel/src/memory/mod.rs": "7b30429ef9fcc21fe3bf8eb77778078288f81051a732c1a6cb48de568cc519f0",
    "crates/kernel/src/native_boot.rs": "4fb5e1b86ed624d4cb2c7fb2f1161262a16b618f0fef1ea7e4c0d9a0d04c4ed8",
    "crates/kernel/src/percpu.rs": "5910cf7b2ba96d8812bb390249ea70bc8e016cc8554aeea2b8ed07b6b2c39185",
    "crates/kernel/src/platform/mod.rs": "bfb5f120aabbb9a776ff9a61bb1ac1d9b974a4663e94265454d9afc635f9c7d4",
    "crates/kernel/src/process.rs": "cd560c0b57d78d3ed169b0c5d50a13f0a245cd0a811a39d50456ac9478829810",
    "crates/kernel/src/scheduler/local.rs": "0e295809eab3d48636ae39b0d5ab3e10ea37d1ec63361a005afcf5b028727429",
    "crates/kernel/src/scheduler/mod.rs": "cd9b1aee50697e7a3e69ee3650a754731325d00d3315332c549873f2c68d6c43",
    "crates/kernel/src/scheduler/task.rs": "82295a88f8943ee273ef718dd5823c24db9923b253f734a5089f0b98015581df",
    "crates/kernel/src/security.rs": "4a46f6eaf4d6dd5882aae5c1bb002f3b4451d40fc1bfbef1c1c5eb153fa96806",
    "crates/kernel/src/smp.rs": "5be7c210395aa8b4fb08677928fcd33add7113cb46ace17bbd5f7fb4bb9bbaf3",
    "crates/kernel/src/supervision.rs": "b665f610da6512e79c90a71994547c9ba793d1f8751a095bf3c515d93c00567a",
    "crates/kernel/src/sync/mod.rs": "4425454d73f2cb1b355ae34891612f3480d8463cadcc02babb034179700547da",
    "crates/kernel/src/time/mod.rs": "cefff7cdc9c810edcf5f6024dd62f78df68d01fa2338741257a6a1c9d557c69a",
    "crates/kernel/src/user_copy.rs": "796bda906820e7b9944b7512b4adcd3502c491599f83e8622933555950fd6ae0"
  },
  "nodes": [
    {
      "id": "kolvrt.kernel.view.boot",
      "label": "Boot and platform",
      "layer": "EL1",
      "responsibility": "Initialize the selected platform and admit the initial runtime.",
      "members": [
        {
          "name": "Boot admission sequence",
          "sources": ["crates/kernel/src/main.rs"]
        },
        {
          "name": "Selected platform initialization",
          "sources": ["crates/kernel/src/platform/mod.rs"]
        },
        {
          "name": "Hardware abstraction selection",
          "sources": ["crates/kernel/src/hal/mod.rs"]
        },
        {
          "name": "Platform description decoding",
          "sources": ["crates/kernel-core/src/platform.rs"]
        }
      ],
      "contracts": ["adr.0010", "adr.0005"]
    },
    {
      "id": "kolvrt.kernel.view.memory",
      "label": "Memory and address spaces",
      "layer": "EL1",
      "responsibility": "Own physical frames, mappings and ASID lifetimes.",
      "members": [
        {
          "name": "Physical and virtual memory ownership",
          "sources": ["crates/kernel/src/memory/mod.rs"]
        },
        {
          "name": "Frame allocation state",
          "sources": ["crates/kernel-core/src/memory.rs"]
        },
        {
          "name": "Address-space identifier lifecycle",
          "sources": ["crates/kernel/src/asid.rs"]
        }
      ],
      "contracts": ["adr.0010", "kolvrt.process.asid"]
    },
    {
      "id": "kolvrt.kernel.view.interrupts",
      "label": "Exceptions and interrupts",
      "layer": "EL1",
      "responsibility": "Save architectural context and dispatch synchronous exceptions and interrupts.",
      "members": [
        {
          "name": "Architectural context save and restore",
          "sources": ["crates/kernel/src/arch/aarch64/entry.S"]
        },
        {
          "name": "Vector installation and exception dispatch",
          "sources": ["crates/kernel/src/arch/aarch64/mod.rs"]
        },
        {
          "name": "Interrupt-controller dispatch",
          "sources": ["crates/kernel/src/interrupt/mod.rs"]
        }
      ],
      "contracts": ["adr.0014", "adr.0010"]
    },
    {
      "id": "kolvrt.kernel.view.smp",
      "label": "SMP and per-CPU state",
      "layer": "EL1",
      "responsibility": "Coordinate secondary CPUs, IPIs and acquired invalidation acknowledgements.",
      "members": [
        {
          "name": "Secondary CPU and invalidation coordination",
          "sources": ["crates/kernel/src/smp.rs"]
        },
        {
          "name": "Per-CPU identity and local state",
          "sources": ["crates/kernel/src/percpu.rs"]
        }
      ],
      "contracts": ["adr.0012"]
    },
    {
      "id": "kolvrt.kernel.view.scheduler",
      "label": "Scheduling and task ownership",
      "layer": "EL1",
      "responsibility": "Own runnable task state, dispatch and completion publication.",
      "members": [
        {
          "name": "Native-call and scheduling dispatch",
          "sources": ["crates/kernel/src/scheduler/mod.rs"]
        },
        {
          "name": "Task context and completion state",
          "sources": ["crates/kernel/src/scheduler/task.rs"]
        },
        {
          "name": "Guarded per-CPU scheduler storage",
          "sources": ["crates/kernel/src/scheduler/local.rs"]
        },
        {
          "name": "Task ownership transitions",
          "sources": ["crates/kernel-core/src/scheduling/ownership.rs"]
        }
      ],
      "contracts": ["doc.kolvrt.kernel.scheduler", "adr.0016"]
    },
    {
      "id": "kolvrt.kernel.view.process",
      "label": "Processes, images and lifecycle",
      "layer": "EL1",
      "responsibility": "Admit images and enforce process creation, completion, reclamation and lifecycle mechanisms.",
      "members": [
        {
          "name": "Process admission and resource lifecycle",
          "sources": ["crates/kernel/src/process.rs"]
        },
        {
          "name": "Process registry transitions",
          "sources": ["crates/kernel-core/src/process.rs"]
        },
        {
          "name": "ELF image validation",
          "sources": ["crates/kernel-core/src/elf.rs"]
        },
        {
          "name": "Granted lifecycle operation enforcement",
          "sources": ["crates/kernel/src/supervision.rs"]
        },
        {
          "name": "Initial native admission and completion reporting",
          "sources": ["crates/kernel/src/native_boot.rs"]
        },
        {
          "name": "Immutable bootstrap grant validation",
          "sources": ["crates/kernel-core/src/supervision.rs"]
        }
      ],
      "contracts": [
        "kolvrt.process.lifecycle",
        "kolvrt.services.supervision",
        "kolvrt.apps.native-elf"
      ]
    },
    {
      "id": "kolvrt.kernel.view.user-copy",
      "label": "User-copy boundary",
      "layer": "EL1",
      "responsibility": "Validate user ranges and copy through the protected user-memory boundary.",
      "members": [
        {
          "name": "Protected user-memory copy boundary",
          "sources": ["crates/kernel/src/user_copy.rs"]
        },
        {
          "name": "User range and snapshot validation",
          "sources": ["crates/kernel-core/src/user_copy.rs"]
        }
      ],
      "contracts": ["kolvrt.memory.user-copy"]
    },
    {
      "id": "kolvrt.kernel.view.authority",
      "label": "Names, rights and resource authority",
      "layer": "EL1",
      "responsibility": "Resolve local handles, enforce rights and account domain resources.",
      "members": [
        {
          "name": "Local handle resolution",
          "sources": ["crates/kernel/src/handles.rs"]
        },
        {
          "name": "Capability and domain enforcement",
          "sources": ["crates/kernel/src/security.rs"]
        },
        {
          "name": "Generational handle namespace",
          "sources": ["crates/kernel-core/src/handles.rs"]
        },
        {
          "name": "Domain rights and resource accounting",
          "sources": ["crates/kernel-core/src/domain.rs"]
        }
      ],
      "contracts": [
        "kolvrt.handles.local",
        "kolvrt.security.capability-revocation",
        "kolvrt.security.domains"
      ]
    },
    {
      "id": "kolvrt.kernel.view.wait",
      "label": "Wait, wake and synchronization",
      "layer": "EL1",
      "responsibility": "Maintain wait identities and coordinate wakeups with task state.",
      "members": [
        {
          "name": "Wait identity and terminal transitions",
          "sources": ["crates/kernel-core/src/wait.rs"]
        },
        {
          "name": "Kernel synchronization primitives",
          "sources": ["crates/kernel/src/sync/mod.rs"]
        },
        {
          "name": "Task wait and wake integration",
          "sources": ["crates/kernel/src/scheduler/mod.rs"]
        }
      ],
      "contracts": ["doc.kolvrt.kernel.wait", "kolvrt.ipc.wait"]
    },
    {
      "id": "kolvrt.kernel.view.ipc",
      "label": "Bounded IPC",
      "layer": "EL1",
      "responsibility": "Enforce bounded requests, payload transfer, completion and cancellation.",
      "members": [
        {
          "name": "Bounded endpoint and request state machine",
          "sources": ["crates/kernel-core/src/ipc.rs"]
        },
        {
          "name": "IPC runtime integration",
          "sources": ["crates/kernel/src/ipc.rs"]
        },
        {
          "name": "Endpoint and request storage",
          "sources": ["crates/kernel/src/ipc/storage.rs"]
        },
        {
          "name": "Admission, payload copy and completion",
          "sources": ["crates/kernel/src/ipc/native.rs"]
        },
        {
          "name": "Deferred wake and owner continuation",
          "sources": ["crates/kernel/src/ipc/deferred.rs"]
        }
      ],
      "contracts": ["kolvrt.ipc.transport", "kolvrt.ipc.request"]
    },
    {
      "id": "kolvrt.kernel.view.clock",
      "label": "Time and own-task observations",
      "layer": "EL1",
      "responsibility": "Expose time and already-accounted current-task execution windows.",
      "members": [
        {
          "name": "Counter reads and deadline arithmetic",
          "sources": ["crates/kernel/src/time/mod.rs"]
        },
        {
          "name": "Own-task execution observation service",
          "sources": ["crates/kernel/src/execution.rs"]
        },
        {
          "name": "Time value arithmetic",
          "sources": ["crates/kernel-core/src/time.rs"]
        },
        {
          "name": "Checked immutable window reduction",
          "sources": ["crates/kernel-core/src/window.rs"]
        },
        {
          "name": "Execution observation ABI and report bounds",
          "sources": ["crates/kernel-core/src/execution.rs"]
        },
        {
          "name": "Native CLOCK dispatch",
          "sources": ["crates/kernel/src/scheduler/mod.rs"]
        },
        {
          "name": "Current-task accumulated observations",
          "sources": ["crates/kernel/src/scheduler/task.rs"]
        }
      ],
      "contracts": ["kolvrt.clock.query", "kolvrt.clock.query.api"]
    },
    {
      "id": "kolvrt.kernel.view.devices",
      "label": "Immutable device identity",
      "layer": "EL1",
      "responsibility": "Validate immutable platform device descriptors without granting device authority.",
      "members": [
        {
          "name": "Immutable descriptor validation",
          "sources": ["crates/kernel-core/src/device.rs"]
        },
        {
          "name": "Platform identity extraction",
          "sources": ["crates/kernel-core/src/platform.rs"]
        },
        {
          "name": "Boot descriptor publication",
          "sources": ["crates/kernel/src/main.rs"]
        }
      ],
      "contracts": ["kolvrt.devices.observations"]
    },
    {
      "id": "kolvrt.kernel.view.diagnostics",
      "label": "Diagnostics and boot console",
      "layer": "EL1",
      "responsibility": "Emit bounded diagnostics through the reserved boot console.",
      "members": [
        {
          "name": "Bounded event serialization and console output",
          "sources": ["crates/kernel/src/diagnostics/mod.rs"]
        }
      ],
      "contracts": ["adr.0003", "adr.0010"]
    },
    {
      "id": "kolvrt.kernel.view.supervisor",
      "label": "Native supervisor",
      "layer": "EL0",
      "responsibility": "Apply production service supervision policy through public lifecycle and IPC interfaces.",
      "members": [
        {
          "name": "Service policy and readiness coordination",
          "sources": ["apps/native-apps/src/supervisor.rs"]
        },
        {
          "name": "Production recovery operation",
          "sources": ["apps/native-apps/src/supervision.rs"]
        }
      ],
      "contracts": ["kolvrt.services.supervision", "kolvrt.apps.native-elf"]
    },
    {
      "id": "kolvrt.kernel.view.counter-service",
      "label": "Counter service",
      "layer": "EL0",
      "responsibility": "Execute production counter requests and publish replies.",
      "members": [
        {
          "name": "Counter request execution",
          "sources": ["apps/native-apps/src/counter-service.rs"]
        },
        {
          "name": "Public native ABI call interface",
          "sources": ["apps/native-runtime/src/lib.rs"]
        }
      ],
      "contracts": ["kolvrt.apps.native-elf"]
    }
  ],
  "edges": [
    {
      "to": "kolvrt.kernel.view.memory",
      "kind": "call",
      "label": "Initialize physical memory, MMU and heap",
      "evidence": ["crates/kernel/src/main.rs"],
      "from": "kolvrt.kernel.view.boot"
    },
    {
      "to": "kolvrt.kernel.view.interrupts",
      "kind": "call",
      "label": "Install exception vectors and interrupt controller",
      "evidence": ["crates/kernel/src/main.rs"],
      "from": "kolvrt.kernel.view.boot"
    },
    {
      "to": "kolvrt.kernel.view.smp",
      "kind": "call",
      "label": "Initialize per-CPU state and start secondary CPUs",
      "evidence": ["crates/kernel/src/main.rs"],
      "from": "kolvrt.kernel.view.boot"
    },
    {
      "to": "kolvrt.kernel.view.process",
      "kind": "call",
      "label": "Admit initial native images",
      "evidence": [
        "crates/kernel/src/main.rs",
        "crates/kernel/src/native_boot.rs"
      ],
      "from": "kolvrt.kernel.view.boot"
    },
    {
      "to": "kolvrt.kernel.view.devices",
      "kind": "call",
      "label": "Validate and publish reserved console descriptor",
      "evidence": [
        "crates/kernel/src/main.rs",
        "crates/kernel-core/src/device.rs"
      ],
      "from": "kolvrt.kernel.view.boot"
    },
    {
      "to": "kolvrt.kernel.view.diagnostics",
      "kind": "data",
      "label": "Validated reserved console address configures diagnostics",
      "evidence": [
        "crates/kernel/src/main.rs",
        "crates/kernel/src/diagnostics/mod.rs"
      ],
      "from": "kolvrt.kernel.view.devices"
    },
    {
      "to": "kolvrt.kernel.view.diagnostics",
      "kind": "call",
      "label": "Initialize console and emit boot diagnostics",
      "evidence": ["crates/kernel/src/main.rs"],
      "from": "kolvrt.kernel.view.boot"
    },
    {
      "to": "kolvrt.kernel.view.scheduler",
      "kind": "call",
      "label": "Dispatch timer interrupt to scheduler",
      "evidence": ["crates/kernel/src/interrupt/mod.rs"],
      "from": "kolvrt.kernel.view.interrupts"
    },
    {
      "to": "kolvrt.kernel.view.smp",
      "kind": "call",
      "label": "Dispatch inter-processor interrupt",
      "evidence": ["crates/kernel/src/interrupt/mod.rs"],
      "from": "kolvrt.kernel.view.interrupts"
    },
    {
      "to": "kolvrt.kernel.view.smp",
      "kind": "lifetime",
      "label": "Require acquired invalidation acknowledgements before reuse",
      "evidence": [
        "crates/kernel/src/memory/mod.rs",
        "crates/kernel/src/smp.rs"
      ],
      "from": "kolvrt.kernel.view.memory"
    },
    {
      "to": "kolvrt.kernel.view.memory",
      "kind": "lifetime",
      "label": "Own and release process user-space frames",
      "evidence": ["crates/kernel/src/process.rs"],
      "from": "kolvrt.kernel.view.process"
    },
    {
      "to": "kolvrt.kernel.view.scheduler",
      "kind": "call",
      "label": "Admit task contexts and acquire completion",
      "evidence": ["crates/kernel/src/process.rs"],
      "from": "kolvrt.kernel.view.process"
    },
    {
      "to": "kolvrt.kernel.view.authority",
      "kind": "authority",
      "label": "Enforce domain charges and local namespace limits",
      "evidence": ["crates/kernel/src/process.rs"],
      "from": "kolvrt.kernel.view.process"
    },
    {
      "to": "kolvrt.kernel.view.ipc",
      "kind": "authority",
      "label": "Create endpoints and senders under existing grants",
      "evidence": [
        "crates/kernel/src/process.rs",
        "crates/kernel/src/supervision.rs"
      ],
      "from": "kolvrt.kernel.view.process"
    },
    {
      "to": "kolvrt.kernel.view.ipc",
      "kind": "call",
      "label": "Dispatch prepare, execute, copy and finish operations",
      "evidence": ["crates/kernel/src/scheduler/mod.rs"],
      "from": "kolvrt.kernel.view.scheduler"
    },
    {
      "to": "kolvrt.kernel.view.authority",
      "kind": "authority",
      "label": "Resolve current-task handle and security operations",
      "evidence": ["crates/kernel/src/scheduler/mod.rs"],
      "from": "kolvrt.kernel.view.scheduler"
    },
    {
      "to": "kolvrt.kernel.view.clock",
      "kind": "data",
      "label": "Publish own-task counters through CLOCK",
      "evidence": [
        "crates/kernel/src/scheduler/mod.rs",
        "crates/kernel/src/scheduler/task.rs"
      ],
      "from": "kolvrt.kernel.view.scheduler"
    },
    {
      "to": "kolvrt.kernel.view.wait",
      "kind": "call",
      "label": "Integrate event waits and wakeups with task state",
      "evidence": ["crates/kernel/src/scheduler/mod.rs"],
      "from": "kolvrt.kernel.view.scheduler"
    },
    {
      "to": "kolvrt.kernel.view.user-copy",
      "kind": "data",
      "label": "Snapshot and copy initialized user payload bytes",
      "evidence": ["crates/kernel/src/ipc/native.rs"],
      "from": "kolvrt.kernel.view.ipc"
    },
    {
      "to": "kolvrt.kernel.view.authority",
      "kind": "authority",
      "label": "Check namespace rights and charge retained requests",
      "evidence": [
        "crates/kernel/src/ipc/native.rs",
        "crates/kernel-core/src/ipc.rs"
      ],
      "from": "kolvrt.kernel.view.ipc"
    },
    {
      "to": "kolvrt.kernel.view.wait",
      "kind": "lifetime",
      "label": "Retain exact wait identities until terminal wakeup",
      "evidence": [
        "crates/kernel/src/ipc/native.rs",
        "crates/kernel/src/ipc/deferred.rs",
        "crates/kernel/src/scheduler/mod.rs"
      ],
      "from": "kolvrt.kernel.view.ipc"
    },
    {
      "to": "kolvrt.kernel.view.smp",
      "kind": "call",
      "label": "Ping owner CPU for deferred continuations",
      "evidence": ["crates/kernel/src/ipc/deferred.rs"],
      "from": "kolvrt.kernel.view.ipc"
    },
    {
      "to": "kolvrt.kernel.view.memory",
      "kind": "data",
      "label": "Use protected user-address range boundaries",
      "evidence": ["crates/kernel/src/user_copy.rs"],
      "from": "kolvrt.kernel.view.user-copy"
    },
    {
      "to": "kolvrt.kernel.view.scheduler",
      "kind": "call",
      "label": "Dispatch scheduler IPI and secondary polling",
      "evidence": ["crates/kernel/src/smp.rs"],
      "from": "kolvrt.kernel.view.smp"
    },
    {
      "to": "kolvrt.kernel.view.process",
      "kind": "authority",
      "label": "Request lifecycle operations within immutable grants",
      "evidence": [
        "apps/native-apps/src/supervisor.rs",
        "crates/kernel/src/supervision.rs"
      ],
      "from": "kolvrt.kernel.view.supervisor"
    },
    {
      "to": "kolvrt.kernel.view.ipc",
      "kind": "call",
      "label": "Issue readiness and service control requests",
      "evidence": ["apps/native-apps/src/supervisor.rs"],
      "from": "kolvrt.kernel.view.supervisor"
    },
    {
      "to": "kolvrt.kernel.view.ipc",
      "kind": "call",
      "label": "Receive, commit and reply through native runtime",
      "evidence": [
        "apps/native-apps/src/counter-service.rs",
        "apps/native-runtime/src/lib.rs"
      ],
      "from": "kolvrt.kernel.view.counter-service"
    },
    {
      "to": "kolvrt.kernel.view.diagnostics",
      "kind": "data",
      "label": "Publish lifecycle and generic report events",
      "evidence": [
        "crates/kernel/src/native_boot.rs",
        "crates/kernel/src/supervision.rs"
      ],
      "from": "kolvrt.kernel.view.process"
    }
  ]
}
```

[Russian translation](../../translations/ru/docs/architecture/kernel-component-map.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.architecture.kernel-component-map",
  "kind": "subsystem-contract",
  "summary": "Source-derived logical view of kernel responsibilities, EL0 services and typed interactions.",
  "depends_on": ["adr.0010", "adr.0008", "doc.kolvrt.architecture.arena"],
  "units": [
    {
      "id": "kolvrt.kernel.view.boot",
      "anchor": "kolvrt-kernel-view-boot",
      "kind": "subsystem-contract",
      "summary": "View grouping: Boot and platform.",
      "depends_on": ["adr.0010", "adr.0005"]
    },
    {
      "id": "kolvrt.kernel.view.memory",
      "anchor": "kolvrt-kernel-view-memory",
      "kind": "subsystem-contract",
      "summary": "View grouping: Memory and address spaces.",
      "depends_on": ["adr.0010", "kolvrt.process.asid"]
    },
    {
      "id": "kolvrt.kernel.view.interrupts",
      "anchor": "kolvrt-kernel-view-interrupts",
      "kind": "subsystem-contract",
      "summary": "View grouping: Exceptions and interrupts.",
      "depends_on": ["adr.0014", "adr.0010"]
    },
    {
      "id": "kolvrt.kernel.view.smp",
      "anchor": "kolvrt-kernel-view-smp",
      "kind": "subsystem-contract",
      "summary": "View grouping: SMP and per-CPU state.",
      "depends_on": ["adr.0012"]
    },
    {
      "id": "kolvrt.kernel.view.scheduler",
      "anchor": "kolvrt-kernel-view-scheduler",
      "kind": "subsystem-contract",
      "summary": "View grouping: Scheduling and task ownership.",
      "depends_on": ["doc.kolvrt.kernel.scheduler", "adr.0016"]
    },
    {
      "id": "kolvrt.kernel.view.process",
      "anchor": "kolvrt-kernel-view-process",
      "kind": "subsystem-contract",
      "summary": "View grouping: Processes, images and lifecycle.",
      "depends_on": [
        "kolvrt.process.lifecycle",
        "kolvrt.services.supervision",
        "kolvrt.apps.native-elf"
      ]
    },
    {
      "id": "kolvrt.kernel.view.user-copy",
      "anchor": "kolvrt-kernel-view-user-copy",
      "kind": "subsystem-contract",
      "summary": "View grouping: User-copy boundary.",
      "depends_on": ["kolvrt.memory.user-copy"]
    },
    {
      "id": "kolvrt.kernel.view.authority",
      "anchor": "kolvrt-kernel-view-authority",
      "kind": "subsystem-contract",
      "summary": "View grouping: Names, rights and resource authority.",
      "depends_on": [
        "kolvrt.handles.local",
        "kolvrt.security.capability-revocation",
        "kolvrt.security.domains"
      ]
    },
    {
      "id": "kolvrt.kernel.view.wait",
      "anchor": "kolvrt-kernel-view-wait",
      "kind": "subsystem-contract",
      "summary": "View grouping: Wait, wake and synchronization.",
      "depends_on": ["doc.kolvrt.kernel.wait", "kolvrt.ipc.wait"]
    },
    {
      "id": "kolvrt.kernel.view.ipc",
      "anchor": "kolvrt-kernel-view-ipc",
      "kind": "subsystem-contract",
      "summary": "View grouping: Bounded IPC.",
      "depends_on": ["kolvrt.ipc.transport", "kolvrt.ipc.request"]
    },
    {
      "id": "kolvrt.kernel.view.clock",
      "anchor": "kolvrt-kernel-view-clock",
      "kind": "subsystem-contract",
      "summary": "View grouping: Time and own-task observations.",
      "depends_on": ["kolvrt.clock.query", "kolvrt.clock.query.api"]
    },
    {
      "id": "kolvrt.kernel.view.devices",
      "anchor": "kolvrt-kernel-view-devices",
      "kind": "subsystem-contract",
      "summary": "View grouping: Immutable device identity.",
      "depends_on": ["kolvrt.devices.observations"]
    },
    {
      "id": "kolvrt.kernel.view.diagnostics",
      "anchor": "kolvrt-kernel-view-diagnostics",
      "kind": "subsystem-contract",
      "summary": "View grouping: Diagnostics and boot console.",
      "depends_on": ["adr.0003", "adr.0010"]
    },
    {
      "id": "kolvrt.kernel.view.supervisor",
      "anchor": "kolvrt-kernel-view-supervisor",
      "kind": "subsystem-contract",
      "summary": "View grouping: Native supervisor.",
      "depends_on": ["kolvrt.services.supervision", "kolvrt.apps.native-elf"]
    },
    {
      "id": "kolvrt.kernel.view.counter-service",
      "anchor": "kolvrt-kernel-view-counter-service",
      "kind": "subsystem-contract",
      "summary": "View grouping: Counter service.",
      "depends_on": ["kolvrt.apps.native-elf"]
    }
  ]
}
```
