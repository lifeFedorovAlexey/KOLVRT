# ADR-0022 — Ограниченные grants и отзыв прав для собственного Event

Статус: **Принято для минимального среза authority Event**. Дата: 2026-10-04.

Заменяет только положения ADR-0020 о наборе прав и отложенном отзыве в этом срезе.
Решения ADR-0020 об identity, transfer, retention, close, pool, квотах и fixed affinity
остаются в силе.

## Контекст

Issue #24 требует явных native grants, attenuation и отзыва, сериализованного с admission.
ADR-0020 намеренно определил SEND/TRANSFER и отложил revoke до отдельного решения.
Текущие process-local handle table и shared Event pool уже дают ограниченный механизм
authority и lifetime; универсальный policy interpreter или hierarchy для одного Event
operation не нужны.

## Решение

- Authority хранится в caller-local handle. Caller определяется по scheduler-owned task и
  привязанному namespace; EL0, routing metadata, diagnostics и package policy не могут
  задавать caller, authority pointer или права.
- Известные rights: `SEND=1`, `TRANSFER=2`, `REVOKE=4`. Неизвестные bits отклоняются.
  Bootstrap Event по умолчанию получает SEND, Completion — NONE; более широкие grants
  выдаются только явным trusted-вызовом `create_*_with_rights`. В этом срезе нет EL0
  операции создания объекта или выдачи гранта.
- TRANSFER по-прежнему требует TRANSFER у источника и разрешённое подмножество его прав.
  Переданная запись получает новый receiver-local token и сохраняет тот же TargetId и
  состояние Event. REVOKE можно делегировать, только если он есть в исходном grant.
- Handle с REVOKE может запретить новые SEND admissions для общего Event target. Signal
  admission и revoke состязаются атомарно на состоянии target: если побеждает signal CAS,
  coalesced pending effect уже зафиксирован; если revoke — signal возвращает `Revoked`
  (wire status 11). Operation синхронный и не оставляет отложенного эффекта, который
  пришлось бы drain-ить.
- Revoke не освобождает Event, не удаляет уже pending работу и не инвалидирует retained
  references. Close убирает одну caller-local запись и не является revoke. Последняя
  ссылка освобождает pool slot; новый target получает новое nonwrapping generation и
  чистое состояние. Revoke постоянен для одного поколения target; для возобновления
  создаются новый target и новый явный grant.
- Механизм остаётся resource-specific. Он не разрешает менять Completion, обращаться к
  device, вызывать произвольные service operations, управлять security domains или
  отменять/drain-ить асинхронную работу. Для будущих async operations нужны отдельные
  admission и terminal lifetime contracts.

## Вывод инвариантов

LAW-009 сохраняется проверкой фактических SEND/REVOKE прав caller на native boundary;
LAW-013 разделяет identity target, состояние admission и lifetime хранения. Требование
единого арбитра LAW-025 выполняется хранением revoke в том же shared Event state, что и
signal admission; второго policy ledger нет. Ограниченный SVC использует безопасную
копию request и существующее per-CPU владение namespace; в EL1 не добавляются общий
interpreter, allocation, lock или service policy.

## Альтернативы

- Оставить неявные `ALL` grants для Event и отложить revoke. Это не удовлетворяет
  требованиям missing grants и revoke/admission из issue #24.
- Отзывать только одну запись handle. Делегированные aliases останутся действующими,
  значит target-scoped отзыв полномочий не реализован.
- Инвалидировать target под уже принятой работой или освобождать его при revoke. Это
  смешивает authority с lifetime и нарушает retention семантику ADR-0020.
- Добавить общий EL1 policy interpreter или universal capability/object framework. Один
  ограниченный Event operation не оправдывает такие privileged mechanisms.

## Почему отклонены

Неявный ALL grant скрывает отсутствие полномочий за поведением по умолчанию. Close
одного handle намеренно локален и не останавливает admission через aliases. Освобождение
отозванного target инвалидировало бы retained borrowers или удалило pending event. Общий
interpreter добавил бы произвольную policy и failure paths вне операции, требующей защиты.

## Последствия

В provisional 48-byte request добавляется operation 4 (`revoke`) без изменения размера
frame. Добавляется status 11; предыдущие значения сохраняют смысл. Старые decoders
отклоняют неизвестные operations. Это не frozen ABI и не завершает issue #24: независимое
управление issuer, production restart/rebind, nested consumer/service authority, security
domains и общие IPC grants остаются отдельными задачами.

## Влияние на совместимость

В provisional 48-byte request добавляется operation 4 (`revoke`) без изменения frame.
Добавляется status 11; предыдущие значения сохраняют смысл. Старые decoders отклоняют
неизвестные operations. Формат остаётся provisional и не заморожен.

## Влияние на производительность

Signal admission использует цикл повторных compare/exchange; revoke выполняет атомарный
fetch-or над тем же общим байтом Event. Эти lock-free операции не гарантируют конечного
числа повторов на один вызов при произвольной конкуренции. Новых allocation, global lock
или дополнительных scans таблицы нет. До получения QEMU/profile измерений пропускная
способность не заявляется.

## Влияние на безопасность

Публиковать может только caller с SEND, а остановить новые публикации — только caller с
REVOKE. Transfer может лишь сужать права. Состояние revoke общее для target, поэтому все
aliases видят одно authoritative решение. Event остаётся выделенным до обычного release
последней ссылки.

## Проверка

Host checks покрывают attenuation, явные rights, различие close/revoke, lifetime aliases
и linearization signal/revoke. Реальный EL0 fixture должен выполнить revoke в DEV и PROD,
затем получить отказ SEND admission. До заявления о приёмке нужны CI и exact-source
evidence.

## Обратимость

Замена битов прав, wire operation, точки linearization или семантики уже принятой работы
требует reviewed successor decision и migration evidence. Compatibility exception не
может расширять authority.

[English original](../../../../docs/architecture-decisions/0022-native-event-grants-and-revocation.md)
