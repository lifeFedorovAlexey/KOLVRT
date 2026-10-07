# Native ELF applications и постоянный сервис

Document status: CURRENT
Evidence scope: экспериментальная реализация Phase 3.7; прежние проверки копий service/supervisor и изменённых исходников не подтверждают production E2E. Полная acceptance не завершена.
Current reference: [Принятая основа supervision](supervision.md)

<a name="native-elf-applications"></a>

## Срез native-приложений

Обычные counter-service, counter-client и native-supervisor — отдельные no_std/no_main ELF в apps/native-apps. Kernel получает полные исходные ELF bytes и использует настоящий loader; counter protocol и ожидаемые значения проверяются приложениями и host harness. GET и checked ADD(delta) используют private userspace state; RESET отсутствует. ADD(5), ADD(7), GET дают 5, 12, 12. Нормальный сервис не содержит намеренного падения, test request counter или повреждения reply.

Finite client SEND binding использует существующий checkpoint supervision и точные instance tokens. Kernel предоставляет generic process/ELF/IPC/authority/lifetime механизмы. Обычная реализация не должна содержать callbacks приложения или тестовые вмешательства. Hardware, disk durability и stable ABI не заявляются.

## Классификация тестов и единственная реализация

| Прежняя проверка                               | Уровень и проверяемый production-код                               | Отдельный ELF и решение                                                               |
| ---------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| native-userspace tests                         | UNIT: Counter::apply, CounterRequest::decode, frame и reply codecs | Не нужен; apps/native-runtime/tests/counter.rs импортирует настоящие методы           |
| selftest-process                               | SYSTEM: kernel ELF/process/EXIT/fault containment                  | Внешний lifecycle-peer реально падает, а настоящий сервис продолжает отвечать         |
| selftest-ipc                                   | INTEGRATION/SYSTEM: настоящий native IPC через public SDK          | Внешний ABI-клиент допустим; production service и supervisor остаются настоящими      |
| selftest-service                               | SYSTEM: настоящий kernel/service и recover_service; test root      | Дублирующий ELF удалён; stop/replacement/stale/shutdown проверены через lifecycle ABI |
| selftest-elf                                   | E2E: настоящий loader и application ELF                            | Используется обычный counter-client                                                   |
| smoke-client                                   | E2E: обычные приложения и IPC                                      | Дублирующий ELF удалён; app-smoke выбирает counter-client                             |
| selftest-counter-service / selftest-supervisor | Копии production-реализаций                                        | Удалены; не считаются test instruments                                                |
| isolated_build / mutations.json                | Изменение копий kernel source                                      | Удалены; прежние controls не подтверждают production execution                        |

В tests/native-apps остаются только внешние ABI/fault actors. Они содержат тестовые входы, последовательности public calls и assertions; не реализуют counter service, обычный client или supervisor. Fault actor содержит собственную illegal instruction. Рабочий SDK не содержит test-only fault helper. Kernel/application source copies не создаются.

Production supervisor теперь проверяет настоящий terminal outcome через lifecycle API перед replacement, сохраняет принятую failure backoff и проверяет readiness новой instance реальным GET(0). После service RPC failure обычный client запрашивает новую binding, требует новую identity и initial value 0, отвергает старый SEND и повторяет бизнес-операции только в новой instance. Effect-unknown ADD не повторяется в прежней instance. needs_replacement, fresh_binding и SessionPolicy::decode импортируются UNIT-тестами; recover_service импортируется integration actor, а не копируется.

Текущий список внешних ELF указан явно: selftest-abi-client вызывает настоящий native-userspace IPC API с production service/supervisor; selftest-lifecycle-client вызывает public lifecycle/IPC API и импортирует native-apps::supervision::recover_service; selftest-lifecycle-peer проверяет binding, stale SEND и fault containment. Каждый является тестовым клиентом без production-аналога. Копии production supervisor нет. Lifecycle SYSTEM-сценарий использует selftest-lifecycle-client как root вместо native-supervisor, поэтому production_supervisor_tested=false: production supervisor в этом сценарии не выполняется. Используется настоящий counter-service. Неиспользуемый selftest-fault-client и мёртвые native-*-negative features удалены; выполняемый внешний fault scenario предоставляет lifecycle-peer. Host UNIT-тесты apps/native-apps/tests/supervision.rs импортируют needs_replacement и fresh_binding без ELF.

