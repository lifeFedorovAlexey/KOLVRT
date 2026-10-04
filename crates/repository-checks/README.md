# Research checks

This Rust command-line tool checks research data and documentation. It does not execute a kernel or establish historical truth. The workspace pins the host compiler in `rust-toolchain.toml` and dependencies in `Cargo.lock`. Python is not required.

Run these commands from the repository root:

```text
cargo test --locked
cargo run --locked -p repository-checks -- check
cargo run --locked -p repository-checks -- validate
cargo run --locked -p repository-checks -- check-cost-l
cargo run --locked -p repository-checks -- check-docs
cargo run --locked -p repository-checks -- report --check
cargo run --locked -p repository-checks -- check-translations
```

`check` performs all checks. `validate` applies JSON Schema Draft 2020-12 with formats, strict duplicate-key parsing and cross-record consistency. `check-docs` verifies local links, source metadata, law obligations and architecture-decision sections. Law count has no minimum or maximum quota; the set must be nonempty with unique IDs and complete obligations. `report --check` rejects stale English or Russian indexes.

After updating a case, review the corresponding bilingual entry and its `source_sha256` in `research/sources/case-index-text.json`. Run `report` without `--check` to regenerate both indexes. Regeneration does not approve translations. Review each changed document pair and record it explicitly:

```text
cargo run --locked -p repository-checks -- record-translation ru docs/research/case-index.md
```

This command only records the selected existing pair; it cannot approve meaning. Then run `check` again. Keep all updated files in the same commit. Do not record hashes merely to suppress a stale-translation failure.

The first build downloads dependencies. Subsequent checks can run with Cargo's `--offline` option once dependencies are cached. No external source retrieval occurs during validation. On Windows the MSVC Rust target also requires C++ build tools. A repository-local installation may set `CARGO_HOME` and `RUSTUP_HOME` under ignored `.toolchains` and invoke its Cargo executable explicitly; no machine-wide PATH change is required. These host tools are separate from the future ARM64 kernel toolchain.

`check` and `validate` also validate the bounded COST-L allocation ledger and records. `check-cost-l` runs only that validation and accepts `--directory PATH` for rejection fixtures. The [registry contract](../../docs/architecture/compatibility-debt.md) describes lifecycle, finite support, unknown observations and retained artifact digests. Passing establishes offline declaration consistency, not Linux driver support, native authority, historical truth or runtime quiescence.

[Russian translation](../../translations/ru/crates/repository-checks/README.md)

## Documentation knowledge commands

The docs subcommand and cargo xtask docs facade share the existing library. See the [knowledge contract](../../docs/knowledge-system.md) and [retrieval contract](../../docs/ai-retrieval.md). Generate catalog/graph/README rows with docs generate; verify with --check. Run docs pilot to record the real deterministic Phase 3 query, and docs pilot --check to reject stale size/source/selection receipts. Normal check includes both gates. Optional docs check-issues queries GitHub and reports UNKNOWN when unavailable; it does not establish behavioral acceptance. CI docs check-change BASE enforces enrolled identity/history and declared implementation impact.
