# Documentation and translations

Canonical Markdown is written entirely in English. Translations live under `translations/<language>/` and mirror the canonical repository paths, including filenames. Russian uses `translations/ru/`. Use descriptive directory names, such as `architecture-decisions`; avoid unexplained abbreviations in new directory names.

Translate the complete meaning: requirements, exceptions, uncertainty, examples and conclusions. Do not combine English prose with Russian prose. Preserve code, commands, identifiers, proper names and external URLs where translation would change their meaning. Russian navigation links stay within the Russian tree; links to shared data and source code resolve to canonical files.

Every canonical Markdown document requires a counterpart for each registered language. Keep the same heading hierarchy, lists, table shape, ordered link targets and law identifiers. Both documents link to each other. New languages are registered in `translations/manifest.json` and supply the complete mirrored tree.

The manifest records SHA-256 hashes of both reviewed texts, with CRLF normalized to LF. A change to either text invalidates the recorded pair. Checks never silently refresh hashes. After reviewing the complete pair, run the explicit recording command described in the [tooling guide](../crates/repository-checks/README.md). Structural and revision checks detect drift; they cannot prove linguistic equivalence. Human review remains required.

The original request is preserved as a historical text attachment. Research JSON retains its original source-analysis prose; it is not a Markdown translation. The bilingual case-index catalog provides reviewed English and Russian titles, subsystem labels and questions, tied to the research-record hash. A record change requires reviewing that catalog before regenerating the indexes.

[Russian translation](../translations/ru/docs/documentation-policy.md)
