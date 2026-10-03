# ADR-0020 — Передача дескрипторов с ограничением прав и удержание targets

Статус: **Принято для ограниченной реализации issue #23**. Дата: 2026-10-03.

Document status: CURRENT  
Evidence scope: kernel-local lookup, close, transfer и удержание общего Event; два CPU в QEMU DEV/PROD.
Current reference: [Контракт handles](../kernel/handles.md)

## Контекст

ADR-0019 закрепил безопасную identity по поколениям и синхронный borrowed lookup. Issue #23 дополнительно требует проверки прав, receiver-local transfer, удержания принятой работы и SMP гонок close/lookup. В kernel действуют фиксированное владение scheduler для каждого CPU, общий allocator и framework объектов отсутствуют.

## Решение

- Сохранить восемь caller-local записей, восемь младших slot bits и 56 generation bits без wrap. Lookup проверяет владельца, generation, live state, конкретный primitive kind и обязательные права под одним exclusive scheduler operation.
- Определить только `SEND=1` и `TRANSFER=2`. Transfer требует TRANSFER, отвергает неизвестные права, применяет attenuation, выделяет новую receiver-local generation и сохраняет тот же kernel-only TargetId. Получатель не меняется, если проверка или allocation slot не прошла.
- Разрешить EL0 transfer в namespace на том же fixed-affinity CPU. Cross-CPU transfer выполняет только coordinator, когда оба процесса эксклюзивно принадлежат ему; user request не получает доступ к scheduler storage другого CPU.
- Оставить только два resource types: Event и Completion. Event state размещается в fixed atomic pool на 512 slots с generation без wrap и максимум 1,024 live references на событие. Каждая запись namespace и каждое принятое owned retained reference удерживает один счётчик. Completion хранит immutable copy.
- Borrowed lookup исключает close/retire через Rust ownership. `Retained::retain` создаёт bounded owned reference для работы за пределами вызова. Последняя ссылка освобождает pool slot; close влияет только на одну caller-local запись.
- Для исключения на таблице используется scheduler ownership permit. Обновление shared Event reference выполняют bounded atomic операции. Вложенных table/object locks нет. User input копируется до заимствования namespace; lock или ownership permit не удерживается через allocation, blocking, wait или external call.

## Alternatives

Global IDs, user-visible pointers, поколения с wrap, неограниченная duplication, implicit authority, global spinlock, динамическая allocation для refcounts и универсальный object/policy framework.

## Why rejected

Global selectors обходят caller attribution, а user pointers раскрывают kernel storage. Wrapping generations повторно активируют stale handles. Unrestricted duplication и implicit authority не обеспечивают attenuation. Global lock и allocating refcounts добавляют blocking и failure paths к операциям, уже сериализованным владением CPU. Universal object policy выходит за рамки двух concrete target types и их bounded rights.

## Consequences

48-байтовый version-1 handle request поддерживает lookup, close и transfer. Неизвестные kinds, rights, поколения процессов, операции и повреждённые extents fail closed. EL0 не может подменить principal вызывающего: он берётся из scheduler-owned task. Slot и generation получателя должны соответствовать bound receiver namespace на CPU caller. После close source receiver handles и owned accepted Event references остаются действительными.

Фиксированные квоты ограничивают память и количество ссылок. Переполненная receiver table не даёт частичного transfer. Исчерпанные поколения уходят в quarantine. Global pool на 512 Events ограничивает общее число shared objects; исчерпание ссылок возвращает отдельную ошибку. Completion handles удерживают запись completion, но не процесс и не его address space.

## Concurrency

Ни одно значение EL0 не становится pointer. TargetId остаётся kernel-only и сохраняется при transfer. Отказ прав или получателя не меняет ни одну таблицу. Операции таблиц на одном CPU сериализуются owner permit scheduler. Cross-CPU coordinator transfer требует, чтобы обе таблицы были отключены от исполнения. Конкурентные close/lookup Event безопасны: запись получателя владеет отдельной atomic reference; clone и final release конкурируют через compare/exchange. Тесты используют реальный двухъядерный EL0 lookup/close path и production EL0 transfer decoder.

## Testing

Host tests проверяют exhaustion generation, forged/stale/foreign/wrong-kind handles, attenuation прав, отказ escalation, transactionality заполненного receiver, сохранение binding, close после delegation, owned retention после закрытия всех handles, конкурентные signal/clone/release и исчерпание квот. QEMU запускает EL0 transfer request, close источника, lookup получателя, cross-CPU delegated lookup/close и enforcement mutations в DEV/PROD. Exact-source evidence приведён в контракте handles.

## Compatibility impact

Provisional 48-byte request использует явные слова и status codes; он не раскрывает Rust layout, pointers, global object number или compatibility errno. Соглашения fd/HANDLE остаются над native boundary.

## Performance impact

Lookup, close и transfer используют bounded table scans без allocation. Event ownership использует bounded compare/exchange. QEMU timing receipts — regression observations, не hardware throughput claims.

## Security impact

EL0 не выбирает source process и не передаёт object pointer. Rights checks ограничивают операцию, transfer только ослабляет права, receiver entry публикуется после всех проверок. Закрытый source token нельзя повторно lookup; независимые delegated и accepted-work references сохраняют lifetime.

## Reversibility

Изменение wire frame, rights bits, квот, набора primitives, affinity или retention guarantee требует review контракта. Новые authority policy, revocation, IPC admission и domain budgets требуют отдельных решений.

[Английский оригинал](../../../../docs/architecture-decisions/0020-handle-transfer-and-retention.md)
