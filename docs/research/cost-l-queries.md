# Offline COST-L queries

Document status: CURRENT
Evidence scope: bounded host queries over validated registry declarations; issue #48 remains open for production-manifest integration after #47.
Current reference: [Registry contract](../architecture/compatibility-debt.md)

<a name="kolvrt-cost-l-offline-queries"></a>

## Query contract

Architecture was re-derived from the existing identity, authority, support and evidence rules before extending the registry. Reuse the bounded [validator](../../crates/repository-checks/src/cost_l.rs) and its closed schemas. The [query implementation](../../crates/repository-checks/src/cost_l_queries.rs) is a read-only projection; it neither introduces another registry nor gives a process grants, payload access or retirement authority. No kernel dependency, scheduler policy or Linux-driver implementation is added.

```text
cargo run --locked -p repository-checks -- cost-l show COST-L-0001
cargo run --locked -p repository-checks -- cost-l consumers COST-L-0001 --json
cargo run --locked -p repository-checks -- cost-l deps CONSUMER --json
cargo run --locked -p repository-checks -- cost-l list --status CANDIDATE --json
cargo run --locked -p repository-checks -- cost-l top --sort reach --limit 20
cargo run --locked -p repository-checks -- cost-l top --security reclaim --maintenance context --migration native
cargo run --locked -p repository-checks -- cost-l top --sort cpu --measurement-scope "exact workload" --denominator "exact operation" --json
```

Every query validates the whole selected registry before stdout. `--directory PATH` selects another declaration inventory using the same repository schemas and evidence-reference boundary. `show` returns the selected debt's scope, modules, consumers, cost dimensions, native decision and support/removal statements. `consumers` returns declared direct/transitive relations for a debt; `deps` returns the same relations in reverse for an exact consumer ID. Module ID/version/scope, offline/recovery/archival kind and support deadline remain attached to each relation. TRANSITIVE is an authored declaration: this format records no path, and the CLI computes no package dependency closure. Related debts are not software dependency edges.

The three current candidates have zero **declared** consumers and no module declarations. Global reach and runtime costs remain UNKNOWN; no observed driver count, NATIVE classification or support claim follows. Software compatibility declarations prevent an inference of NATIVE in their stated scope; hardware classification remains a separate field. Production manifests and their bidirectional validation are unavailable pending #47. Runtime inspection is unsupported. The full #48 acceptance therefore remains open.

## Independent dimensions and bounds

`list` defaults to ID ordering; `top` defaults to descending declared reach. Ties use stable IDs and relation content. `--status`/`--category` are exact filters; `--security`, `--maintenance` and `--migration` match the respective authored statements independently, without inventing severity or difficulty scores. Empty filtered results do not assert global absence.

Metric sorting accepts cpu, latency, memory, copies, allocations, context_switches and throughput. It requires an exact observation scope and denominator, rejects inconsistent units, puts matching MEASURED values before UNKNOWN/out-of-scope values and preserves every original metric's provenance/coverage/reason. A measured zero remains zero. Descending throughput is a rate, not a debt-cost ranking. Sorting is descriptive, not proof of comparable machines, equal work, causal attribution or superiority. No combined security/debt score exists.

Pages default to 20 rows and permit 1–64; `--offset` and `pagination.next_offset` expose remaining rows. Final stdout is limited to 1 MiB, including human output; excess requires a smaller page and returns nonzero with no partial stdout. Input limits remain the existing 4096-record/64-KiB-per-record contract. Malformed/reserved/unknown IDs, unknown consumers, unsupported commands/options, duplicate options, invalid pages, incompatible metric dimensions and inconsistent records fail nonzero. No live state, usable grants or request payloads appear in the projection.

`--json` returns machine schema version 1 with command/selector, inventory scope/coverage/gaps, independent ranking/filter selection, pagination and rows. Human output includes text states and UNKNOWN without requiring color. This host CLI output is independent of any future runtime wire ABI.

## Evidence and next gate

[End-to-end tests](../../crates/repository-checks/tests/cost_l_queries.rs) execute the real binary against current research records and explicitly synthetic registries. They cover both directions, shared modules, declared transitive/offline relations, partial coverage, measured zero versus UNKNOWN, exact-scope ranking, deterministic pagination and nonzero/empty-stdout rejection of malformed or inconsistent inputs. [Retained host evidence](../../research/results/issue48-cost-l-queries.json) binds those checks and a current-registry query to exact source digests. No synthetic consumer becomes a supported deployment.

Integrate independently reviewed production manifests after #47, then rerun bidirectional completeness and version/scope/retirement checks before closing #48. Re-derive that architecture from current invariants; existing projections and tests must not freeze the future manifest/consumer model. Broader research/lifecycle acceptance in #45/#46 and runtime gates #49–#52 remain distinct.

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
      "depends_on": ["law.009", "law.036", "law.040", "adr.0007", "adr.0008"],
      "feature": {
        "verification": [
          {
            "receipt_sha256": "beeeca5564e1abf20147531a1bd19b25b34063f70a11c9162c5503359cb13fbb",
            "receipt": "research/results/issue48-cost-l-queries.json",
            "scope": "Windows host only; synthetic fixtures do not establish production consumers or observed runtime costs.",
            "reason": "Five real-binary end-to-end tests and current-registry query passed against these exact declared source digests.",
            "state": "VERIFIED",
            "environment": "host-process"
          },
          {
            "environment": "physical-arm64",
            "reason": "This is read-only offline host tooling; no physical kernel behavior is claimed.",
            "state": "NOT_APPLICABLE"
          }
        ],
        "adrs": ["adr.0007", "adr.0008"],
        "readiness": "NOT_READY",
        "roadmap_gate": "COST-L host tooling #48",
        "limitations": [
          "Direct/transitive relations are declarations without reconstructed package paths; current global coverage is UNKNOWN. #47 production-manifest integration and full #48 acceptance remain open."
        ],
        "acceptance": ["research/results/issue48-cost-l-queries.json"],
        "transitions": [
          {
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "First scoped CLI implementation with exact-source host acceptance; production integration remains gated by #47.",
            "from": "UNRECORDED",
            "acceptance": ["research/results/issue48-cost-l-queries.json"]
          }
        ],
        "issues": [48],
        "implementation_scope": "Host queries over the existing validated registry declarations; no production-manifest or runtime integration.",
        "implementation": "BOUNDED_IMPLEMENTED",
        "next_gate": "Review and integrate #47 production manifests with bidirectional completeness/version/scope/retirement validation before closing #48.",
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
          "research/cost-l/COST-L-0003.json"
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
