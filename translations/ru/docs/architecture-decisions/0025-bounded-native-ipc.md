# ADR-0025 — Concrete bounded IPC and continuous fixed-affinity execution

Status: **Accepted для bounded Phase 3.5 mechanism; final acceptance review pending**. Date: 2026-10-05.

Document status: CURRENT
Evidence scope: принятое архитектурное решение и реализованный mechanism; технические DEV/PROD, mutation и performance receipts сохранены. Итоговое acceptance review открыто; physical ARM не проверен.
Current reference: [IPC contract](../kernel/ipc.md)

## Context

Issue #26 требует реальные EL0 peers, immutable bounded requests, initialized retained responses, scoped authority, blocking receive/wait, cancellation/deadline/death arbitration и actual reclamation. Event-notification pilot Phase 3.4 — evidence, а не architectural authority. [Re-derivation](../architecture/ipc-phase35-review.md) сравнил synchronization A/B/C и execution E1/E2/E3 с действующими laws и ADR до реализации. После review пользователь поручил продолжать до implementation, tests, documentation и PR.

Ordinary locks нельзя вводить внутри exclusive scheduler storage или IRQ callbacks. Ранний возврат из dispatch при отсутствии READY task не обеспечивает persistent blocking transport. Independent admission/retirement меняет lifetime roots, namespaces и owners между CPU и не требуется для этого bounded milestone.

## Decision

Выбрать synchronization A: prepare/copy и retain в scope точного owner-local scheduler; выйти; получить private nonblocking endpoint permit; освободить permit; вновь получить exact executing task для output copy; выйти; commit либо roll back owned reservation под endpoint exclusion. Permit contention deferred отдельно от semantic BUSY. Guard не охватывает scheduler storage, user access, sleep, ERET или другой object guard. IRQ handlers публикуют flags; post-handler/native continuations выполняют bounded drainage.

Выбрать execution E1: добавить `dispatch_ipc()` как continuous fixed-affinity session с сохранением conservative whole-session root/namespace retirement. Без READY task owner остаётся Running и ждёт на permanent native root/stack, обрабатывая bounded deadline/wake/death work. Даже locally terminal CPU остаётся доступным до согласованного peer terminal state и retained source/mailbox drainage. Existing synchronous dispatch и step сохраняют отдельные contracts. Migration и independent admission не добавляются.

Использовать concrete endpoint отдельно от Event/Completion. Trusted bootstrap создаёт exact service binding и nondelegable receiver reference и устанавливает scoped SEND grants. PID, endpoint number и packet caller field не создают authority. SEND/TRANSFER/REVOKE attenuation и shared revoke/admission gate остаются узкими mechanisms, без universal object hierarchy и generic invoke.

Использовать initialized owned payloads, bounded FIFO и отдельно bounded request/result slots. Delivery и collection — explicit copy transactions. Request ID reuse ожидает result consumption либо допустимый cleanup. Один endpoint-local terminal arbiter решает completion, cancellation, expiry и service failure/shutdown. До commitment он может гарантировать отсутствие authorized effect; после commitment возвращает unknown effects вместо rollback. Closing-domain observation участвует до последующей service authorization, даже если deferred death cleanup пропустил busy cell.

Wait state связывает exact process generation, globally unique nonwrapping sequence и object/condition identity. Source records удерживают wake ownership до target acknowledgement. Только target owner меняет BLOCKED на READY. Mailbox pending/ack state атомарен; publish и source retirement используют одно endpoint exclusion. Stale wake не создаёт runnable owner. Само отсутствие READY task никогда не означает quiescence.

## Privileged necessity and policy boundary

EL1 обязан проверять caller/root provenance, handle rights, binding/token generations, private copied storage, domain charges, common arbiter и owner-local execution transitions: untrusted EL0 не может обеспечить их против себя или другой domain. Выбор services, grants, limits, application effects, restarts и orchestration остаётся EL0 policy. Static launchers — trusted bootstrap fixtures, не supervisor и не policy interpreter. Performance не оправдывает перенос policy в EL1.

