# Изолированный supervisor EL0

Document status: CURRENT

Evidence scope: исторически принятая bounded Phase 3.6 после #126; Phase 3.7 меняет source и coordination policy. Current-source acceptance остаётся незавершённой; readiness и performance отдельны.

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

## Политика readiness, restart и shutdown

Политика EL0 отправляет readiness probe точному endpoint с absolute counter deadline 1/8 секунды и признаёт READY только после completed initialized response с ожидаемым marker. Commit/reply доступен только bound receiver сервиса и accepted service token; requester SEND handle не может подделать ответ. Expired probe не означает READY. Эти длительности QEMU fixture — явные параметры политики, а не real-time guarantees.

Committed crashing service сообщает effect-unknown и exact fault observation. Supervisor запускает fresh instance и проверяет отказ предыдущего token и SEND handle. Silent service получает timeout до commitment и явно останавливается. Crash storm получает три выбранные EL0 replacement attempts с backoff 1/128 секунды по counter clock; затем supervisor прекращает retries, хотя native authority credits ещё остаются. Здоровый peer завершает native probes до и после bounded crash storm.

Сервис отправляет подтверждение commit через feedback endpoint своего точного instance только после успешного COMMIT. Supervisor проверяет и отвечает на подтверждение до cancellation; прошедшие scheduler quanta не доказывают commitment. При accepted load supervisor наблюдает cancellation после commitment как effect-unknown, прекращает admission, запрашивает owner-local termination и ждёт точного terminal event. Он не обещает rollback и не повторяет операции с неизвестным эффектом. Final teardown закрывает все endpoints, завершает pending copies, дренирует source/mailbox ownership, освобождает namespaces/domains/ASIDs и private frames. Внешний test watchdog десять секунд диагностирует неудачный workload; он не доказывает успешный shutdown и не является production policy. Неожиданный отказ supervisor, watchdog expiry или failed quiescence останавливает/карантинирует development fixture без reclaim live resources; deadline никогда не разрешает освобождать reachable state.

## Проверка и следующий gate

[Новая проверка приёмки](../architecture/supervision-phase36-acceptance-review.md) выполнена относительно merged main ddd526cc5a522d97029d0324e6e619f5fd6ec6e5 после #126. Codex повторно проверил полные architecture/code и EN/RU semantic requirements, включая обязательный publication bound для checkpoint(None), отдельный copy-drain bound и различие relaxed/ordered counter reads. Functional implementation — BOUNDED_IMPLEMENTED. Readiness остаётся NOT_READY, без введения нового числового performance gate и без запрета использования функциональной основы в #28.

[Fresh current-source receipt](../../../../research/results/supervision-phase36-main126.json) независимо проверяет CI run 37439023443 на final #126 head cceb17e74a7a249d76d0f10905de23205f69b1dd: все source digests совпадают, проверены фактические ELF/result hashes и полный inventory 142 задач — 125 checks в каждом профиле, оба ordinary boots, 138 controls. Все десять supervision controls сохранены. Новые DEV/PROD publication controls требуют одновременно CheckpointPublicationHeld с quiescent=true и CompletionPublicationTimeout; чужой panic не засчитывается.

[Старый receipt](../../../../research/results/supervision-phase36-current.json) и [предложение до #126](../architecture/supervision-phase36-pre126-review.md) сохранены только для исторического scope 9cf87bc/692b024. Они не подтверждают текущие исходники.

[Исходные performance observations](../../../../research/results/checkpoint-publication-bound.json) показывают median +1.57% с tagged ASID и +6.64% с ASID-zero. Sequential before/after runs при uncontrolled host load объединяют lifecycle correction и counter refinement; attribution к ISB и отсутствие регрессии не доказаны. Строгий первоначальный PROD IPC benchmark также отказал на blocked_requester_wait с block_delta=0. Failure сохранён и control не ослаблен. Performance acceptance не заявляется; это отдельная открытая область, не новый универсальный процентный threshold.

Восемь real EL0 scenario groups проверяют ordered startup, dependency/authority denial, finite grants/seal, forged readiness, crash/fresh restart/stale binding, startup timeout, bounded storm и committed-load shutdown. Bitmap 255 дополнен actual completion, released CPU owners, zero live processes/domains и restored physical pages. Host tests не заменяют EL0; QEMU не является physical ARM. Старые timer/stripped-PROD failures остаются в истории; universal race-freedom не заявляется. #28 — persistent service integration; #38 — production bootstrap trust.

Исходники: [lifecycle mechanism](../../../../crates/kernel/src/supervision.rs), [EL0 images](../../../../crates/kernel/src/supervision_workload.S), [bootstrap fixture](../../../../crates/kernel/src/supervision_workload.rs), [process ownership](../../../../crates/kernel/src/process.rs), [checkpoint](../../../../crates/kernel/src/scheduler/mod.rs).

[English original](../../../../docs/kernel/supervision.md)

## Граница исходников при интеграции CI

Current main после #126 содержит 142 задачи и 138 controls из общего serial/four-shard inventory. Publication failure не возвращает фиктивный success: resources остаются retained до acquired quiescence. Новая verification относится к этому exact source set; historical inventories не переписываются. Performance и readiness остаются отдельными от functional acceptance.

Phase 3.7 расширяет механизм immutable image-format grants и operation 6 для одной конечной client SEND edge. Инициализированные x0=6, x1=client selector, x2=точный client token, x3=точный destination token, x4=1 обрабатываются только для захваченного supervisor после acquired quiescence. Успех возвращает status, SEND handle в локальном namespace клиента, target token и client generation; заменяется только previous sender, выданный этой entry. Stale tokens, завершённые clients/targets и отсутствующие edges отвергаются. Этот профиль исходных ELF описан в [native applications](native-applications.md); прежние ограничения fixtures Phase 3.6 и исторические receipts сохраняют свой исходный scope.

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
        "implementation_scope": "Real isolated EL0 supervisor and static service images; exact-authority lifecycle rendezvous with actual acquired completion, root/owner quiescence and copy/source/ack drainage. Phase 3.7 removes arbitrary coordination cutoffs at the maintainer request; published CPU failure retains resources, and silent stalls require external diagnosis. Ordered readiness, fresh replacement, finite backoff/restart policy and under-load shutdown remain separate application policies.",
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
          "crates/xtask/src/timing.rs"
        ],
        "acceptance": ["research/results/supervision-phase36-main126.json"],
        "issues": [27],
        "adrs": ["adr.0026"],
        "limitations": [
          "Development static manifest/image assurance only; production trust remains #38.",
          "Fixed affinity and two-CPU lifecycle barrier; generic wait-any, migration, independent live admission and persistent services are excluded.",
          "Codex semantic/EN-RU review completed after #126; performance attribution/admissibility and physical ARM/production acceptance remain open."
        ],
        "next_gate": "Phase 3.7/#28 persistent ELF integration over the accepted bounded functional foundation; performance readiness and production trust remain separate review scopes.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "STALE",
            "reason": "Phase 3.7 changes coordination policy and removes source-copy mutations/application copies. Prior receipts remain immutable historical evidence; current-source full supervisor crash/recovery and final acceptance are incomplete.",
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
