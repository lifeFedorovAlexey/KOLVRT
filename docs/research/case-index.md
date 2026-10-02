# Pathology cases — Phase 0.1

Research date: 2026-10-02. KOLVRT decisions are design conclusions, not implemented behavior.

| ID                                                       | Case                                                      | Subsystem                    | Decision             | Compatibility | Confidence |
| -------------------------------------------------------- | --------------------------------------------------------- | ---------------------------- | -------------------- | ------------- | ---------- |
| [KOL-PATH-0001](../../research/cases/KOL-PATH-0001.json) | close(): error after descriptor release                   | VFS                          | COMPAT_ONLY          | YES           | MEDIUM     |
| [KOL-PATH-0002](../../research/cases/KOL-PATH-0002.json) | POSIX locks: any close releases process locks             | VFS locking                  | COMPAT_ONLY          | YES           | MEDIUM     |
| [KOL-PATH-0003](../../research/cases/KOL-PATH-0003.json) | epoll: duplication and open-file lifetime                 | Event notification           | COMPAT_ONLY          | YES           | MEDIUM     |
| [KOL-PATH-0004](../../research/cases/KOL-PATH-0004.json) | select: fd_set limits belong to libc                      | System calls / libc          | COMPAT_ONLY          | YES           | MEDIUM     |
| [KOL-PATH-0005](../../research/cases/KOL-PATH-0005.json) | ioctl time32 and the year 2038                            | Time / userspace API         | COMPAT_ONLY          | YES           | MEDIUM     |
| [KOL-PATH-0006](../../research/cases/KOL-PATH-0006.json) | ioctl: padding, pointers and compatibility layouts        | Driver userspace API         | NATIVE_FIX           | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0007](../../research/cases/KOL-PATH-0007.json) | Dirty COW: a fix reverted because of s390                 | Memory management            | NATIVE_FIX           | NO            | MEDIUM     |
| [KOL-PATH-0008](../../research/cases/KOL-PATH-0008.json) | Dirty Pipe: uninitialized buffer flags                    | IPC / pipes / page cache     | NATIVE_FIX           | NO            | MEDIUM     |
| [KOL-PATH-0009](../../research/cases/KOL-PATH-0009.json) | GUP pinning: a reference is not a DMA lease               | Memory / DMA                 | NATIVE_FIX           | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0010](../../research/cases/KOL-PATH-0010.json) | Coherent DMA still requires barriers                      | DMA / memory ordering        | ACCEPTED_TRADEOFF    | NO            | MEDIUM     |
| [KOL-PATH-0011](../../research/cases/KOL-PATH-0011.json) | Cortex-A53 843419: linker workaround                      | ARM64                        | HARDWARE_TRANSLATION | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0012](../../research/cases/KOL-PATH-0012.json) | Device tree bindings as an external ABI                   | Firmware / device tree       | HARDWARE_TRANSLATION | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0013](../../research/cases/KOL-PATH-0013.json) | ACPI _OSI: OS identity instead of capabilities            | Firmware / ACPI              | HARDWARE_TRANSLATION | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0014](../../research/cases/KOL-PATH-0014.json) | USB persistence: descriptors do not prove identity        | USB / suspend                | RESEARCH_REQUIRED    | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0015](../../research/cases/KOL-PATH-0015.json) | blk-mq: scalable queues do not guarantee ordering         | Block I/O                    | ACCEPTED_TRADEOFF    | NO            | MEDIUM     |
| [KOL-PATH-0016](../../research/cases/KOL-PATH-0016.json) | cgroup v1: incompatible controller hierarchies            | Resource management          | NATIVE_FIX           | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0017](../../research/cases/KOL-PATH-0017.json) | User namespaces: dropping groups increased access         | Namespaces / security        | NATIVE_FIX           | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0018](../../research/cases/KOL-PATH-0018.json) | Seccomp notification: mutable-pointer TOCTOU              | Security / syscall mediation | NATIVE_FIX           | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0019](../../research/cases/KOL-PATH-0019.json) | PID reuse: pidfd and atomic creation                      | Process lifecycle            | NATIVE_FIX           | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0020](../../research/cases/KOL-PATH-0020.json) | clone3: sized structure replaces overloaded arguments     | Process / syscall ABI        | NATIVE_FIX           | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0021](../../research/cases/KOL-PATH-0021.json) | Autogroup: nice applies within a task group               | Scheduler                    | ACCEPTED_TRADEOFF    | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0022](../../research/cases/KOL-PATH-0022.json) | RCU: unlinking does not permit freeing                    | Concurrency / reclamation    | ACCEPTED_TRADEOFF    | NO            | MEDIUM     |
| [KOL-PATH-0023](../../research/cases/KOL-PATH-0023.json) | Removal of the numeric _sysctl ABI                        | System calls / configuration | COMPAT_ONLY          | YES           | MEDIUM     |
| [KOL-PATH-0024](../../research/cases/KOL-PATH-0024.json) | SO_REUSEPORT: shared ports require common authority       | Networking                   | NATIVE_FIX           | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0025](../../research/cases/KOL-PATH-0025.json) | Robust futexes: list cleanup instead of scanning all VMAs | Synchronization / IPC        | NATIVE_FIX           | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0026](../../research/cases/KOL-PATH-0026.json) | futex_waitv: waiting on multiple addresses                | Synchronization / IPC        | ACCEPTED_TRADEOFF    | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0027](../../research/cases/KOL-PATH-0027.json) | PCI MSI: broken bridges require scoped fallback           | PCIe / interrupts            | HARDWARE_TRANSLATION | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0028](../../research/cases/KOL-PATH-0028.json) | USB internal API redesign without a permanent driver ABI  | Driver model                 | ACCEPTED_TRADEOFF    | CONDITIONAL   | MEDIUM     |
| [KOL-PATH-0029](../../research/cases/KOL-PATH-0029.json) | Removal of mandatory file locking                         | VFS locking                  | NOT_APPLICABLE       | NO            | MEDIUM     |
| [KOL-PATH-0030](../../research/cases/KOL-PATH-0030.json) | epoll fix regression: a dying file cannot be resurrected  | VFS / eventpoll / fuzzing    | NATIVE_FIX           | NO            | MEDIUM     |