CI на exact 7d64fac остановился на user-context (shard 0), retirement (shard 1), shootdown (shard 2) и remote-tlbi (shard 3). Компиляция retirement исправлена в 71753ae. Затем static этого коммита упал из-за устаревшего final manifest digest в graph; c77f6e8 перегенерировал точный graph и прошёл CI static. Четыре corruption-based controls теперь перенесены согласно карте ниже. Остальные legacy controls ещё проверяются полной matrix; отдельные passes не подтверждают зелёный foundation или полную acceptance Phase 3.7.

| Перенесённый control | Настоящие входы / observations                                                                                                       | Предел evidence                                                                  |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| user-context         | valid_user_context отвергает режимы EL1, AArch32 и masked IRQ; обычные EL0 context preservation и timer switches                     | Нет повреждения saved frame; validator + реальные round-trip checks              |
| user-root            | настоящие EL0 reads kernel/foreign memory приводят к fault; private-memory isolation и clean reclaim                                 | Настоящие запрещённые accesses; нет sabotage root tables                         |
| shootdown            | живой secondary reader сохраняет pending retirement; frame не переиспользуется до ACK; release/reuse следуют за ACK                  | Наблюдаемый retirement ordering; нет намеренно потерянного ACK или timeout claim |
| user-retirement      | Registry::reclaim отвергает живой process; state и удержанные frames не меняются; окончательное reclamation проверено                | Настоящий public ownership API; private frame не извлекается                     |
| remote-tlbi          | acknowledged отвергает будущую generation; настоящий TLBI observer предшествует ACK; access к retired mapping даёт translation fault | Instrumented test build; нет skipped-TLBI mutation claim                         |

Эти controls используют обычную kernel-tests сборку и требуют каждую mapped test ровно один раз со status pass. Missing, duplicate, failed и unrelated observations отвергаются. Task inventory сохранён. Четыре старые corruption features удалены. Обычный suite теперь содержит 147 checks на profile. REMOTE_TLBI_COMPLETED — только observation instrumentation в kernel-tests; эта evidence не является byte-identical production-binary execution. Новый production crash/recovery/shutdown сценарий отделён от этих обычных kernel checks.

Следующие старые controls классифицированы явно. completion-publication-state-inputs импортирует production Ownership state machine, отвергает completion при удерживаемом access или отсутствии quiescence, отвергает inspection до Done и проверяет сохранение phase/generation. Эта UNIT evidence не проверяет зависшую publication удалённого CPU или её liveness/failure handling. process-exit вызывает единственный production Table::complete с повторным completion и проверяет сохранение terminal record, state и live-slot count; test-only wrapper Registry::repeat_completion удалён. process-rollback использует настоящие creation failures при нулевом memory budget и проверяет resource rollback. asid-reuse-invariants наблюдает настоящие same-VA isolation, invalidation counters и pool exhaustion; это не negative control пропущенного invalidation.

shootdown-invariants и remote-tlbi-invariants являются positive invariant coverage; прежние control flags остаются CLI aliases. Они не подтверждают обнаружение отсутствующего ACK или пропущенного TLBI. Plan и archived counts различают ordinary, negative-input, invariant и оставшиеся legacy-failure tasks. SYSTEM coverage зависшей publication/отсутствующего ACK/пропущенного invalidation остаётся вне этого покрытия; отдельный production crash/recovery/shutdown E2E не устанавливает обнаружение пропущенного TLBI или отсутствующего ACK.

