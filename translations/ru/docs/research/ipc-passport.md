# Pilot Arena для IPC и production-сервиса

Document status: CURRENT
Evidence scope: реализация issue #33 продолжается; протокол и изменения исходников не подтверждают успешное выполнение или acceptance.
Current reference: [Native IPC](../kernel/ipc.md); [Контракт измерений Arena](../architecture/arena-measurement-contract.md)

<a name="kolvrt-arena-ipc-passport"></a>

## Полезные операции и полномочия

[Протокол pilot](../../../../research/arena/ipc-query/protocol.json) задаёт двенадцать сценариев. Девять транспортных сценариев сочетают запросы 0/8/256 bytes с одним отправителем на том же CPU, одним на другом CPU и двумя отправителями через CPU. Отдельный сценарий capacity-one проверяет отказ и опустошение очереди. Ещё два выполняют неизменённый production counter-service с его точным 16-byte протоколом Add/Get, одним либо двумя cross-CPU отправителями.

Достаточно существующих неизменяемых grants: root CPU0 отправляет child slot1 CPU0 либо service slot0 CPU1. Root и slot1 получают отдельные настоящие SEND capabilities к slot0. Число handle не передаётся как полномочие; новые deployment inputs и расширение grants не вводятся. Same-CPU contention, обратные направления и same-CPU counter не входят в выбранное покрытие. Транспортный responder и клиенты — внешние ABI-инструменты без production-аналога, а не копии сервиса, supervisor или ядра. Внешний root не проверяет production supervisor.

Responder проверяет payload и возвращает sequence фактического исполнения. Нулевой payload требует проверки sequence/ledger, а не только пустого успешного ответа. Ответы counter и конечное состояние должны совпасть с действительно успешными Add обоих отправителей. Saturation использует обычный feedback IPC: responder ждёт ответа root, пока root заполняет настоящую очередь, получает отказ следующего Submit, освобождает responder и проверяет опустошение. Порядок не устанавливается задержкой таймера или тестовым rendezvous ядра. Envelope принятого A намеренно включает вложенные probes отказа B и feedback-release RPC до Collect A; это отдельная управляемая нагрузка, а не обычный независимый round-trip.

## Исходы и измерения

Каждая предложенная попытка сохраняет client/request identity, stage/status, граничные timestamps и результат. Deadline запроса в одну секунду — существующая policy публичного SDK rpc: start плюс частота счётчика; это не гарантия latency и не новый coordination deadline ядра. Ошибки не приводят к увеличению timeout или retry. Ожидаемый Exhausted подтверждает отказ очереди, но не считается полезным успехом. Неожиданный expiry, незавершённый Collect либо неверный эффект сервиса отклоняет correctness. Два запущенных клиента сами по себе не доказывают временное перекрытие. Пересечение userspace envelopes попыток с успешным admission вычисляется по timestamps отдельно от отказов очереди; оно не доказывает одновременное нахождение запросов внутри ядра.

Три общих CLOCK окружают Submit, промежуточный результат и завершённый Collect. Wall envelope измеряет userspace submit-to-collected-result, а не точную внутреннюю admission-to-terminal latency. x2 — частичное окно исполнения клиента с переходами и stalls; x3 относится к READ_WINDOW, а не IPC service. Queue residence, CPU сервиса, capability/lock costs, PMU, динамические copy counts, peak memory и switches отдельного запроса остаются unavailable без attributable evidence. Throughput учитывает полезные успехи только измеряемой фазы. Общий интервал начинается после обеих warmup-фаз до peer GO и заканчивается после peer DONE (либо последнего root sample при одном отправителе), до массового экспорта raw. Управляющие GO/DONE входят в интервал. Нельзя обращать median latency или складывать перекрывающиеся длительности.

Recorder ON добавляет ровно одну volatile-запись промежуточных CLOCK ticks в private память actor до конечного CLOCK; OFF сохраняет те же полезные операции, CLOCK и oracle. Это стоимость дополнительного recorder, а не всех probes; её нельзя вычитать как точную поправку. Популяции DEV и PROD разделены.

## Pilot, экспорт и admission

