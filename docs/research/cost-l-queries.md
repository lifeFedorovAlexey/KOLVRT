# Offline COST-L queries

Document status: CURRENT
Evidence scope: bounded host queries over reciprocal registry/module declarations; production data, completeness and human acceptance remain open.
Current reference: [Registry contract](../architecture/compatibility-debt.md)

<a name="kolvrt-cost-l-offline-queries"></a>

## Query contract

Architecture was re-derived from the existing identity, authority, support and evidence rules before extending the registry. Reuse the bounded [validator](../../crates/repository-checks/src/cost_l.rs) and its closed schemas. The [query implementation](../../crates/repository-checks/src/cost_l_queries.rs) is a read-only projection; it neither introduces another registry nor gives a process grants, payload access or retirement authority. No kernel dependency, scheduler policy or Linux-driver implementation is added.

```text
cargo run --locked -p repository-checks -- cost-l show COST-L-0001
cargo run --locked -p repository-checks -- cost-l consumers COST-L-0001 --json
cargo run --locked -p repository-checks -- cost-l deps CONSUMER --json
cargo run --locked -p repository-checks -- cost-l list --directory PATH --manifests MANIFEST_PATH --json
cargo run --locked -p repository-checks -- cost-l list --status CANDIDATE --json
cargo run --locked -p repository-checks -- cost-l top --sort reach --limit 20
cargo run --locked -p repository-checks -- cost-l top --security reclaim --maintenance context --migration native
cargo run --locked -p repository-checks -- cost-l top --sort cpu --measurement-scope "exact workload" --denominator "exact operation" --json
```

Every query validates the whole selected registry before stdout. `--directory PATH` selects another declaration inventory using the same repository schemas and evidence-reference boundary. `show` returns the selected debt's scope, modules, consumers, cost dimensions, native decision and support/removal statements. `consumers` returns declared direct/transitive relations for a debt; `deps` returns the same relations in reverse for an exact consumer ID. Module ID/version/scope, offline/recovery/archival kind and support deadline remain attached to each relation. TRANSITIVE is an authored declaration: this format records no path, and the CLI computes no package dependency closure. Related debts are not software dependency edges.

The three current candidates have zero **declared** consumers and no module declarations. Global reach and runtime costs remain UNKNOWN; no observed driver count, NATIVE classification or support claim follows. Software compatibility declarations prevent an inference of NATIVE in their stated scope; hardware classification remains a separate field. The default repository inventory now joins the [module declarations](../architecture/compatibility-manifests.md) through their existing validator before any stdout. Every debt/module/consumer/version/scope/artifact/support relation must agree in both directions, and Cargo/source/native-boundary checks still apply. Each selected row carries sorted `module_declarations`; consumer rows include only their exact module references. SYNTHETIC contracts without debt references are counted in the inventory, never assigned to a research candidate. Runtime inspection is unsupported. Production data, source completeness and human acceptance under #47/#48 remain open.

An explicit `--directory PATH` keeps a custom registry independent; `--manifests PATH` explicitly joins a bounded manifest to that selected registry. The same reciprocal validator runs, with Cargo metadata and source/artifact paths resolved against the repository root. Invalid manifests fail even when the broken relation is outside the selected page. Missing manifests never silently fall back. Architecture was re-derived from LAW-008/009/013/018/035/036/040 and ADR-0007/0013/0015: these are offline declarations, never execution, permission, current support or safe reclamation evidence.

## Independent dimensions and bounds

`list` defaults to ID ordering; `top` defaults to descending declared reach. Ties use stable IDs and relation content. `--status`/`--category` are exact filters; `--security`, `--maintenance` and `--migration` match the respective authored statements independently, without inventing severity or difficulty scores. Empty filtered results do not assert global absence.

Metric sorting accepts cpu, latency, memory, copies, allocations, context_switches and throughput. It requires an exact observation scope and denominator, rejects inconsistent units, puts matching MEASURED values before UNKNOWN/out-of-scope values and preserves every original metric's provenance/coverage/reason. A measured zero remains zero. Descending throughput is a rate, not a debt-cost ranking. Sorting is descriptive, not proof of comparable machines, equal work, causal attribution or superiority. No combined security/debt score exists.

Pages default to 20 rows and permit 1–64; `--offset` and `pagination.next_offset` expose remaining rows. Final stdout is limited to 1 MiB, including human output; excess requires a smaller page and returns nonzero with no partial stdout. Input limits remain the existing 4096-record/64-KiB-per-record contract. Malformed/reserved/unknown IDs, unknown consumers, unsupported commands/options, duplicate options, invalid pages, incompatible metric dimensions and inconsistent records fail nonzero. No live state, usable grants or request payloads appear in the projection.

`--json` returns machine schema version 2 with command/selector, inventory scope/coverage/gaps, independent ranking/filter selection, pagination and rows. Human output includes text states and UNKNOWN without requiring color. This host CLI output is independent of any future runtime wire ABI.

## Evidence and next gate

[End-to-end tests](../../crates/repository-checks/tests/cost_l_queries.rs) execute the real binary against current research records and explicitly synthetic registries. They cover both directions, shared modules, declared transitive/offline relations, partial coverage, measured zero versus UNKNOWN, exact-scope ranking, deterministic pagination and nonzero/empty-stdout rejection of malformed or inconsistent inputs. [Retained host evidence](../../research/results/issue48-cost-l-queries.json) binds those checks and a current-registry query to exact source digests. No synthetic consumer becomes a supported deployment.