Семейство process transitions теперь использует явные negative inputs настоящих production-методов: Table::reclaim без quiescence сохраняет completed record/live-slot count (UNIT); Registry::start для уже admitted process сохраняет state/frames; stale IDs отвергаются при start/data/receipt validation после настоящего slot reuse; Registry::reclaim живого process сохраняет state/frames. Намеренный scheduler unlink не заявляется. Старые process-contract features и ранние panic branches удалены.
Семейство scheduler теперь разделяет три уровня доказательств. Registry::create отвергает входные контексты AArch32, привилегированный режим и замаскированный пользовательский IRQ до выделения памяти; число процессов и доступных страниц не меняется. UNIT-тесты Ownership вызывают единственный production-протокол с чужим CPU, разрешённым IRQ, повторным входом при удержанном доступе, устаревшим поколением, повторным start, сбросом работающего поколения и преждевременной инспекцией; фаза и поколение сохраняются. UNIT-тесты адаптера создают настоящие fixture-значения Local/Task/State и вызывают реальные методы с удержанным lock, lock внутри callback владения, устаревшим поколением задачи либо уже работающей соседней задачей. Эти четыре фатальные проверки выполняются в DEV и PROD; host требует свежий guest panic в месте единственной соответствующей production-проверки, поскольку PROD не выводит диагностическое сообщение. Тела реализации не копируются и не изменяются. Совместимый флаг IRQ/SIMD теперь выбирает инвариант настоящего round-trip через прерывание; он не доказывает обнаружение пропущенного восстановления. Эти тесты не устанавливают полноту аппаратного fault coverage адаптера или текущую приёмку Phase 3.7.

CI на точном d33ce92 прошёл static, native, host, kernel DEV/PROD, routing и ASID, но все matrix shard упали: scheduler-user-irq (0), scheduler-context (1), scheduler-inner-lock (2), scheduler-aarch32 (3). Итоговый foundation упал, evidence был пропущен. Миграция scheduler исправляет эти устаревшие controls и соседние проверки семейства; полная текущая matrix всё ещё обязательна.

Оставшиеся no-op selectors мутации user-copy, handles, domain/capability и IPC удалены. Controls используют настоящие production API и существующие реальные EL0 отрицательные входы/инварианты; добавлены UNIT-входы retirement Namespace, неверного поколения Mailbox и неправильного consumer/token/повторного terminal transition Endpoint. Пять фатальных входов адаптера IPC прошли через matrix runner в DEV/PROD (десять выполнений): storage внутри scheduler ownership, публикация в READY, wake чужого процесса, unlink заблокированной задачи и drop закрытого endpoint с удержанной ссылкой. Отказ публичного повторного terminal transition сохраняет outcome/accounting; мутация приватного второго release charge не заявляется. Текущий plan различает отрицательные входы, инварианты, смешанное покрытие и по-прежнему выполняемые legacy failure/oracle checks. Архив измерений теперь сохраняет текущую полную matrix execution и классы покрытия, а не требует отказа от каждой IPC-задачи. ipc-controls выбирает все зарегистрированные IPC-задачи, включая мигрированные положительные/инвариантные проверки.

CI на точном 0439387 прошёл scheduler и затем упал на user-copy-recovery (0), handle-generation (1), handle-owner (2), handle-type (3). Static/native/host/kernel DEV/PROD/routing/ASID прошли; evidence пропущен, итоговый foundation красный. Общая миграция исправляет этот scope и соседние семейства; полное текущее выполнение 144 задач идёт и ещё не объявлено пройденным.

По явному требованию maintainer Phase 3.7 убирает произвольное двухсекундное ограничение из настоящего SMP wait и путей scheduler completion/copy-drain. Acquired completion, detached roots, released owners и transport drainage остаются обязательными; опубликованный secondary failure по-прежнему останавливает работу с удержанием ресурсов. Конечного внутрядерного детектора молча зависшего CPU в этой foundation нет; host watchdog диагностирует тесты и не доказывает completion или reclaim. Workload expiry и canonical supervisor restart backoff 1/128 секунды остаются отдельными policy. Исторический timeout evidence #126 не является current-source acceptance.

### SYSTEM-сценарий crash-recovery production supervisor

cargo xtask crash-recovery (также обязателен в selftest) собирает настоящие native-supervisor, counter-service и counter-client. tests/system/native-crash-recovery.cjs проверяет digests оригинальных ELF и выбранного kernel, затем использует внешние QEMU GDB hardware breakpoints на публичном IPC SVC сервиса. После ответа readiness и фактической доставки клиентского запроса, перед первым ADD COMMIT, harness меняет только EL0 PC CPU1 на unmapped zero. Исходники и ELF bytes остаются неизменными; test entry point, копия реализации или отдельный test ELF не нужны.

