# Контракт измерений IPC

Document status: DESIGN BASELINE
Evidence scope: нормативный протокол измерений основной кампании; без результатов выполнения, состояния verification или acceptance receipt.
Current reference: [Native IPC](../kernel/ipc.md); [Методология benchmark](../architecture/benchmarking.md)

<a name="kolvrt-arena-ipc-measurement-contract"></a>

## Нагрузки и полномочия

Контракт задаёт двенадцать сценариев через существующие неизменяемые grants native bootstrap. Сценарии 0–8 используют транспортные запросы 0, 8 и 256 bytes: для каждого размера последовательно один отправитель на том же CPU, один через CPU и два через CPU. Сценарий 9 проверяет saturation и опустошение очереди ёмкостью один. Сценарии 10–11 выполняют исходный production counter-service с точным 16-byte контрактом Add/Get и одним либо двумя cross-CPU отправителями.

Root CPU0 обращается к responder slot1 CPU0 для same-CPU транспорта либо slot0 CPU1 для cross-CPU транспорта/сервиса. Root и slot1 получают отдельно выданные SEND capabilities к slot0 для двух отправителей. Числовое имя, сообщённое владеющему им процессу, не передаёт полномочия между namespaces. Новые deployment inputs, расширение grants, принудительное rendezvous ядра и копии production-сервиса запрещены. Same-CPU contention, обратные направления CPU, same-CPU counter и выполнение production supervisor не входят в покрытие.

Внешний транспортный responder проверяет объявленные байты запроса и возвращает sequence исполнения. Успешные запросы нулевой длины требуют такого же независимого server count/sequence ledger, как непустые. Production counter начинается с нуля; успешные результаты Add(1) должны быть уникальны и согласованы с конечным Get и общим числом успешных сложений обоих клиентов. Отказы не увеличивают число полезных успехов. Неверный результат, несовпадение identity, неожиданный expiry, незавершённый Collect, нештатное завершение или неуспешный reclaim отклоняют correctness. Наблюдения сопровождаются source review и сохранением исходных ELF; host oracle не защищает от фальсификации всей цепочки evidence.

## Время и исходы

Каждая попытка использует неизменённую policy запроса SDK: проверенное сложение `start.ticks + start.frequency`, один интервал частоты счётчика. Это deadline клиента, а не timeout синхронизации ядра, допустимая latency или policy retry. Для каждой предложенной попытки сохраняются client/request identity, фаза warmup/measured, исходные start/end, stage/status результата, дельты частичного окна исполнения и READ_WINDOW, полезный результат. Ошибки и незавершённые попытки сохраняются; замена, retry, выборочное исключение и синтетические успешные samples запрещены.

Три общих вызова CLOCK наблюдают состояние до Submit, сразу после Submit и после terminal Collect либо отказа admission. Измеряемый интервал — внешний userspace envelope от submit до получения результата/отказа. Он включает probes и scheduling effects и не является точным внутренним временем admission-to-terminal. Проверенные разности и остаток не должны давать underflow. CLOCK x2 — частичное окно исполнения клиента, а не exclusive CPU или стоимость сервиса. x3 учитывает READ_WINDOW, а не IPC service. Queue residence, locks, capability-validation cost, динамические copy counts, пики памяти, switches отдельного запроса и PMU cycles недоступны без отдельных attributable evidence. DEV и PROD остаются разными классами; QEMU не устанавливает физический performance.

Saturation ёмкостью один использует обычный feedback IPC. Responder ждёт ответа root; root допускает A и требует Exhausted от B, затем освобождает responder, собирает A и проверяет последующий C. Интервал A намеренно включает probes/отказ B и feedback-release RPC. A, отказанный B и recovery C публикуются отдельно; A не является независимым обычным round-trip. Ожидаемый Exhausted подтверждает correctness и даёт нулевой вклад в полезный throughput. Два клиента не доказывают одновременную работу допущенных запросов в ядре: пересечения измеренных cross-client envelopes и отказы очереди публикуются отдельно, включая наблюдаемую сериализацию.

## Семейства метрик и observer pairs

Семейство request-envelope сохраняет все предложенные измеряемые интервалы, включая отказы с точными исходами. Successful latency и rejection latency — отдельно обозначенные условные сводки; их нельзя усекать до одинаковой длины или выдавать за полностью успешную популяцию. Публикуются counts, nearest-rank median/p95/p99, mean, выборочные variance и standard deviation, когда они определены. Nearest rank задаётся `sorted[ceil(p*n)-1]`. Пустые либо недостаточные популяции явно unavailable/inconclusive; p99 при не более 32 успешных samples выбирает максимум и не доказывает точность хвоста. Неоднородная сводка успешных A/C saturation не заменяет отдельные распределения.

Throughput — отдельное семейство метрик с одним агрегатным наблюдением на boot и нулём агрегатных warmup observations. Это не отменяет conditioning запросов: сохраняются четыре warmup и 32 measured offered requests либо три warmup и 33 measured offers для saturation. Общая фаза начинается после warmup обоих отправителей до peer GO; заканчивается после peer DONE либо последнего root sample при одном отправителе, до массового экспорта raw. GO/DONE control overhead входит в интервал. Rate равен числу полезных успехов measured-фазы, умноженному на записанную частоту и делённому на общие elapsed ticks. Сохраняются числитель, знаменатель и все исходы. Нельзя обращать median latency или складывать перекрывающиеся интервалы клиентов. Нулевое elapsed time недопустимо; отсутствие успехов даёт наблюдаемый нулевой rate, а не выдуманный latency sample.

