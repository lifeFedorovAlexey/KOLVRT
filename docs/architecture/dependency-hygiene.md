# Dependency hygiene

Document status: CURRENT
Evidence scope: host Cargo metadata, source audit and dependency policy enforcement; no kernel behavior claim.
Current reference: [Dependency checker](../../scripts/dependency-audit.cjs)

<a name="kolvrt-dependency-hygiene"></a>

## Policy and implementation

### Kernel boundary

Production EL1/TCB admits exactly `kolvrt-kernel → kernel-core`, with no external Cargo crates. Both crates have zero build/dev dependencies. The checker rejects any additional declaration, including optional dependencies, renamed packages, target predicates and paths to host crates. This declaration-level invariant covers every feature combination without enumerating exponentially many configurations or relying on workspace feature unification. Rust `core` and compiler-builtins remain compiler trust; zero external Cargo crates is not a claim of zero compiler/runtime code or measured TCB bytes.

A dependency exception requires a separate accepted architectural decision: need, small custom alternative, size/closure, unsafe, maintenance, license, security history, TCB impact, and removal/replacement. Convenience is insufficient. No exceptions are currently supported. An accepted change must explicitly update the checker and rejection controls. Implementation order is not architectural authority.

### Host supply chain

[Retention policy](../../dependencies/policy.json) records each consumer/crate/dependency-kind requirement, purpose and retention reason. New direct dependencies and requirement changes fail until this decision is reviewed. Generated inventory adds version, license, source, features, direct/transitive status, closure and reachability. Routing/demo dependencies are explicitly outside production EL1; demo EL0 code is not labelled purely host-only. Path dependencies outside the workspace count as external and are denied by the current crates.io-only source policy. Six previously unlicensed workspace crates are marked `publish = false`; this assigns no license and prevents accidental publication. Private workspace license metadata is skipped; their external dependencies are fully checked.

`deny.toml` allows only crates.io sources and explicit licenses (including MIT-0), denies external wildcard versions, OpenSSL and RustSec/yanked findings, and reports duplicate versions. Local unpublished path dependencies are permitted. There are no advisory exemptions. Dependency count is not an optimization target or a fixed quota. Every graph growth is flagged for review, including additions masked by removals with the same total count.

Dependabot checks GitHub Actions, Cargo and npm weekly on Mondays at 06:00/06:15/06:30 Europe/Moscow, labels updates dependencies, and opens at most five PRs per ecosystem. Direct Rust updates require retention requirement review; automatic merge is not configured. Configuration becomes active after integration into the default branch.

### Evidence and checks

Run `npm run check:dependencies` or `node scripts/dependency-audit.cjs check`. Run the latter with `--supply-chain` after installing `cargo-deny` 0.19.9. The script uses Node built-ins and existing Cargo metadata; no kernel or host crate dependency is introduced. `npm run check` includes the boundary/inventory/rejection tests. CI adds all four cargo-deny checks on pushes, PRs and weekly advisory refreshes.

Kernel foundation runs the dependency boundary checks once in its mandatory static workload for main pushes, PRs and manual dispatch. The legacy serial mode is removed, so a successful baseline job cannot bypass the partitioned host/kernel evidence gate. The separate Dependency hygiene workflow continues to enforce the supply-chain audit.

`target/dependencies/inventory.json` contains default and all-workspace-features graphs for every workspace crate, normal/build/dev edges and target predicates, per-direct-dependency external closures, compiler/lock/policy provenance, duplicate versions and supply-chain diagnostics. Counts exclude the root and distinguish unique crate names from resolved package versions. These are conservative workspace-unified/all-target graphs, not isolated consumer feature counts or actual linked binary sizes. A skipped or incomplete supply check has explicit status and null metrics, never fabricated zeroes. RustSec database commit and raw diagnostics are retained when checked.

CI compares the actual PR base SHA using current checker code against historical manifests, publishes `before → after` in the PR check summary and uploads commit-named evidence for 90 days. No write token or automated PR comment is required; historical audit scripts are not executed. Downloaded artifacts provide recent commit history; archive artifacts externally before expiry for long-term retention. The checked-in [main baseline](../../dependencies/baseline-main.json) remains immutable evidence of main `599fa24a3278297f9fc9614b0882dd4afc98d423`; it is graph evidence, not a historical claim that current supply policy already existed.

After integration of #92, an additional [current main snapshot](../../dependencies/baselines/0a3a3d69ff2381b6ec922682af03c1fd13e41a23.json) records `0a3a3d69ff2381b6ec922682af03c1fd13e41a23`. The first baseline is unchanged; the new host-process-metrics/serde dependency is included in retention policy. No external package versions were added.

### First heavy dependency audit

The [machine-readable audit](../../dependencies/heavy-audit.json) records pinned-source hashes, source bytes, lexical unsafe token occurrences, features, licenses, security review, TCB impact and removal conditions. Source bytes/token counts include cfg-disabled code and comments; they are neither compiled size nor a safety proof. Maintenance review is the pinned version/source and checked RustSec snapshot, not an assertion that a crate is latest or vulnerability-free.

| Dependency          | Decision        | Transitive package versions | Reason                                                                                               |
| ------------------- | --------------- | --------------------------- | ---------------------------------------------------------------------------------------------------- |
| jsonschema 0.33.0   | KEEP            | 98 → 98                     | Draft 2020-12 validator; HTTP/file resolution already disabled; no reqwest.                          |
| ed25519-dalek 2.2.0 | KEEP            | 34 → 34                     | Strict provenance verification and fixture signatures; retain zeroization and reviewed cryptography. |
| quanta 0.13.0       | REDUCE FEATURES | 28 → 28                     | Disable unused mock/flaky_tests; retain calibrated clock semantics.                                  |