Oracle требует реальный completion старого сервиса с lower-EL instruction abort, свежую ProcessId generation сервиса, успешный exit того же production client и итоговое значение counter 12 с fresh token при продолжении production supervisor. Настоящий client проверяет отказ старого SEND, initial value 0 и ADD(5)/ADD(7)/GET после recovery через настоящий supervisor main loop. Обычный supervisor теперь завершает сессию после успеха настоящего recovered client: наблюдает owner-local service termination и штатно выходит. Затем тот же guest освобождает ownership, восстанавливает frames и сообщает zero live processes/domains до poweroff. DEV/PROD executions прошли локально. Host oracle отдельно отклоняет обычный успех без fault, неверные CPU/EL, остановленный сервис, stale replacement, посторонний panic, неправильный client completion, поддельную report signature и неверный порядок событий. Captured observation fixture — вход HOST oracle, а не SYSTEM evidence.

Это выбранное post-binding, pre-commit crash coverage. Оно устанавливает normal production-session shutdown/reclamation после выбранного recovery. Startup-crash recovery, post-commit replay, performance и physical ARM не устанавливаются. Старый lifecycle test root по-прежнему имеет production_supervisor_tested=false; новый отдельный сценарий сообщает true, не переписывая прежний scope.

CI на точном 17ac5e9 прошёл все 144 tasks и полный foundation. Более поздний 66b497a упал на shard 2 внутри настоящего supervision cancellation fixture (AlreadyTerminal, coverage 127); остальные shard и обычные workloads прошли. Его load и commit-ack запросы ошибочно использовали короткий deadline readiness probe, разрешая expiry победить до требуемого Cancel. Эти тестовые входы теперь задают максимальный допустимый absolute deadline (zero недопустим/Expired по native ABI); readiness deadlines и canonical restart backoff 1/128 сохранены. Тест по-прежнему требует реальный commitment acknowledgement, успешную явную cancellation и EffectUnknown, не принимая expiry за cancellation. Production IPC transitions и timing policy не меняются. Новые exact-source полный CI/acceptance пока ожидаются.

Прежний CI b037833 упал на matrix shard 1 в supervision_workload во время initial readiness, а не в observable process-stale guards. Остальные три shard, static, host, native, routing, ASID и kernel DEV/PROD прошли; native включает настоящий production crash-recovery SYSTEM-сценарий. Диагностика probe stages/timestamps теперь сохраняет принятый readiness deadline для расследования admission expiry до выбора исправления. Итоговые CI и current-source acceptance пока ожидаются.

Последний завершённый CI cc50a5a упал только на matrix shard 0 в process_quantum_return_and_peer_progress; три остальных shard и static/host/native/routing/ASID/kernel DEV/PROD прошли. Readiness в этом запуске не упала. Quantum fixture теперь инициализирует x20 согласно переданному TPIDR и сохраняет каждую execution/context/resource check, добавляя detailed failure-bit diagnostics. Полный CI нового source и retained acceptance остаются обязательными.

## Команды и пределы evidence

Обычный supervisor принимает application SessionPolicy argument 1 (Persistent) или 2 (FinishAfterClient), отвергая неизвестную конфигурацию. Это настоящие политики времени жизни приложения, а не test implementations. cargo xtask service-run выбирает Persistent и наблюдает сервис после client exit; host VM stop не измеряет reclamation. cargo xtask app-smoke выбирает FinishAfterClient, проверяет настоящий client success, owner-local service termination и нормальный supervisor exit, затем требует acquired owner release, restored frames и zero live processes/domains от generic kernel retirement. В immutable ELF kernel bootstrap больше нет произвольного watchdog; зависания тестов ограничивает внешний runner.

cargo xtask selftest запускает production component tests, обычные завершённые сессии, persistent runtime scenario и внешний ABI-client в DEV/PROD. ABI actor проверяет denied operations, Submit/Wait/Collect, consumed receipt и три counter requests. Команда также выполняет lifecycle integration через настоящие kernel и counter-service. Внешний клиент импортирует единственный production recover_service, останавливает сервис через public owner API, вызывает recovery, проверяет новую identity, old token/SEND rejection, fresh state 0 и clean reclamation. Отдельный ABI actor действительно падает; последующий ответ настоящего сервиса подтверждает fault containment. Этот прежний lifecycle scope не исполняет production supervisor loop; отдельный внешний crash scenario теперь проверяет настоящие supervisor recovery и normal shutdown.

