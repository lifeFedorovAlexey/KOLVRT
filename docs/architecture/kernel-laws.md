# Kernel laws

Document status: CURRENT
Evidence scope: Accepted intended obligations; implementation and verification remain separately scoped.
Current reference: [Documentation policy](../documentation-policy.md)

These are language-independent architectural obligations, not a numbered design target. The set has no required size. Stable IDs preserve history; gaps are intentional. Enforcement and tests are obligations for future implementations, not claims of completed kernel tests.

Each law addresses a distinct review obligation. Consolidated requirements and implementation-specific policies are traced in the [law review](law-review.md). Changing a law requires an architecture decision with evidence and tests.

<a name="law-001"></a>

## LAW-001 — Native authority and dependency isolation

**Rule:** Native behavior is defined by current native contracts; core implementations must not depend on legacy layouts, adapters or consumer-version branches. Internal implementation layouts are not a permanent external ABI.

**Rationale:** Compatibility requirements must remain removable without freezing internal design.

**Historical evidence:** [KOL-PATH-0006](../../research/cases/KOL-PATH-0006.json), [KOL-PATH-0028](../../research/cases/KOL-PATH-0028.json)

**Prevents:** Legacy dependencies that prevent native-only operation.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Audit dependency closure, generated interfaces and exported layouts. [ADR-0001](../architecture-decisions/0001-native-authority.md) requires explicit ABI publication stages and a per-contract/version ABI-FREEZE before stable status.

**Testing:** Remove all software adapters and run the unchanged native contract suite.

<a name="law-003"></a>

## LAW-003 — Concurrent native and compatibility routes

**Rule:** Routing must support different consumers and API families concurrently; one global compatibility mode must not determine their behavior.

**Rationale:** A migration can involve only part of a workload.

**Historical evidence:** [KOL-PATH-0002](../../research/cases/KOL-PATH-0002.json), [KOL-PATH-0020](../../research/cases/KOL-PATH-0020.json)

**Prevents:** Mutual exclusion of native and legacy consumers.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Review route selection scope and manifest constraints.

**Testing:** Run native, mixed and personality-bound consumers together without changing a global switch.

<a name="law-004"></a>

## LAW-004 — Explicit semantic binding

**Rule:** A binding pins a supported semantic version and a separate implementation identity. Missing, conflicting or ambiguous requirements fail explicitly; no silent fallback or version downgrade is allowed. An alternate requires explicit trusted policy, preserved authority/state and visible outcome; different semantics require consumer opt-in and a new valid binding. Unknown effects never authorize blind replay. Profiles cannot waive these requirements.

**Rationale:** An implementation update and a semantic change are different events.

**Historical evidence:** [KOL-PATH-0020](../../research/cases/KOL-PATH-0020.json), [KOL-PATH-0023](../../research/cases/KOL-PATH-0023.json)

**Prevents:** Unannounced behavior changes during dispatch.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Resolve dependency closure and permission intersections before binding. [ADR-0002](../architecture-decisions/0002-stateful-routing.md) separates finite selection identity from rights, device scope and quota constraints and requires evidence for new selectors.

**Testing:** Missing modules, conflicting policies and unsupported versions never execute a substitute contract.

<a name="law-005"></a>

## LAW-005 — Shared-state consistency

**Rule:** Operations sharing state use compatible bindings and one object identity. Handle transfer cannot reinterpret existing state through receiver defaults.

**Rationale:** Call-level independence cannot be assumed from API names.

**Historical evidence:** [KOL-PATH-0002](../../research/cases/KOL-PATH-0002.json), [KOL-PATH-0003](../../research/cases/KOL-PATH-0003.json), [KOL-PATH-0017](../../research/cases/KOL-PATH-0017.json), [KOL-PATH-0026](../../research/cases/KOL-PATH-0026.json)

**Prevents:** Split lock, descriptor, credential or synchronization state.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Declare state domains and explicit cross-domain conversion contracts.

