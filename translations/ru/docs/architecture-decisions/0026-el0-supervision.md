# ADR-0026 — Изолированный supervisor EL0 и ограниченный lifecycle rendezvous

Status: **Accepted for bounded Phase 3.6 after #126; Codex semantic review at the maintainer’s request, readiness separate**. Date: 2026-10-06.

Document status: CURRENT

Evidence scope: принятая ограниченная архитектура Phase 3.6 после #126; current-source QEMU verification отделена от открытого performance-вопроса и production/hardware readiness.

Current reference: [Контракт supervisor](../kernel/supervision.md)

## Контекст

Issue #27 требует реального изолированного supervisor EL0, аутентификации readiness, fresh restart, ограниченной failure policy и честного shutdown. Reviewed base — main `4e713a2194742fa2d5c75fd58d2dd7608e7617be`. Основа — принятая Phase 3.5, а не её historical planning snapshot. Изучены [ADR-0025](0025-bounded-native-ipc.md), [process lifecycle](../kernel/processes.md), [scheduler ownership](0016-scheduler-ownership.md), [user-copy](0018-safe-user-copy.md), [handles](0019-process-local-handles.md), [retention](0020-handle-transfer-and-retention.md), [ASIDs](0021-asid-lifecycle.md), [grants](0022-native-event-grants-and-revocation.md), [domains](0023-domains-and-scoped-grants.md) и [Kernel Laws](../architecture/kernel-laws.md), особенно LAW-009, LAW-013, LAW-025 и LAW-043.

Существующее whole-session IPC retirement запрещает replacement до выхода всех peers. Ранняя реализация не делает это ограничение обязательным для supervision. Перенос manifest, dependency resolution, readiness или restart decisions в scheduler нарушил бы принятую границу политики. Предлагаемое расширение сохраняет инварианты и явно меняет session admission boundary.

## Решение

Использовать isolated EL0 supervisor и неизменяемые конечные bootstrap grants на image/CPU/quota. Только его точная исполняемая identity вызывает lifecycle mechanism. Service selectors явно выбирают initial grant; replacement tokens неделегируемые, свежие и связаны с supervisor и entry. Они не выбирают посторонние процессы и не расширяют images, placement, limits или instance credits. Seal уничтожает unused initial grants; отдельно retained service capabilities остаются конечными и учтёнными. Scope retirement отключает вызовы; bootstrap устанавливается однократно за загрузку.

Использовать явные lifecycle checkpoints двух CPU. Owners возвращаются к native roots после конечных execution boundaries; CPU0 приобретает оба completion и detached execution owners перед изменением process ownership или allocation/reclaim frames. IPC wait identities и owned continuation state удерживаются через checkpoint. Pending copy transactions завершаются до передачи namespace. Время жизни endpoint/source/mailbox отделено от root detachment; возврат checkpoint не разрешает reclaim blocked context.

Source/ack drainage остаётся вне scheduler storage. SGI-only entry не расходует checkpoint timer quantum до исполнения EL0. Продолжение round-robin selection требует точной живой process generation cursor. Termination — точный owner-local terminal transition с ASID retirement до frame reuse. Квоты транзакционно учитываются до publication; failed creation откатывает unpublished resources.

Manifest и supervisor EL0 выбирают порядок зависимостей, readiness probes, deadline, backoff, restart budget, discovery и shutdown. Readiness — initialized completed native IPC response от exact authorized receiver, а не записанный caller READY bit. Shutdown прекращает admission и наблюдает реальные commitment-aware terminal results перед owner termination и окончательным source/ack reclamation. Fixture демонстрирует worker и здорового peer; generic process creation, migration, wait-any и persistent services исключены.

## Альтернативы

Сохранить существующий whole-session barrier и заранее создать spare services; перенести policy в EL1; реализовать independent live admission и per-process asynchronous retirement; либо использовать bounded checkpoint membership changes с сохранением IPC ownership.

## Причины отказа от альтернатив

Заранее созданные spares не доказывают fresh allocation/restart identity и прекращение bootstrap grants. Policy в EL1 нарушает privileged necessity. Independent live admission потребовал бы более крупного asynchronous root/namespace publication и per-object retirement protocol. Checkpoints дают проверяемое ограниченное расширение с acquired owner/root quiescence, настоящими fresh instances и явным retained IPC ownership. Pause overhead и polling lifecycle queries обозначены как ограничения, а не автономная persistent supervision.

Policy update 2026-10-07: явное решение maintainer для Phase 3.7 заменяет только finite coordination deadlines #126. Authority, retained ownership и реальная quiescence обязательны. Это архитектурное решение, а не acceptance новой реализации.

## Последствия

Историческое решение #126 требовало конечных deadline публикации и copy-drain. По явному требованию maintainer для Phase 3.7 от 2026-10-07 это требование заменено: обычный coordination wait не имеет произвольного ограничения по времени. Возврат требует фактических acquired completion, detached roots, released owners и завершённых copy/source/ack obligations. Workload expiry по-прежнему даёт BudgetExpired; readiness deadline и restart backoff supervisor остаются policy. Опубликованный CPU FAILED требует fail-stop с удержанием ресурсов. Молчаливое зависание CPU не имеет конечного внутрядерного детектора в текущей foundation; внешние watchdogs тестов диагностируют его, но не доказывают completion или reclaim. Исторический review/receipts #126 сохраняют только прежний source scope.

Lifecycle membership changes приостанавливают здоровых peers на acquired checkpoints. Scope, authority credits и retained transport ownership остаются явными; failed quiescence требует quarantine или global stop, но никогда deadline-based reclaim.

## Влияние на безопасность

Только EL1 может обеспечить executing process/root, private memory, exact receiver/SEND bindings, finite launch grants, quotas и owner-local ASID retirement против untrusted children. Он не выбирает момент restart или смысл READY. Static images и manifest дают только development assurance. Production использование требует в #38 связать authorized exact images, grants и текущую trust/freshness policy; CANON/Phoenix Root, reproducibility и anti-rollback не подразумеваются.

## Влияние на compatibility

Lifecycle register entry имеет статус EXPERIMENTAL и не заморожен. Существующие native.request/1 framing и terminal arbiter сохраняются. Native dependency closure исключает routing/compatibility/advisor policy.

## Влияние на производительность

Checkpoint приостанавливает peers во время membership change; это не заявление hardware throughput или real-time.

## Проверка

Выполнить реальные isolated EL0 images на обоих fixed-affinity CPU в DEV и PROD, все прежние controls, focused mutations supervisor authority/stale-token/wait-retention, host Clippy/tests и repository checks. Сохранить exact source и artifact identities. Proposals, host fixtures и QEMU не доказывают production или physical ARM64 acceptance. [Текущая проверка после #126](../architecture/supervision-phase36-acceptance-review.md) фиксирует completed Codex architecture/code и EN/RU semantic review. Performance acceptance и independent human sign-off не заявляются.

## Обратимость

Экспериментальный entry можно заменить без замораживания internal representations; historical measurements и authority/lifetime obligations сохраняются при замене.

[English original](../../../../docs/architecture-decisions/0026-el0-supervision.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,

  "id": "adr.0026",

  "kind": "adr",

  "summary": "Accepted isolated EL0 supervision; acquired quiescence with no arbitrary coordination cutoff.",

  "aliases": ["ADR-0026"]
}
```
