# Typed interfaces

## Source finding

CE uses IDL/CDL/EDL to describe interfaces and components and generate transport, security-module and initialization code. The descriptions do not specify a component's internal implementation. [Formal specifications](https://support.kaspersky.com/help/KCE/1.2/en-US/ice.htm). IDL defines method signatures and parameter types. [IDL description](https://support.kaspersky.com/help/KCE/1.2/en-US/ice_idl.htm).

## KOLVRT recommendation

Use narrow versioned frames with explicit sizes, enums, bounds, handle types and rights. Keep legacy layout conversion in adapters and validate the same owned snapshot that executes. Reuse typed time/ownership primitives without a universal process/device/file/socket supertype. A new capability type requires a distinct authority/lifetime model, a named workload and a reason existing bounded operations are insufficient.

Generated code belongs in the dependency/unsafe/TCB inventory when trusted. Compile-time types can reject local misuse; they cannot establish caller authority, peer correctness, shared-state equivalence, temporal races or device containment. Schema field limits describe a review artifact, not a stable ABI or universal kernel capacity.

## Counterexamples and evidence

A correctly typed file path can escape the caller's root; a valid integer can refer to another domain; an immutable signature can still expose an unrestricted raw operation. Require path/resource-scope checks, caller-local handle lookup and rejection of unknown flags/conversions. Fuzz raw encodings and use compile-fail tests only for meaningful local ownership restrictions. Larger interface families increase generated code and maintenance costs; general-purpose benefits need concrete workloads.

[Russian translation](../../../translations/ru/research/other-systems/kasperskyos/typed-interfaces.md)
