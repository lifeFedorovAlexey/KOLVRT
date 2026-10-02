# Compatibility dependency diagnostics

Do not collapse dependency into an arbitrary weighted score. A rare compatibility call
can be mandatory for startup, while frequent translations can consume little processor
time. Call shares, execution cost, retained memory and module removability are distinct.
The diagnostics describe dependence, not application quality.

## Measurement contract

For each consumer, route generation and observation window report declared and transitive
dependencies, loaded modules, active bindings, unique consumers and unobserved required paths.
Count externally admitted native and compatibility operations separately from completed,
failed, cancelled and in-flight operations. Adapter calls into the native backend are not
additional external native operations.

Compatibility call share is compatibility admissions divided by all classified external
admissions. A zero denominator is unknown. Native share is its complement only with full
classification. Exclusive processor time distinguishes adapter, native backend, consumer
and unattributed work. A compatibility request can use native backend time without becoming
a native entry operation. State the processor-share denominator explicitly.

Also report latency, throughput, allocation count and bytes, private/shared/pinned memory,
copies, context switches, lock contention, translations, serialization/deserialization
and API frequency. Record scope, coverage, sampling, lost events and missing attribution.
Asynchronous work inherits causal identifiers. Unassignable shared work stays unattributed.
Nested spans must not duplicate processor time; overlapping wall times are not a process total.
Do not expose foreign credentials or user payload through diagnostics.

## Text states

| State | Required evidence | Color hint |
|---|---|---|
| LEGACY | Only legacy entry contracts are declared; no native contract is declared | Brown |
| COMPAT | Every used entry family in the complete observation scope requires an adapter | Red |
| MIXED | Both routes occur; no declared migration budget is met | Yellow |
| MOSTLY_NATIVE | Mixed routes meet an explicit consumer-specific multidimensional migration budget | Light green |
| NATIVE | No direct or transitive software compatibility dependency in the stated support scope; sufficient coverage | Green |

Insufficient evidence produces an additional UNKNOWN state. Otherwise classify NATIVE,
then manifest-defined LEGACY, then COMPAT, then MOSTLY_NATIVE or MIXED. Always identify
whether the scope is a package declaration, observed process workload or device binding.
Every color has a text label, values, units and confidence. Hardware workarounds form a
separate dimension; they do not turn native applications into legacy applications.

Fully stripped production can show declared dependencies, not invented dynamic shares.
MOSTLY_NATIVE needs published thresholds and a rationale from that consumer's migration
plan, not a universal marketing percentage. Zero observed calls cannot erase a declared
dependency. Loaded, bound and called module counts differ. Report a faster compatibility
implementation without a score penalty; use the [benchmark methodology](benchmarking.md).

[Russian translation](../../translations/ru/docs/architecture/diagnostics.md)
