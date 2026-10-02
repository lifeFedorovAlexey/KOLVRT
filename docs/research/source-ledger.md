# Source ledger

Research date: **2026-10-02**. [Linux sources](../../research/sources/case-sources.json) record URLs, repositories, confirmed commits/tags/dates, locators and limitations. Cases embed the same metadata plus subsystem labels. [Other sources](../../research/sources/other-sources.json) cover other systems and foundations.

Prefer source code and commits, official documentation, bug reports, reproducers, CVE records and maintainer discussions over secondary analysis. Linux man-pages are primary documentation for the userspace API. The Dirty Pipe discoverer's report is cross-checked against its fix commit.

The original research read web pages. It did not clone all of Linux, run local history/bisection, build historical kernels or reproduce runtime behavior. Full copies of third-party pages are not stored here. Live documentation can change; immutable documentation pins and retrieval hashes remain Phase 0.2 work.

Never guess missing commits or dates. Resolve recorded commit prefixes to full hashes before checkout. ARM64 Kconfig was read at v6.12. Man-pages 6.19 is a documentation version, not a claim about Linux kernel 6.19.

## Access limitations

- Shell access to raw.githubusercontent.com was blocked; the web tool provided the research path.
- The Redox book host returned 403; the official GitHub mirror of `our-goals.md` was read.
- The FreeBSD handbook was initially unavailable; an official architecture article and Q2 2023 report were available. The older i386 article does not describe every current port.
- The LKML/lore discussion for case 30 was unavailable. Dashboard, reproducer and fix evidence remain; intent beyond the commit message is not asserted.
- set_fs removal and futex PI vulnerability candidates remain backlog because primary commits could not be read sufficiently.

Reopen the locator and pinned history to resolve contested claims. Inaccessibility creates an open question, not permission to invent a source. Validation makes no HTTP requests and promises no future link availability.

[Russian translation](../../translations/ru/docs/research/source-ledger.md)
