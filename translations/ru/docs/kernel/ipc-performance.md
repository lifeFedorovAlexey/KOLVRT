<!-- markdownlint-disable MD041 -->
<!-- Stable knowledge anchor расположен перед видимым заголовком документа. -->

<a name="measurement-method"></a>

# Паспорт производительности IPC Phase 3.5

Document status: CURRENT
Evidence scope: воспроизводимый regression baseline в QEMU для экспериментального bounded IPC; это не заявление о производительности на оборудовании.
Current reference: [Контракт native IPC](../../../../docs/kernel/ipc.md)

## Метод измерения

Запустите `cargo xtask ipc-bench`, чтобы собрать и выполнить AArch64 kernel в профилях DEV и PROD под QEMU и обновить raw [артефакт baseline](../../../../research/measurements/ipc-phase35-baseline.json). Runner сохраняет инвентарь исходников с LF-нормализацией и их hashes, сведения о сборке, machine events и каждое измерение. Неполная матрица и изменения исходников во время запуска отклоняются.

В каждом профиле 144 группы: четыре размещения (CPU 0→0, 1→1, 0→1 и 1→0), четыре размера payload (0, 8, 64 и 256 байт) и девять scopes. Пять scopes операций — submit, receive, reply, terminal collect и полный round trip. Ещё четыре scopes измеряют hot-ready receive, пробуждение blocked receiver, ожидание blocked requester и отказ полной очереди. В каждой группе четыре warmup и шестнадцать измерений. Единица — ticks архитектурного счётчика при записанной частоте. Raw наблюдения содержат median, p95, p99, mean, population standard deviation, число context switches и блокировок actor.

EL0 probe измеряет выбранный API operation либо полный submit/wait/collect round trip через native counter call. Значения включают вход и возврат probe, подготовку аргументов и проверку результата, а также реальное блокирование и wake scheduling, если actor блокируется. Синтетическая поправка overhead не вычитается. DEV с диагностической инструментацией и оптимизированная PROD — отдельные выборки; сравнивать их как одинаковые пути кода нельзя.

Fixture контролируемых путей проверяет каждую warmup и измеряемую операцию через owner-local счётчик блокировок по условию в существующем clock probe. Hot receive требует нулевую разницу. Обратный IPC handshake удерживает service, пока основной request не поставлен в очередь; готовность устанавливается независимо от скорости scheduler. Blocked receive и ожидание blocked requester требуют разницу ровно в одну блокировку. Конечные вычисления в течение пяти миллисекунд boot-local времени дают peer возможность заблокироваться; проверка счётчика отклоняет запуск, если требуемый путь не возник. Для отказа полной очереди service удерживается handshake, заполняются все четыре queue slots, а дополнительная отправка должна вернуть явное exhaustion с нулём блокировок по условию. После неуспешного admission выполняется успешное опустошение очереди и проверка нулевых domain charges и отсутствия утечки frames.

Наблюдение счётчика блокировок через clock существует только в measurement build. Оно не даёт authority и не меняет IPC transitions. Обычный receive остаётся выборкой смешанных путей. Cross-CPU submit включает публикацию retained wake и notification; receive и round-trip включают возобновление target. Эти наблюдения не выделяют стоимость одного SGI handler.

## Базовый путь ownership и копирования

Безопасный путь выполняет четыре копирования: user memory клиента в initialized kernel request; kernel request в user memory service; ответ service в initialized kernel result; kernel result в user memory клиента. Zero-copy путь не измеряется и не заявляется.

## Ограничения и продолжение

Это regression baseline в QEMU, а не задержка на silicon и не целевой показатель скорости. Он не отделяет накладные расходы отдельных инструкций/исключений, не моделирует поведение физических cache/interconnect и не задаёт универсальное распределение нагрузки. Метки контролируемых путей описывают проверенные операции, а не все операции control handshake группы. Измерения оборудования требуют указать платформу и сохранить сведения о firmware и инструментах.

Перед использованием артефакта как текущего свидетельства выполните `cargo xtask ipc-bench` после последних изменений реализации. Артефакт со stale source hashes остаётся историческими данными и не подтверждает более новый код.

[English source](../../../../docs/kernel/ipc-performance.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.ipc-performance",
  "kind": "benchmark-evidence",
  "summary": "Метод измерения в QEMU, copy count и ограничения bounded native IPC.",
  "units": [
    {
      "id": "kolvrt.ipc.performance",
      "anchor": "measurement-method",
      "kind": "contract-section",
      "summary": "Raw baseline, области измерения, границы интерпретации и контролируемые дальнейшие испытания.",
      "depends_on": ["kolvrt.ipc"]
    }
  ]
}
```
