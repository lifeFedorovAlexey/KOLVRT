# Source ledger

Исследование: **2026-10-02**. [linux-sources.json](linux-sources.json) содержит URL,
repository, commit/tag/date там, где подтверждены, locator и ограничения. Те же metadata
включены в cases вместе с subsystem для самостоятельного машинного анализа.
[other-sources.json](other-sources.json) содержит источники остальных систем и foundations.

Приоритет: source/commit, official docs, bug report, reproducer, CVE record, maintainer
discussion, secondary analysis. Linux man-pages — первичный проект документации Linux
userspace API, а не сторонний opinion blog. Dirty Pipe report написан обнаружившим ошибку
исследователем и сопоставлен с fix commit.

Прочитаны web pages через browser search/open tool. Полного Linux checkout, локального
git log/bisect, исторических kernel builds и runtime reproductions не было. Полные
копии чужих страниц в repo не сохранялись. Live docs могут изменяться; они не выдаются
за immutable snapshots. Pinning конкретных docs commits и hash snapshots — Phase 0.2 task.

Null commit/date не заменяется догадкой. Commit prefix допускается как увиденная ссылка;
перед local checkout его нужно разрешить в полный hash и записать. v6.12 ARM64 Kconfig
прочитан по tag. Man-pages pages просмотрены в опубликованной версии 6.19; это версия
документации, не Linux kernel 6.19.

## Access limitations

- Shell HTTP запрос к raw.githubusercontent.com заблокирован sandbox network policy;
  web tool работал, поэтому исследование продолжено через него.
- Redox book public host вернул 403; прочитан официальный GitHub mirror `our-goals.md`.
- FreeBSD handbook сначала недоступен напрямую; доступны search excerpt, официальный
  architecture article и project report Q2 2023. Старую i386-specific статью не выдаём
  за полное описание текущего FreeBSD.
- LKML/lore thread case 30 не был доступен для чтения. Сохранены проверенные dashboard,
  reproducer и fix; maintainer intent сверх commit message не утверждается.
- set_fs removal и futex PI CVE candidates не включены в 30 cases: найденные указатели
  на commits не удалось достаточно прочитать. Они находятся в backlog, не в подсчёте.

При необходимости подтверждения спорного claim открыть locator в source, затем pinned
commit/history. Недоступность страницы — повод отметить unresolved, не написать источник
по памяти. Validator не делает HTTP requests и не обещает будущую доступность ссылок.
