# Изолированный supervisor EL0

Document status: CURRENT

Evidence scope: исторически принятая bounded Phase 3.6 после #126; Phase 3.7 меняет source и coordination policy. Сохранённый receipt Phase 3.6 устарел для изменённых исходников; bounded acceptance Phase 3.7 записана в [native-приложениях](native-applications.md). Readiness и performance отдельны.

Current reference: [Предложение lifecycle rendezvous](../architecture-decisions/0026-el0-supervision.md)

ABI contract: native.lifecycle/1
Publication stage: EXPERIMENTAL
ABI-FREEZE: none

<a name="isolated-el0-supervision"></a>

## Ограниченный жизненный цикл сервисов

Supervisor — реальный изолированный образ EL0. Он выбирает порядок запуска, проверяет зависимости, получает readiness через native.request/1, наблюдает точное завершение сервиса, выбирает свежие instances, ограничивает попытки restart, применяет backoff по counter clock и прекращает admission перед shutdown. EL1 обеспечивает provenance, конечные разрешения на образы/placement/квоты, создание приватных процессов, native IPC bindings и фактическое освобождение owner/root. Он не интерпретирует политику зависимостей, readiness, restart или discovery. Compatibility, routing, package manager и advisor не входят в native dependency closure.

Development manifest выбирает worker на CPU1 и здорового peer на CPU0; supervisor имеет собственное адресное пространство CPU0. Peer должен завершить аутентифицированный readiness probe перед запуском зависимого worker. Запись manifest с отсутствующей readiness dependency не запускается. Это конечный статический workload, а не production package format или реализация persistent service.

## Bootstrap и время жизни authority

Trusted native bootstrap устанавливает ровно один Scope, связанный с точным ProcessId supervisor и неизменяемыми linked images, целевыми CPU, квотами каждого instance и общими instance credits. У каждого сервиса четыре handle slots, один endpoint, один queued request и два retained request charges; supervisor имеет два endpoint slots для feedback каждого instance. Сервис получает только SEND authority к собственному feedback endpoint, receiver которого принадлежит supervisor; память — private raw-image space. Worker допускает максимум восемь instantiations, peer — одну. Третий single-instance grant остаётся неиспользованным из-за отсутствующей зависимости manifest EL0; seal прекращает этот grant, последующий launch отвергается. Native authority ceiling и restart policy EL0 независимы. Переданные image pointer, manifest, principal, quota или placement не попадают в lifecycle entry. Сами числовые selectors и tokens не дают authority.

SVC 0xb0 использует инициализированные регистры: x0 — операция, x1 — grant selector, x2 — точный service token, x3 — opaque initial service argument, x4 — version 1. Unsupported versions отвергаются до command publication. Операции: initial launch 1, exact completion observation 2, replacement 3, stop admission и terminate 4, irreversible bootstrap seal 5. Авторизованные checkpoint replies инициализируют x0–x4: status, fresh token/event, caller-local SEND handle/detail, caller-local feedback RECEIVE handle/detail и process generation. Вне checkpoint mode entry возвращает denial в x0 и сохраняет остальные caller registers. Кодирование экспериментальное, без ABI freeze и обещаний публичной поддержки. Неизвестные операции и неверные unused fields явно отвергаются.

Native entry сравнивает исполняемый Task с установленной identity supervisor. Child или reused namespace не может стать supervisor, передав identifier. Успешный launch расходует instance credit, выдаёт свежий nonwrapping token и свежие sender/receiver bindings. Replacement требует точного current token и фактического завершения прежнего процесса; granted image, placement и quotas сохраняются. Старые process identities, instance tokens и закрытые SEND handles не перенаправляются на replacement. При неудачной publication unpublished namespace завершается, private frames откатываются; runnable partial instance не публикуется.

