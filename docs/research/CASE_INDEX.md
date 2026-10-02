# Pathology cases — Phase 0.1

Дата исследования: 2026-10-02. Решения KOLVRT — проектные выводы, не реализованное поведение.

| ID | Случай | Подсистема | Решение | Compat | Уверенность |
|---|---|---|---|---|---|
| [KOL-PATH-0001](../../research/pathology/KOL-PATH-0001.json) | close(): ошибка после освобождения fd | VFS | COMPAT_ONLY | YES | MEDIUM |
| [KOL-PATH-0002](../../research/pathology/KOL-PATH-0002.json) | POSIX locks: любой close снимает блокировки процесса | VFS locking | COMPAT_ONLY | YES | MEDIUM |
| [KOL-PATH-0003](../../research/pathology/KOL-PATH-0003.json) | epoll: dup и lifetime open file description | Event notification | COMPAT_ONLY | YES | MEDIUM |
| [KOL-PATH-0004](../../research/pathology/KOL-PATH-0004.json) | select: предел fd_set находится в libc | Syscalls / libc | COMPAT_ONLY | YES | MEDIUM |
| [KOL-PATH-0005](../../research/pathology/KOL-PATH-0005.json) | ioctl time32: переход времени через 2038 | Time / UAPI | COMPAT_ONLY | YES | MEDIUM |
| [KOL-PATH-0006](../../research/pathology/KOL-PATH-0006.json) | ioctl: padding, pointers и compat layout | Driver UAPI | NATIVE_FIX | CONDITIONAL | MEDIUM |
| [KOL-PATH-0007](../../research/pathology/KOL-PATH-0007.json) | Dirty COW: откат исправления из-за s390 | Memory management | NATIVE_FIX | NO | MEDIUM |
| [KOL-PATH-0008](../../research/pathology/KOL-PATH-0008.json) | Dirty Pipe: неинициализированные flags буфера | IPC / pipes / page cache | NATIVE_FIX | NO | MEDIUM |
| [KOL-PATH-0009](../../research/pathology/KOL-PATH-0009.json) | GUP pinning: ссылка не равна DMA lease | Memory / DMA | NATIVE_FIX | CONDITIONAL | MEDIUM |
| [KOL-PATH-0010](../../research/pathology/KOL-PATH-0010.json) | Coherent DMA всё равно требует barriers | DMA / memory ordering | ACCEPTED_TRADEOFF | NO | MEDIUM |
| [KOL-PATH-0011](../../research/pathology/KOL-PATH-0011.json) | Cortex-A53 843419: linker workaround | ARM64 | HARDWARE_TRANSLATION | CONDITIONAL | MEDIUM |
| [KOL-PATH-0012](../../research/pathology/KOL-PATH-0012.json) | Device Tree bindings — внешний ABI | Firmware / Device Tree | HARDWARE_TRANSLATION | CONDITIONAL | MEDIUM |
| [KOL-PATH-0013](../../research/pathology/KOL-PATH-0013.json) | ACPI _OSI: идентичность ОС вместо capability | Firmware / ACPI | HARDWARE_TRANSLATION | CONDITIONAL | MEDIUM |
| [KOL-PATH-0014](../../research/pathology/KOL-PATH-0014.json) | USB persist: descriptors не доказывают идентичность | USB / suspend | RESEARCH_REQUIRED | CONDITIONAL | MEDIUM |
| [KOL-PATH-0015](../../research/pathology/KOL-PATH-0015.json) | blk-mq: масштабирование очередей не гарантирует ordering | Block I/O | ACCEPTED_TRADEOFF | NO | MEDIUM |
| [KOL-PATH-0016](../../research/pathology/KOL-PATH-0016.json) | cgroup v1: несовместимые иерархии controllers | Resource management | NATIVE_FIX | CONDITIONAL | MEDIUM |
| [KOL-PATH-0017](../../research/pathology/KOL-PATH-0017.json) | User namespaces: dropping groups расширяло доступ | Namespaces / security | NATIVE_FIX | CONDITIONAL | MEDIUM |
| [KOL-PATH-0018](../../research/pathology/KOL-PATH-0018.json) | Seccomp notification: TOCTOU mutable pointers | Security / syscall mediation | NATIVE_FIX | CONDITIONAL | MEDIUM |
| [KOL-PATH-0019](../../research/pathology/KOL-PATH-0019.json) | PID reuse: pidfd и атомарное создание | Process lifecycle | NATIVE_FIX | CONDITIONAL | MEDIUM |
| [KOL-PATH-0020](../../research/pathology/KOL-PATH-0020.json) | clone3: размерная структура вместо перегрузки аргументов | Process / syscall ABI | NATIVE_FIX | CONDITIONAL | MEDIUM |
| [KOL-PATH-0021](../../research/pathology/KOL-PATH-0021.json) | Autogroup: nice действует внутри task group | Scheduler | ACCEPTED_TRADEOFF | CONDITIONAL | MEDIUM |
| [KOL-PATH-0022](../../research/pathology/KOL-PATH-0022.json) | RCU: удаление ссылки не разрешает free | Concurrency / memory reclamation | ACCEPTED_TRADEOFF | NO | MEDIUM |
| [KOL-PATH-0023](../../research/pathology/KOL-PATH-0023.json) | Удаление числового _sysctl ABI | Syscalls / configuration | COMPAT_ONLY | YES | MEDIUM |
| [KOL-PATH-0024](../../research/pathology/KOL-PATH-0024.json) | SO_REUSEPORT: shared port требует общей authority | Networking | NATIVE_FIX | CONDITIONAL | MEDIUM |
| [KOL-PATH-0025](../../research/pathology/KOL-PATH-0025.json) | Robust futex: cleanup по списку вместо всех VMA | Synchronization / IPC | NATIVE_FIX | CONDITIONAL | MEDIUM |
| [KOL-PATH-0026](../../research/pathology/KOL-PATH-0026.json) | futex_waitv: ожидание нескольких адресов | Synchronization / IPC | ACCEPTED_TRADEOFF | CONDITIONAL | MEDIUM |
| [KOL-PATH-0027](../../research/pathology/KOL-PATH-0027.json) | PCI MSI: неисправный bridge требует scoped fallback | PCIe / interrupts | HARDWARE_TRANSLATION | CONDITIONAL | MEDIUM |
| [KOL-PATH-0028](../../research/pathology/KOL-PATH-0028.json) | USB internal API redesign без вечного driver ABI | Driver model | ACCEPTED_TRADEOFF | CONDITIONAL | MEDIUM |
| [KOL-PATH-0029](../../research/pathology/KOL-PATH-0029.json) | Удалённая mandatory file locking | VFS locking | NOT_APPLICABLE | NO | MEDIUM |
| [KOL-PATH-0030](../../research/pathology/KOL-PATH-0030.json) | epoll fix regression: нельзя воскресить dying file | VFS / eventpoll / fuzzing | NATIVE_FIX | NO | MEDIUM |

