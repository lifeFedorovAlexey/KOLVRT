# Linux archaeology: scope and coverage

Phase 0.1 — 30 законченных документальных разборов, а не полный аудит Linux. Отбор
stratified по подсистемам и типам причины: ABI, lifetime, concurrency, security,
hardware necessity и performance tradeoff. Случаи с полезными решениями включены
специально, чтобы не отбирать только подтверждения исходной философии.

| Область задания | Case evidence | Граница покрытия |
|---|---|---|
| Stable/obsolete/removed userspace ABI | 01–06, 20, 23, 29 | Introducing history многих старых ABI неизвестна |
| Syscalls, deprecated APIs | 04–06, 19, 20, 23, 27, 28 | Не полный syscall catalogue |
| Scheduler | 21 | Policy autogroup; не аудит CFS/EEVDF/RT |
| Memory management | 07–10, 22 | COW, pinning, ordering, reclamation |
| VFS | 01–03, 15, 29, 30 | Locks/lifetime/ordering; filesystems не аудированы |
| Networking | 24 | Port sharing; TCP congestion, netfilter, XDP — backlog |
| Synchronization/IPC | 08, 22, 25, 26 | Pipes, robust futex, multiwait; SysV IPC — backlog |
| Driver model, DMA, interrupts | 09–11, 27, 28 | Contracts и конкретные hardware failure modes |
| ARM64, DT, ACPI | 11–13 | Конкретный A53 erratum и firmware ABI; ещё не Arm spec audit |
| PCI/PCIe, USB, block | 14, 15, 27 | MSI bridge failure, persistence, queue ordering |
| Security, namespaces, cgroups | 07, 08, 16–18, 24 | Не полный threat model/LSM audit |
| Historical vulnerabilities | 07 Dirty COW; 08 CVE-2022-0847 | Fix/report evidence; не live exploit testing |
| syzkaller/syzbot, regressions | 30 и 07 | Dashboard, reproducer, fix; без локального bisect |
| Git history | 07, 08, 30 | Прочитанные commit messages/diffs; не полный clone |
| LKML/maintainer discussion | source links case 30; commit rationale 07/30 | Прямое чтение lore заблокировано; остаётся debt |

Cases 02 и 29 независимы: process-owned advisory locks vs удалённая mandatory locking.
Cases 03 и 30 независимы: public epoll semantics vs internal regression in lifetime upgrade.
Cases 05 и 06 независимы: time-width migration vs payload padding/layout.
Они не являются переименованием одного бага ради числа 30.

## Как оценивать выводы

Исторический факт должен опираться на source locator. Root cause, который явно описан
commit author, отличается от предположения о мотивах. Временная привязка introduction,
fix, backport и документации различается. `first_known_version` не подменяет всю цепочку.
Change impact без измеренных consumers — потенциальный, не quantified installed-base факт.

Native/compat decision — проектный вывод KOLVRT. Hardware necessity не переносится в
software compat score. Accepted tradeoff не означает выбранный алгоритм implementation.
Удаление Linux ABI в cases 23/29 — контрпример утверждению «Linux ничего не может удалить».
Case 28 — контрпример утверждению «Linux internal ABI всегда заморожен».

## Следующее доказательство

Для глубокого case: фиксируем introducing/fix/revert commits, affected release interval,
архитектуру/config, reproducible input, observed failing outcome, passing fixed outcome,
backport differences и negative control. Case 30 показывает, почему no-op fix-bisect
из dashboard нельзя считать доказательством причинности. Эти эксперименты должны быть
изолированы от хоста; нынешний repo только хранит ссылки и требования.