## Alternatives

Использовать Event/Completion как transport, выдавать authority по global selector, удерживать единый lock между независимыми областями ownership, терять request/result при copy failure, разделить terminal arbitration или завершать CPU независимо.

## Why rejected

Такие варианты смешивают notification с transport, превращают identifiers в authority, нарушают scheduler/user-copy ownership, теряют принятые состояния, допускают противоречивые terminal results либо без принятого требования вводят независимый root/namespace retirement.

## Consequences

Atomic multiword queues требуют более крупного publication/reclamation proof; flag не может скрывать ordinary lock внутри scheduler scope. Endpoint-owner command mailboxes добавляют remote admission и accepted-result ownership protocols. Independent CPU sessions требуют independent root/namespace retirement уже сейчас. Coordinator step pumping сохраняет polling и не устанавливает autonomous blocked deadline/wake behavior. Возможные будущие преимущества не оправдывают введение без соответствующих invariants.

## Compatibility impact

Контракт native.request/1 остаётся экспериментальным и не заморожен. Фактическая кодировка handle и scoped receiver authority должны быть описаны с code-bound receipt до публичной поддержки.

## Performance impact

Сохранённый QEMU baseline охватывает контролируемые same-CPU/cross-CPU hot, blocked и full-queue пути. Это regression baseline; измерения физического оборудования отсутствуют.

## Security impact

PID, endpoint selector и packet fields не предоставляют authority. Перед каждым object transition нужны copied request data и точные generation checks; permit не охватывает user copy, scheduler storage или IRQ work.

## Testing

DEV/PROD QEMU fixtures, focused mutation controls с точным event, host endpoint tests и repository checks дают ограниченные свидетельства. Они не доказывают непроведённые mutations или поведение физического ARM.

## Reversibility

Protocol остаётся экспериментальным; сохранить version rejection и явные lifecycle notices. Будущие supervisor/service задачи должны заново вывести архитектуру из действующих laws и accepted decisions.

Выбранный mechanism вводит private fixed endpoint storage и аудируемые synchronization/idle boundaries. Core state остаётся safe Rust. Fixed affinity, immutable mappings и whole-session lifetime ограничивают scope и resources. Closed endpoints reclaim только при zero request/result и wait ownership и final reference quiescence; session retirement также проверяет pending copy work и mailboxes. Grace period не заменяет эти условия.

## Protocol publication

`native.request/1` имеет EXPERIMENTAL stage и ABI-FREEZE none. Он использует фактический LE64 namespace handle 8/56 и typed receipts/service tokens, заменяя historical host proposal 32/32 для EL0 transport. Version 0 явно unsupported на новом entry; historical host models сохраняют scope согласно lifecycle notice. Source publication и этот architectural ADR не объявляют PUBLIC/STABLE support и не замораживают encoding.

## Verification and next gates

Исходный технический receipt сохраняет 124 проверки каждого DEV/PROD профиля и 46 IPC mutation runs с точным expected event; положительные integration checks свежего main прошли в обоих профилях. Сохранены controlled performance paths, unsafe inventory и source-bound receipts. Итоговое acceptance review остаётся открытым. QEMU — regression platform, не silicon evidence.

Следующий scope — supervisor #27, затем isolated persistent service #28. Их execution, admission и retirement выводятся заново из accepted invariants. Existing E1 code, effort и tests не определяют будущую architecture. Mutable mappings, zero-copy, wait-any, device/network/storage policy и independent service supervision здесь исключены.

[English source](../../../../docs/architecture-decisions/0025-bounded-native-ipc.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0025",
  "kind": "adr",
  "summary": "Ограниченный IPC через native endpoint и непрерывное исполнение с фиксированной привязкой.",
  "aliases": ["ADR-0025"]
}
```
