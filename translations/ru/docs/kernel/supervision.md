# Изолированный supervisor EL0

Document status: CURRENT

Evidence scope: экспериментальная development fixture Phase 3.6 и реализация ограниченного lifecycle; acceptance review, production bootstrap trust и physical ARM64 остаются открытыми.

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

Lifecycle work выполняется на явном fixed-affinity checkpoint двух CPU. Каждый owner исполняет bounded timer quantum, terminal transition, block или explicit lifecycle yield, восстанавливает native root и освобождает execution ownership. Затем CPU0 приобретает оба completion перед изменением Registry или allocation/reclaim frames. Pending copy transactions завершаются до возвращения namespaces. Saved IPC wait identities, retry reasons и counters сохраняются через barrier и восстанавливаются только для того же ProcessId; endpoint и source/mailbox ownership удерживаются. Отсоединение root blocked процесса само по себе не разрешает reclaim.

SGI-only entry не расходует checkpoint quantum до первой инструкции EL0; deferred work дренируется, а заведённый timer продолжает обеспечивать progress. Scheduling cursor продолжается только для соответствующей живой process generation. Death/stop очищает blocked ownership точного процесса, прекращает admission и выполняет ASID retirement на owning CPU. Old endpoint requests, terminal results и wake acknowledgements сохраняют независимое ownership до drainage. Reclamation не основывается на истекшем grace period.

Этот bounded checkpoint приостанавливает здоровых peers на время lifecycle changes; он не добавляет migration или independent admission во время исполнения user code на CPU. Death observation использует exact lifecycle queries; generic wait-any и автономная persistent orchestration исключены. Исходный continuous IPC path имеет отдельный scope и проверки. Readiness, effects и failure policy не выводятся из timeout или остановленного CPU.

## Политика readiness, restart и shutdown

Политика EL0 отправляет readiness probe точному endpoint с absolute counter deadline 1/8 секунды и признаёт READY только после completed initialized response с ожидаемым marker. Commit/reply доступен только bound receiver сервиса и accepted service token; requester SEND handle не может подделать ответ. Expired probe не означает READY. Эти длительности QEMU fixture — явные параметры политики, а не real-time guarantees.

Committed crashing service сообщает effect-unknown и exact fault observation. Supervisor запускает fresh instance и проверяет отказ предыдущего token и SEND handle. Silent service получает timeout до commitment и явно останавливается. Crash storm получает три выбранные EL0 replacement attempts с backoff 1/128 секунды по counter clock; затем supervisor прекращает retries, хотя native authority credits ещё остаются. Здоровый peer завершает native probes между сбоями worker.

Сервис отправляет подтверждение commit через feedback endpoint своего точного instance только после успешного COMMIT. Supervisor проверяет и отвечает на подтверждение до cancellation; прошедшие scheduler quanta не доказывают commitment. При accepted load supervisor наблюдает cancellation после commitment как effect-unknown, прекращает admission, запрашивает owner-local termination и ждёт точного terminal event. Он не обещает rollback и не повторяет операции с неизвестным эффектом. Final teardown закрывает все endpoints, завершает pending copies, дренирует source/mailbox ownership, освобождает namespaces/domains/ASIDs и private frames. Внешний test watchdog десять секунд диагностирует неудачный workload; он не доказывает успешный shutdown и не является production policy. Неожиданный отказ supervisor, watchdog expiry или failed quiescence останавливает/карантинирует development fixture без reclaim live resources; deadline никогда не разрешает освобождать reachable state.

## Проверка и следующий gate

[Exact-source execution receipt](../../../../research/results/supervision-phase36.json) содержит 125 DEV и 125 PROD checks, оба non-test boot и 132 rejected control runs. Шесть supervisor-specific mutations входят в 52 IPC/supervision controls. Последний source set различает фактические Terminated и BudgetExpired; два предыдущих development snapshots остаются неизменными. Technical QEMU verification не завершает proposed architecture или human EN/RU acceptance.