OFF и ON выполняют те же предложенные операции, три CLOCK и проверки корректности. ON добавляет ровно одну volatile-запись промежуточных CLOCK ticks в private память actor до конечного CLOCK. Парные observer arrays содержат каждый offered measured envelope с request/phase identity и исходом; они не фильтруются по успеху и не сокращаются до одинакового количества успехов. Если outcome vectors различаются, интерпретация пары observer помечается INCONCLUSIVE; различия условных популяций нельзя интерпретировать как стоимость recorder. Обе полные популяции сохраняются в любом случае. Даже при совпадении исходов сравнивается дополнительное поведение recorder под scheduling, а не все timestamp probes или изолированная стоимость CPU. Точную corrected latency не вычитают. OFF — основная серия операций.

## Фиксированная основная кампания и неопределённость

После review pilot основная кампания содержит двенадцать свежих пар для каждого case/profile: 576 новых boots. Внешний цикл — pair index 0–11, следующий — case 0–11, затем DEV и PROD; для чётных pair indices порядок OFF/ON, для нечётных ON/OFF. Получаются шесть пар каждого порядка на case/profile. Исторические pilot runs не заполняют slots основной кампании. Stopping rule — фиксированный план, а не наблюдаемый performance; все предпринятые ошибки сохраняются, незавершённые slots отмечаются. Изменение исходников во время кампании нарушает её source-bound применимость. Ревизия реализации сама по себе не создаёт новый comparison class; изменения конфигурации, функциональных гарантий, нагрузки, безопасности или методологии создают его.

Каждый boot предлагает всего 36 запросов. Обычные сценарии сохраняют четыре warmup и 32 measured offers; при двух отправителях каждый даёт два warmup и шестнадцать measured offers. Saturation содержит двенадцать циклов по три операции, первый — warmup. Это finite-session conditioning protocol, а не доказательство steady-state stabilization. Stack actor, endpoint и report capacity не увеличиваются. Отчёт содержит 128 header words и восемь слов на наблюдение, всего 416 слов, используя существующие 2048 слов хранилища и события по 64 слова. Массовый экспорт peer и итоговая отчётность находятся вне индивидуальных timing intervals и объявленного общего интервала. Отсутствующие или неверные chunks нарушают полноту.

Единицы анализа — свежие boots/pairs; запросы одного boot могут коррелировать. Для каждого case/profile сохраняются все двенадцать парных разностей и log ON/OFF ratios per-boot medians и achieved rates, когда они определены. Публикуются median парной статистики и order-statistic interval от третьего до десятого отсортированного парного значения; для ratios границы экспоненцируются. При независимых одинаково распределённых непрерывных парных наблюдениях покрытие равно `1 - 2*(1+12+66)/4096 = 0.96142578125`. Это консервативный marginal 95% interval для median парного эффекта, а не mean effect, одновременное покрытие всех scenarios или гарантия точности. Нулевые знаменатели, отсутствующие пары и разные observer outcomes делают соответствующий ratio/интерпретацию unavailable либо inconclusive; нельзя заменять пары ради двенадцати пригодных значений.

Двенадцать пар — ограниченный дизайн оценки неопределённости, а не performance allowance или гарантия мощности. Fresh boot не обеспечивает independence или exchangeability при host drift. Публикуются pair index, порядок и все исходы; предположения раскрываются, выводы остаются условными при отсутствии подтверждения. Нельзя усреднять per-run p99 и называть результат pooled quantile. Объединённые описания не превращают коррелирующие запросы в независимые tail evidence. Regression margin, решение о превосходстве и изменение stopping rule нельзя придумывать после наблюдения результатов.

## Admission и применимость

Используются существующие Arena schema, versioned profiles, registry и assessor. Этот стабильный контракт, объявленные workload/input/resource/environment definitions и oracle фиксируются отдельно от изменяемых acceptance documents. Точные implementation/image/source identities относятся к provenance запуска. Изменения functional/workload/security/methodology задают новые comparison classes. DEV/PROD и QEMU/hardware популяции нельзя смешивать.

Реальное SEC evidence должно покрывать объявленные native-request requirements: caller identity, authority attenuation, stale capabilities, isolation, bounded charging и проверенную revoke semantics. Отсутствие либо отказ обязательного correctness или SAR evidence запрещает qualifying admission; ожидаемый отказ ёмкости не отменяет gates. Structural admission не означает аутентифицированное выполнение, достаточную статистическую мощность или eligible record. Этот контракт не даёт campaign PASS, source-bound acceptance, hardware result и не меняет record eligibility. Текущая реализация и evidence ведутся отдельно в [IPC passport](ipc-passport.md).

[Английский оригинал](../../../../docs/research/ipc-measurement-contract.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.research.ipc-measurement-contract",
  "kind": "subsystem-contract",
  "summary": "Fixed IPC measurement populations, useful-result oracles, observer pairing and bounded uncertainty.",
  "units": [
    {
      "id": "kolvrt.arena.ipc-measurement.contract",
      "anchor": "kolvrt-arena-ipc-measurement-contract",
      "kind": "contract-section",
      "summary": "Normative external IPC and production-counter measurement contract; no execution acceptance.",
      "depends_on": [
        "kolvrt.ipc.request",
        "kolvrt.clock.query.api",
        "kolvrt.apps.native-elf",
        "doc.kolvrt.architecture.benchmarking",
        "doc.kolvrt.arena.measurement-contract"
      ]
    }
  ]
}
```