## Decision groups

- **NATIVE_FIX (12)**: KOL-PATH-0006, KOL-PATH-0007, KOL-PATH-0008, KOL-PATH-0009, KOL-PATH-0016, KOL-PATH-0017, KOL-PATH-0018, KOL-PATH-0019, KOL-PATH-0020, KOL-PATH-0024, KOL-PATH-0025, KOL-PATH-0030.
- **COMPAT_ONLY (6)**: KOL-PATH-0001, KOL-PATH-0002, KOL-PATH-0003, KOL-PATH-0004, KOL-PATH-0005, KOL-PATH-0023.
- **HARDWARE_TRANSLATION (4)**: KOL-PATH-0011, KOL-PATH-0012, KOL-PATH-0013, KOL-PATH-0027.
- **ACCEPTED_TRADEOFF (6)**: KOL-PATH-0010, KOL-PATH-0015, KOL-PATH-0021, KOL-PATH-0022, KOL-PATH-0026, KOL-PATH-0028.
- **RESEARCH_REQUIRED (1)**: KOL-PATH-0014.
- **NOT_APPLICABLE (1)**: KOL-PATH-0029.

## Record questions

- **KOL-PATH-0001**: How does native completion report an error after destroying the last handle?; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0002**: Deadlock detection semantics for native lock tokens remain open.
- **KOL-PATH-0003**: Should native nested wait sets be allowed? Not before cycle analysis.
- **KOL-PATH-0004**: Choose a native multiple-wait bound by memory and fairness, not FD_SETSIZE.; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0005**: Is there an actual time32 consumer in Phase 0.2?; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0006**: Which zero-copy types can be proved safe without an implicit Rust layout ABI?; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0007**: Reproduce the causal chain on historical commits in an isolated VM; the fix has not been run.; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0008**: Model all constructor and reuse transitions before choosing a native zero-copy API.
- **KOL-PATH-0009**: Which bounded long-term pinning policy can support future RDMA?; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0010**: Choose barriers from Arm and VirtIO transport specifications in Phase 0.2.; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0011**: The first workaround version and applicability to the selected Rust linker remain unverified.; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0012**: Which bindings enter the initial support manifest beyond QEMU virt?; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0013**: The AML runtime and its isolation remain open choices.; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0014**: Is acceptable identity evidence possible without cryptographic device identification?; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0015**: Does the first VirtIO target need multiple queues or a minimal backend?; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0016**: How should shared file cache and DMA leases be charged across consumers?; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0017**: Define the relationship between POSIX denial semantics and native authority grants.
- **KOL-PATH-0018**: Which Linux continuation modes are possible without falsely promising a sandbox?; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0019**: Define process-handle revocation and lifetime after process exit.
- **KOL-PATH-0020**: Is native fork needed, or only an explicit address-space snapshot primitive?
- **KOL-PATH-0021**: Choose fairness objectives and bounded starvation before the algorithm.
- **KOL-PATH-0022**: Choose RCU, epochs or reference counts for the initial handle table after modeling.; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0023**: Is there a real consumer of this removed ABI? Without one, do not implement a module.
- **KOL-PATH-0024**: Which native join/revocation policy handles credential changes?
- **KOL-PATH-0025**: Choose a native registration protocol preserving the uncontended fast path.; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0026**: Define native bounds and fairness when the first item is always ready.; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0027**: When does the initial VirtIO target need PCIe, and who implements MSI translation?; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0028**: Find specific USB commits from the historical document; driver placement remains open.; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.
- **KOL-PATH-0029**: Does a consumer need mandatory locking, and can a lease express that requirement?
- **KOL-PATH-0030**: Repeat bisection: the dashboard has an unverifiable no-op fix result and the LKML discussion was unavailable.; Establish the introducing commit/tag from Git history; the read source confirms the mechanism but not its exact introduction date.

[Russian translation](../../translations/ru/docs/research/case-index.md)
