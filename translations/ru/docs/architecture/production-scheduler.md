# Production scheduling и lifecycle запросов: следующий порядок реализации

Document status: DESIGN BASELINE
Document scope: последовательность реализации по указанию пользователя; production scheduler, blocking и IPC этим документом не реализованы.
Status reference: [Текущий scheduler foundation](../kernel/scheduler.md)

## Scope и существующий механизм

Пользователь выбрал эту последовательность 2026-10-03 для [native foundation #37](https://github.com/lifeFedorovAlexey/KOLVRT/issues/37). Она уточняет порядок реализации и не ослабляет принятые authority, isolation, ownership и retirement contracts. Текущий timer-driven scheduler уже вытесняет EL0 по типизированному quantum. Оставшаяся архитектурная проблема — synchronous round completion: координатор ждёт завершения всех admitted programs. Бесконечная жизнь процесса и монополизация CPU — разные условия. Persistent services могут жить бесконечно, получая ограниченные execution slices.

После появления следующего production-механизма сохранить текущий synchronous dispatcher как bootstrap/test execution path. Не чинить его скрытым dispatch timeout, завершением каждого persistent service по фиксированному lifetime или трактовкой timeout как quiescence. Свидетельства существующего bootstrap path сохраняют отдельный scope.

## Упорядоченные условия реализации

| Порядок | Механизм                        | Граница acceptance и координация                                                                                                                                                                                                                      |
| ------- | ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1       | Preemption и deadlines          | Timer preemption продолжается без cooperative yield; CPU fairness/progress и истечение запрошенного budget/deadline не зависят от выхода всех peers. Сохранить architectural context и fixed owner checks. Расширяет scheduler/process work #17/#20.  |
| 2       | Wait/block primitive            | READY → RUNNING → BLOCKED; atomic wait registration с condition recheck, generation-safe wakeup и отсутствие lost wakeup. Нет ready tasks — idle/wait события, а не runtime shutdown. Координация #20/#26.                                            |
| 3       | IPC transport                   | Bounded immutable messages, admission/backpressure, publication и event-driven wakeup. Сначала только явно разрешённые bootstrap peers; user-copy #22, identity/domain checks и retained ownership обязательны. Координация #26.                      |
| 4       | Handles и capabilities          | Caller-local generation-safe handles и native authorization каждой операции перед публичными send/receive/create. Process IDs, endpoint numbers и protocol opcodes не дают прав. Координация #23/#24/#25.                                             |
| 5       | Resource grants и revocation    | Временный ограниченный доступ, retained resource ownership, attenuation и admission/revocation linearization. Close/revoke не освобождает ресурсы, удерживаемые accepted work. Координация #24/#25 и scoped device contracts.                         |
| 6       | Полный протокол исходов запроса | Один terminal arbiter для completed / cancelled / unknown effect, deadline, caller/service death и shutdown. Cancellation до commitment гарантирует отсутствие эффекта; uncertainty после commitment запрещает blind replay. Координация #26/#27/#30. |

Это последовательность реализации, а не разрешение открыть unauthorized IPC API на шаге 3 или добавить outcome correctness только на шаге 6. Начальный transport уже должен идентифицировать requests, удерживать accepted work и отклонять duplicate terminal publication. До выполнения определить его ограниченные effects/cancellation limits. Шаг 6 завершает outcome protocol для races, revocation, deadlines, failures и реальных service effects; он обязателен до admission persistent production service по #28.

## Production execution model

Разделить process lifetime/identity, scheduler execution state и request outcomes. READY означает возможность выполнения; RUNNING имеет одного indexed CPU owner; BLOCKED зарегистрирован на конкретном retained event/condition. Terminal process states остаются отдельными. Request cancellation не является process exit, а истёкший scheduling budget не доказывает отсутствие эффекта запроса.

Timer trap сохраняет полный context, выполняет bounded accounting и публикует/планирует preemption. IRQ paths ничего не выделяют, не берут blocking locks и не решают service policy. Event publication будит допустимое точное generation; duplicate/stale wakeups не создают двух runnable owners. State ownership, CPU migration и cross-CPU wakeup требуют явного протокола до ослабления fixed affinity. Не вводить universal object hierarchy или arbitrary-command capability.

Production scheduler возвращается в event loop, даже когда peer остаётся жив или blocked. Он не должен заимствовать всех process owners до завершения каждого peer. Reclamation требует retained lookup/admission references, точной terminal identity, scheduler detachment и acquired CPU/TLB quiescence конкретного объекта. Deadline останавливает/отменяет admission/work по своему контракту; memory освобождается только после фактического retirement. Сохранить консервативный существующий barrier до получения собственных свидетельств replacement protocol.

## Требуемые свидетельства до каждого admission

- Preemption: noncooperative infinite EL0 loop рядом с productive peer на каждом CPU, сохранение полного context, bounded peer progress и работа scheduler без завершения infinite process только ради возврата из dispatch.
- Deadline: expiry в READY/RUNNING/BLOCKED и на native-call/IRQ boundaries, ровно одно наблюдаемое expiry, отсутствие unauthorized effect и premature reclamation. Явно указать wall-time либо CPU-budget units.
- Blocking: событие до registration, во время registration/recheck и после blocking; duplicate/stale wakeup; пустая ready queue; close/death/shutdown races. Без busy-loop worker и lost event.
- IPC/rights: full queue, malformed frame, failed copy, unauthorized/stale handle, foreign buffer, revoke-versus-admission, retained references после close и denial до effect.
- Outcomes: cancel до commitment, cancellation после uncertain effect, delayed completion после restart, timeout против completion, service death и shutdown under load. Ровно один правдивый terminal result; unknown effect сохраняется, а не переименовывается в rollback.

Это будущие требования executable tests, а не сохранённые успешные результаты. Использовать реальное two-CPU QEMU DEV/PROD execution, meaningful negative controls, exact-source receipts и отдельный physical-hardware scope при необходимости. Host protocol tests дополняют execution evidence. ASID optimization #18, ELF #21 и benchmark/reliability tooling — отдельные работы, которые не оправдывают обход safety gates выше.

[Английский оригинал](../../../../docs/architecture/production-scheduler.md)