**Testing:** Reject incompatible close/lock and wait/wake bindings and unauthorized handle imports.

<a name="law-008"></a>

## LAW-008 — Controlled evolution and retirement

**Rule:** Correct native bugs with regression tests. Retain safe old behavior only for demonstrated consumers in separate modules. Rebinding and unloading require stopped admission, drained state and atomic commitment; referenced code cannot be removed.

**Rationale:** Changing behavior or code does not erase existing state or consumers.

**Historical evidence:** [KOL-PATH-0007](../../research/cases/KOL-PATH-0007.json), [KOL-PATH-0008](../../research/cases/KOL-PATH-0008.json), [KOL-PATH-0022](../../research/cases/KOL-PATH-0022.json), [KOL-PATH-0030](../../research/cases/KOL-PATH-0030.json)

**Prevents:** Permanent native bugs, half-migrated domains and callbacks into unloaded code.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Require a migration state machine, dependency evidence and explicit rollback limits. [ADR-0007](../architecture-decisions/0007-bug-compat.md) requires finite owned support, per-semantic-version lifecycle records and the owned exception registry; successor publication cannot renew support or authorize unsafe unloading. Exception review deadlines flag a decision and never disable essential supported-target workarounds under LAW-031.

**Testing:** Exercise timeout, outstanding callbacks, dormant dependencies and failure before and after irreversible effects.

<a name="law-009"></a>

## LAW-009 — Translation cannot expand effective authority

**Rule:** For consumer C and operation O, EffectiveAuthority_via_compat(C,O) ⊆ NativeAuthorizedAuthority(C,O) in the applicable native authorization/revocation context. Translation, identity mapping and delegation cannot bypass memory protection, quotas or delegation restrictions. Service-private capabilities may differ from consumer capabilities but enable only caller-authorized objects and effects. Additional consumer grants require a separate native flow before admission, never compat issuance.

**Rationale:** Check caller effects rather than literal equality of service and consumer capability inventories, preventing confused deputies.

**Historical evidence:** [KOL-PATH-0007](../../research/cases/KOL-PATH-0007.json), [KOL-PATH-0008](../../research/cases/KOL-PATH-0008.json), [KOL-PATH-0017](../../research/cases/KOL-PATH-0017.json), [KOL-PATH-0018](../../research/cases/KOL-PATH-0018.json)

**Prevents:** Authority laundering disguised as compatibility or mediation.

**Allowed exceptions:** No compatibility exception may expand effective authority.

**Enforcement:** Review issuer, consumer, operation, scope and revocation context at every boundary; follow [ADR-0013](../architecture-decisions/0013-security-boundaries.md).

**Testing:** Sufficient and denied authority, missing grants, broader substitutes, nested adapters, other/private authority, restart/rebind, PROD, private disk and split/combined operations.

<a name="law-013"></a>

## LAW-013 — Explicit identity and lifetime ownership

**Rule:** Authority-bearing objects, locks and event registrations have explicit identities and owners. Release consumes ownership once; reused names do not retarget operations. Retiring objects cannot be resurrected or freed while readers or requests retain access.

**Rationale:** Visibility, ownership and storage reclamation are distinct lifecycle properties.

**Historical evidence:** [KOL-PATH-0001](../../research/cases/KOL-PATH-0001.json), [KOL-PATH-0002](../../research/cases/KOL-PATH-0002.json), [KOL-PATH-0003](../../research/cases/KOL-PATH-0003.json), [KOL-PATH-0019](../../research/cases/KOL-PATH-0019.json), [KOL-PATH-0022](../../research/cases/KOL-PATH-0022.json), [KOL-PATH-0030](../../research/cases/KOL-PATH-0030.json)

**Prevents:** Double release, unrelated lock removal, stale events and use after free.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Specify creation, upgrade, cancellation and destruction transitions for each object. The [capability admission record](native-model.md) and [ADR-0013](../architecture-decisions/0013-security-boundaries.md) require a demonstrated authority/lifetime distinction before adding a type; this also enforces LAW-009.

