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

[Russian translation](translations/ru/CONTRIBUTING.md)