Seal уничтожает unused initial launch grants и запрещает повторный initial launch. Уже выданные service capabilities отдельно сохраняют явно учтённые remaining instance credits до Scope retirement. Seal не пополняет credits; replacement single-credit peer отвергается даже тогда, когда peer здоров. Retirement отключает native entry; bootstrap нельзя повторно установить в пределах этой загрузки. Static linked-image и manifest binding обеспечивает только test/development assurance. Production authorization требует в #38 связать exact authorized images, manifest, grants и актуальное trust/freshness decision; acceptance production trust chain, reproducibility или anti-rollback не заявляется.

Completion observation возвращает event 0 для admitted live instance, 1 для Exited(code), 2 для Faulted(class,address), 3 для explicit Terminated и 4 для BudgetExpired. Unknown/stale identity — ошибка, а не выдуманный live status. Принудительный stop не выдаётся за budget expiration.

## Исполнение и освобождение

Workload expiry определяет только BudgetExpired. Публикация completion, drainage pending-copy и quiescence source/ack не имеют произвольного ограничения по прошедшему времени. CPU0 удерживает Registry ownership, namespaces и frames, пока не приобретены оба completion, не отсоединены roots и не освобождены execution owners; pending copies завершаются до namespace transfer. Общий SMP wait также удерживает фактическое состояние boot, rendezvous, retirement и shutdown до выполнения требуемых предикатов. Опубликованный secondary FAILED по-прежнему вызывает fail-stop с удержанием ресурсов. Конечного внутрядерного детектора молча зависшего CPU в этой foundation нет: внешний watchdog теста диагностирует зависание, но не доказывает completion и не разрешает reclaim. Эта явная policy Phase 3.7 заменяет фиксированные coordination deadlines по требованию maintainer; workload deadlines и длительности policy supervisor остаются отдельными.

Исторический checkpoint publication control #126 удерживал CPU1 после quiescence и требовал CheckpointPublicationHeld вместе с CompletionPublicationTimeout. Эта mutation и принудительный rendezvous удалены. Текущий completion-publication-state-inputs вызывает единственный production Ownership protocol с запрещённым completion при borrow/отсутствии quiescence и преждевременным inspection; проверяет реальный отказ и сохранённые phase/generation. Это UNIT coverage не устанавливает SYSTEM liveness при удержании publication или детектор молча отказавшего CPU. Исторические receipts сохраняют исходные source и control scope.

Чтение counter разделяет ticks_relaxed() для scheduler workload-deadline polling и ticks_ordered() для ordering относительно observations/context. Существующие callers ticks() сохраняют ordered semantics для measurements и accounting; ISB при trap entry сохранены. Performance results ограничены измеренными QEMU scheduler workloads, с сохранением failures и variability; из них не следует утверждение о physical performance или отсутствии regression.

Lifecycle work выполняется на явном fixed-affinity checkpoint двух CPU. Каждый owner исполняет bounded timer quantum, terminal transition, block или explicit lifecycle yield, восстанавливает native root и освобождает execution ownership. Затем CPU0 приобретает оба completion перед изменением Registry или allocation/reclaim frames. Pending copy transactions завершаются до возвращения namespaces. Saved IPC wait identities, retry reasons и counters сохраняются через barrier и восстанавливаются только для того же ProcessId; endpoint и source/mailbox ownership удерживаются. Отсоединение root blocked процесса само по себе не разрешает reclaim.

SGI-only entry не расходует checkpoint quantum до первой инструкции EL0; deferred work дренируется, а заведённый timer продолжает обеспечивать progress. Scheduling cursor продолжается только для соответствующей живой process generation. Death/stop очищает blocked ownership точного процесса, прекращает admission и выполняет ASID retirement на owning CPU. Old endpoint requests, terminal results и wake acknowledgements сохраняют независимое ownership до drainage. Reclamation не основывается на истекшем grace period.

Этот bounded checkpoint приостанавливает здоровых peers на время lifecycle changes; он не добавляет migration или independent admission во время исполнения user code на CPU. Death observation использует exact lifecycle queries; generic wait-any и автономная persistent orchestration исключены. Исходный continuous IPC path имеет отдельный scope и проверки. Readiness, effects и failure policy не выводятся из timeout или остановленного CPU.

