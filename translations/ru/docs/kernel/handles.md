# Локальные дескрипторы процессов

Document status: CURRENT
Evidence scope: ограниченные caller-local handles с явными правами SEND/TRANSFER/REVOKE, attenuation общих targets и отзывом admission общего Event; два CPU с fixed affinity.
Current reference: [ADR-0020](../architecture-decisions/0020-handle-transfer-and-retention.md) для identity/transfer/lifetime handles с дополнением [ADR-0022](../architecture-decisions/0022-native-event-grants-and-revocation.md) для минимального Event grant/revoke среза и [native IPC](ipc.md) для конкретных endpoint bindings.

<a name="kolvrt-handles-local"></a>

## Границы функции

Каноническая запись описывает ограниченную реализацию. Доказательства ограничены входными данными записи; аппаратная проверка и готовность к эксплуатации являются отдельными этапами.

<a name="kolvrt-handles-identity"></a>

## Представление и caller context

[Примитивы](../../../../crates/kernel-core/src/handles.rs) кодируют opaque LE64: восемь младших бит slot, 56 бит generation; capacity — восемь записей. Это явный предварительный wire encoding, а не layout внутренней структуры. Generation zero — некорректная кодировка, а не специальный ресурс. Предсказуемые integer identity не являются секретами или authority.

Process slot сохраняет линейный namespace при ProcessId reuse. Binding хранит точный ProcessId; lookup проверяет caller, bounds, generation, live entry, requested kind и необходимые rights. Одинаковые числа в разных namespaces могут означать разные ресурсы. Wire handle не выбирает таблицу другого процесса. [Native request path](../../../../crates/kernel/src/handles.rs) получает caller из scheduler-owned task и bound namespace, проверяет executing context и копирует 48-byte request в immutable snapshot до обращения к namespace.

```text
LE64[6] version=1, operation=(1 lookup | 2 close | 3 transfer | 4 revoke)
lookup/close: handle, kind, required_rights, reserved=0
transfer:     handle, receiver_process_slot, receiver_generation, requested_rights
revoke:       handle, reserved=0, reserved=0, reserved=0
safe copy -> decode snapshot -> current caller table -> generation/type/rights check
           -> receiver-local handle with the same target binding
```

EL0 доступны lookup, close, transfer и revoke общего Event. Rights: `SEND=1`, `TRANSFER=2`, `REVOKE=4`; неизвестные bits отклоняются. Bootstrap по умолчанию выдаёт Event только SEND, а Completion не получает прав; более широкие grants требуют явного trusted `create_*_with_rights`. REVOKE применим к Event и endpoint SEND grants; такой grant для Completion отклоняется до резервирования поколения slot и публикации. Transfer требует TRANSFER и subset от выданных прав. Revoke требует REVOKE и атомарно запрещает новые signal admissions через все aliases. Сигнал, состязающийся с revoke, упорядочивается CAS состояния target; ранее принятая работа и уже pending notification сохраняются. Close удаляет только локальную ссылку и не является revoke. Получатель задаётся live ProcessId, уже привязанным к namespace на CPU вызывающего; запрос на другой CPU возвращает ForeignProcess. Status words: 0 success, 1 invalid encoding/request, 2 stale, 3 wrong type, 4 foreign context, 5 inactive, 6 capacity, 7 generation exhaustion, 8 copy failure, 9 rights denied, 10 retained-reference quota exhausted, 11 revoked. Unknown version/opcode/kind отклоняется. Close проверяет kind и generation. Pointer, physical address, global object ID, internal enum layout и compatibility errno не передаются EL0. Создание остаётся trusted bootstrap operation, не unprivileged object-creation authority API.

Real EL0 fixture отправляет revoke как copied 48-byte request и проверяет, что следующее SEND admission возвращает `Revoked`. Host controls проверяют, что delegated alias остаётся отозванным после source close. Эти проверки покрывают только Event slice и не подтверждают полные grant, service-lifecycle и security-domain критерии issue #24.

<a name="kolvrt-handles-lifetime"></a>

## Владение и target lifetime

Каждый принадлежащий namespace ресурс имеет неизменяемый внутренний TargetId (создавший ProcessId, slot и generation резервирования), отдельный от process-local wire Handle. Перемещение namespace сохраняет TargetId; повторное использование slot создаёт другой ресурс. TargetId никогда не сериализуется в EL0 и не предоставляет authority.

Конкретные targets — coalescing wait Event, принадлежащая namespace неизменяемая process Completion record и ограниченный IPC endpoint. Completion удерживает значение, а не живой процесс или address space; close не завершает процесс. Делегированные Event handles разделяют atomic latch в фиксированном kernel pool из 512 слотов. Каждый слот имеет nonwrapping generation и допускает не более 1 024 живых references. Namespace процесса содержит восемь записей; исчерпание pool или reference quota возвращается явно. Этот узкий набор не является universal KernelObject hierarchy или generic invocation interface.

