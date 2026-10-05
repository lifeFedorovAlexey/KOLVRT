<!-- markdownlint-disable MD041 -->
<!-- Stable knowledge anchor расположен перед видимым заголовком документа. -->

<a name="bounded-native-ipc"></a>

# Ограниченный IPC между нативными EL0-процессами

Document status: CURRENT
Evidence scope: техническая реализация Phase 3.5 сохранена; после integration с main ac63c75 прошли 124 DEV и 124 PROD checks. Окончательное acceptance review остаётся открытым; physical ARM не проверен.
Current reference: [Повторное выведение архитектуры](../architecture/ipc-phase35-review.md)

ABI contract: native.request/1
Publication stage: EXPERIMENTAL
ABI-FREEZE: none

<a name="authority-and-identities"></a>

## Authority and identities

Concrete endpoint отделён от Event и Completion. Trusted bootstrap связывает его с точным service ProcessId и фиксированным owner CPU. Nondelegable receiver binding разрешает receive, commit, reply и shutdown. SEND grant разрешает submit на этот endpoint. Identity caller берётся из выполняющейся namespace и immutable task; packet не выбирает principal. Service-private handles не подменяют caller SEND или accepted service request token.

Handles сохраняют фактический LE64 encoding: low 8 bits выбирают namespace slot, high 56 bits задают nonwrapping generation. Private pool identity endpoint имеет собственную generation. Requester receipts и service tokens — глобальные nonwrapping request identities, проверяемые с точным requester или receiver binding. Их числа сами по себе не дают authority.

Caller request IDs уникальны в scope requester ProcessId плюс endpoint, пока существуют queued, delivered, committed или retained terminal work. Успешный collect либо допустимый abandon/death cleanup освобождает ID. Close SEND не отзывает aliases и не стирает accepted work. Общая revoke/admission serialization запрещает последующие SEND, сохраняя ранее принятые запросы.

<a name="bounded-storage-and-accounting"></a>

## Bounded storage and accounting

Fixed pool содержит восемь endpoints. Acceptance fixtures явно используют queue capacity 1 и 4; request/result storage отдельно содержит восемь slots на endpoint. Payload ограничен 256 bytes. Один точный receiver регистрирует один readable wait; requester terminal waits используют bounded registry из шестнадцати retained wait records. У процесса один active owner-local blocking reason.

Endpoint ownership учитывается в service domain. Admission транзакционно получает requester request charge, service queue charge и service request charge. Failed admission не публикует queue entry и освобождает подготовленные charges. Успешный receive освобождает queue charge. Единственный terminal arbiter освобождает service work charges один раз; requester/result ownership остаётся до успешного collect или cleanup. Наблюдение closing domain запрещает commitment uncommitted work умершего requester до того, как deferred death drainage достигнет cell. Committed work умершего requester сохраняется до actual terminal resolution.

<a name="copied-transport"></a>

## Copied transport

SVC `0xa0` получает input address/length в x0/x1 и output address/capacity в x2/x3. Возвращает status в x0 и receipt, outcome либо output length в x1. Input — owned initialized snapshot, скопированный в scope точного executing task/root. Endpoint work выполняется после выхода из scheduler storage. Output copy вновь получает scope той же executing task; endpoint permit не охватывает user access, scheduler access, sleep или context switch.

Input header содержит 40 bytes с явными little-endian fields:

| Offset | Bytes    | Field                                                        |
| ------ | -------- | ------------------------------------------------------------ |
| 0      | 2        | Version 1                                                    |
| 2      | 2        | Operation                                                    |
| 4      | 4        | Точная total input length                                    |
| 8      | 8        | Caller-local endpoint handle либо accepted requester receipt |
| 16     | 8        | Submit client ID либо commit/reply service token             |
| 24     | 8        | Submit absolute deadline в boot-local counter ticks          |
| 32     | 4        | Payload length, 0–256                                        |
| 36     | 4        | Reserved zero                                                |
| 40     | variable | Copied payload                                               |

Operations: submit 1, receive 2, commit 3, reply 4, wait-terminal 5, collect 6, cancel 7, shutdown 8 и abandon 9. Payload имеют только submit/reply. Deadline имеет только submit. ID/token field имеют только submit/commit/reply. Unknown versions/operations и nonzero unused fields явно отклоняются; downgrade к native.request/0 отсутствует.

Receive output использует initialized 40-byte header: version, reserved zero, total length, service token, client ID, deadline, payload length, reserved zero, затем payload. Collect output использует 24 bytes: version, outcome, total length, client ID, payload length, reserved zero, затем response. Rust structure layout и padding не пересекают EL0 boundary.

<a name="copy-transactions-and-terminal-arbitration"></a>

## Copy transactions and terminal arbitration

