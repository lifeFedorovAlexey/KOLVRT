# ADR-0019 — Линейные namespace дескрипторов процессов

Status: **Accepted for bounded Phase 3.3**. Date: 2026-10-03.

Document status: HISTORICAL
Evidence scope: process-local identity/type/lifetime; transfer и retained-target guarantees описаны в [ADR-0020](0020-handle-transfer-and-retention.md).
Current reference: [Контракт handles](../kernel/handles.md)

## Context

Phase 3.2 даёт bounded immutable request snapshots и retained current-process memory. Issue #23 смешивает handles с будущими capability/transfer guarantees; явная Phase 3.3 реализует identity/lifetime отдельно от authority. Existing process ownership и quiescent scheduler permits дают exclusion без global lock.

## Decision

Один non-cloned eight-entry namespace на process slot сохраняет 56-bit slot generations при ProcessId reuse. Линейное значение перемещается в indexed CPU state и возвращается после acquired quiescence. Namespace владеет concrete wait Event/Completion targets и возвращает synchronous Retained borrows. Caller, encoding, generation, live state и kind проверяются. Failed publication использует reservation identity; exhausted slots quarantined. Copied versioned words декодируются явно; EL0 получает только lookup/close.

## Alternatives

Рассмотрены global numeric IDs, raw kernel pointers, reset-on-process-reuse counters, generic KernelObject/Arc storage, global locks, rights placeholders и asynchronous leases.

## Why rejected

Pointers/global selectors раскрывают или обходят namespace attribution. Reset counters alias stale values. Existing exclusive fixed-affinity ownership и direct concrete storage исключают reclamation races. Universal storage, locks, asynchronous leases и undefined rights добавляют unsupported contracts. Real delegation/accepted asynchronous work требуют отдельного retention review.

## Consequences

Каждый принадлежащий namespace ресурс имеет неизменяемый внутренний TargetId (создавший ProcessId, slot и generation резервирования), отдельный от process-local wire Handle. Перемещение namespace сохраняет TargetId; повторное использование slot создаёт другой ресурс. TargetId никогда не сериализуется в EL0 и не предоставляет authority.

Lookup удерживает synchronous borrow; close/retire не инвалидируют используемую ссылку. Bounded targets освобождаются с namespace owner. Process slot reuse сохраняет generations. Copy publication errors не оставляют live entries. Shared process execution, transfer и asynchronous retention не поддерживаются.

## Compatibility impact

Provisional LE64 identity и explicit 32-byte request не зависят от fd/HANDLE numbers, legacy errno или internal struct layout. Compatibility может отображать identity поверх будущих independently authorized native operations.

## Performance impact

Bounded checked lookup и inline targets не выделяют память на lookup. Пять DEV/PROD QEMU scopes на CPU измеряют lookup/create/close/reuse. Unchecked index и profile-specific correctness bypass не допускаются.

## Security impact

Protected resource — kernel-owned wait/completion storage. EL0 не может безопасно менять protected scheduler namespace state или enforce kernel frame/borrow exclusion через существующий handle primitive, которого ещё нет. Минимальное добавление EL1 resolve/retains concrete native reference и закрывает её; creation policy, resource effects, authority и delegation отдельно. Service-only reference table не обеспечит lifetime kernel-owned target. Квота — восемь targets, storage ownership линейный. Новый production unsafe не требуется; linked EL0 fixture slices используют existing INV-USER-IMAGE. Global object policy и implicit authority не добавлены.

## Testing

Требуются real two-CPU copied-request execution, foreign/stale/wrong-kind rejection, close/rollback/exhaustion/reuse, exit/fault cleanup, compile-time retained-borrow exclusion, пять actual enforcement controls, existing lifecycle/user-copy/routing regressions, physical native-only removal, host/Clippy/repository checks, exact-source receipts и green CI. Результаты локальной проверки связаны с контрактом handles; окончательная приёмка требует green CI опубликованного кандидата.

## Reversibility

Encoding/capacity изменяются с explicit provisional ABI review. Ослабление affinity, synchronous borrows или private mappings требует нового proof. Capability admission использует unchanged identity/type lookup, но независимо задаёт rights, revocation и accepted-work retention. Handles не являются authority.

[Английский оригинал](../../../../docs/architecture-decisions/0019-process-local-handles.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0019",
  "kind": "adr",
  "aliases": ["ADR-0019"],
  "summary": "Навигация по документу: ADR-0019 — Линейные namespace дескрипторов процессов. Доказательства имеют указанные границы.",
  "tags": ["architecture", "decision"]
}
```