Endpoint extension сохраняет handle encoding 8/56 и существующие rights bits. Trusted bootstrap устанавливает scoped SEND grant либо nondelegable receiver binding для точного service generation. Receiver authority — отдельный concrete binding, а не новые rights RECEIVE/REPLY и не ProcessId из EL0. Endpoint SEND grants могут содержать attenuated TRANSFER/REVOKE; Completion и receiver bindings отвергают такую delegation. Endpoint revoke прекращает будущий admission через aliases и сохраняет accepted requests. Close receiver handle убирает только lookup authority; отдельный transport shutdown прекращает admission и разрешает accepted work. Endpoint references и accepted request receipts переживают close отдельного handle до actual quiescence.

Каждая запись namespace владеет одной ссылкой на target. Lookup возвращает Retained borrow; `.retain()` создаёт owned reference для accepted work. Используемый borrow исключает mutable close/retire на уровне компилятора. Transfer проверяет обе таблицы до публикации новой receiver-local generation, attenuates rights и сохраняет TargetId. Revoke Event действует на все aliases; close убирает только одну запись и запрещает дальнейший lookup по этому token, а delegated entries и owned retained work сохраняют target. Double close — Stale. Resource-specific методы остаются kernel-only; rights проверяются перед admission, но не добавляют EL0 object pointer или универсальный invoke interface.

```mermaid
flowchart TD
    R[Registry owns namespace and retained frames] --> M[Move namespace to indexed CPU state]
    M --> L[Masked owner lookup and retained borrow]
    L --> B[Borrow or owned accepted reference]
    B --> Q[Close drops one reference; final drop reclaims pool slot]
    Q --> T[Both CPUs restore native roots and complete TLBI]
    T --> R
    T --> S{Process survives scheduling step?}
    S -->|yes| R
    S -->|exit or fault| E[Retire entries and unbind namespace]
    E --> F[Reclaim frames while preserving slot generations]
```

[Process ownership](../../../../crates/kernel/src/process.rs) сохраняет поколения namespace после retirement. [Scheduler integration](../../../../crates/kernel/src/scheduler/mod.rs) перемещает namespace в owner state через существующие publication permits и возвращает через quiescent editing после завершения двух CPU. Same-CPU lookup и EL0 transfer используют уже существующий scheduler ownership permit; table/object lock не приобретается. Cross-CPU тесты передают handle, пока обе таблицы принадлежат coordinator, затем запускают close исходной записи одновременно с lookup получателя на реальных EL0 CPU. Atomic reference count защищает shared Event. Новые UnsafeCell, unsafe Sync, raw-pointer cache и unchecked indexing не добавлены. User copy выполняется до доступа к namespace; lock или ownership permit не удерживаются через allocation, wait или external call.

DEV после quiescence показывает process slot/generation, capacity, active count, поколения/kinds live slots и retiring lifecycle; полные raw handle values, raw kernel pointers и payload не выводятся. PROD исключает diagnostics, сохраняя проверки. Квоты: восемь записей на процесс, 512 shared Event slots и 1,024 references на target. Lookup/close/transfer не выделяют память, не блокируются, не уступают CPU, не логируют и не вызывают external service.

Подготовка runqueue инициализирует существующее хранилище под исключительным preparing permit, без передачи State по значению через фиксированный DEV-стек. Итоговый аудит воспроизвёл повреждение BSS из-за крупных временных объектов на стеке прежней реализации. Инициализация на месте устраняет эти копии и сохраняет continuity отчётов и ownership namespace.

<a name="kolvrt-handles-publication"></a>

## Generation и transactional publication

```text
reserve vacant slot -> burn generation -> prepare owned resource
 -> publish explicit handle bytes through safe copy -> infallible live commit
close -> remove owned resource -> lookup is stale
reuse same slot -> strictly greater generation
process exit/fault -> retire all entries -> unbind -> preserve counters
next ProcessId generation -> rebind same namespace -> old handle stays stale
```

Generation увеличивается при reservation, включая failed publication. Close немедленно убирает live state; следующая allocation увеличивает generation. При 2^56-1 vacant slot навсегда исчерпан. Wrap, truncation и automatic namespace reset отсутствуют. Другие available slots работают; Capacity отличается от GenerationExhausted. One-slot/two-generation instantiation проверяет тот же exhaustion algorithm в host и actual EL0 fixture. Практическая невозможность exhaustion не заявляется.

Создание владеет prepared target на kernel stack до успешной publication. Failure освобождает его, оставляя vacant slot с burned generation. Callback не может обращаться к exclusively borrowed namespace. В текущем private fixed-affinity процессе EL0 не может исполняться/читать partially published bytes во время synchronous callback; commit завершается до возврата. Partial copy failure может оставить stale bytes в user RAM, но никогда live entry. Shared mapping изменит этот publication proof и потребует redesign.

