# Arena measurement contract

Document status: CURRENT
Evidence scope: experimental general offline A0/A1 contracts and import admission plus the accepted bounded QEMU CLOCK pilot; no records service, general independent attestation or certification.
Current reference: [Arena](component-arena.md); [ADR-0006](../architecture-decisions/0006-metrics.md); [knowledge contract](../knowledge-system.md)

## Architecture and scope

Issue [#93](https://github.com/lifeFedorovAlexey/KOLVRT/issues/93) defines the shared Arena model. This first implementation uses the existing repository-checks library and xtask facade. There is no new kernel dependency, instrumentation, architecture registry or score. Canonical Markdown owns block/contract IDs; current profile checks resolve them through the existing knowledge builder. This does not implement versioned runtime graph edges or historical graph reconstruction.

The [range profile](../../research/arena/profiles/user-copy-range.json) is a PROPOSED synthetic host profile for an existing kernel-core range-validation contract. It is not an accepted SafeCopy PERF/SEC passport or a measured kernel run. Its custom measure and SFR/SAR are KOLVRT_DEFINED. Current canonical text bytes are pinned conservatively as the contract digest; a later semantic-subset revision requires reviewed rules, not automatic compatibility.

The [CLOCK pilot](../research/clock-passport.md) now specifies a bounded real-kernel pipeline using this same schema and registry. Its reviewed DEV/PROD profiles measure a three-CLOCK envelope with one useful query and matched intermediate-recorder ON/OFF observations. Existing public execution-window counters provide partial attribution, not exclusive processor cost. The accepted execution retains twelve fresh successful boots and six portable pair passports, with source-bound review and full regression evidence. It does not change offline record eligibility or claim the full cost of timestamp probes.

## Standards registry

The [registry](../../research/arena/standards.json) and [closed schema](../../schemas/arena-standards.schema.json) pin stable ID/version, scope, authoritative source and source revision, source kind, Arena applicability, publication state, verification date and supersession. ISO/IEC 25010:2023 is taxonomy; ISO/IEC 25023:2016 supplies quality measures and records its draft expected replacement separately; ISO/IEC 25040:2024 is the evaluation framework. Applicable ISO/IEC 15408 parts have separate versions. CC:2022/CEM:2022 include Release 1 and errata 1.2; ISO/IEC 18045:2026 is a distinct reference. NIST SP 800-55 Vol.1/2 (2024) are separate guidance entries.

SPEC CPU 2017 rules supply reproducibility, validation, repeat-run and disclosure principles; Arena does not run SPEC CPU suites or claim SPEC conformity. Linux perf bench v6.12 and fio 3.42 are pinned de-facto references, not ISO standards. RFC 2544 and its applicability/update references are separately versioned and do not authorize tests on live production networks. KOLVRT-defined methodology version 1 defines the experimental adaptation and descriptive analysis below. No normative ISO/CC clause mapping or certification is inferred from public metadata.

Registry entries are keyed by ID/version. Duplicates, malformed dates, floating branch locators, source-kind/authority mismatches, absent successors, draft successors and supersession cycles fail. Updating the current registry cannot update embedded historical snapshots. Publisher/reviewer authenticity and real-world equivalence of a claimed source revision remain review obligations. Reverse metric/security references are derived from profiles by the registry command.

## Measurement Profile and comparison class

The [profile schema](../../schemas/arena-profile.schema.json) requires canonical target/functional version and contract digest, workload/input/oracle identities and digests, environment/hardware/architecture/toolchain/config/resource policy, sampling/stopping/analysis, observer-cost protocol, units/denominator, source sections/versions and metric visibility. PERF, REL and RES are dimensions; RES means resource usage/cost, not a new ISO quality characteristic. SEC has TOE, SPD assumptions/threats/policies/boundaries, explicit custom SFR and required SAR methods/scopes, plus AVA_VAN/AP exclusions. Undefined SFR/SAR links and undeclared methodologies fail.

Version 1 uses a conservative comparison key over all profile fields except review_state, plus equality of every referenced frozen methodology definition. Schema, profile version, functional/workload/security/methodology, environment, toolchain, budgets, sampling and observer protocol changes therefore create different classes. SHARED is a collection visibility declaration: it does not mix DEV/PROD. DEV_INTERNAL requires a DEV internal mechanism; PROD only permits EXTERNAL. Exact implementation commit/source/image identities live in run provenance, so an implementation change does not freeze a series. Admission still requires newly supplied exact artifacts and applicable evidence; old assertions do not establish a new image's behavior.

## Frozen imports and admission

The [run schema](../../schemas/arena-run.schema.json) embeds frozen registry/profile snapshots with canonical JSON SHA-256 digests, exact commit/source/image identities, submission/contributor roles, complete environment manifest and byte-digested bundle artifacts. Canonical JSON digests use serde_json ordered maps and compact serialization; artifact digests use original bytes without newline normalization. Raw observations and marked warmup remain in the receipt. Bundle references must be relative, stay inside the bundle after canonicalization and point to verified files. Inputs/artifacts are bounded to 32 MiB each. These imports neither persist immutable history nor attest execution.

All declared metrics are required for structural admission in v1; optional-metric admission needs a reviewed policy revision. Every metric occurs exactly once as available raw samples or a typed missing observation with reason/count/evidence. Failures, dropped/unfinished samples, inadequate declared sample/warmup count, failed or absent correctness evidence, unreviewed profile, missing mandatory SFR/SAR and absent matched on/off observer observations yield INELIGIBLE. Unknown/duplicate SFR/SAR/metric IDs, bad digests and unrelated evidence fail validation. A PASS assertion without all required SAR artifacts cannot satisfy the SFR. Matched observer protocol ID/version/workload must agree; overhead is reported as separate evidence and is never subtracted as an exact correction.

The command reports descriptive nearest-rank median/p95/p99, arithmetic mean or raw observation count according to the declared metric. It makes no throughput, PMU, tail-adequacy, repeated-run regression, uncertainty interval or superiority inference. A producer assertion passing these gates is STRUCTURALLY_ADMISSIBLE only. record_eligible is always false in this stage: general independently verified execution/applicability/contribution admission, record computation/co-holders and append-only invalidation/history are not implemented. The CLOCK pilot retains its own bounded independent source and execution review; that review does not implement the general records workflow. Synthetic fixtures are explicitly ineligible as kernel evidence. Required repeated-run/uncertainty and authenticity policy must be added before publication; supplied REVIEWED/PASS/ACCEPTED labels are not trusted attestations.

## Commands and verification

Run from the repository root:

```text
pwsh -File scripts/run-arena.ps1
pwsh -File scripts/run-arena.ps1 registry
pwsh -File scripts/run-arena.ps1 profile
cargo xtask arena check
cargo xtask arena registry
cargo xtask arena profile research/arena/profiles/user-copy-range.json
cargo xtask arena assess PATH_TO_RUN_JSON
cargo xtask arena compare LEFT_RUN_JSON RIGHT_RUN_JSON
cargo test --locked -p repository-checks --test arena
```

The shared repository check validates the checked-in registry and proposed profiles. assess prints the retained rejection reasons and exits nonzero for INELIGIBLE; compare rejects incompatible or structurally rejected imports. Neither command publishes a record. Tests mutate provenance, methodology, environment, oracle, missing SAR, observation loss, overhead and artifact paths/digests; synthetic numbers never become measured kernel results. Closed schemas also reject added universal scores and missing units/source fields.

The one-command launcher `scripts/run-arena.ps1` supports check/registry/profile/assess/compare, defaults to check, and selects the proposed range profile when profile has no path. It runs current source through the existing xtask, resolves relative inputs from the project root and preserves rejection exit codes. Cargo comes from PATH or an existing project `.toolchains` installation (including the shared Git checkout for worktrees); environment is restored afterward. `-Help` starts no build. It adds no graphical interface or record publication.

## Remaining gates

The bounded CLOCK pilot for #32 supplies the first accepted real QEMU mechanism passport: DEV/PROD external envelopes, partial execution-window accounting, matched recorder cost and scoped host admission/security evidence. It does not establish exclusive CPU attribution, broad SEC coverage or physical performance. #33 supplies IPC after #26; #34/#35 supply REL/fuzz/SAR campaigns; #36 integrates publication/regression gates; #50 owns native/compat equivalence and COST-L attribution. CLOCK acceptance closes none of those later scopes or the complete #93 acceptance. Reviewed source mappings, signed/independent custody where required, accepted contribution verification, record history and canonical versioned graph projections remain open. Physical ARM64 performance requires its own campaign; QEMU is a separate reproducibility/regression environment.

[Russian translation](../../translations/ru/docs/architecture/arena-measurement-contract.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.arena.measurement-contract",
  "kind": "subsystem-contract",
  "summary": "Experimental standards registry, frozen measurement profiles and offline import admission; no kernel records.",
  "depends_on": [
    "adr.0006",
    "adr.0003",
    "adr.0024",
    "kolvrt.memory.user-copy.api"
  ],
  "tags": ["arena", "standards", "measurement", "security"],
  "read_when": ["Arena Measurement Profile comparison class SFR SAR admission"]
}
```
