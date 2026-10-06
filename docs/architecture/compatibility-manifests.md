# Compatibility module declarations

Document status: CURRENT
Evidence scope: bounded host/CI declarations and synthetic regression fixtures; no loader, Linux driver support or deployed module acceptance.
Current reference: [Checker](../../crates/repository-checks/src/compat_modules.rs)

<a name="kolvrt-compatibility-manifests"></a>

## Architecture and authority

This gate follows LAW-008/009/013/018/035/040, ADR-0007 and ADR-0013. Module identity, debt identity, exception identity, Cargo package version and runtime authorization remain distinct. Earlier COST-L query projections do not constrain this contract: their input is validated research declarations, whereas this gate independently checks declared module distribution. No manifest grants authority, selects a route, admits a request or unloads code. Native authorization and runtime quiescence still require their own mechanisms and evidence.

[Closed schema v1](../../schemas/compatibility-modules.schema.json) describes [the declaration inventory](../../policy/compatibility-modules.json). It is a bounded host projection of module declarations, not a second debt registry. A module pins behavior ID, semantic version, scope, Cargo package/version/feature, classification, distribution state, owner, finite support deadline, source hashes, artifact and named consumers. No consumer count is stored: membership is derived from the arrays. Semantic versions are deliberately numeric triplets under the existing COST-L format; package versions and executable hashes are independent.

## Distribution and reciprocal relations

`DECLARED` denotes current declared inclusion, including optional features; it does not assert active runtime use. `RETIRED` retains a tombstone with artifact, sources, named historical consumers and debt relations. A retired identity cannot remain in a current Cargo marker. Removing a marker is not proof of drained references or safe unloading. Archived installation and renewal require separate reviewed support and native authorization; this format supplies neither.

Every `PRODUCTION` declaration requires at least one existing applicable COST-L ID and retained artifact bytes with matching SHA-256. Every referenced debt must reciprocate the same ID/version/scope/artifact. Every consumer edge must agree in both directions on identity, kind, direct/transitive classification, scope, deadline and exact module reference. Shared debts and multiple modules are permitted. A module's deadline cannot exceed the supporting debt's deadline; named consumer deadlines cannot exceed the module's. Debt and module owners may differ: neither name establishes authority or evidence of ownership. Current production declarations require ACTIVE or DEPRECATED debt; retired tombstones require RETIRING or RETIRED debt. Candidate research cannot silently become production support.

Consumers may include explicitly named offline recovery or archival packages; zero sampled calls cannot erase their declared obligations. No clock, runtime counters or observed consumer coverage is inferred from this inventory. The gate compares declared calendar deadlines; the existing exception gate checks due software reviews/support. It does not stop admissions at expiry or prove that an application is supported today.

`SYNTHETIC` declarations require an explanation, empty production-debt references and no production artifact. Current data contains only the three existing `window-compat` fixture contracts: inclusive, counted and empty-first. Their semantic IDs/versions already occur in routing. Their support deadline is fixture policy, not an ABI promise. The empty-first route references EXC-0001, whose source boundary and software deadline are checked. Existing COST-L-0001…0003 remain research candidates with no invented module, consumer, Linux history or measured costs.

## Cargo and source boundary

Each declared package lists its ID/version markers in `[package.metadata.kolvrt].compatibility_modules`. Every marker must have one matching current manifest, and every current manifest must have its marker, exact package version and existing feature. Sources include the Cargo manifest and all Cargo target entry files, with normalized-LF SHA-256. Auxiliary source/build-input completeness still needs review; entry-file coverage is not a proof of every transitive source byte. A retained artifact hash identifies bytes, not execution, producer honesty or correspondence to source.

The native roots are fixed in code as `kolvrt-kernel` and `kernel-core`; inventory edits cannot remove them. Cargo's locked offline `metadata --no-deps` output provides actual parsed declarations. The conservative traversal includes normal/build/dev, optional, target-specific and renamed edges, even when disabled. Reaching a compatibility package or an external/unresolved dependency fails. This intentionally rejects an optional compatibility dependency in a native root rather than treating disabled code as acceptable kernel architecture. It is a declaration invariant across features, not a linker/runtime isolation proof. Separate native-only routing tests remain required; this gate does not classify every host or EL0 package as a native root.

An unmarked adapter cannot be discovered from semantics by Cargo metadata. Review must identify and mark compatibility packages; omission of both code intent and metadata is outside structural completeness. Cargo metadata is generated from the reviewed checkout, not accepted from an untrusted caller as runtime policy. Future formats must be re-derived from accepted authority/lifetime invariants rather than preserving this early representation for convenience.

## Bounds and checks

Inventory files are limited to 64 KiB, 128 modules, 32 debts/exceptions per module, 16 source files and 128 consumers per module. Schemas reject unknown fields, invalid dates/versions, empty identities and oversized values. Strict JSON parsing rejects duplicate keys. Duplicate module and consumer identities fail even when their other fields differ. Repository file references must remain inside the checkout. Artifact reads are limited to 64 MiB, including a growth check; Cargo metadata output is limited to 16 MiB after the trusted child completes. These are host input limits, not runtime budgets or protection against an adversarial Cargo executable/build configuration.

```text
cargo run --locked -p repository-checks -- check-compatibility
cargo test --locked -p repository-checks --test compat_modules
```