Исторический cancellation fixture использовал длительность readiness probe для независимых load и commit-ack запросов. Expiry мог сделать любой из них terminal до явного Cancel, для которого AlreadyTerminal является правильным production response. Текущий fixture задаёт максимальный допустимый absolute deadline этим двум тестовым входам, сохраняя проверки настоящего commit acknowledgement, явной cancellation и EffectUnknown. Zero уже expired по native IPC contract. Readiness probes сохраняют canonical deadline, restart сохраняет backoff 1/128 секунды. Исправляется предпосылка теста; production arbiter и performance policy не меняются. [Native-приложения](native-applications.md) добавляют внешний SYSTEM-сценарий настоящих production supervisor/service/client после выбранного crash сервиса.

Исторический CI b037833 упал в ordinary suite, вызванном process-stale-control: Exited(1000), coverage 0, actual status 16. Это не отказ process-stale assertion. Текущий diagnostic fixture сохраняет readiness deadline 1/8 секунды и задаёт первым peer и worker probes stage IDs 1010 и 1020. Operation IDs различают Submit (1), Wait (2), Collect (3) и reply validation (4); при отказе записываются clock при создании запроса, absolute deadline, clock после отказа, frequency и исходный IPC status. Проверка production adapter показывает, что Expired status 16 возвращается при Submit admission, тогда как Wait/Collect сообщают terminal outcomes отдельно. Это сужает расследование, но не доказывает attribution scheduler latency. Один зелёный локальный повтор не устранял тот исторический отказ CI. Более поздняя bounded acceptance записана в [native-приложениях](native-applications.md); она не заявляет универсальное исправление readiness expiry и не обосновывает увеличение readiness deadline.

## Политика readiness, restart и shutdown

Политика EL0 отправляет readiness probe точному endpoint с absolute counter deadline 1/8 секунды и признаёт READY только после completed initialized response с ожидаемым marker. Commit/reply доступен только bound receiver сервиса и accepted service token; requester SEND handle не может подделать ответ. Expired probe не означает READY. Эти длительности QEMU fixture — явные параметры политики, а не real-time guarantees.

Committed crashing service сообщает effect-unknown и exact fault observation. Supervisor запускает fresh instance и проверяет отказ предыдущего token и SEND handle. Silent service получает timeout до commitment и явно останавливается. Crash storm получает три выбранные EL0 replacement attempts с backoff 1/128 секунды по counter clock; затем supervisor прекращает retries, хотя native authority credits ещё остаются. Здоровый peer завершает native probes до и после bounded crash storm.

Сервис отправляет подтверждение commit через feedback endpoint своего точного instance только после успешного COMMIT. Supervisor проверяет и отвечает на подтверждение до cancellation; прошедшие scheduler quanta не доказывают commitment. При accepted load supervisor наблюдает cancellation после commitment как effect-unknown, прекращает admission, запрашивает owner-local termination и ждёт точного terminal event. Он не обещает rollback и не повторяет операции с неизвестным эффектом. Final teardown закрывает все endpoints, завершает pending copies, дренирует source/mailbox ownership, освобождает namespaces/domains/ASIDs и private frames. Внешний test watchdog десять секунд диагностирует неудачный workload; он не доказывает успешный shutdown и не является production policy. Неожиданный отказ supervisor, watchdog expiry или failed quiescence останавливает/карантинирует development fixture без reclaim live resources; deadline никогда не разрешает освобождать reachable state.

## Проверка и следующий gate

[Историческая проверка приёмки Phase 3.6](../architecture/supervision-phase36-acceptance-review.md) выполнена относительно merged main ddd526cc5a522d97029d0324e6e619f5fd6ec6e5 после #126. Codex повторно проверил полные architecture/code и EN/RU semantic requirements, включая обязательный publication bound для checkpoint(None), отдельный copy-drain bound и различие relaxed/ordered counter reads. Functional implementation — BOUNDED_IMPLEMENTED. Readiness остаётся NOT_READY, без введения нового числового performance gate и без запрета использования функциональной основы в #28.