**Testing:** Explore last-release/upgrade races, name reuse, unrelated close and stalled readers.

<a name="law-018"></a>

## LAW-018 — Validated boundary data

**Rule:** External messages define widths, lengths, versions, byte order and reserved fields. Initialize all observable data before publication. Validate bounds, provenance, permissions and lifetimes; authorization and execution use the same immutable data. Time includes a clock domain and checked conversion. Unchecked boundary operations require documented assumptions, necessity, review and negative tests.

**Rationale:** Neither a convenient data layout nor an earlier check proves that later access is valid.

**Historical evidence:** [KOL-PATH-0005](../../research/cases/KOL-PATH-0005.json), [KOL-PATH-0006](../../research/cases/KOL-PATH-0006.json), [KOL-PATH-0007](../../research/cases/KOL-PATH-0007.json), [KOL-PATH-0008](../../research/cases/KOL-PATH-0008.json), [KOL-PATH-0018](../../research/cases/KOL-PATH-0018.json)

**Prevents:** Layout drift, data leaks, stale buffer flags, overflow, authorization loss and check/use races.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Audit boundary contracts and maintain a reviewable inventory of unverified assumptions. [Safe user-copy](../kernel/user-copy.md) centralizes bounded initialized snapshots, current-space lifetime and EL0 permission checks, with implementation and test links in its contract.

**Testing:** Fuzz malformed messages, reuse poisoned buffers, mutate foreign memory and test clock/size overflow. The [EL0 copy contract](../kernel/user-copy.md) links checks for range/permission errors, poisoned partial-fault scratch, initialized output and user mutation after snapshot in both profiles. General message fuzzing and shared-mapping races remain future gates.

<a name="law-020"></a>

## LAW-020 — Device memory ownership and ordering

**Rule:** Device access uses an accounted memory lease distinct from a processor reference. Release waits for completion or proven reset. Publication and ownership transitions include required ordering and cache maintenance even on coherent memory.

**Rationale:** External agents outlive ordinary call scopes and observe memory independently.

**Historical evidence:** [KOL-PATH-0009](../../research/cases/KOL-PATH-0009.json), [KOL-PATH-0010](../../research/cases/KOL-PATH-0010.json)

**Prevents:** Device writes after free and partially visible descriptors.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Review device ownership transitions, pin quotas and platform ordering contracts.

**Testing:** Test device timeout, concurrent unmapping, noncoherent memory and weak-order publication.

<a name="law-025"></a>

## LAW-025 — Single resource authority and accounting

**Rule:** Shared resources use one authoritative ownership and quota arbiter across native and compatibility clients. Every charge has an identified owner or an explicit shared-accounting rule; queues and retained resources have limits.

**Rationale:** Independent ledgers can each grant exclusive access or lose shared costs.

**Historical evidence:** [KOL-PATH-0002](../../research/cases/KOL-PATH-0002.json), [KOL-PATH-0016](../../research/cases/KOL-PATH-0016.json), [KOL-PATH-0024](../../research/cases/KOL-PATH-0024.json)

**Prevents:** Conflicting grants, uncharged retention and quota bypass.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Review resource-domain identity, charge transfer and removal.

**Testing:** Compete across personalities, exhaust quotas and remove domains with outstanding shared charges.

<a name="law-026"></a>

## LAW-026 — Observable outcomes are explicit

**Rule:** Contracts distinguish acceptance, completion, durable effect, cancellation and failure. Required I/O ordering is explicit. Owner death is reported without claiming that protected application data has been repaired.

**Rationale:** A successful intermediate step does not establish the final guarantee.

**Historical evidence:** [KOL-PATH-0001](../../research/cases/KOL-PATH-0001.json), [KOL-PATH-0015](../../research/cases/KOL-PATH-0015.json), [KOL-PATH-0025](../../research/cases/KOL-PATH-0025.json)

