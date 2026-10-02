# Routing model v0.1

Один глобальный compatibility switch запрещён. DEV/PROD — профили исполнения,
а не выбор native/compat. Resolve происходит при launch/bind, dispatch использует
зафиксированный route. Dynamic discovery на каждом syscall не требуется.

## Входы и результат resolver

Ключ: `(consumer, API family, protocol, requested semantic version, capability,
state-domain identity, device identity?)`. Package manifest задаёт defaults;
process manifest сужает их; driver/device policies добавляют ограничения; administrator
policy ограничивает допустимые modules/rights. Порядок не является «last write wins»:
сначала пересечение разрешений, затем один exact compatible route. Два допустимых
неупорядоченных выбора требуют явной pinning policy, иначе bind отклоняется.

Результат: immutable `RouteBinding` с native operation set, adapter chain (обычно 0 или 1),
semantic version, artifact digest, dependency closure, state-domain ID, rights,
limits, profile и route generation. Неподдерживаемая family не падает молча в compat.
Разрешение native route не может быть отозвано в пользу более привилегированного adapter.

## Уровни и пределы свободы

| Уровень | Начальная роль |
|---|---|
| Executable/package | Declared behavior requirements; inherited defaults |
| Process | Bound handles, process personality и policy restrictions |
| Driver/device | Protocol binding и scoped hardware translator |
| Subsystem/API family | Минимальный public routing selector |
| Protocol/version | Immutable semantic contract |
| Compatibility capability | Узкое отклонение с доказанным state independence |

Не строить универсальный graph router до доказанного workload. Первоначальная модель —
конечная family table с fixed bindings. Capability-level split разрешён ADR только после
анализа общего состояния. Process ABI personality и CPU execution architecture различны:
Linux AArch64 personality не означает поддержку x86 или AArch32 instruction sets.

Примеры допустимого намерения:

| Consumer | memory | files | network | synchronization |
|---|---|---|---|---|
| A | native | native | native | old-sync-v2 в отдельной synchronization domain |
| B | native | native | native | native |
| C | Linux personality | Linux personality | Linux personality | Linux personality |

Это иллюстрация binding, не существующие modules. A возможен только если старый sync
adapter и native memory разделяют согласованный shared-memory identity и lifetime.
File operations, locks и epoll часто связаны общей fd/OFD graph; независимо переключить
только close нельзя. Credentials, namespace и authorization образуют security domain.
Read/write одного stream используют один protocol state. Переименование selector не
делает состояния независимыми.

## Shared objects и миграция

При передаче handle получатель получает object identity и immutable behavior binding,
а не право заново интерпретировать state по своим package defaults. Export/import между
domains выполняется явным gateway, который проверяет rights и semantics; невыразимое
сочетание отклоняется. Поддержка mixed consumers не разрешает два независимых lock arbiters.

Безопасное DEV переключение выполняется как transaction:

1. Проверить migration capability, semantic equivalence scope и права оператора.
2. Перекрыть admission новых operations на всей state domain.
3. Drain in-flight requests, waiters, callbacks, locks и device I/O; timeout оставляет старый binding.
4. Снять snapshot или выполнить явно описанную state conversion; проверить invariants.
5. Атомарно опубликовать binding generation для всех входов domain.
6. Открыть admission; старый adapter удалить лишь после завершения references/grace period.

Ошибки до commit оставляют старую domain работоспособной. После externally visible
actions откат не обещается. Если quiescence не достижим или формат состояния несовместим,
миграция требует restart. PROD не предоставляет experimental live switching.
Security checks нужны в любом profile, включая fast dispatch.

Phase 0.2 должен model-check resolver conflicts, downgrade attempts, handle transfer,
concurrent rebind и unload. Основание: cases 02, 03, 17, 18, 25, 26, 30.