[Исторический receipt точных исходников #126](../../../../research/results/supervision-phase36-main126.json) независимо проверяет CI run 37439023443 на final #126 head cceb17e74a7a249d76d0f10905de23205f69b1dd: все source digests совпадают, проверены фактические ELF/result hashes и полный inventory 142 задач — 125 checks в каждом профиле, оба ordinary boots, 138 controls. Тот inventory содержал десять supervision controls. Его DEV/PROD publication controls требовали одновременно CheckpointPublicationHeld с quiescent=true и CompletionPublicationTimeout; чужой panic не засчитывался. Publication mutation и фиксированные coordination bounds с тех пор удалены, как описано выше; этот receipt не подтверждает текущие исходники.

[Старый receipt](../../../../research/results/supervision-phase36-current.json) и [предложение до #126](../architecture/supervision-phase36-pre126-review.md) сохранены только для исторического scope 9cf87bc/692b024. Они не подтверждают текущие исходники.

[Исходные performance observations](../../../../research/results/checkpoint-publication-bound.json) показывают median +1.57% с tagged ASID и +6.64% с ASID-zero. Sequential before/after runs при uncontrolled host load объединяют lifecycle correction и counter refinement; attribution к ISB и отсутствие регрессии не доказаны. Строгий первоначальный PROD IPC benchmark также отказал на blocked_requester_wait с block_delta=0. Failure сохранён и control не ослаблен. Performance acceptance не заявляется; это отдельная открытая область, не новый универсальный процентный threshold.

Восемь real EL0 scenario groups проверяют ordered startup, dependency/authority denial, finite grants/seal, forged readiness, crash/fresh restart/stale binding, startup timeout, bounded storm и committed-load shutdown. Bitmap 255 дополнен actual completion, released CPU owners, zero live processes/domains и restored physical pages. Host tests не заменяют EL0; QEMU не является physical ARM. Старые timer/stripped-PROD failures остаются в истории; universal race-freedom не заявляется. #28 — persistent service integration; #38 — production bootstrap trust.

Исходники: [lifecycle mechanism](../../../../crates/kernel/src/supervision.rs), [EL0 images](../../../../crates/kernel/src/supervision_workload.S), [bootstrap fixture](../../../../crates/kernel/src/supervision_workload.rs), [process ownership](../../../../crates/kernel/src/process.rs), [checkpoint](../../../../crates/kernel/src/scheduler/mod.rs).

[English original](../../../../docs/kernel/supervision.md)

## Граница исходников при интеграции CI

Исторический inventory #126 содержал 142 задачи и 138 controls. Принятый [receipt native-приложений](../../../../research/results/native-phase37-43af402.json) Phase 3.7 содержит 144 задачи и 140 control slots с классификацией фактического coverage; [обновлённая проверка](../architecture/native-phase37-fixture-review.md) фиксирует его исходники и scope приёмки. Publication failure не возвращает фиктивный success: resources остаются retained до acquired quiescence. Historical inventories не переписываются. Performance и readiness остаются отдельными от functional acceptance.

Phase 3.7 расширяет механизм immutable image-format grants и operation 6 для одной конечной client SEND edge. Инициализированные x0=6, x1=client selector, x2=точный client token, x3=точный destination token, x4=1 обрабатываются только для захваченного supervisor после acquired quiescence. Успех возвращает status, SEND handle в локальном namespace клиента, target token и client generation; заменяется только previous sender, выданный этой entry. Stale tokens, завершённые clients/targets и отсутствующие edges отвергаются. Этот профиль исходных ELF описан в [native applications](native-applications.md); прежние ограничения fixtures Phase 3.6 и исторические receipts сохраняют свой исходный scope.

Обычный ELF supervisor выбирает время жизни через application SessionPolicy: bootstrap argument 1 сохраняет persistent работу после клиента, argument 2 завершает сессию после настоящего завершения клиента. Недопустимая конфигурация отвергается. Оба режима используют одну production реализацию service/client/recovery. FinishAfterClient закрывает admission, наблюдает успешный client exit, запрашивает точное owner-local termination сервиса, наблюдает Terminated и выходит штатно; затем generic kernel retirement требует настоящую quiescence и освобождение ресурсов. Политика времени жизни приложения не является kernel test switch. Immutable ELF bootstrap больше не содержит workload watchdog; внешний runner ограничивает diagnostic hangs. Отдельный исторический assembly fixture сохраняет явно ограниченные policy/watchdog и readiness deadline 1/8.

Bootstrap grant shape теперь имеет единственную production реализацию в kernel_core::supervision. Kernel Scope::install использует общий тип Grant и вызывает validate_grants до публикации ACTIVE ownership. Быстрые host component tests импортируют ту же функцию и проверяют непустой image input, существующие границы instance credits 1..8 и отказ self/missing SEND destinations. Это проверка immutable bootstrap authority shape, а не интерпретатор EL0 dependency policy или generic package manifest format. Проверки image format/geometry и process/domain quota/placement остаются в настоящих ELF/process/domain методах, проверяемых их component tests; успех grant-shape не доказывает format validity или caller provenance.

CI точного 3873bc8 наблюдал worker readiness stage 1020, Submit operation 1, Expired status 16 до admission. submitted_at=1078757950, deadline=1086570450, failed_at=1086709243 и frequency=62500000 показывают 127.22 ms между clocks против policy 125 ms; запрос не достиг сервиса. Это не устанавливает service reply latency или kernel performance defect. Diagnostics теперь сохраняют public CLOCK residency counters на обоих samples, чтобы отделять caller execution-window counter intervals от остального elapsed time; эти counters не локализуют off-CPU причину. Принятый deadline /8, строгое требование READY и production scheduling остаются неизменными до attribution.

## Диагностика #133 без изменения production deadline

Fixture теперь различает readiness нового replacement (stage 1030), peer после replacement (1040), readiness во время restart storm (1050), peer после storm (1060) и committed load (1005). payload_operation отделяет payload 1/2/3 от номера операции диагностики Submit/Wait/Collect; probe_timing_valid помечает наличие применимых clock samples. Диагностические регистры инициализированы; lifecycle failure не выдаётся за IPC probe. Прямой путь запуска по manifest также сбрасывает прежние маркеры payload и operation, сохраняя результат сравнения зависимостей. CLOCK residency — сумма интервалов архитектурного счётчика в EL0 execution windows, а не host CPU time. Разница с elapsed time сама по себе не разделяет kernel, checkpoint и host scheduling.

Детерминированные UNIT-проверки вызывают настоящий Endpoint::submit на границах deadline и проверяют сохранение ресурсов при отказе. Внешний Windows harness tests/system/supervision-host-stall.cjs с hold-qemu-thread.ps1 приостанавливает принадлежащий его QEMU host thread CPU0 на 500 ms. [Сохранённый диагностический запуск](../../../../research/results/supervision-readiness-host-stall.json) воспроизвёл Expired до admission на stage 1030, coverage 15, с прежним /8 и неизменным hash образа. Это доказывает возможность такого результата при host starvation, но не причину исходного CI failure. Harness сохраняет исходный диагностический отказ вместе со всеми ошибками cleanup в AggregateError, пытается выполнить всю очистку принадлежащих ему процессов и публикует receipt только после успешной очистки и проверки неизменности входов. Production-код, deadline, retry policy и обязательные assertions не изменены; эксперимент не является успешным readiness или приёмкой regression. Владелец закрыл #133, а master #37 закрыт после прохождения всех обязательных проверок PR #137 на 7de095b и [merged main bc8222a](https://github.com/lifeFedorovAlexey/KOLVRT/actions/runs/37688334584). Исходная причина задержки остаётся UNKNOWN; disposition владельца и последующие успешные проверки не устанавливают историческую причинность и не обновляют receipts для более поздних изменений исходников.

[Повторная диагностика после ревью](../../../../research/results/supervision-readiness-host-stall-review.json) сохраняет отдельный результат для исправленных маркеров прямого запуска и очистки. На head PR 4e37987 [CI 37584043283](https://github.com/lifeFedorovAlexey/KOLVRT/actions/runs/37584043283) прошёл все четыре matrix shard и обязательные задания foundation; это не устанавливает причину предыдущего отказа readiness. Пассивная запись планирования хоста локально недоступна: Windows отклонила профилирование WPR CPU с ошибкой 0xc5585011. WPR не записал трассу; из этого отказа не выводится причина задержки планирования хоста.

## Использование evidence matrix

B5 issue #109 меняет только потребление evidence эквивалентного обычного suite на host. Receipts выполнения schema 2 сохраняют source plans schema 1 и все 144 обязательные проверки. Последующий consumer может использовать более ранний executed ordinary donor только из того же вызова и shard при совпадении source/profile/features/ELF/QEMU identities; каждый обязательный именованный assertion независимо проверяется по сохранённым events. Fatal controls, другие profiles и сборки, исторические результаты и цепочки reuse исключены. KOLVRT_MATRIX_REUSE=off заново исполняет reference. Это не меняет readiness policy supervisor, restart backoff, acquired quiescence или reclamation. Существующие receipts сохраняют исторический scope; это изменение runner не повышает verification владельца и не даёт performance или readiness acceptance. [Контракт CI reuse](../ci/performance.md) определяет provenance и ожидаемые counts.

## Зависимость от обнаружения устройств

При загрузке Phase 4.0 адрес консоли берётся из неизменяемого проверенного [дескриптора PL011](devices.md) с явным резервированием для загрузочной консоли. Это публикует идентичность наблюдения, а не MMIO/IRQ grant приложения. Поэтому выполнение native-приложений, IPC и supervision сохраняет существующее владение консолью; привязка драйвера или доступ к устройству не добавляются к их полномочиям. Runner требует ровно одно проверенное событие инвентаризации для каждого свежего выполнения с машинным evidence; исторический reader отдельно сохраняет возможность чтения старых свидетельств без этого события. Предыдущие свидетельства feature сохраняют свою область исходников и не подтверждают текущую проверку устройств.

Регрессионная проверка кванта процесса допускает ноль в первом снимке счётчика, когда pending timer IRQ предшествует первой инструкции участника. Она по-прежнему требует последующего роста счётчиков на обоих CPU, сохранности контекста, прогресса соседнего процесса до завершения spinner, неизменного бюджета slices и итогового reclaim. Это исправляет неверное предположение теста; политика планирования и runtime deadlines не меняются.

## Интеграция пилота CLOCK

Пилот CLOCK выбирает внешний ABI root через общий сборщик оригинальных ELF и не заявляет покрытие production-supervisor. Обычные native-команды сохраняют свои валидаторы supervisor и crash-recovery. Изменение выбора oracle в общем executor сохраняет обработку ошибок, свежее наблюдение устройства и существующие watchdogs; deadlines supervision, restart backoff и политика quiescence не меняются.

## Интеграция issue 33

Инструменты IPC issue #33 используют существующие неизменяемые selectors и SEND bindings; новая deployment-конфигурация и дополнительные lifecycle-полномочия не вводятся. Измерением управляет внешний root, который не выполняет production supervisor. Существующие policy readiness, restart, quiescence и отказов сохраняются. [Scope IPC pilot](../research/ipc-passport.md) отделяет работу настоящего counter-service от покрытия supervisor.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.supervision",
  "kind": "subsystem-contract",
  "summary": "Изолированный supervisor EL0 с ограниченным lifecycle rendezvous и точной authority.",
  "units": [
    {
      "id": "kolvrt.services.supervision",
      "anchor": "isolated-el0-supervision",
      "kind": "feature",
      "summary": "Readiness EL0, конечный restart и shutdown поверх privileged lifecycle mechanisms.",
      "tags": ["supervisor", "service", "lifecycle", "bootstrap", "el0"],
      "depends_on": [
        "kolvrt.ipc.transport",
        "kolvrt.process.lifecycle",
        "kolvrt.security.domains",
        "law.013",
        "law.025",
        "law.043",
        "adr.0026"
      ],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "Real isolated EL0 supervisor and static service images; exact-authority lifecycle rendezvous with actual acquired completion, root/owner quiescence and copy/source/ack drainage. Phase 3.7 removes arbitrary coordination cutoffs at the maintainer request; published CPU failure retains resources, and silent stalls require external diagnosis. Ordered readiness, fresh replacement, finite backoff/restart policy and under-load shutdown remain separate application policies. Immutable bootstrap grant shape has one shared kernel-core production type/validator imported directly by host component tests.",
        "sources": [
          "crates/kernel-core/src/process.rs",
          "crates/kernel-core/tests/process_protocol.rs",
          "crates/kernel/Cargo.toml",
          "crates/kernel/src/main.rs",
          "crates/kernel/src/process.rs",
          "crates/kernel/src/arch/aarch64/mod.rs",
          "crates/kernel/src/smp.rs",
          "crates/kernel/src/scheduler/mod.rs",
          "crates/kernel/src/scheduler/task.rs",
          "crates/kernel/src/supervision.rs",
          "crates/kernel/src/supervision_workload.S",
          "crates/kernel/src/supervision_workload.rs",
          "crates/kernel/src/tests.rs",
          "crates/xtask/src/main.rs",
          "crates/xtask/src/matrix.rs",
          "crates/xtask/src/output.rs",
          "crates/xtask/src/timing.rs",
          "crates/kernel-core/src/lib.rs",
          "crates/kernel-core/src/supervision.rs",
          "crates/kernel-core/tests/bootstrap_grants.rs"
        ],
        "acceptance": ["research/results/supervision-phase36-main126.json"],
        "issues": [27],
        "adrs": ["adr.0026"],
        "limitations": [
          "Development static manifest/image assurance only; production trust remains #38.",
          "Fixed affinity and two-CPU lifecycle barrier; generic wait-any, migration, independent live admission and persistent services are excluded.",
          "Codex semantic/EN-RU review completed after #126; performance attribution/admissibility and physical ARM/production acceptance remain open."
        ],
        "next_gate": "Phase 3.7/#28 bounded persistent ELF integration is accepted under docs/kernel/native-applications.md. Performance readiness, physical ARM and production trust remain separate review scopes.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "STALE",
            "reason": "The retained Phase 3.6/#126 receipt does not match the changed coordination policy and test architecture, so this owner-specific verification remains STALE. Bounded Phase 3.7 production supervisor crash/recovery/shutdown and acceptance are separately verified under docs/kernel/native-applications.md; historical receipts are not relabeled.",
            "receipt": "research/results/supervision-phase36-main126.json",
            "receipt_sha256": "52a48b8246f516b7d393c1cecb6a7a0a58bac5a9af31822cda15390747c2907f",
            "scope": "Real isolated EL0 supervisor and static worker/peer images on two fixed-affinity QEMU CPUs; ordered readiness, unused grant extinction, finite credits, version rejection, fresh restart, timeout, storm, truthful Terminated/effect-unknown and complete ownership/resource/source drainage. Not physical ARM64 or production trust."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical ARM64 receipt."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Implement isolated EL0 policy with a separately derived lifecycle barrier; acceptance review remains pending.",
            "acceptance": []
          },
          {
            "from": "EXPERIMENTAL",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "Post-#126 code/architecture and full EN/RU semantic review with independent current-source 142-task functional verification; readiness/performance acceptance remain separate.",
            "acceptance": ["research/results/supervision-phase36-main126.json"]
          }
        ]
      }
    }
  ]
}
```