Normal `check` and `validate` commands run the gate, so existing repository CI includes it. Default COST-L queries now validate and join this inventory before stdout; custom `--directory` inventories remain independent unless `--manifests PATH` explicitly selects the join. See the [query contract](../research/cost-l-queries.md) for machine schema version 2 and exact-source host evidence. Production data/completeness and human acceptance remain separate #47/#48 gates.

Thirteen tests cover current synthetic inventory and the real CLI; isolated production-shaped and retirement fixtures; many-to-many edges; missing justification, dangling relations, artifact/source/version/scope mismatches; offline consumers/deadlines; schema bounds; EXC expiry; Cargo markers/features; native optional/renamed/target/build/dev rejection. A temporary real Cargo workspace tests parsed disabled renamed target declarations. Production-shaped fixture artifacts are deliberately non-executable bytes; passing is no evidence of a supported driver or executable module. [Exact-source evidence](../../research/results/issue47-compatibility-manifests.json) records this host scope.

Remaining #47 gates include reviewed production module/debt/consumer data when actual support exists, source/feature completeness and owner/semantic review. Runtime retirement, distribution/archival policy and effective-authority enforcement are not implemented here. #46 research/history and #48 production-query completeness remain open. This bounded checker does not close those issues or establish production readiness.

[Russian translation](../../translations/ru/docs/architecture/compatibility-manifests.md)

<!-- knowledge -->

```json
{
  "id": "doc.kolvrt.architecture.compatibility-manifests",
  "schema_version": 1,
  "units": [
    {
      "id": "kolvrt.compatibility.manifests",
      "summary": "Bounded compatibility module declarations and reciprocal COST-L/Cargo checks without runtime authority.",
      "anchor": "kolvrt-compatibility-manifests",
      "tags": [
        "compatibility",
        "debt",
        "module",
        "manifest",
        "cargo",
        "retirement"
      ],
      "depends_on": [
        "law.008",
        "law.009",
        "law.013",
        "law.018",
        "law.035",
        "law.040",
        "adr.0007",
        "adr.0013",
        "adr.0015"
      ],
      "kind": "feature",
      "feature": {
        "roadmap_gate": "COST-L module/CI declarations #47",
        "sources": [
          "Cargo.lock",
          "Cargo.toml",
          "crates/host-process-metrics/Cargo.toml",
          "crates/kernel-core/Cargo.toml",
          "crates/kernel/Cargo.toml",
          "crates/migration-advisor/Cargo.toml",
          "crates/migration-workbench/Cargo.toml",
          "crates/native-protocol-model/Cargo.toml",
          "crates/native-state-models/Cargo.toml",
          "crates/repository-checks/Cargo.toml",
          "crates/repository-checks/src/compat_modules.rs",
          "crates/repository-checks/src/cost_l.rs",
          "crates/repository-checks/src/lib.rs",
          "crates/repository-checks/src/main.rs",
          "crates/repository-checks/tests/compat_modules.rs",
          "crates/routing-demo/Cargo.toml",
          "crates/routing/Cargo.toml",
          "crates/window-compat/Cargo.toml",
          "crates/window-compat/src/lib.rs",
          "crates/window-compat/tests/protocols.rs",
          "crates/xtask/Cargo.toml",
          "policy/compatibility-modules.json",
          "policy/exceptions.json",
          "rust-toolchain.toml",
          "schemas/compatibility-modules.schema.json"
        ],
        "limitations": [
          "Unmarked adapters and auxiliary source completeness require review; no production module/driver data, loader, live consumers, admission or runtime retirement proof. Current Linux debts remain research-only."
        ],
        "adrs": ["adr.0007", "adr.0013", "adr.0015"],
        "verification": [
          {
            "environment": "host-process",
            "reason": "Phase 3.7 native ELF runtime, finite client binding or shared build/owner inputs changed; current-source applicability is being refreshed. Historical receipts retain their original scope.",
            "state": "STALE",
            "receipt": "research/results/issue47-compatibility-manifests.json",
            "receipt_sha256": "95766b2a15e17e5d3324ff9692e3f4c7ec2725e6971c0af8b249e1cfc113decb",
            "scope": "Host consistency and declaration boundary only; production-shaped artifacts are non-executable test bytes, not supported modules."
          },
          {
            "reason": "Host declaration checking does not verify kernel, driver or physical execution.",
            "state": "NOT_APPLICABLE",
            "environment": "physical-arm64"
          }
        ],
        "acceptance": ["research/results/issue47-compatibility-manifests.json"],
        "next_gate": "Review real production declarations and source/feature completeness; obtain #48 semantic/EN-RU acceptance without promoting synthetic evidence.",
        "issues": [47, 46, 48],
        "implementation_scope": "Closed module inventory, reciprocal debt/consumer/version/scope checks, source/artifact/EXC/support identities and Cargo-parsed native declaration closure, with existing synthetic window contracts only.",
        "transitions": [
          {
            "acceptance": [
              "research/results/issue47-compatibility-manifests.json"
            ],
            "reason": "First bounded manifest gate with synthetic current inventory and exact-source host controls; production/runtime gates remain open.",
            "from": "UNRECORDED",
            "to": "BOUNDED_IMPLEMENTED"
          }
        ],
        "implementation": "BOUNDED_IMPLEMENTED",
        "readiness": "NOT_READY"
      }
    }
  ],
  "summary": "Bounded compatibility module declarations and reciprocal COST-L/Cargo checks without runtime authority.",
  "kind": "subsystem-contract"
}
```
