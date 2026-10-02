# Linux research coverage

Phase 0.1 contains 30 documentary analyses, not a complete Linux audit. Selection spans subsystems and causes: ABI, lifetime, concurrency, security, hardware necessity and performance tradeoffs. Successful redesigns are included to counter confirmation bias.

| Area                                       | Cases                     | Coverage limit                                                       |
| ------------------------------------------ | ------------------------- | -------------------------------------------------------------------- |
| Stable, obsolete and removed userspace ABI | 01–06, 20, 23, 29         | Introducing history is often unknown.                                |
| System calls and deprecated APIs           | 04–06, 19, 20, 23, 27, 28 | Not a complete catalog.                                              |
| Scheduling                                 | 21                        | Autogroup policy, not a CFS/EEVDF/RT audit.                          |
| Memory management                          | 07–10, 22                 | Copy-on-write, pinning, ordering and reclamation.                    |
| Virtual file system                        | 01–03, 15, 29, 30         | Locks, lifetimes and ordering; no filesystem audit.                  |
| Networking                                 | 24                        | Port sharing; TCP, netfilter and XDP remain backlog.                 |
| Synchronization and IPC                    | 08, 22, 25, 26            | Pipes, robust futexes and multiple waits; SysV IPC remains backlog.  |
| Drivers, DMA and interrupts                | 09–11, 27, 28             | Contracts and specific hardware failure modes.                       |
| ARM64, device tree and ACPI                | 11–13                     | One A53 erratum and firmware ABI; no Arm specification audit yet.    |
| PCI/PCIe, USB and block I/O                | 14, 15, 27                | MSI bridge failures, persistence and queue ordering.                 |
| Security, namespaces and cgroups           | 07, 08, 16–18, 24         | Not a complete threat model or LSM audit.                            |
| Historical vulnerabilities                 | 07, 08                    | Dirty COW and Dirty Pipe: reports and fixes, no live exploit tests.  |
| syzkaller/syzbot and regressions           | 07, 30                    | Dashboard, reproducer and fix; no local bisection.                   |
| Git history                                | 07, 08, 30                | Read commit messages and changes, not a full clone.                  |
| Maintainer discussions                     | 07, 30                    | Commit rationale is available; direct LKML/lore reading was blocked. |

Cases 02 and 29 distinguish process-owned advisory locks from removed mandatory locking. Cases 03 and 30 distinguish public epoll semantics from an internal lifetime regression. Cases 05 and 06 distinguish time-width migration from payload layout. These are independent mechanisms, not renamed duplicates.

## Interpretation

Historical facts require source locators. A cause stated by a commit author differs from inferred intent. Introduction, fix, backport and documentation dates differ; `first_known_version` does not replace the chain. Consumer impact without measurements is potential, not a quantified installed-base claim.

KOLVRT decisions are design conclusions. Hardware necessity is separate from software compatibility. Accepted tradeoffs do not select implementation algorithms. Cases 23 and 29 disprove universal ABI immutability; case 28 disproves universal internal-API immutability.

## Further evidence

For a deep case, pin introducing, fixing and reverting commits, affected releases, architecture and configuration, reproducible input, failing and passing outcomes, backport differences and a negative control. Case 30 shows why a dashboard no-op fix bisection does not establish causality. Experiments must be isolated from the host; this repository currently stores evidence and requirements only.

[Russian translation](../../translations/ru/docs/research/reference-coverage.md)