Первый функциональный pilot предлагает 4 warmups и 32 измеряемые попытки на загрузку; saturation — 3 и 33. Две свежие пары OFF/ON и ON/OFF для двенадцати сценариев и обоих профилей дают 96 загрузок. Каждая ошибочная или незавершённая попытка сохраняется без замены. Запросы одной загрузки могут коррелировать. Отдельный основной план фиксируется после review pilot; описательные p95/p99 не доказывают достаточную точность хвостов.

Текущий actor ограничивает pilot 36 наблюдениями суммарно из-за существующего стека 16 KiB. Два отправителя дают по 18: два warmups и шестнадцать измеряемых попыток. Отчёт содержит заголовок в 128 слов и восемь слов на наблюдение, всего 416; протокол фиксирует все поля и резервные слова. Для benchmark не увеличиваются stack, endpoint, report buffer или logger limits. Существующие отчёты до 2048 слов экспортируются событиями по 64 слова с проверкой полноты и identity. Экспорт находится вне latency envelope отдельной операции; его влияние на общий интервал раскрывается.

Producer повторно использует обычный build/execution путь CLOCK и общие механизмы Arena. Существующие schema, registry и correctness/security/loss gates сохраняют силу. Популяция успешных операций с неполным фиксированным sample plan может оставаться INELIGIBLE; отказы не удаляются и не заменяются успехами. Pilot сам по себе не закрывает #33. Основные profiles, exact-source SEC/regression receipts, независимое review и полная EN/RU проверка ещё обязательны. QEMU не доказывает физический ARM64 performance или превосходство над Linux/seL4.

[Английский оригинал](../../../../docs/research/ipc-passport.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.research.ipc-passport",
  "kind": "subsystem-contract",
  "summary": "External IPC pilot with complete outcomes, real counter-service and bounded report export.",
  "units": [
    {
      "id": "kolvrt.arena.ipc-passport",
      "anchor": "kolvrt-arena-ipc-passport",
      "kind": "feature",
      "summary": "IPC transport and useful-service measurement through ordinary production interfaces.",
      "depends_on": [
        "kolvrt.ipc",
        "kolvrt.arena.clock-passport",
        "doc.kolvrt.arena.measurement-contract",
        "adr.0025",
        "adr.0026",
        "kolvrt.apps.native-elf"
      ],
      "feature": {
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "External functional IPC pilot covering three transport payloads, selected same/cross CPU placements, two requesters, capacity-one rejection and unchanged production counter-service; source implementation and acceptance are in progress.",
        "sources": [
          "tests/native-apps/Cargo.toml",
          "tests/native-apps/src/ipc-measure-root.rs",
          "tests/native-apps/src/ipc-measure-client.rs",
          "tests/native-apps/src/ipc-measure-peer.rs",
          "tests/native-apps/src/ipc_measure.rs",
          "crates/xtask/src/arena_ipc.rs",
          "crates/xtask/src/arena_common.rs",
          "crates/xtask/src/native_apps.rs",
          "crates/xtask/src/main.rs",
          "crates/kernel/src/native_boot.rs",
          "research/arena/ipc-query/protocol.json",
          "research/arena/ipc-query/input.json",
          "research/arena/ipc-query/resources.json",
          "research/arena/ipc-query/dev-environment.json",
          "research/arena/ipc-query/prod-environment.json",
          "tests/native-apps/src/ipc_measure_protocol.rs",
          "tests/native-apps/tests/ipc_measure_protocol.rs",
          "apps/native-runtime/src/lib.rs",
          "apps/native-apps/src/counter-service.rs"
        ],
        "acceptance": [],
        "issues": [33],
        "adrs": ["adr.0006", "adr.0025", "adr.0026"],
        "limitations": [
          "Pilot does not establish final main-campaign acceptance, adequate tail precision, physical performance or eligible records. No full Cartesian CPU/contention coverage or same-CPU counter claim."
        ],
        "next_gate": "Execute retained functional pilot, review actual outcome/stack/export behavior, freeze main sampling and admission profiles, obtain current-source SEC and regression evidence and independent EN/RU review before closing issue33.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "UNKNOWN",
            "reason": "Current IPC actor/runner changes have not completed a reviewed campaign."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical IPC campaign."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Introduce the external IPC producer without claiming execution or issue acceptance."
          }
        ]
      }
    }
  ]
}
```