**Prevents:** False durable success, assumed rollback and silent owner-death recovery.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Review terminal result types and external-effect boundaries.

**Testing:** Reorder completions, fail flush, cancel after effects and kill synchronization owners.

<a name="law-027"></a>

## LAW-027 — Visible scheduling scope

**Rule:** Priority and fairness parameters identify their scheduling domain. Group weights and task weights are separate; unrelated session changes do not silently replace native policy.

**Rationale:** Relative priority has no useful meaning without its competition scope.

**Historical evidence:** [KOL-PATH-0021](../../research/cases/KOL-PATH-0021.json)

**Prevents:** Unexplained priority behavior and hidden grouping heuristics.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Expose policy domains and review scheduler interfaces.

**Testing:** Compare identical competing tasks in one and multiple domains against declared shares.

<a name="law-030"></a>

## LAW-030 — Atomic waiting and rollback of preparation

**Rule:** Waiting protocols define atomic registration, rechecking and notification. Partial preparation is undone; concurrent wake, cancellation and deadline expiry produce one defined terminal outcome.

**Rationale:** Sequential waits do not implement atomic wait-any semantics.

**Historical evidence:** [KOL-PATH-0026](../../research/cases/KOL-PATH-0026.json)

**Prevents:** Lost wakeups, leaked registrations and duplicate completion.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Review the concurrency model before implementation.

**Testing:** Insert wakeups between every preparation step and race cancellation with timeouts.

<a name="law-031"></a>

## LAW-031 — Scoped platform adaptation

**Rule:** Firmware input is validated and translated into native descriptors. Workarounds identify affected hardware, obligations, owners and removal conditions. Reconnection revokes old identity unless a recovery protocol proves continuity; matching descriptions alone is insufficient.

**Rationale:** Hardware and firmware constraints are real but must not become unbounded generic exceptions.

**Historical evidence:** [KOL-PATH-0011](../../research/cases/KOL-PATH-0011.json), [KOL-PATH-0012](../../research/cases/KOL-PATH-0012.json), [KOL-PATH-0013](../../research/cases/KOL-PATH-0013.json), [KOL-PATH-0014](../../research/cases/KOL-PATH-0014.json), [KOL-PATH-0027](../../research/cases/KOL-PATH-0027.json)

**Prevents:** Global quirks, core dependence on firmware formats and device substitution under old handles.

**Allowed exceptions:** Machine-wide scope requires evidence of a machine-wide defect. A mandatory workaround may be inseparable from support for the affected target.

**Enforcement:** Audit support manifests, parser boundaries and device generations.

**Testing:** Test affected and unaffected devices, malformed firmware and replacement devices with identical descriptors.

<a name="law-035"></a>

## LAW-035 — Profiles preserve correctness

**Rule:** Execution profiles share native semantics, mandatory checks and failure outcomes. Optional diagnostics can disappear; progress, lifetime and protection cannot depend on them.

**Rationale:** Instrumentation can alter timing but must not implement correctness.

**Historical evidence:** [KOL-PATH-0007](../../research/cases/KOL-PATH-0007.json), [KOL-PATH-0010](../../research/cases/KOL-PATH-0010.json), [KOL-PATH-0018](../../research/cases/KOL-PATH-0018.json)

**Prevents:** Production-only safety failures and architecture forks.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Use one contract suite across profiles and optimization settings.

**Testing:** Compare diagnostic-on/off outcomes and inject supported failures in controlled test builds.

<a name="law-036"></a>

## LAW-036 — Honest observations and comparisons

**Rule:** Metrics state scope, units, denominator, coverage and attribution. Missing data is unknown, not zero. Comparisons use equivalent work and guarantees, predeclared sampling, retained raw results and reported failures; neither route receives an artificial advantage.

**Rationale:** A dependency label or favorable average cannot establish cost or correctness.