## Группировка решений

- **NATIVE_FIX (12)**: KOL-PATH-0006, KOL-PATH-0007, KOL-PATH-0008, KOL-PATH-0009, KOL-PATH-0016, KOL-PATH-0017, KOL-PATH-0018, KOL-PATH-0019, KOL-PATH-0020, KOL-PATH-0024, KOL-PATH-0025, KOL-PATH-0030.
- **COMPAT_ONLY (6)**: KOL-PATH-0001, KOL-PATH-0002, KOL-PATH-0003, KOL-PATH-0004, KOL-PATH-0005, KOL-PATH-0023.
- **HARDWARE_TRANSLATION (4)**: KOL-PATH-0011, KOL-PATH-0012, KOL-PATH-0013, KOL-PATH-0027.
- **ACCEPTED_TRADEOFF (6)**: KOL-PATH-0010, KOL-PATH-0015, KOL-PATH-0021, KOL-PATH-0022, KOL-PATH-0026, KOL-PATH-0028.
- **RESEARCH_REQUIRED (1)**: KOL-PATH-0014.
- **NOT_APPLICABLE (1)**: KOL-PATH-0029.

## Вопросы по записям

- **KOL-PATH-0001**: Как native completion сообщает ошибку после уничтожения последнего handle?; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0002**: Семантика deadlock detection native token API ещё не выбрана.
- **KOL-PATH-0003**: Разрешать ли native nested waitsets? До анализа циклов — нет.
- **KOL-PATH-0004**: Выбрать предел native wait-many по памяти и fairness, не по FD_SETSIZE.; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0005**: Нужен ли реальный consumer time32 в Phase 0.2?; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0006**: Сколько zero-copy типов можно доказать безопасными без неявного Rust layout ABI?; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0007**: Воспроизвести причинную цепочку на исторических commits в изолированной VM; fix не запускался.; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0008**: Проверить model test всех constructor/reuse переходов до выбора native zero-copy API.
- **KOL-PATH-0009**: Какая политика bounded long-term pins допустима для future RDMA?; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0010**: Выбрать барьеры по Arm specification и VirtIO transport specification в Phase 0.2.; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0011**: Точная первая версия workaround и применимость к выбранному Rust linker ещё не проверены.; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0012**: Какие bindings входят в первоначальный support manifest кроме QEMU virt?; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0013**: Выбор AML runtime и его isolation остаётся открытым.; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0014**: Есть ли приемлемое подтверждение identity для устройств без криптографической идентификации?; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0015**: Нужна ли сложная multiqueue реализация первому VirtIO target или достаточно минимального backend?; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0016**: Как отнести shared file cache и DMA leases к нескольким consumers?; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0017**: Определить отношения между POSIX deny semantics и native capability grants.
- **KOL-PATH-0018**: Какие Linux continuation modes можно предоставить без ложного обещания sandbox?; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0019**: Определить revocation и post-mortem lifetime process handle.
- **KOL-PATH-0020**: Нужен ли native fork или только explicit address-space snapshot primitive?
- **KOL-PATH-0021**: Выбрать native fairness objectives и bounded starvation до выбора алгоритма.
- **KOL-PATH-0022**: RCU, epochs или refcount для первого native handle table? Решение после моделей.; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0023**: Есть ли востребованный consumer этого removed ABI? Если нет, модуль не реализовывать.
- **KOL-PATH-0024**: Какая native policy join/revoke нужна при смене credentials?
- **KOL-PATH-0025**: Выбрать native registration protocol, сохраняющий uncontended fast path.; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0026**: Определить native bound и fairness при постоянно готовом первом элементе.; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0027**: Когда PCIe нужен первому VirtIO target и кто реализует MSI translation?; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0028**: Найти конкретные USB commits из исторического документа; placement драйверов ещё не выбран.; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
- **KOL-PATH-0029**: Нужен ли consumer с mandatory locking и выражается ли его требование через lease?
- **KOL-PATH-0030**: Повторить bisection: dashboard содержит неверифицируемый no-op fix result; LKML thread не удалось прочитать.; Установить первый introducing commit/tag по git history; прочитанный источник подтверждает механизм, но не его точную дату появления.