cargo xtask native-controls проверяет неверные входы настоящих ELF/parser/policy методов, реальные denied/stale/binding/IPC calls и resource invariants. UNIT изменения observation fixtures проверяют тот же host oracle; это не execution failures ядра. Source mutations запрещены. scripts/native-closure-proof.cjs подключает оригинальные implementation directories через directory references, проверяет их физическую identity, отсутствие несвязанных crates в workspace, отсутствие external native packages и выполняет DEV/PROD runtime/lifecycle. Копируется только build metadata; копий реализации нет. Host runner dependency closure исключена из этого scope. Старые receipts остаются историческими.

L1 проверяет компоненты, L2 — внешние ABI actors, L3 — production QEMU system scenarios. Ни один из этих уровней не доказывает L4 physical ARM: NOT_RUN/UNKNOWN. Filesystem, initramfs, storage, DMA, network, package manager, Linux ABI и dynamic linking остаются вне scope.

Полная acceptance Phase 3.7 остаётся незавершённой до текущего полного foundation и финального exact-source semantic/EN-RU review. Настоящий crash path production supervisor теперь выполняется отдельным внешним post-binding pre-commit SYSTEM-сценарием. Lifecycle stop/replacement/stale/fresh binding/peer fault/reclamation сохраняют отдельный test-root scope. Normal production-session shutdown/reclamation теперь выполняется; ограничения startup-crash и post-commit replay остаются явными. Исторические receipts не переименовываются в current-source passes.

[English original](../../../../docs/kernel/native-applications.md)