**Historical evidence:** [KOL-PATH-0015](../../research/cases/KOL-PATH-0015.json), [KOL-PATH-0021](../../research/cases/KOL-PATH-0021.json), [KOL-PATH-0025](../../research/cases/KOL-PATH-0025.json), [KOL-PATH-0026](../../research/cases/KOL-PATH-0026.json)

**Prevents:** Invented scores, double accounting, cherry-picking and hidden tail failures.

**Allowed exceptions:** Optional production counters may be absent; the value is unavailable rather than fabricated.

**Enforcement:** Review metric definitions and experiment manifests; keep hardware adaptation separate from software dependence. Follow the [document status policy](../documentation-policy.md) and scoped ADR-0010 evidence when reporting implementation milestones.

**Testing:** Check zero denominators, lost events, nested spans, censored tails and a legitimately faster compatibility result.

<a name="law-040"></a>

## LAW-040 — Evidence and reviewable architectural change

**Rule:** Every architectural obligation has a concrete failure or constraint, evidence, scope and a falsifiable test. Distinguish historical facts, inferences and proposals. Add or remove laws because obligations change, never to meet a numerical quota; record the disposition of superseded requirements.

**Rationale:** A language choice, desired list length or another project surviving is not engineering evidence. Selection bias can hide failed alternatives and unreported defects.

**Historical evidence:** [KOL-PATH-0007](../../research/cases/KOL-PATH-0007.json), [KOL-PATH-0028](../../research/cases/KOL-PATH-0028.json), [KOL-PATH-0029](../../research/cases/KOL-PATH-0029.json), [KOL-PATH-0030](../../research/cases/KOL-PATH-0030.json)

**Prevents:** Slogans, unsupported claims and silent loss of requirements during consolidation.

**Allowed exceptions:** No implicit exceptions. Any change must preserve security obligations and be approved through an architecture decision.

**Enforcement:** Review source records, requirement mapping and architecture decisions. Require a concrete KOLVRT need, disconfirming evidence, applicability limits and a comparison with deferring the feature.

**Testing:** Reject missing evidence, unmapped prior obligations and fabricated execution claims; permit any justified nonempty law set.

<a name="law-041"></a>

## LAW-041 — Progress has explicit dependencies and bounds

**Rule:** Every blocking protocol states its progress assumptions, lock order, cancellation path and resource bounds. Do not hold a resource across a call that can require the same resource to complete. Interrupt and non-preemptible work must be bounded.

**Rationale:** Correct object ownership and visible scheduling scope alone do not prevent deadlock or starvation.

**Historical evidence:** [KOL-PATH-0021](../../research/cases/KOL-PATH-0021.json), [KOL-PATH-0026](../../research/cases/KOL-PATH-0026.json); the native [scenario audit](law-audit.md).

**Prevents:** Cyclic wait dependencies, unbounded cleanup and progress depending on diagnostic activity.

**Allowed exceptions:** An operation may have conditional progress only when the missing environmental guarantee is explicit and failure has a defined outcome.

**Enforcement:** Review the wait-for graph, lock hierarchy, admission limits and scheduler assumptions before publication.

**Testing:** Exhaust every bounded queue, enumerate runnable sets, interrupt dependency transitions and test timeout without reclaiming still-accessible storage.

<a name="law-042"></a>

## LAW-042 — Failure containment and recovery are explicit

**Rule:** Each component declares its trust and failure domain. Recovery starts from a known valid state with explicitly restored authority; it cannot treat unknown effects as absent or silently continue after a trusted invariant failure.

**Rationale:** Safe parsing and permissions alone do not determine what survives a component crash.

**Historical evidence:** [KOL-PATH-0025](../../research/cases/KOL-PATH-0025.json), [KOL-PATH-0030](../../research/cases/KOL-PATH-0030.json); the native [threat model](threat-model.md).

**Prevents:** Restart under stale authority, false successful recovery and a library boundary being mistaken for isolation.