Receive резервирует и копирует queue head. Failed copy-out сохраняет position, payload и charges; token не может commit queued work. Successful copy фиксирует delivery и dequeues один раз. Collect резервирует immutable terminal result. Failed copy-out сохраняет result и client ID; successful copy consumes один раз. Reservations удерживают точную endpoint/request identity между фазами и безопасно отклоняют concurrent cancellation или slot reuse.

Единственный terminal arbiter обрабатывает completion, cancellation, expiry, service death и shutdown. До commitment cancellation/death возвращают cancelled-before-effect, expiry — expired-before-effect. После commitment эти пути возвращают effect-unknown. Completed results содержат initialized service response. Последующие terminal attempts не изменяют winner и не освобождают charges дважды. Commitment — transport authorization, без обещания durability или rollback.

<a name="blocking-cross-cpu-wake-and-idle"></a>

## Blocking, cross-CPU wake and idle

Readable и terminal waits выполняют check, register, recheck и block только при false condition. Ready condition может завершиться сразу. Producers сохраняют exact wait identity до acknowledgement mailbox record target owner. Mailbox атомарно хранит одну pending-or-acknowledged sequence; publication и source acknowledgement retirement разделяют endpoint exclusion, исключая delayed publication после удаления source. Busy control storage сохраняет work для deferred retry.

Только target CPU переводит свою task из BLOCKED в READY, проверяя process generation и сохранённую globally unique wait sequence. SGI/physical timer handlers публикуют flags; bounded native continuations обрабатывают object state вне scheduler storage. SGI rescheduling не создаёт ложных timer slices. Endpoint permit contention отделён от state BUSY и использует kernel retry без EL0 polling loop.

`Registry::dispatch_ipc()` сохраняет continuous admitted session. Когда READY task отсутствует, owner возвращается к permanent native root/stack и ждёт IRQ, сохраняя contexts, roots и namespaces. Ближайший request/session deadline и bounded retry timer обеспечивают deferred work во время BLOCKED peers. Idle IRQ flags сбрасываются до возобновления task accounting. Отсутствие READY task не означает completion.

<a name="teardown-and-reclamation"></a>

## Teardown and reclamation

Death/closing запрещает новый admission и resolves либо retains accepted work согласно commitment. Shutdown пробуждает readable и terminal waiters. Source wait records живут до exact acknowledgement; stale target work не создаёт runnable owner. Endpoint reclaimable только после closed, при отсутствии requests/results и wait records и при единственной собственной reference. Whole-session retirement дополнительно требует terminal state обоих CPUs, отсутствия pending copy completion, drained mailboxes и native-root ownership. Reclamation не основан на elapsed grace delay.

Independent admission, migration, mutable user mappings, supervisor/service policy, generic wait-any, generic invoke и zero-copy исключены. Для последующих #27/#28 архитектура выводится заново: implementation order не становится authority.

## Evidence and remaining acceptance

Метод измерения, четыре копирования и raw observations описаны в [паспорте производительности](ipc-performance.md). Сохранены 144 группы каждого профиля, включая проверенные hot/blocked/full-queue пути. Базовый артефакт относится к исходникам предыдущего integration; production IPC и benchmark source при обновлении main не изменились.

Текущие реальные EL0 fixtures проверяют оба cross-CPU направления и same-CPU placement на каждом CPU, capacity 1/4, initialized payload boundaries, mutation после submit/reply, duplicate IDs до collection, failed receive/collect copy-out, blocked expiry до/после commitment, service/requester death, cancellation через отдельный IPC handshake, authority denial, revoke/close retention, FIFO/full rejection и zero charges/frames после quiescence. Payload stress выполняет 24 requests на CPU/capacity configuration, включая четыре прохода размеров 0/1/8/64/255/256.

Двадцать три focused mutation controls проверяют authority, queue bounds/FIFO, client-ID и internal request generation, service-token substitution, wait recheck, wake generation/publication, duplicate READY, wake неверного процесса, blocked reclaim, storage scope, terminal arbitration, cancel/commit, deadlines, потерю и повторный release charge, endpoint teardown, copy transactions и service death. Все 46 DEV/PROD прогонов отклонены с точным expected event либо named failing test; любой panic не считается достаточным. Kernel failure CPU1 передаёт точную причину через bounded atomic record на CPU0, который прекращает continuous session без ложного reclaim.

