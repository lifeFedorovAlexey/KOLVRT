# ADR-0015 — Версионированная маршрутизация в изолированных EL0 consumers

Status: **Accepted для ограниченной демонстрации Phase 2**. Date: 2026-10-02.

## Контекст

Разрешённая работа Phase 2 требует одновременных native и versioned adapters, удаляемой compatibility, настоящего преобразования и измерений. [SMP](../kernel/smp.md) и [EL0](../kernel/el0.md) теперь предоставляют основу исполнения. Существующая routing groundwork ещё не была подключена к native execution. [ADR-0013](0013-security-boundaries.md) запрещает обосновывать привилегированное размещение только удобством, глобальным владением или скоростью.

## Решение

Routing, version decoding, legacy layouts, switching policy и adapter counters остаются в отдельно собираемом EL0-образе, выбранном при boot. Native kernel Cargo closure содержит только kernel-core. Native capability предоставляет immutable snapshot защищённой timer-service истории текущего task и reduction проверенного half-open window. Identity устанавливает текущая runqueue; операции не принимают чужой task ID, root, pointer или authority-bearing handle. Capture идемпотентен в пределах lifetime этого task.

Привилегированный механизм ограничен установлением caller identity и доступом к kernel-owned наблюдениям исполнения, созданным timer handler. EL0 не может самостоятельно подтвердить эти наблюдения или прочитать защищённое task storage. Арифметика исполняется в EL0 общим безопасным native window алгоритмом; это не основание переносить общий reduction service, routing policy или authorization database в EL1. Allocation и static task creation сохраняют проверенный bootstrap scope. Перед общей service/capability model необходимо пересмотреть placement и заменить неявный own-task доступ её native grant contract.

Два обычных adapters преобразуют inclusive LE16 endpoints с empty sentinel и BE32 start/count в единственный current native span. Отдельно выбранный bug adapter сохраняет только безопасное synthetic zero-count-as-one behavior с обязательными native bounds и caller scope. Он не восстанавливает security defect и не заявляет Linux compatibility.

Каждый EL0 consumer эксклюзивно владеет route, generation и transaction state. Синхронная transaction удерживает binding. DEV rebinding разрешён только после завершения guard; забытый guard оставляет admission заблокированным. Разные consumers исполняются на обоих CPU. Общей mutable routing table, предположения о CPU0 writer, routing IRQ work или global compatibility lock нет. Код immutable и остаётся resident до native quiescence и frame reclamation; runtime unloading не поддерживается.

Scheduler заменяет запрет второго session атомарным Idle → Admitted → Running → Done admission protocol. CAS claim отклоняет устаревший secondary poll при reset. CPU0 получает Done обоих CPU через acquire, наблюдает завершённые native-root/TLBI и нулевой active execution, затем делает reset или reclaim. Overlapping batch, migration и shared queue mutation отсутствуют. Четыре guarded stack pages и bounded opaque RX image pages расширяют прежний layout одностраничной fixture.

Profiles фиксируют schema, native contract, generation, exact routes и implementation source identities. DEV создаёт profile и отдельно передаваемый expected digest; build-time и EL0 validation отклоняют invalid или unsupported bindings. SHA-256 обеспечивает integrity, а не signature. Source identities не являются executable signatures: сохранённые manifests отдельно фиксируют точные user image и kernel ELF hashes, compiler, features и emulator. Authentic boot/package signature infrastructure не реализована; trusted build/boot selection — явное допущение.

## Проверка привилегированных обязанностей

[Admission policy](../architecture/kernel-admission-policy.md) применяется к каждой обязанности; [threat model](../security/threat-model.md) фиксирует scope и последствия компрометации.

| Поле                   | Решение для ограниченной Phase 2                                                                                                                                                                                                                                                                                                                          |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Consumer и инвариант   | Реальные EL0 consumers на двух CPU запрашивают только собственные защищённые timer observations. Поддельные width, bounds или выбор owner не должны раскрывать state чужой задачи/ядра.                                                                                                                                                                   |
| Observation mechanism  | Current-task attribution, history timer handler и проверенное чтение frozen capture через регистры. EL0 не читает защищённое task storage; EL0 service всё равно нужен этот узкий query. Read-only published mapping требует нового mapping/lifetime primitive. Арифметика уже вынесена в EL0.                                                            |
| Clock/slices mechanism | Native counter access и защищённый поиск собственного task slice. CLOCK SVC возвращает накопленные тики EL0 residency и тела native `READ_WINDOW` для текущего процесса, частоту счётчика и число его slices. EL0 измеряет разности вокруг `Consumer::call_with()` и записывает их в KVR3. Счётчик native-сервиса охватывает только `native_call` внутри `READ_WINDOW`; общий вход/выход SVC, CLOCK, планирование и прерывания не включены. Прямой EL0 counter access всё ещё требует review управления timer controls. |
| Evidence mechanism | Только с machine-events bounded слова отчёта KVR3 сохраняются в защищённой области процесса и выводятся через protected UART после остановки обоих CPU. Отчёт хранит исходные выборки задержки, EL0 и native-сервиса, счётчики и идентичность процесса. Это test instrumentation, исключённая из stripped PROD. Arbitrary memory и foreign report access не разрешены. |
| Bootstrap mechanism    | Existing page-table/cache/TLBI/context primitives публикуют retained private RX mappings и guarded stacks. Host/EL0 policy выбирает image/profile; kernel не разбирает ELF или legacy data. CAS admission исключает stale reset polls. General loading/allocation policy не допускается.                                                                  |
| Authority              | Native bootstrap task creation разрешает только собственные observations. Caller identity берётся из защищённой runqueue, никогда из user registers. Delegation отсутствует; revocation — завершение task. Повторно используемый static ID создаёт fresh state только после acquired quiescence; user authority reference не переживает предыдущий batch. |
| Lifetime и failure     | CPU с соответствующим индексом меняет task state с masked IRQ; capture immutable. Оба CPU восстанавливают native roots и завершают TLBI до reset/reclamation. Timeout не устанавливает quiescence и не разрешает освобождение executing storage. DMA participation отсутствует.                                                                           |
| TCB и unsafe           | Native scope/bounds/page/context enforcement и trusted boot/compiler входят в integrity TCB. EL0 adapters, арифметика и profile policy находятся вне privileged enforcement. Privileged compromise fatal; adapter faults ограничены private task. Entry и SVC invariants включены в inventory.                                                            |
| Profiles и validation  | DEV/PROD сохраняют scope, initialized replies, width/bounds, page protections и reclamation. Реальные concurrent workloads, malformed calls, profile failures, adapter containment, accounting corruption и native removal checks проверяют записанный scope. Silicon, hostile boot и general availability не проверены.                                  |
| Reconsideration        | Пересмотреть capture/publication, direct counter access, scoped inspection transport и grant/revocation при native IPC/authority slice. Скорость или global-owner argument не расширяют этот механизм.                                                                                                                                                    |