<a name="kolvrt-handles-evidence"></a>

## Verification и performance gate

Retirement-control требует отдельное событие handle-retirement-reject: посторонний panic не засчитывается как ожидаемое отклонение нарушения cleanup invariant.

[EL0 verification](../../../../crates/kernel/src/handles/testing.rs) проверяет forged/stale/wrong-kind lookup, double close, required rights и transfer через production copied request path. Получатель получает собственный token на тот же TargetId с правом только SEND; escalation прав и заполненная receiver table отклоняются без частичного transfer. Исходный handle закрывается до завершения accepted work. Два CPU также получают делегированные ссылки и выполняют конкурентные close исходной записи/lookup получателя на одном atomic Event. Восемь двухъядерных циклов creation/exit/fault/reclaim сохраняют handle generations при ProcessId slot reuse; проверяются frame counts, target identity и cleanup namespace. Fixture также покрывает completion ownership, capacity, small-limit generation exhaustion, failed user publication и repeated close/reuse.

Шесть build controls отключают реальные generation validation, owner validation, type checking, generation advancement, retirement cleanup или transfer rights enforcement. Runner должен обнаружить каждый в DEV и PROD. Изменяется enforcement, а не test expectation. Retire проверяется отдельным handle-retirement-reject событием, чтобы посторонний panic не считался нужным отказом. Проверенная матрица включает existing Phase 3.2/lifecycle controls, native-only removal, routing regression, host tests, Clippy и repository checks.

Five per-CPU scopes сохраняют четыре warmups и 32 raw timer observations: successful lookup, failed lookup, create, close и slot reuse. Create измеряет target construction и namespace commit с no-op publication callback; user-copy/EL0 setup исключены. Close preparation вне интервала. Reuse измеряет create/close/create/close. Это regression baselines QEMU TCG, а не hardware throughput claims. Compiler-observation barriers материализуют промежуточные состояния namespace, исключая свёртку create/close/reuse. Эти числа — исходный baseline Phase 3.3; текущий exact-source receipt issue #23 записан отдельно.

Измерения timer ticks при 62,5 МГц, 32 samples на ячейку. Single-operation PROD observations близки к timer granularity; zero median означает квантованное измерение, а не нулевую стоимость выполнения. Hardware speedup из samples не следует:

| Scope                 | DEV CPU0 median / p95 | DEV CPU1 median / p95 | PROD CPU0 median / p95 | PROD CPU1 median / p95 |
| --------------------- | --------------------- | --------------------- | ---------------------- | ---------------------- |
| handle_lookup_success | 44 / 50               | 50 / 75               | 6 / 7                  | 6 / 13                 |
| handle_lookup_failure | 38 / 50               | 37 / 44               | 6 / 125                | 6 / 7                  |
| handle_create         | 68 / 81               | 69 / 88               | 6 / 31                 | 6 / 13                 |
| handle_close          | 44 / 56               | 44 / 50               | 6 / 31                 | 6 / 63                 |
| handle_slot_reuse     | 206 / 218             | 212 / 225             | 6 / 13                 | 6 / 19                 |

