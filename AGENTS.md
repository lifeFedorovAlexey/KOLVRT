# Repository agent instructions

## Architecture and implementation order

Earlier implementation of later-stage functionality is permitted. Implementation order must never become architectural authority. Before building a later milestone on existing early code, re-derive its required architecture from current invariants and accepted decisions. Refactor or remove code that constrains, contradicts or prematurely freezes that architecture. Existing implementation, effort spent, compatibility with tests and avoiding rework are insufficient reasons to preserve a design.

## Documentation and evidence

Follow [documentation policy](docs/documentation-policy.md), [feature truth and impact rules](docs/knowledge-system.md) and [AI retrieval](docs/ai-retrieval.md) in every implementation change. Find the current canonical feature, laws and accepted ADRs before editing. Register new functionality in its canonical Markdown feature unit. Update meaningful scope/status/evidence changes, complete Russian mirrors and affected public summaries in the same change.

For changed implementation/build files, author [the per-change impact declaration](docs/implementation-impact.json) with the reviewed base commit, source digests and explicit dispositions. A no-impact explanation requires human review; whitespace touches do not establish semantic or evidence updates. A source mismatch requires STALE or new exact-source evidence before VERIFIED. Preserve historical receipts and append-only feature transitions. Implementation status, environment verification and readiness are separate.

Run `cargo xtask docs generate`, `cargo xtask docs pilot`, `cargo run --locked -p repository-checks -- record-translation ru PATH` for each reviewed changed canonical Markdown pair, `cargo xtask docs check-change BASE` and `npm run check`. Use the actual reviewed base revision. Do not claim issue completion while required semantic or EN/RU review remains pending.

[Russian translation](translations/ru/AGENTS.md)

## Performance and test integrity

Do not reduce runtime performance, weaken invariants or alter production semantics merely to make tests pass. Tests must exercise the intended architecture. Do not add arbitrary fixed-duration publication/copy-drain deadlines to normal runtime synchronization. Completion and reclamation require actual acquired quiescence, never elapsed time. Diagnose test hangs with explicit host-runner watchdogs; do not hide failures by increasing timeouts, adding retries or inventing performance allowances.