Быстрый COMPONENT layer дополнительно импортирует настоящий общий kernel_core::supervision::validate_grants, вызываемый Kernel Scope::install: empty images, zero/excess instance credits и self/missing SEND bindings отвергаются. Единственный production тип Grant заменяет прежнее kernel-local определение и inline validation; модель/копия для этих тестов не создаётся. Отдельные настоящие ELF, lifecycle, domain/quota и application protocol методы сохраняют собственные input tests.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.native-applications",
  "kind": "subsystem-contract",
  "summary": "Требования Phase 3.7 к standalone ELF/native IPC applications и persistent counter service.",
  "units": [
    {
      "id": "kolvrt.apps.native-elf",
      "anchor": "native-elf-applications",
      "kind": "feature",
      "summary": "Standalone native ELF applications, private persistent counter state и layered userspace acceptance.",
      "depends_on": [
        "kolvrt.services.supervision",
        "kolvrt.ipc.transport",
        "kolvrt.process.lifecycle",
        "law.009",
        "law.013",
        "law.018",
        "law.025",
        "law.041",
        "law.043",
        "adr.0026",
        "law.044"
      ],
      "feature": {
        "implementation": "EXPERIMENTAL",
        "implementation_scope": "Original standalone no_std ELF supervisor/service/client loaded by the actual kernel ELF loader. One production SDK/service/supervisor implementation; external ABI actors only. Actual persistent requests and post-binding pre-first-COMMIT fault/recovery/fresh binding followed by normal production-session shutdown and acquired reclamation. SessionPolicy selects persistent or finish-after-client lifetime through an opaque application bootstrap argument; no kernel watchdog or application-specific callbacks.",
        "sources": [
          ".github/workflows/ci-windows-workload.yml",
          ".github/workflows/kernel.yml",
          "Cargo.lock",
          "Cargo.toml",
          "apps/native-apps/Cargo.toml",
          "apps/native-apps/build.rs",
          "apps/native-apps/linker.ld",
          "apps/native-apps/src/client.rs",
          "apps/native-apps/src/counter-client.rs",
          "apps/native-apps/src/counter-service.rs",
          "apps/native-apps/src/lib.rs",
          "apps/native-apps/src/supervision.rs",
          "apps/native-apps/src/supervisor.rs",
          "apps/native-apps/tests/supervision.rs",
          "apps/native-runtime/Cargo.toml",
          "apps/native-runtime/src/lib.rs",
          "apps/native-runtime/tests/counter.rs",
          "crates/kernel-core/Cargo.toml",
          "crates/kernel-core/src/domain.rs",
          "crates/kernel-core/src/elf.rs",
          "crates/kernel-core/src/handles.rs",
          "crates/kernel-core/src/ipc.rs",
          "crates/kernel-core/src/ipc/identity.rs",
          "crates/kernel-core/src/lib.rs",
          "crates/kernel-core/src/process.rs",
          "crates/kernel-core/src/supervision.rs",
          "crates/kernel-core/src/wait.rs",
          "crates/kernel-core/tests/bootstrap_grants.rs",
          "crates/kernel/Cargo.toml",
          "crates/kernel/build.rs",
          "crates/kernel/src/arch/aarch64/context.rs",
          "crates/kernel/src/arch/aarch64/entry.S",
          "crates/kernel/src/arch/aarch64/mod.rs",
          "crates/kernel/src/asid.rs",
          "crates/kernel/src/boot_workload.rs",
          "crates/kernel/src/handles/testing.rs",
          "crates/kernel/src/ipc/deferred.rs",
          "crates/kernel/src/ipc/native.rs",
          "crates/kernel/src/ipc_benchmark.rs",
          "crates/kernel/src/ipc_workload.rs",
          "crates/kernel/src/main.rs",
          "crates/kernel/src/memory/mod.rs",
          "crates/kernel/src/native_boot.rs",
          "crates/kernel/src/platform/config.rs",
          "crates/kernel/src/process.rs",
          "crates/kernel/src/process_workload.rs",
          "crates/kernel/src/scheduler/local.rs",
          "crates/kernel/src/scheduler/mod.rs",
          "crates/kernel/src/scheduler/task.rs",
          "crates/kernel/src/security.rs",
          "crates/kernel/src/security/testing.rs",
          "crates/kernel/src/smp.rs",
          "crates/kernel/src/supervision.rs",
          "crates/kernel/src/supervision_workload.S",
          "crates/kernel/src/supervision_workload.rs",
          "crates/kernel/src/sync/mod.rs",
          "crates/kernel/src/test_support/arch_probes.S",
          "crates/kernel/src/test_support/ipc_inputs.rs",
          "crates/kernel/src/test_support/scheduler_inputs.rs",
          "crates/kernel/src/tests.rs",
          "crates/kernel/src/user_copy.rs",
          "crates/kernel/src/user_copy/testing.rs",
          "crates/xtask/src/main.rs",
          "crates/xtask/src/matrix.rs",
          "crates/xtask/src/native_apps.rs",
          "package.json",
          "scripts/ci-gate.cjs",
          "scripts/ci-workload.ps1",
          "scripts/native-closure-proof.cjs",
          "scripts/tests/ci-infrastructure.test.cjs",
          "tests/fixtures/native-lifecycle-observations.json",
          "tests/fixtures/native-production-crash-observations.json",
          "tests/fixtures/native-production-crash-shutdown-observations.json",
          "tests/fixtures/native-production-session-observations.json",
          "tests/native-apps/Cargo.toml",
          "tests/native-apps/build.rs",
          "tests/native-apps/src/abi-client.rs",
          "tests/native-apps/src/abi_steps.rs",
          "tests/native-apps/src/lifecycle-client.rs",
          "tests/native-apps/src/lifecycle-peer.rs",
          "tests/system/native-crash-recovery.cjs",
          "tests/system/native-crash-recovery.test.cjs"
        ],
        "acceptance": [],
        "issues": [28],
        "adrs": ["adr.0026"],
        "limitations": [
          "Physical ARM NOT_RUN/UNKNOWN; no production trust, filesystem, disk durability, migration, generic spawn or stable ABI.",
          "Final exact-source receipts, physical-absence proof review and complete foundation regression acceptance are pending."
        ],
        "next_gate": "Complete current-source full foundation, retained exact-source receipt and semantic/EN-RU acceptance. Selected production crash/recovery/normal shutdown executes; startup and post-commit replay are outside this slice. No issue closure while required gates remain pending.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "UNKNOWN",
            "reason": "Phase 3.7 changes actual application session lifetime, external production crash/recovery/shutdown oracle, generic bootstrap and quantum fixture inputs. Prior receipts retain their earlier source scope; new full exact-source evidence and semantic review remain required."
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical ARM execution; NOT_RUN."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Begin Phase 3.7 on merged current main after #125/#126; register the partial scope without claiming application/QEMU acceptance.",
            "acceptance": []
          }
        ]
      }
    }
  ]
}
```