**Allowed exceptions:** A privileged component may share the kernel failure domain only with an explicit trusted-base justification; its failure is then fatal to that domain.

**Enforcement:** Record isolation, termination, supervisor responsibility, recovery state and externally observable failure. [ADR-0008](../architecture-decisions/0008-placement.md) and the [admission policy](kernel-admission-policy.md) require privileged necessity for each EL1 responsibility; performance or authoritative state alone cannot revise a protection boundary.

**Testing:** Kill a service before and after acceptance, reject malformed replies, preserve supervisor operation and restart without inheriting old handles.

<a name="law-043"></a>

## LAW-043 — Publication and shutdown follow dependency order

**Rule:** Publish a subsystem only after its dependencies and rollback obligations are satisfied. Shutdown stops entry, drains or terminates admitted work and quiesces asynchronous sources before releasing reachable resources.

**Rationale:** Individually valid object lifetimes do not establish safe ordering across interrupts, mappings and subsystem entry points.

**Historical evidence:** [KOL-PATH-0008](../../research/cases/KOL-PATH-0008.json), [KOL-PATH-0030](../../research/cases/KOL-PATH-0030.json); the native [scenario audit](law-audit.md).

**Prevents:** Half-initialized public services and callbacks into released memory during shutdown.

**Allowed exceptions:** Failed quiescence can require quarantine or global stop; a deadline alone never proves absence of access.

**Enforcement:** Maintain a dependency graph and explicit publication, rollback and shutdown states.

**Testing:** Fail each initialization stage and race admission, completion and shutdown; a negative control must expose premature resource release.

[Russian translation](../../translations/ru/docs/architecture/kernel-laws.md)

<a name="law-044"></a>

## LAW-044 — Tests validate architecture; they do not define it

**Rule:** Production semantics follow independently justified invariants and accepted contracts. A test failure requires establishing either a proven implementation defect or an incorrect test and repairing the faulty side. Never add workarounds, special cases, synthetic results, weaker validation, delays or performance regressions merely to make tests falsely pass. Production source contains no test-specific corruption, forced rendezvous or fault-injection entry points; test logic and mutations live in separate test artifacts. Positive tests exercise the ordinary implementation through its actual interfaces.

**Rationale:** Passing a test is evidence only when the implementation satisfies the intended contract and the test measures it honestly. A fabricated observation or implementation adapted solely to the expected result destroys that evidence.

**Historical evidence:** [KOL-PATH-0008](../../research/cases/KOL-PATH-0008.json) illustrates repairing a demonstrated constructor invariant violation while preserving the useful transfer mechanism. It does not establish test isolation; that obligation records the explicit Phase 3.7 architectural decision.

**Prevents:** False-positive tests, test-induced production behavior, hidden fault-injection APIs and performance sacrificed for green checks.

**Allowed exceptions:** No implicit exceptions. Correcting an independently demonstrated implementation defect is permitted and must retain the evidence establishing the defect; test expectations alone are insufficient justification.

**Enforcement:** UNIT and INTEGRATION import the single production implementation. E2E uses the actual kernel and applications; an external harness runs the scenario through public interfaces. A separate ELF is allowed only for a necessary external ABI/syscall client without a production counterpart. Production implementation copies and source-copy mutations are forbidden, including isolated builds.