Реальные images проверяют восемь групп сценариев: ordered normal startup; missing dependency и forged grant selector; irreversible seal, прекращение unused launch grant, исчерпание instance credits, отказ unsupported version и repeated launch denial; forged readiness denial; crash, fresh restart и stale binding denial; startup timeout; bounded restart storm и healthy peer progress; shutdown under committed load. Bitmap возвращается control flow EL0 и проверяется вместе с actual process completion, освобождёнными CPU owners, отсутствием live domains/processes и восстановленным числом physical pages. Это не заменяет source review.

Focused mutations удаляют supervisor provenance, stale service-token rejection или retained IPC wait identity. Каждая должна вызвать named exact rejection в DEV и PROD. Все прежние kernel checks и controls остаются обязательными. [Runner](../../../../crates/xtask/src/main.rs) поддерживает `cargo xtask test --positive-only [--prod]` для итераций; эта команда не выполняет и не заменяет full matrix. Exact-source receipts записываются после full matrix. Перед закрытием issue #27 обязательны semantic и complete EN/RU human review. #28 отвечает за persistent service integration; #38 — за production bootstrap trust.

Исходники: [lifecycle mechanism](../../../../crates/kernel/src/supervision.rs), [EL0 images](../../../../crates/kernel/src/supervision_workload.S), [bootstrap fixture](../../../../crates/kernel/src/supervision_workload.rs), [process ownership](../../../../crates/kernel/src/process.rs), [checkpoint](../../../../crates/kernel/src/scheduler/mod.rs).

[English original](../../../../docs/kernel/supervision.md)

## Граница исходников при интеграции CI

Ветка интегрирует reviewed main `096977f9a434398130b6d18ddbd6cbba20f2116f`, включая общий serial/four-shard CI task inventory. Все шесть supervisor controls обязательны, поэтому plan содержит 136 задач. Matrix runner распознаёт точные supervision: failure events, а не любой panic. Предыдущие execution receipts сохраняют свои source scopes и имеют статус STALE для изменённых shared runner inputs до записи нового combined-source receipt. Human architecture/EN-RU acceptance остаётся открытой.

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
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "Real isolated EL0 supervisor and static service images; exact-authority lifecycle rendezvous, ordered readiness, fresh replacement, finite backoff/restart policy and under-load shutdown.",
        "sources": [
          "crates/kernel-core/src/process.rs",
          "crates/kernel-core/tests/process_protocol.rs",
          "crates/kernel/Cargo.toml",
          "crates/kernel/src/main.rs",
          "crates/kernel/src/process.rs",
          "crates/kernel/src/scheduler/mod.rs",
          "crates/kernel/src/scheduler/task.rs",
          "crates/kernel/src/supervision.rs",
          "crates/kernel/src/supervision_workload.S",
          "crates/kernel/src/supervision_workload.rs",
          "crates/kernel/src/tests.rs",
          "crates/xtask/src/main.rs",
          "crates/xtask/src/matrix.rs",
          "crates/xtask/src/output.rs",
          "crates/xtask/src/timing.rs"
        ],
        "acceptance": [],
        "issues": [27],
        "adrs": ["adr.0026"],
        "limitations": [
          "Development static manifest/image assurance only; production trust remains #38.",
          "Fixed affinity and two-CPU lifecycle barrier; generic wait-any, migration, independent live admission and persistent services are excluded.",
          "Semantic architecture and complete EN/RU human review remain pending; QEMU is not physical ARM64 acceptance."
        ],
        "next_gate": "Complete architecture/code semantic and full EN/RU human review before bounded Phase 3.6 acceptance; #28 owns persistence and #38 production bootstrap trust.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "STALE",
            "reason": "Merged CI runner/source changes invalidate the previous exact-source scope; immutable receipts are retained, and combined DEV/PROD verification is in progress.",
            "receipt": "research/results/supervision-phase36.json",
            "receipt_sha256": "81274a301885a7de99e87696394caa52e1ba01fe3c9a3aedc52bc82e489124ab",
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
          }
        ]
      }
    }
  ]
}
```