Baseline root source sizes are 671,464 / 111,014 / 63,273 bytes respectively; lexical unsafe occurrences are 0 / 5 / 6. Transitively reachable code may contain unsafe even when the root has none. Kernel external crates remain zero. The workspace has eight direct external crate names and 130 external package versions (128 names); `getrandom` and `syn` each have two versions. The completed 2026-10-05 check found zero matching advisories and zero license/source violations under the retained RustSec database snapshot. This is scoped evidence, not certification.

Historical [RUSTSEC-2022-0093](https://rustsec.org/advisories/RUSTSEC-2022-0093.html) affects pre-v2 ed25519-dalek; pinned 2.2.0 uses `SigningKey`, strict verification, and no hazmat feature. Transitive [RUSTSEC-2024-0344](https://rustsec.org/advisories/RUSTSEC-2024-0344.html) is covered by the current RustSec scan of curve25519-dalek 4.1.3. Replacing quanta requires a new timing profile and overhead/resolution evidence; replacing jsonschema requires equivalent schema rejection tests; custom cryptography is not an acceptable count reduction.

### Verification

Rejection controls include real Cargo metadata for optional/target-specific, build and dev dependencies, disguised paths, host reachability, missing TCB members, graph cycles/diamonds, equal-count replacements, feature changes, retention changes and incomplete supply-chain output. Workbench/advisor/schema-validator regression tests cover the quanta feature reduction. CI supplies live advisory/license/source enforcement and records the results separately from kernel behavior evidence.

Both CI workflows use Node 24.21.0 from `.nvmrc`; locally, install/use that version with nvm (`nvm install 24.21.0`, `nvm use 24.21.0`). Checkout 7.0.1, setup-node 7.0.0 and upload-artifact 7.0.1 explicitly use the Node 24 action runtime. The dependency runner is pinned to `ubuntu-24.04`, so a future `ubuntu-latest` migration cannot silently change the audit environment. The kernel runner remains `windows-2022`. Local checks under Node 24 do not verify either hosted runner; fresh CI evidence is required before restoring exact-source VERIFIED status.

Issue #109 adds an integrity-checked pinned cargo-deny 0.19.9 tool cache and mandatory version checks on Ubuntu. The kernel workflow splits static/host checks and runs a complete four-shard matrix with exact-source aggregation; npm run check preserves all unique host validations. Download caches and compiled artifacts never authorize skipping supply-chain or kernel checks. Hosted cache behavior remains unverified; historical dependency receipts stay STALE.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.dependencies.hygiene",
  "kind": "policy",
  "summary": "Production TCB dependency boundary and observable host supply chain.",
  "units": [
    {
      "id": "kolvrt.dependencies.hygiene",
      "anchor": "kolvrt-dependency-hygiene",
      "kind": "feature",
      "summary": "All-declaration kernel boundary, feature-expanded inventory, graph deltas and supply-chain checks.",
      "tags": ["dependencies", "supply-chain", "kernel", "tcb"],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "Host-only metadata and policy checker; no external production kernel dependency, no count quota.",
        "sources": [
          "scripts/dependency-audit.cjs",
          "scripts/dependency-base.cjs",
          "scripts/audit-heavy-dependencies.cjs",
          "scripts/tests/dependency-audit.test.cjs",
          ".github/workflows/dependencies.yml",
          "dependencies/policy.json",
          "deny.toml",
          "package.json",
          "crates/kernel/Cargo.toml",
          "crates/kernel-core/Cargo.toml",
          "crates/routing/Cargo.toml",
          "crates/routing-demo/Cargo.toml",
          "crates/window-compat/Cargo.toml",
          "crates/xtask/Cargo.toml",
          "crates/migration-workbench/Cargo.toml",
          ".github/dependabot.yml",
          ".github/workflows/kernel.yml",
          ".nvmrc"
        ],
        "acceptance": ["research/results/issue94-dependency-hygiene.json"],
        "issues": [94],
        "adrs": ["adr.0013"],
        "readiness": "NOT_READY",
        "next_gate": "Review EN/RU policy, rerun Node 24 CI on Windows 2022 and Ubuntu 24.04, and review PR-base delta before integration.",
        "limitations": [
          "Workspace-unified all-target closures are conservative, not compiled size or isolated consumer counts. Artifact history expires after 90 days without external archival. No kernel execution claim."
        ],
        "verification": [
          {
            "environment": "host-process",
            "state": "STALE",
            "reason": "Phase 3.7 removes source-copy mutation builds and application implementation copies. Tests use the actual production code; replacement fault/restart/shutdown and related acceptance scenarios remain incomplete. Prior receipts retain their historical scope; partial passes are not full current-source acceptance.",
            "scope": "Host dependency policy and compile checks only; no kernel runtime or physical-hardware claim.",
            "receipt": "research/results/issue94-dependency-hygiene.json",
            "receipt_sha256": "a27a01010ef49bbfcb545513c59d932849f91901e562fee57752786d95789b2f"
          },
          {
            "environment": "physical-arm64",
            "state": "NOT_APPLICABLE",
            "reason": "Host dependency checks do not establish kernel behavior."
          }
        ],
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "First bounded implementation of the dependency hygiene policy, inventory and CI controls.",
            "acceptance": ["research/results/issue94-dependency-hygiene.json"]
          }
        ]
      }
    }
  ]
}
```