## Альтернативы

Связать adapters с EL1; добавить compatibility hooks в native dispatch; использовать одну shared global route table; закончить весь IPC/service stack перед ограниченной демонстрацией; оставить dispatch только на host.

## Почему отклонены

Adapters и route selection не требуют privileged instruction и увеличили бы integrity TCB. Shared locking добавляет синхронизацию без shared state. Чистое синхронное own-task observation не требует cross-task IPC или transferred capabilities. Host-only calls не доказывают настоящие EL0 dispatch, SMP execution или containment.

## Последствия

Phase 2 — настоящий static vertical slice, а не package resolver, arbitrary ELF loader, personality service или завершённый native IPC slice. PROD фиксирует profile при build/launch и не содержит rebind methods, route search или A/B code. Evidence builds добавляют явно названные conformance/reporting features; stripped PROD image отдельно собирается и запускается. Native tests не меняются и исполняются без optional payload. См. [реализованный контракт](../kernel/routing.md).

## Влияние на compatibility

Старые и новые adapters сосуществуют per consumer. Отсутствующие modules, invalid encodings и backend rejection дают явную ошибку. Silent downgrade, fallback и native semantic branches по adapter identity отсутствуют. Removal проходит через rebuild boundary; нулевые observed calls сами по себе не разрешают unload.

## Влияние на производительность

Сохраняются equivalent nonempty workloads, immutable native snapshots, отмеченные warmup и raw timer samples. Consumers чередуют route order, чтобы выявить влияние порядка. Observed median/p95/p99 — эмпирические значения TCG latency; 128 samples на route не подтверждают tails или статистически установленного победителя. KVR3 отдельно хранит CPU потребителя EL0 и тела `READ_WINDOW`; сумма не включает общие исключения и работу ядра. Source-boundary snapshots/conversions — logical work counts, а не physical-copy counters, а resident pages учитывают только кадры. В тесте нет идентичности установленного состояния или каталога; production attribution и saturation throughput недоступны. Наблюдаемое преимущество compatibility medians требует исследования без artificial penalties.

## Влияние на безопасность

Adapters не расширяют native-authorized effects: доступны только bounded frozen observations текущего task, а EL1 повторяет width/bounds validation. User pointers, writable kernel aliases, IRQ allocations и interrupted locks не добавляются. Reports содержат test observations, а не raw addresses или payloads. DEV reporting ограничен и отсутствует в stripped builds. Это не устанавливает общие grant/revocation policy, inspection authentication, DMA containment или защиту от hostile boot.

## Проверки

QEMU исполняет native/v1/v2/bug consumers одновременно, DEV и PROD profiles, feature-minimal adapter builds, native-only operation, transaction switching и rejection, accounting, profile versions/identity/integrity и controlled bug behavior. Negative controls повреждают profile, вызывают fault одного adapter и искажают accounting report. Host checks проверяют malformed images, report identity/chunks, изменение каждого profile byte, provider denial и dependency direction. Native foundation tests и существующие failure controls не меняются. Dependency audit и source guards запрещают core imports и compatibility conditionals; guards — ограниченное enforcement, а не доказательство против произвольного generated code.

## Обратимость

Optional image можно исключить без изменения native tests. Будущая service model может заменить experimental native call numbers и own-task snapshot interface. Stateful adapters, live shared publication, code unloading, migration и delegated authority требуют нового lifetime/security review и evidence.

## Доказательная база

[Kernel Laws](../architecture/kernel-laws.md) не меняются: применяются LAW-001, LAW-003, LAW-004, LAW-005, LAW-008, LAW-009, LAW-013, LAW-018, LAW-025, LAW-035, LAW-036 и LAW-041. Значимые failure mechanisms: [shared identity](../../../../research/cases/KOL-PATH-0002.json), [unsafe old behavior](../../../../research/cases/KOL-PATH-0007.json), [check/use mutation](../../../../research/cases/KOL-PATH-0018.json), [retiring references](../../../../research/cases/KOL-PATH-0030.json). [Результаты](../../../../research/results/routing-phase2.json) сохраняют execution observations и точные source inventories.

[Английский оригинал](../../../../docs/architecture-decisions/0015-el0-versioned-routing.md)
