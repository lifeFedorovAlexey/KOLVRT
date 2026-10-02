# Project checks and formatting

Use standard tool defaults: rustfmt for Rust, Prettier for Markdown/JSON/YAML, Taplo for TOML, Clippy for Rust diagnostics, and Markdownlint for document structure. Versions are pinned in `rust-toolchain.toml` and `package-lock.json`. Node.js 18 or later runs document tools; project research tools remain Rust. No Python is required.

```text
npm ci --ignore-scripts
npm run format
npm run check
```

`format` processes all supported repository files, including generated Markdown. It excludes downloaded tools, dependencies, build outputs and Cargo's generated lockfile. Original text evidence has no formatter and must not be rewritten. `check` fails on formatting, lint, test, model-evidence or documentation errors. Rust uses the standard formatter without custom layout rules. Prettier keeps paragraph wrapping; Markdownlint does not impose a line-length limit on prose or tables. Repeated headings are allowed in different sections.

VS Code workspace settings enable formatting on save and recommend the matching extensions. TOML uses Taplo in the project commands; save formatting is disabled until a compatible TOML extension is installed and selected. Other editors can use `.editorconfig` and the same commands. Formatting and linting never grant approval to a changed translation. Review English and Russian pairs, then record each changed pair using `repository-checks record-translation ru PATH`. Reformat the manifest afterwards. Research-record formatting can change its byte hash: verify that JSON values are unchanged before updating the bilingual catalog hash.

Before a commit, run `npm run check` and `git diff --check`. Keep commits focused with short English subjects. Do not commit downloaded toolchains, dependency directories, executables or temporary evidence copies. A host model test does not establish kernel correctness. Never suppress a check or regenerate evidence just to conceal a failure.

## Time, hardware values and behavioral settings

Pass time as a type, name hardware values, and separate behavioral settings from mechanics.

This applies to all magic numbers where a type or name explains units, purpose or a contract, not only to time. Use typed values and named constants when they improve understanding without adding runtime work or memory allocation. Keep obvious literals inline, such as zero initialization, a unit increment or a self-explanatory local calculation; do not introduce names merely to replace every number.

- **Time:** express durations with `Duration`, for example `Duration::from_millis(1)` instead of `1_000_000`. Keep conversion to hardware ticks at the hardware boundary.
- **Hardware values:** use named constants for registers and bit masks.
- **Other meaningful values:** name buffer sizes, capacity limits, retry counts and protocol codes when their purpose is not clear from the local expression. Prefer compile-time constants; do not add heap objects or runtime initialization just to name a value.
- **Behavioral settings:** define the scheduler quantum, intervals and timeouts next to the subsystem that chooses them. The mechanism accepts these settings rather than choosing behavioral values itself.

Desired API:

```rust
const TEST_TIMER_DELAY: Duration = Duration::from_millis(1);

cpu::timer(time::deadline_after(TEST_TIMER_DELAY));
```

[Russian translation](translations/ru/CONTRIBUTING.md)