The [manifest integration receipt](../../research/results/issue48-manifest-queries.json) retains seven real query tests and thirteen manifest tests with exact sources. Joined CLI fixtures exercise many-to-many relations, versioned modules, transitive offline consumers, all five commands, deterministic ordering, pagination, human/JSON output and version/scope/artifact/deadline/retirement/missing-relation/duplicate-key rejection before stdout. Their retained artifacts are deliberately non-executable bytes, not supported deployments. Historical receipts remain unchanged.

Review real production data and source/feature completeness under #47, then obtain semantic and complete EN/RU human review before closing #48. Declared TRANSITIVE paths are still not reconstructed package closure; global runtime coverage and costs remain UNKNOWN. Broader research/lifecycle acceptance in #45/#46 and runtime gates #49–#52 remain distinct.

[Russian translation](../../translations/ru/docs/research/cost-l-queries.md)

<!-- knowledge -->

```json
{
  "summary": "Bounded offline projections of COST-L declarations, independent dimensions and explicit production/runtime gaps.",
  "schema_version": 1,
  "units": [
    {
      "id": "kolvrt.cost-l.offline-queries",
      "anchor": "kolvrt-cost-l-offline-queries",
      "depends_on": [
        "law.009",
        "law.036",
        "law.040",
        "adr.0007",
        "adr.0008",
        "kolvrt.compatibility.manifests"
      ],
      "feature": {
        "verification": [
          {
            "environment": "host-process",
            "state": "STALE",
            "receipt": "research/results/issue48-manifest-queries.json",
            "receipt_sha256": "c4b07aeddc7c6ae61f463ffbeadfb7b2c65d2ee73871dee39fa2eb1d7294514d",
            "scope": "Bounded host declaration/query consistency with synthetic non-executable production-shaped artifacts only; no observed runtime consumers or costs.",
            "reason": "Phase 3.7 removes source-copy mutation builds and application implementation copies. Tests use the actual production code; replacement fault/restart/shutdown and related acceptance scenarios remain incomplete. Prior receipts retain their historical scope; partial passes are not full current-source acceptance."
          },
          {
            "environment": "physical-arm64",
            "reason": "This is read-only offline host tooling; no physical kernel behavior is claimed.",
            "state": "NOT_APPLICABLE"
          }
        ],
        "adrs": ["adr.0007", "adr.0008", "adr.0013", "adr.0015"],
        "readiness": "NOT_READY",
        "roadmap_gate": "COST-L host tooling #48",
        "limitations": [
          "TRANSITIVE relations remain declared without reconstructed package paths; global runtime coverage is UNKNOWN. Production data/source completeness under #47 and semantic/EN-RU human acceptance for #48 remain open."
        ],
        "acceptance": [
          "research/results/issue48-cost-l-queries.json",
          "research/results/issue48-cost-l-arena-dispatch.json",
          "research/results/issue48-cost-l-arena-rebased.json",
          "research/results/issue48-manifest-queries.json"
        ],
        "transitions": [
          {
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "First scoped CLI implementation with exact-source host acceptance; production integration remains gated by #47.",
            "from": "UNRECORDED",
            "acceptance": ["research/results/issue48-cost-l-queries.json"]
          }
        ],
        "issues": [48],
        "implementation_scope": "Bounded host queries over registry and reciprocal module declarations, default repository join and explicit custom-inventory join; no runtime inspection or production deployment.",
        "implementation": "BOUNDED_IMPLEMENTED",
        "next_gate": "Review actual production module data and source/feature completeness under #47; obtain semantic and complete EN/RU human acceptance before closing #48.",
        "sources": [
          "crates/repository-checks/src/cost_l_queries.rs",
          "crates/repository-checks/src/cost_l.rs",
          "crates/repository-checks/src/main.rs",
          "crates/repository-checks/src/lib.rs",
          "crates/repository-checks/tests/cost_l_queries.rs",
          "crates/repository-checks/Cargo.toml",
          "Cargo.lock",
          "rust-toolchain.toml",
          "schemas/cost-l.schema.json",
          "schemas/cost-l-registry.schema.json",
          "research/cost-l/registry.json",
          "research/cost-l/COST-L-0001.json",
          "research/cost-l/COST-L-0002.json",
          "research/cost-l/COST-L-0003.json",
          "Cargo.toml",
          "crates/host-process-metrics/Cargo.toml",
          "crates/kernel-core/Cargo.toml",
          "crates/kernel/Cargo.toml",
          "crates/migration-advisor/Cargo.toml",
          "crates/migration-workbench/Cargo.toml",
          "crates/native-protocol-model/Cargo.toml",
          "crates/native-state-models/Cargo.toml",
          "crates/repository-checks/src/compat_modules.rs",
          "crates/repository-checks/tests/compat_modules.rs",
          "crates/routing-demo/Cargo.toml",
          "crates/routing/Cargo.toml",
          "crates/window-compat/Cargo.toml",
          "crates/window-compat/src/lib.rs",
          "crates/window-compat/tests/protocols.rs",
          "crates/xtask/Cargo.toml",
          "policy/compatibility-modules.json",
          "policy/exceptions.json",
          "schemas/compatibility-modules.schema.json"
        ]
      },
      "summary": "Read-only host COST-L show/consumers/deps/list/top with UNKNOWN, exact-scope sorting and pagination.",
      "kind": "feature"
    }
  ],
  "id": "doc.kolvrt.research.cost-l-queries",
  "kind": "subsystem-contract"
}
```
