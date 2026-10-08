# Repository agent instructions

## Architecture and implementation order

Earlier implementation of later-stage functionality is permitted. Implementation order must never become architectural authority. Before building a later milestone on existing early code, re-derive its required architecture from current invariants and accepted decisions. Refactor or remove code that constrains, contradicts or prematurely freezes that architecture. Existing implementation, effort spent, compatibility with tests and avoiding rework are insufficient reasons to preserve a design.

## Documentation and evidence

Follow [documentation policy](docs/documentation-policy.md), [feature truth and impact rules](docs/knowledge-system.md) and [AI retrieval](docs/ai-retrieval.md) in every implementation change. Find the current canonical feature, laws and accepted ADRs before editing. Register new functionality in its canonical Markdown feature unit. Update meaningful scope/status/evidence changes, complete Russian mirrors and affected public summaries in the same change.

For changed implementation/build files, author [the per-change impact declaration](docs/implementation-impact.json) with the reviewed base commit, source digests and explicit dispositions. A no-impact explanation requires human review; whitespace touches do not establish semantic or evidence updates. A source mismatch requires STALE or new exact-source evidence before VERIFIED. Preserve historical receipts and append-only feature transitions. Implementation status, environment verification and readiness are separate.

Run `cargo xtask docs generate`, `cargo xtask docs pilot`, `cargo run --locked -p repository-checks -- record-translation ru PATH` for each reviewed changed canonical Markdown pair, `cargo xtask docs check-change BASE` and `npm run check`. Use the actual reviewed base revision. Do not claim issue completion while required semantic or EN/RU review remains pending.

[Russian translation](translations/ru/AGENTS.md)

## Architecture map changes and level consistency

Changes to architecture, component responsibilities or their interactions must update the [canonical kernel component map](docs/architecture/kernel-component-map.md) and its complete Russian mirror in the same task and pull request as the implementation. Record the concrete mechanisms, direct interactions, boundary member endpoints and reviewed source evidence; do not substitute file adjacency, transitive shortcuts or synthetic links. Check every affected level: root incoming/outgoing relations must continue through the corresponding real members in detail views, including compound branches and external-group navigation. Review the model semantically against the changed code before refreshing source digests; silently refreshing hashes is not a map review.

Run `cargo xtask arena map --check` for every such change. The ordinary repository check also invokes this mandatory level-consistency check. Passing structural checks establishes identifiers, evidence freshness and root/detail correspondence in the declared model, not exhaustive semantic coverage of every production interaction. The checker does not execute the HTML renderer; renderer changes also require rendered-output and interaction review. Complete source and EN/RU semantic review in the same task and PR; do not defer missing map updates to a later issue or describe unreviewed coverage as complete.

## Performance and test integrity

Do not reduce runtime performance, weaken invariants or alter production semantics merely to make tests pass. Tests must exercise the intended architecture. Do not add arbitrary fixed-duration publication/copy-drain deadlines to normal runtime synchronization. Completion and reclamation require actual acquired quiescence, never elapsed time. Test watchdogs provide diagnostics and do not replace required kernel failure handling. Do not hide failures by increasing timeouts, adding retries or inventing performance allowances.

## Architecture: tests do not define implementation

Production behavior must follow independently justified invariants and accepted contracts, never a test's expected output. When a test fails, establish whether there is a proven implementation defect or an incorrect test, and repair that defect. Do not add workarounds, special cases, synthetic results, weakened checks, delays or performance regressions to make a test falsely pass. Production source must not contain test-specific corruption, forced rendezvous or fault-injection entry points. Keep test logic and mutations in separate test artifacts; positive tests exercise the ordinary implementation through its actual interfaces.

UNIT and INTEGRATION import the single production implementation. E2E builds the actual kernel and applications; an external harness executes scenarios through public interfaces. A separate ELF is permitted only as a necessary external ABI/syscall client without a production counterpart. Copies of service/supervisor/kernel implementations, modifications of source copies and source-mutating test builds are forbidden.
