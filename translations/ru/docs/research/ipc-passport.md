# Паспорт Arena для IPC и production-сервиса

Document status: CURRENT
Evidence scope: сохранены 96 функциональных pilot boots; отдельно зарегистрированная основная кампания из 576 boots и current-source acceptance ещё не завершены. Исторический pilot не проверяет последующие изменения writer.
Current reference: [Native IPC](../kernel/ipc.md); [Контракт измерений Arena](../architecture/arena-measurement-contract.md)

<a name="kolvrt-arena-ipc-passport"></a>

## Полезные операции и полномочия

[Основной протокол](../../../../research/arena/ipc-query/protocol.json) задаёт двенадцать сценариев. Девять транспортных сценариев сочетают запросы 0/8/256 bytes с одним отправителем на том же CPU, одним на другом CPU и двумя отправителями через CPU. Отдельный сценарий capacity-one проверяет отказ и опустошение очереди. Ещё два выполняют неизменённый production counter-service с его точным 16-byte протоколом Add/Get, одним либо двумя cross-CPU отправителями.

Достаточно существующих неизменяемых grants: root CPU0 отправляет child slot1 CPU0 либо service slot0 CPU1. Root и slot1 получают отдельные настоящие SEND capabilities к slot0. Число handle не передаётся как полномочие; новые deployment inputs и расширение grants не вводятся. Same-CPU contention, обратные направления и same-CPU counter не входят в выбранное покрытие. Транспортный responder и клиенты — внешние ABI-инструменты без production-аналога, а не копии сервиса, supervisor или ядра. Внешний root не проверяет production supervisor.

Responder проверяет payload и возвращает sequence фактического исполнения. Нулевой payload требует проверки sequence/ledger, а не только пустого успешного ответа. Ответы counter и конечное состояние должны совпасть с действительно успешными Add обоих отправителей. Saturation использует обычный feedback IPC: responder ждёт ответа root, пока root заполняет настоящую очередь, получает отказ следующего Submit, освобождает responder и проверяет опустошение. Порядок не устанавливается задержкой таймера или тестовым rendezvous ядра. Envelope принятого A намеренно включает вложенные probes отказа B и feedback-release RPC до Collect A; это отдельная управляемая нагрузка, а не обычный независимый round-trip.

## Исходы и измерения

Каждая предложенная попытка сохраняет client/request identity, stage/status, граничные timestamps и результат. Deadline запроса в одну секунду — существующая policy публичного SDK rpc: start плюс частота счётчика; это не гарантия latency и не новый coordination deadline ядра. Ошибки не приводят к увеличению timeout или retry. Ожидаемый Exhausted подтверждает отказ очереди, но не считается полезным успехом. Неожиданный expiry, незавершённый Collect либо неверный эффект сервиса отклоняет correctness. Два запущенных клиента сами по себе не доказывают временное перекрытие. Пересечение userspace envelopes попыток с успешным admission вычисляется по timestamps отдельно от отказов очереди; оно не доказывает одновременное нахождение запросов внутри ядра.

Три общих CLOCK окружают Submit, промежуточный результат и завершённый Collect. Wall envelope измеряет userspace submit-to-collected-result, а не точную внутреннюю admission-to-terminal latency. x2 — частичное окно исполнения клиента с переходами и stalls; x3 относится к READ_WINDOW, а не IPC service. Queue residence, CPU сервиса, capability/lock costs, PMU, динамические copy counts, peak memory и switches отдельного запроса остаются unavailable без attributable evidence. Throughput учитывает полезные успехи только измеряемой фазы. Общий интервал начинается после обеих warmup-фаз до peer GO и заканчивается после peer DONE (либо последнего root sample при одном отправителе), до массового экспорта raw. Управляющие GO/DONE входят в интервал. Нельзя обращать median latency или складывать перекрывающиеся длительности.

Recorder ON добавляет ровно одну volatile-запись промежуточных CLOCK ticks в private память actor до конечного CLOCK; OFF сохраняет те же предложенные операции, CLOCK и oracle. Это стоимость дополнительного recorder, а не всех probes; её нельзя вычитать как точную поправку. Популяции DEV и PROD разделены.

## Сохранённый pilot и основная кампания

