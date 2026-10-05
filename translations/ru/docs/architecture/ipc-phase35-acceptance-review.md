# Приёмочное review Phase 3.5

Document status: CURRENT
Evidence scope: bounded IPC на main 9b4687a48a9d702261898ee4f20434c6bf7046ec; смысловое review выполнено Codex, execution evidence получено на двух QEMU CPU. Отдельное review человеком и проверка physical ARM не заявляются.
Current reference: [Контракт IPC](../kernel/ipc.md)

## Решение и область

Запрошенный bounded mechanism Phase 3.5 имеет BOUNDED_IMPLEMENTED и READY для объявленного IPC-контракта и следующего reader #27. READY относится к этому milestone, а не к общей готовности production или физической платформы. Publication stage native.request/1 остаётся EXPERIMENTAL, ABI freeze отсутствует. Maintainer прямо поручил завершить смысловое review, свежие enforcement controls и promotion.

Эта приёмочная правка не меняет runtime source. Все записанные hashes совпадают с main до и после новых controls. Существующий source-matched positive receipt подтверждает 124 DEV и 124 PROD checks. Новый receipt содержит все 23 enforcement mutations каждого профиля: 46 независимо проверенных nonzero failures с точным требуемым test либо rejection event. Исторические receipts сохраняют исходные snapshots.

## Сверка требований и доказательств

| Область acceptance                | Enforcing code и исполненные доказательства                                                                                                                                                                                                                                                                                                                                                                                                                                 | Вывод review                                                                                                                                                                                                                                                                              |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Архитектура и authority           | ADR-0025 выбирает E1/A; native prepare определяет executing task/namespace до decode скопированного input. Receiver binding непередаваемый и связан с точным service; SEND rights и generations проверяются до admission. Реальные authority/revoke fixtures и соответствующие mutations проверяют enforcement.                                                                                                                                                             | Event notification не становится payload queue; ambient principal, generic invoke и policy interpreter отсутствуют.                                                                                                                                                                       |
| Payload и response                | Input проверяет version/operation/length/unused fields; Payload владеет initialized bytes; Output начинается с нулевого массива и выдаёт только initialized prefix. Реальный payload stress проверяет 0/1/8/64/255/256 bytes и mutation после submit/reply.                                                                                                                                                                                                                 | Данные bounded и скопированы, последующая user mutation их не меняет; Rust padding не экспортируется.                                                                                                                                                                                     |
| Exhaustion и publication failures | Submit проверяет fixed queue/request cells и получает caller/service charges до gate publication. Request admission не выделяет fallible heap buffer; недостаток storage или quota возвращает native Exhausted. Реальные queue-full и zero-budget fixtures проверяют rollback и reclaim; отдельные host tests покрывают endpoint/reference/receiver quotas. Handle-bearing frames отклоняются как unsupported.                                                              | ENOMEM здесь означает exhaustion ограниченного storage/quota. Host pool-limit tests не выдаются за дополнительные EL0 прогоны; произвольный heap-backed IPC не заявляется.                                                                                                                |
| Copy transactions                 | Receive/collect reservations удерживают identity и commit выполняется после успешного current-task copy. Реальные copy failures и две mutations выявляют преждевременное consumption.                                                                                                                                                                                                                                                                                       | Failed output copy сохраняет request/result и reservation client ID.                                                                                                                                                                                                                      |
| Waiters и wakeup                  | Один точный receiver, один active blocking reason процесса; reserve_wait отклоняет вторую live registration этого процесса. Terminal waits имеют bounded registry. Реальный multiple-producer fixture выполняет submit/wait/collect двух отдельно авторизованных clients для 32 requests в четырёх placements; host test отдельно отклоняет duplicate readable registration. Wait-recheck/publication/generation/duplicate-READY/wrong-process controls выявляют нарушения. | Поддерживаются несколько авторизованных terminal waiters. Два разных readable receiver principals не являются admitted configuration. Fixed-affinity ownership исключает выполнение одного процесса на двух CPU одновременно. Multi-thread/multi-process-domain guarantee не добавляется. |
| Cancel/deadline/death             | Одна terminal function решает completion/cancel/deadline/service death. Прерывание committed work даёт EffectUnknown, до commit — отсутствие authorized effect. Closing requester проверяется до нового commitment даже при отложенном death drainage. Реальные before/after-commit, requester/service/both-peers death и shutdown-load fixtures покрывают оба направления CPU.                                                                                             | Второй terminal attempt не меняет winner и не освобождает active charge повторно. Durability и rollback произвольных внешних effects исключены.                                                                                                                                           |
| SMP lifetime и cleanup            | Split continuation освобождает scheduler scope до endpoint exclusion и вновь получает exact task scope для output. IRQ публикует flags; deferred scans держат wake/ack records и отправляют SGI после release endpoint permit. Reap требует closed и нулевое request/wait/reference ownership; session retirement дренирует copies/mailboxes и восстанавливает native roots. Teardown/blocked-reclaim/double-charge/storage-scope controls проверяют нарушения.             | Пустой READY set и elapsed deadline не заменяют reclamation acknowledgement; ownership удерживается до реальной quiescence.                                                                                                                                                               |
| Измерения                         | Source-matched final baseline содержит 144 группы профиля: четыре placements, четыре размера, девять scopes, четыре warmups и шестнадцать samples. Controlled paths проверяют реальные condition-block counters.                                                                                                                                                                                                                                                            | Четыре payload copies; clock probes и scheduling включены без искусственного вычитания overhead. QEMU не доказывает hardware speed.                                                                                                                                                       |

## Смысловое соответствие EN/RU

Контракт IPC, ADR-0025, architecture re-derivation, performance passport и README обоих языков сверены с code и receipts. Сохраняются одинаковые principals/roles/generations, 40-byte input/receive и 24-byte result layouts, operation values 1–9, payload bounds, моменты release charges, commit outcomes, copy rollback, waiter limits, ordering и excluded scope. Feature metadata совпадает, кроме localized summaries. Исправлены устаревший RU paragraph о baseline и README future-IPC/cancellation. Pending acceptance заменено явной записью этого review и scoped readiness; отдельная человеческая подпись не выдумана.

## Доказательства и следующий gate

- [Свежие controls](../../../../research/results/ipc-phase35-controls-current.json): 46 mutations, точные failures, build/run hashes и неизменный source inventory.
- [Текущие positive checks](../../../../research/results/ipc-phase35-main-integration.json): 124 checks каждого DEV/PROD профиля с совпадающими source hashes.
- [Scoped readiness](../../../../research/results/ipc-phase35-readiness.json): отдельная запись review actor, scope и acceptance mapping.
- [Final baseline](../../../../research/measurements/ipc-phase35-baseline.json) и [паспорт](../kernel/ipc-performance.md).

Physical ARM остаётся UNKNOWN. Supervision/restart/discovery относится к #27, persistent service — к #28. Migration, mutable/shared mappings, zero-copy, generic wait-any/invoke и stable ABI требуют отдельных решений. Будущая architecture выводится из актуальных invariants; готовый Phase 3.5 code не становится архитектурной властью.

[English source](../../../../docs/architecture/ipc-phase35-acceptance-review.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.ipc.acceptance-review",
  "kind": "security-analysis",
  "summary": "Явное source-grounded acceptance и смысловое EN/RU review Phase 3.5 в области bounded IPC mechanism.",
  "relationships": [
    {
      "type": "related_to",
      "to": "kolvrt.ipc.transport",
      "scope": "Evidence and scoped readiness review; no physical ARM or stable ABI acceptance."
    }
  ]
}
```