Текущие source-bound artifacts: [kernel acceptance](../../../../research/results/ipc-phase35-acceptance.json), [physical package-removal](../../../../research/results/native-compat-removal-phase35.json), [six-configuration routing regression](../../../../research/results/routing-phase35-regression.json) и [unsafe inventory](../../../../research/results/kernel-phase35-unsafe-audit.json). Исходные receipts сохраняют свои snapshots. [Receipt integration с main](../../../../research/results/ipc-phase35-main-integration.json) фиксирует положительные проверки текущих исходников и review неизменности enforcement; изменились package metadata и test-only финализация IRQ. Смысловое review и полное EN/RU review человеком нужны перед закрытием #26 и переходом feature в BOUNDED_IMPLEMENTED. Следующий gate — #27; ABI остаётся EXPERIMENTAL и unfrozen.

Sources: [core endpoint state](../../../../crates/kernel-core/src/ipc.rs), [identity](../../../../crates/kernel-core/src/ipc/identity.rs), [wire](../../../../crates/kernel-core/src/ipc/wire.rs), [mailbox](../../../../crates/kernel-core/src/ipc/mailbox.rs), [native continuation](../../../../crates/kernel/src/ipc/native.rs), [storage](../../../../crates/kernel/src/ipc/storage.rs), [deferred work](../../../../crates/kernel/src/ipc/deferred.rs), [scheduler](../../../../crates/kernel/src/scheduler/mod.rs), [EL0 fixtures](../../../../crates/kernel/src/ipc_workload.rs), [runner](../../../../crates/xtask/src/main.rs).

