# Phase 0.2 research backlog

Phase 1 не начинается автоматически. Порядок ниже выбран по dependencies; это
исследовательские задачи с проверяемым output, а не обещание написать ядро следующей командой.

| Приоритет | Работа | Deliverable / критерий принятия |
|---|---|---|
| P0 | Углубить cases 07, 08, 30 | Полные introducing/fix/revert hashes, release intervals, isolated fail/pass replay и negative control |
| P0 | Threat model | Trust boundaries: malicious app/adapter/driver/device/firmware; explicit TCB и failure domains |
| P0 | Routing state model | Resolver specification + executable host model: conflicts, no downgrade, drain, generation, shared handles |
| P0 | Shared ownership model | File/lock/wait/credential consistency и counterexamples для незаконного API splitting |
| P0 | Platform contract research | Arm architecture/GIC/timer/MMU/SMP и VirtIO spec clauses; Linux сравнение, реальные invariants |
| P0 | Pin environment | Rust/LLVM/linker/QEMU versions, machine/CPU/accelerator manifest; пока без kernel implementation |
| P1 | Placement experiment | Два законченных host prototypes одной request semantics, IPC и in-process; не fake syscall layer |
| P1 | Native ABI v0 candidate | Wire encoding, handles, errors, cancellation, deadlines; compatibility matrix и decoding fuzz corpus |
| P1 | Memory model | COW, DMA leases, user-copy, pin quotas, retirement; litmus/model failures и proofs границ |
| P1 | Bug-compat lifecycle | Real consumer manifest examples, tombstone/migration policy, native-only dependency checker prototype |
| P1 | Telemetry experiment | Causal attribution, nested CPU, lost events и unavailable states; overhead ON/OFF на host workload |
| P1 | Full provenance | Pinned official docs revisions, hashes и retrieval records; проверить все null first versions |
| P1 | Missing historical candidates | arm64 set_fs removal; futex PI CVE-2014-3153; изучить primary commits до включения cases |
| P2 | Расширение Linux coverage | io_uring lifetime, signal restart semantics, SysV IPC, TCP/netfilter, LSM, filesystem durability, EEVDF/RT |
| P2 | Hardware debt audit | Найти подтверждённый случай workaround после исчезновения supported hardware; не выводить его из возраста кода |
| P2 | Other-system postmortems | По одному конкретному failure/fix или migration case для Starnix, Redox, seL4, Theseus, WSL1, Asterinas |
| P2 | Re-evaluate optional compatibility | Нужны ли sysctl obsolete/time32/mandatory behaviors реальным consumers; не писать unused adapters |

Для каждого эксперимента preregister correctness oracle, sample policy и artifacts.
Нет требования доказать, что native быстрее. Если результат отвергает ADR assumption,
пересмотреть ADR и laws, сохранив историю и причину изменения.

Gate к обсуждению Phase 1: принятый threat model, platform/unsafe contracts, объяснённые
state boundaries, минимальный native ABI candidate и решённый scope первого vertical
slice. Phase 0.1 validation сам по себе этот gate не открывает.