[Запечатанный pilot bundle](../../../../research/arena/runs/ipc-pilot-v1/README.txt) сохраняет 96 из 96 успешных запланированных функциональных boots и предшествующие неуспешные подготовительные попытки с исходной provenance. Он содержит 3456 raw records: 3080 измеряемых offers, 2637 полезных успехов и 443 измеряемых Exhausted. Независимое review пересчитало counts, границы времени и пересечения cross-client envelopes. Архивные исходники восстановлены отдельно; исходный dirty запуск не переименован в clean. Проверка архива сверяет 646 исходных members и definition/supplemental bytes без выполнения QEMU. Это функциональное pilot evidence, а не qualifying performance или приёмка основной кампании.

Стабильный [контракт измерений](ipc-measurement-contract.md) и основной протокол задают двенадцать свежих пар на case/profile, 576 boots: pair index 0–11, case 0–11, DEV затем PROD, OFF/ON для чётных пар и ON/OFF для нечётных. Pilot slots не засчитываются как основное выполнение. Популяции сохраняют четыре warmup и 32 measured requests либо три и 33 для saturation. Дизайн увеличивает число повторений со свежей загрузкой без роста хранилища actor. Сохраняется фиксированное finite-session conditioning без утверждений о steady-state stabilization или точности p99.

Actor ограничивает каждый boot 36 наблюдениями суммарно из-за существующего стека 16 KiB. Два отправителя дают по 18: два warmups и шестнадцать измеряемых попыток. Отчёт содержит заголовок в 128 слов и восемь слов на наблюдение, всего 416; протокол фиксирует все поля и резервные слова. Для benchmark не увеличиваются stack, endpoint, report buffer или logger limits. Существующие отчёты до 2048 слов экспортируются событиями по 64 слова с проверкой полноты и identity. Экспорт находится вне latency envelope отдельной операции; его влияние на общий интервал раскрывается.

Producer повторно использует общий build/execution путь, сохранение неизменяемых артефактов и существующий Arena assessor. Все IPC profiles ссылаются на существующий канонический feature `kolvrt.ipc.transport`. Сорок восемь case/profile/family profiles отделяют successful latency от одного achieved-throughput aggregate на boot. У throughput нет отдельного aggregate warmup observation; conditioning реальных запросов сохраняется. Объединённая A/C latency сценария 9 дополняет отдельные A/B/C distributions. Matched observer arrays сохраняют все offered measured requests и исходы без усечения по успеху; разные outcome vectors делают интерпретацию INCONCLUSIVE. Недобор фиксированных success-conditioned samples и отказы остаются видимыми и могут сделать structural admission INELIGIBLE.

Отчёт о парной неопределённости условный: двенадцать boot-pair statistics, median и interval от третьего до десятого отсортированного значения с nominal coverage 96.14% при независимых одинаково распределённых непрерывных парных наблюдениях. Host drift может нарушить эти предположения. Это marginal описание median effect, а не одновременное покрытие scenarios, решение о speedup или гарантия точности хвоста. Наблюдаемая variance не создаёт performance allowance.