[Issue #23 exact-source receipt](../../../../research/measurements/runs/1791022558822-issue23-transfer-bf3f9b298688.json) фиксирует 73 DEV/PROD checks и 69 negative controls. [Исходный baseline Phase 3.3](../../../../research/results/kernel-phase33.json), [native-only receipt](../../../../research/results/native-compat-removal-phase33.json), [routing regression](../../../../research/results/routing-phase33-regression.json) и [unsafe inventory](../../../../research/results/kernel-phase33-unsafe-audit.json) сохраняют отдельные области. Native-only сверяет 59 неизменных native/harness files; production unsafe delta — zero, три linked-fixture sites — test-only.

<a name="kolvrt-handles-limits"></a>

## Пределы authority и координации

Handle generation остаётся identity, а SEND/TRANSFER отдельно ограничивают lookup/admission и delegation. Issue #23 добавляет attenuation и bounded owned Event retention. Phase 3.4 добавляет protected bootstrap issuance и revocation; исторический baseline выше остаётся ограничен Phase 3.3. EL0 transfer адресует receiver namespace того же CPU; другой CPU отклоняется до изменения таблиц. Семантика Linux fd/Windows HANDLE, общий IPC, magic current/root handles и universal object model исключены.

## Phase 3.4 current security boundary

[Security domains](domains.md) связывают каждую process generation с локальными handles, явными bootstrap grants и неизменяемыми memory/handle/queue/request quotas, переданными caller. SEND/TRANSFER rights допускают attenuation; REVOKE=4 означает явную issuer authority. Revocation запрещает новые effects, а принятая работа удерживает consumer charge и target до completion либо service-fault cancellation. Закрытие handle не отзывает aliases. Ограниченный same-CPU notification pilot сообщает EL0 terminal outcomes; general IPC, automatic wakeups, supervisor policy и persistent services относятся к следующим этапам. Перед расширением storage и encoding нужно заново вывести из требований #26/#27.

[Английский оригинал](../../../../docs/kernel/handles.md)

Phase 3.7 удаляет no-op selectors повреждения из текущего runner. Проверки поколения, чужого владельца, неверного типа, повторного использования и transfer rights используют существующие реальные EL0 сценарии и production-методы Namespace. Новый UNIT-вход вызывает Namespace::retire с чужим владельцем и проверяет сохранение owner/живого handle и успешного lookup до законного retirement; повторный retirement и lookup после retirement отклоняются. Наблюдения cleanup/reuse отдельно классифицированы как положительные инварианты lifetime. Исторические counts и receipts mutation-controls не доказывают отрицательное mutation coverage текущего исходника.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.handles",
  "kind": "subsystem-contract",
  "summary": "Навигация по документу: Локальные дескрипторы процессов. Доказательства имеют указанные границы.",
  "units": [
    {
      "id": "kolvrt.handles.local",
      "anchor": "kolvrt-handles-local",
      "kind": "feature",
      "summary": "Каноническая секция: Границы функции.",
      "depends_on": [
        "kolvrt.handles.identity",
        "kolvrt.handles.lifetime",
        "kolvrt.handles.publication",
        "kolvrt.handles.limits",
        "kolvrt.memory.user-copy",
        "kolvrt.process.identity",
        "kolvrt.process.reclamation",
        "law.009",
        "law.013"
      ],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "Bounded caller-local namespaces and receiver-local transfer on fixed-affinity CPUs.",
        "sources": [
          "crates/kernel/src/handles.rs",
          "crates/kernel-core/src/handles.rs"
        ],
        "acceptance": [
          "research/measurements/runs/1791022558822-issue23-transfer-bf3f9b298688.json"
        ],
        "issues": [23],
        "adrs": ["adr.0019", "adr.0020", "adr.0022", "adr.0023"],
        "limitations": [
          "Close does not revoke retained work; no general grants, domains or IPC."
        ],
        "next_gate": "General IPC and supervisor policy installation require independently derived contracts; bounded Phase 3.4 scoped grants/domains are separately documented.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "STALE",
            "reason": "This feature retains historical receipts from before the Phase 3.7 source changes; its verification remains STALE pending a feature-scoped current-source review. The separately accepted bounded native-application fault/restart/shutdown evidence is recorded in docs/kernel/native-applications.md and research/results/native-phase37-43af402.json; it does not automatically renew this feature verification.",
            "scope": "The exact source digests, DEV/PROD and QEMU TCG configuration recorded by this receipt; physical ARM64 excluded.",
            "receipt": "research/measurements/runs/1791022558822-issue23-transfer-bf3f9b298688.json",
            "receipt_sha256": "c82eb2b89a4045ae8cd4b0bcc2f32a0baa07a33a9208f111acfcd5ed88574ed8"
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical ARM64 acceptance is established."
          }
        ],
        "readiness": "NOT_READY",
        "roadmap_gate": "Phase 3.3",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "Initial reviewed catalog adoption of existing scoped contract; not a new implementation transition.",
            "acceptance": [
              "research/measurements/runs/1791022558822-issue23-transfer-bf3f9b298688.json"
            ]
          }
        ]
      }
    },
    {
      "id": "kolvrt.handles.identity",
      "anchor": "kolvrt-handles-identity",
      "kind": "contract-section",
      "summary": "Каноническая секция: Представление и caller context.",
      "depends_on": []
    },
    {
      "id": "kolvrt.handles.lifetime",
      "anchor": "kolvrt-handles-lifetime",
      "kind": "contract-section",
      "summary": "Каноническая секция: Владение и target lifetime.",
      "depends_on": []
    },
    {
      "id": "kolvrt.handles.publication",
      "anchor": "kolvrt-handles-publication",
      "kind": "contract-section",
      "summary": "Каноническая секция: Generation и transactional publication.",
      "depends_on": []
    },
    {
      "id": "kolvrt.handles.evidence",
      "anchor": "kolvrt-handles-evidence",
      "kind": "contract-section",
      "summary": "Каноническая секция: Verification и performance gate.",
      "depends_on": []
    },
    {
      "id": "kolvrt.handles.limits",
      "anchor": "kolvrt-handles-limits",
      "kind": "contract-section",
      "summary": "Каноническая секция: Пределы authority и координации.",
      "depends_on": []
    }
  ]
}
```