**Testing:** Reject injected false success and unrelated failure; verify negative controls detect the selected real defect and positive execution uses unmodified production behavior.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel-laws",
  "kind": "policy",
  "summary": "Normative kernel invariants; choose relevant law units.",
  "units": [
    {
      "id": "law.001",
      "anchor": "law-001",
      "kind": "law",
      "summary": "Native authority and dependency isolation",
      "depends_on": [],
      "aliases": ["LAW-001"]
    },
    {
      "id": "law.003",
      "anchor": "law-003",
      "kind": "law",
      "summary": "Concurrent native and compatibility routes",
      "depends_on": [],
      "aliases": ["LAW-003"]
    },
    {
      "id": "law.004",
      "anchor": "law-004",
      "kind": "law",
      "summary": "Explicit semantic binding",
      "depends_on": [],
      "aliases": ["LAW-004"]
    },
    {
      "id": "law.005",
      "anchor": "law-005",
      "kind": "law",
      "summary": "Shared-state consistency",
      "depends_on": [],
      "aliases": ["LAW-005"]
    },
    {
      "id": "law.008",
      "anchor": "law-008",
      "kind": "law",
      "summary": "Controlled evolution and retirement",
      "depends_on": [],
      "aliases": ["LAW-008"]
    },
    {
      "id": "law.009",
      "anchor": "law-009",
      "kind": "law",
      "summary": "Translation cannot expand effective authority",
      "depends_on": [],
      "aliases": ["LAW-009"]
    },
    {
      "id": "law.013",
      "anchor": "law-013",
      "kind": "law",
      "summary": "Explicit identity and lifetime ownership",
      "depends_on": [],
      "aliases": ["LAW-013"]
    },
    {
      "id": "law.018",
      "anchor": "law-018",
      "kind": "law",
      "summary": "Validated boundary data",
      "depends_on": [],
      "aliases": ["LAW-018"]
    },
    {
      "id": "law.020",
      "anchor": "law-020",
      "kind": "law",
      "summary": "Device memory ownership and ordering",
      "depends_on": [],
      "aliases": ["LAW-020"]
    },
    {
      "id": "law.025",
      "anchor": "law-025",
      "kind": "law",
      "summary": "Single resource authority and accounting",
      "depends_on": [],
      "aliases": ["LAW-025"]
    },
    {
      "id": "law.026",
      "anchor": "law-026",
      "kind": "law",
      "summary": "Observable outcomes are explicit",
      "depends_on": [],
      "aliases": ["LAW-026"]
    },
    {
      "id": "law.027",
      "anchor": "law-027",
      "kind": "law",
      "summary": "Visible scheduling scope",
      "depends_on": [],
      "aliases": ["LAW-027"]
    },
    {
      "id": "law.030",
      "anchor": "law-030",
      "kind": "law",
      "summary": "Atomic waiting and rollback of preparation",
      "depends_on": [],
      "aliases": ["LAW-030"]
    },
    {
      "id": "law.031",
      "anchor": "law-031",
      "kind": "law",
      "summary": "Scoped platform adaptation",
      "depends_on": [],
      "aliases": ["LAW-031"]
    },
    {
      "id": "law.035",
      "anchor": "law-035",
      "kind": "law",
      "summary": "Profiles preserve correctness",
      "depends_on": [],
      "aliases": ["LAW-035"]
    },
    {
      "id": "law.036",
      "anchor": "law-036",
      "kind": "law",
      "summary": "Honest observations and comparisons",
      "depends_on": [],
      "aliases": ["LAW-036"]
    },
    {
      "id": "law.040",
      "anchor": "law-040",
      "kind": "law",
      "summary": "Evidence and reviewable architectural change",
      "depends_on": [],
      "aliases": ["LAW-040"]
    },
    {
      "id": "law.041",
      "anchor": "law-041",
      "kind": "law",
      "summary": "Progress has explicit dependencies and bounds",
      "depends_on": [],
      "aliases": ["LAW-041"]
    },
    {
      "id": "law.042",
      "anchor": "law-042",
      "kind": "law",
      "summary": "Failure containment and recovery are explicit",
      "depends_on": [],
      "aliases": ["LAW-042"]
    },
    {
      "id": "law.043",
      "anchor": "law-043",
      "kind": "law",
      "summary": "Publication and shutdown follow dependency order",
      "depends_on": [],
      "aliases": ["LAW-043"]
    },
    {
      "id": "law.044",
      "anchor": "law-044",
      "kind": "law",
      "summary": "Tests validate architecture; they do not define it.",
      "depends_on": [],
      "aliases": ["LAW-044"]
    }
  ]
}
```