[English source](../../../../docs/kernel/ipc.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.ipc",
  "kind": "subsystem-contract",
  "summary": "Экспериментальный контракт bounded native EL0 IPC и текущие свидетельства Phase 3.5.",
  "units": [
    {
      "id": "kolvrt.ipc.transport",
      "anchor": "bounded-native-ipc",
      "kind": "feature",
      "summary": "Ограниченный IPC через endpoint с копированием запросов, блокирующим ожиданием и точными terminal outcomes.",
      "tags": ["ipc", "endpoint", "native", "el0"],
      "depends_on": [
        "kolvrt.process.identity",
        "kolvrt.handles.local",
        "kolvrt.handles.lifetime",
        "kolvrt.security.domains",
        "kolvrt.memory.user-copy",
        "law.009",
        "law.013",
        "law.025",
        "adr.0016",
        "adr.0017",
        "adr.0018",
        "adr.0019",
        "adr.0020",
        "adr.0021",
        "adr.0022",
        "adr.0023",
        "adr.0025",
        "doc.kolvrt.architecture.native-abi",
        "doc.kolvrt.architecture.first-native-slice",
        "doc.kolvrt.architecture.production-scheduler",
        "doc.kolvrt.kernel.wait"
      ],
      "gaps": [
        "Human semantic and complete EN/RU review remain pending before milestone acceptance/promotion.",
        "Physical ARM is unverified; QEMU does not establish silicon behavior."
      ],
      "feature": {
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "Complete bounded native.request/1 mechanism with verified EL0 DEV/PROD execution; final human semantic/EN/RU review remains pending.",
        "sources": [
          "crates/kernel-core/Cargo.toml",
          "crates/kernel-core/src/domain.rs",
          "crates/kernel-core/src/handles.rs",
          "crates/kernel-core/src/lib.rs",
          "crates/kernel-core/src/process.rs",
          "crates/kernel-core/src/ipc.rs",
          "crates/kernel-core/src/ipc/identity.rs",
          "crates/kernel-core/src/ipc/wire.rs",
          "crates/kernel-core/src/ipc/mailbox.rs",
          "crates/kernel-core/src/ipc/tests.rs",
          "crates/kernel-core/tests/handle_contract.rs",
          "crates/kernel/Cargo.toml",
          "crates/kernel/src/arch/aarch64/mod.rs",
          "crates/kernel/src/boot_workload.rs",
          "crates/kernel/src/handles.rs",
          "crates/kernel/src/handles/testing.rs",
          "crates/kernel/src/main.rs",
          "crates/kernel/src/process.rs",
          "crates/kernel/src/process_workload.rs",
          "crates/kernel/src/scheduler/local.rs",
          "crates/kernel/src/scheduler/mod.rs",
          "crates/kernel/src/scheduler/task.rs",
          "crates/kernel/src/security/testing.rs",
          "crates/kernel/src/smp.rs",
          "crates/kernel/src/tests.rs",
          "crates/kernel/src/user_copy/testing.rs",
          "crates/kernel/src/ipc.rs",
          "crates/kernel/src/ipc/native.rs",
          "crates/kernel/src/ipc/storage.rs",
          "crates/kernel/src/ipc/deferred.rs",
          "crates/kernel/src/ipc_workload.rs",
          "crates/kernel/src/ipc_workload.S",
          "crates/kernel/src/ipc_death_workload.S",
          "crates/kernel/src/ipc_authority_workload.S",
          "crates/kernel/src/ipc_payload_workload.S",
          "crates/kernel/src/ipc_queue_workload.S",
          "crates/kernel/src/ipc_benchmark.rs",
          "crates/kernel/src/ipc_benchmark.S",
          "crates/xtask/src/main.rs",
          "crates/xtask/src/output.rs",
          "crates/kernel/src/ipc_multi_workload.S",
          "crates/kernel/src/ipc_paths_benchmark.S",
          "crates/kernel/src/ipc_revoke_workload.S",
          "crates/kernel/src/test_support/irq_wait.rs"
        ],
        "acceptance": [
          "research/results/ipc-phase35-acceptance.json",
          "research/results/native-compat-removal-phase35.json",
          "research/results/routing-phase35-regression.json",
          "research/measurements/ipc-phase35-baseline.json",
          "research/results/kernel-phase35-unsafe-audit.json",
          "research/results/ipc-phase35-main-integration.json",
          "research/results/routing-phase35-upgrades.json",
          "research/results/phase35-dependency-upgrades.json"
        ],
        "issues": [26],
        "adrs": ["adr.0025"],
        "limitations": [
          "ABI remains experimental and unfrozen.",
          "Fixed affinity, immutable mappings and conservative whole-session retirement; no supervisor, service discovery, migration, zero-copy or universal wait/invoke.",
          "Physical ARM and production bootstrap trust are unverified.",
          "Human semantic/EN/RU review is still pending."
        ],
        "next_gate": "Obtain human semantic/EN/RU review for #26 acceptance, then Phase 3.6/#27 EL0 supervision; rederive its architecture from current laws and accepted decisions.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "VERIFIED",
            "reason": "124 named checks pass in DEV and PROD on main ac63c75 with the requested dependency upgrades. The 46 original IPC mutation runs retain their original source snapshot; enforcement code is unchanged.",
            "receipt": "research/results/ipc-phase35-main-integration.json",
            "receipt_sha256": "23ae23c695ce5528b931769dbca7a0f1af6067fbd965f29c497d7056a22bb863",
            "scope": "Current-source positive QEMU integration with separately retained ELF artifacts; no new mutation/native-only/performance or physical ARM claim. Six upgraded routing configurations and three rejection controls are recorded separately."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical ARM64 receipt exists."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Record the implemented bounded IPC contract while Issue #26 acceptance remains incomplete.",
            "acceptance": []
          }
        ],
        "roadmap_gate": "Phase 3.5"
      },
      "aliases": ["kolvrt.ipc"]
    },
    {
      "id": "kolvrt.ipc.endpoint",
      "anchor": "authority-and-identities",
      "kind": "contract-section",
      "summary": "Concrete endpoint, привязка service и scoped SEND authority.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.queue",
      "anchor": "bounded-storage-and-accounting",
      "kind": "contract-section",
      "summary": "Ограниченные очередь, result slots и domain charges.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.request",
      "anchor": "copied-transport",
      "kind": "contract-section",
      "summary": "Фрейм запросов native.request/1 и операции с копированием.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.payload",
      "anchor": "copied-transport",
      "kind": "contract-section",
      "summary": "Инициализированные bounded snapshots payload и байты ответа.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.receive",
      "anchor": "copy-transactions-and-terminal-arbitration",
      "kind": "contract-section",
      "summary": "Транзакционная доставка receive и сохранение очереди.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.wait",
      "anchor": "blocking-cross-cpu-wake-and-idle",
      "kind": "contract-section",
      "summary": "Блокирующие readable и terminal conditions с регистрацией и повторной проверкой.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.wakeup",
      "anchor": "blocking-cross-cpu-wake-and-idle",
      "kind": "contract-section",
      "summary": "Точная owner wake identity, подтверждение mailbox и прогресс idle.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.terminal",
      "anchor": "copy-transactions-and-terminal-arbitration",
      "kind": "contract-section",
      "summary": "Ровно один сохранённый terminal result и граница commitment.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.cancel",
      "anchor": "copy-transactions-and-terminal-arbitration",
      "kind": "contract-section",
      "summary": "Отмена до и после commitment service effect.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.deadline",
      "anchor": "copy-transactions-and-terminal-arbitration",
      "kind": "contract-section",
      "summary": "Абсолютный monotonic deadline и terminal arbitration.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.cross-cpu",
      "anchor": "blocking-cross-cpu-wake-and-idle",
      "kind": "contract-section",
      "summary": "Cross-CPU signalling с фиксированной привязкой и drainage wake.",
      "depends_on": ["kolvrt.ipc.transport"]
    },
    {
      "id": "kolvrt.ipc.teardown",
      "anchor": "teardown-and-reclamation",
      "kind": "contract-section",
      "summary": "Завершение, shutdown, удержание authority и reclamation endpoint.",
      "depends_on": ["kolvrt.ipc.transport"]
    }
  ]
}
```