SEC связывает шесть ограниченных native-request requirements с четырьмя методами source/runtime/negative evidence. Полное current matrix evidence должно совпадать с реальным plan, всеми обязательствами и source inventory; существующий matrix validator проверяет configuration и donor lineage. Именованным checks нужны настоящие executed witnesses и в DEV, и в PROD, а не partial shard или reused-only assertion. Отсутствующее evidence не становится PASS. Основное выполнение, current-source SEC/regression receipts и независимая приёмка остаются незавершёнными; исторический pilot не закрывает эти gates. QEMU не доказывает физический ARM64 performance или превосходство над Linux/seL4.

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
        "implementation_scope": "Experimental external IPC producer with selected transport and original counter-service workloads, complete bounded report export, shared custody/admission and 48 profiles for a fixed 576-boot main plan. The retained 96-boot functional pilot is historical source-bound evidence; current main execution and SEC acceptance remain pending.",
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
          "apps/native-apps/src/counter-service.rs",
          "crates/xtask/src/arena_common/passport.rs",
          "crates/xtask/src/arena_ipc/passport.rs",
          "research/arena/profiles/ipc-query-0-dev-latency.json",
          "research/arena/profiles/ipc-query-0-dev-throughput.json",
          "research/arena/profiles/ipc-query-0-prod-latency.json",
          "research/arena/profiles/ipc-query-0-prod-throughput.json",
          "research/arena/profiles/ipc-query-1-dev-latency.json",
          "research/arena/profiles/ipc-query-1-dev-throughput.json",
          "research/arena/profiles/ipc-query-1-prod-latency.json",
          "research/arena/profiles/ipc-query-1-prod-throughput.json",
          "research/arena/profiles/ipc-query-2-dev-latency.json",
          "research/arena/profiles/ipc-query-2-dev-throughput.json",
          "research/arena/profiles/ipc-query-2-prod-latency.json",
          "research/arena/profiles/ipc-query-2-prod-throughput.json",
          "research/arena/profiles/ipc-query-3-dev-latency.json",
          "research/arena/profiles/ipc-query-3-dev-throughput.json",
          "research/arena/profiles/ipc-query-3-prod-latency.json",
          "research/arena/profiles/ipc-query-3-prod-throughput.json",
          "research/arena/profiles/ipc-query-4-dev-latency.json",
          "research/arena/profiles/ipc-query-4-dev-throughput.json",
          "research/arena/profiles/ipc-query-4-prod-latency.json",
          "research/arena/profiles/ipc-query-4-prod-throughput.json",
          "research/arena/profiles/ipc-query-5-dev-latency.json",
          "research/arena/profiles/ipc-query-5-dev-throughput.json",
          "research/arena/profiles/ipc-query-5-prod-latency.json",
          "research/arena/profiles/ipc-query-5-prod-throughput.json",
          "research/arena/profiles/ipc-query-6-dev-latency.json",
          "research/arena/profiles/ipc-query-6-dev-throughput.json",
          "research/arena/profiles/ipc-query-6-prod-latency.json",
          "research/arena/profiles/ipc-query-6-prod-throughput.json",
          "research/arena/profiles/ipc-query-7-dev-latency.json",
          "research/arena/profiles/ipc-query-7-dev-throughput.json",
          "research/arena/profiles/ipc-query-7-prod-latency.json",
          "research/arena/profiles/ipc-query-7-prod-throughput.json",
          "research/arena/profiles/ipc-query-8-dev-latency.json",
          "research/arena/profiles/ipc-query-8-dev-throughput.json",
          "research/arena/profiles/ipc-query-8-prod-latency.json",
          "research/arena/profiles/ipc-query-8-prod-throughput.json",
          "research/arena/profiles/ipc-query-9-dev-latency.json",
          "research/arena/profiles/ipc-query-9-dev-throughput.json",
          "research/arena/profiles/ipc-query-9-prod-latency.json",
          "research/arena/profiles/ipc-query-9-prod-throughput.json",
          "research/arena/profiles/ipc-query-10-dev-latency.json",
          "research/arena/profiles/ipc-query-10-dev-throughput.json",
          "research/arena/profiles/ipc-query-10-prod-latency.json",
          "research/arena/profiles/ipc-query-10-prod-throughput.json",
          "research/arena/profiles/ipc-query-11-dev-latency.json",
          "research/arena/profiles/ipc-query-11-dev-throughput.json",
          "research/arena/profiles/ipc-query-11-prod-latency.json",
          "research/arena/profiles/ipc-query-11-prod-throughput.json",
          "crates/xtask/src/arena_ipc/analysis.rs"
        ],
        "acceptance": [],
        "issues": [33],
        "adrs": ["adr.0006", "adr.0025", "adr.0026"],
        "limitations": [
          "Pilot does not establish final main-campaign acceptance, adequate tail precision, physical performance or eligible records. No full Cartesian CPU/contention coverage or same-CPU counter claim."
        ],
        "next_gate": "Execute the preregistered 576-boot main campaign without replacement; retain conditional paired uncertainty, complete outcomes and current-source SEC/full-matrix evidence, then independently review bounded acceptance. Historical pilot boots do not fill main slots.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "UNKNOWN",
            "reason": "The retained 96-boot pilot passed at its named source; subsequent shared writer/security admission changes and the main campaign have no accepted current-source receipt."
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
