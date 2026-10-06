# Проверка приёмки Phase 3.6

Document status: CURRENT
Evidence scope: проверка Codex по исходникам, запрошенная maintainer 2026-10-06, для merged main `9cf87bccff19349d181a1d5a8b09a3ab4033249f` и независимо изученных CI artifacts с актуальными исходниками. Независимая human review, production trust, persistent service и приёмка physical ARM не заявляются.
Current reference: [Контракт supervisor](../kernel/supervision.md)

## Решение и границы

Принять ADR-0026 и ограниченную основу lifecycle Phase 3.6 для следующего этапа Phase 3.7. Feature имеет BOUNDED_IMPLEMENTED и READY только для заявленного development workload: один точный изолированный supervisor EL0, конечные статические grants на image/CPU/quota, аутентифицированная readiness, fresh replacement, ограниченная restart policy и shutdown с учётом commitment. Publication stage native.lifecycle/1 остаётся EXPERIMENTAL и не заморожен. Проверка не принимает более широкий persistent-service workload или production bootstrap authorization.

Maintainer запросил завершение оставшейся проверки приёмки вместо сохранения pending label для merged implementation. Codex выполнил semantic и EN/RU review; отдельный human sign-off не выдумывается. Это же различие использовано в [проверке Phase 3.5](ipc-phase35-acceptance-review.md).

В этом acceptance update implementation source не меняется. Historical receipts сохраняют исходные review state и source scope. [Актуальный receipt](../../../../research/results/supervision-phase36-current.json) указывает каждый reviewed source digest и фактический ELF/result digest. Четыре скачанных shard независимо объединены в полный план 140 задач с актуальными исходниками: 125 именованных positive checks в каждом профиле, оба ordinary boots и 136 negative controls. Все десять supervision controls содержат зарегистрированный точный отказ, а не произвольный panic.

## Проверка соответствия требований и evidence

| Требование                                 | Обеспечение и evidence                                                                                                                                                                                                                                                                                                                                                                                                                      | Вывод                                                                                                                                                                                                                                    |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Изолированная policy и trusted bootstrap   | supervision_workload.S исполняет startup, dependency guard, readiness probes, три restart attempts с backoff и shutdown в EL0. Scope::install однократно за загрузку связывает immutable grants и точный ProcessId supervisor. capture проверяет slot/generation исполняемого Task и active scope до публикации initialized commands. Authority mutation вызывает отказ настоящего supervision scenario в обоих профилях.                   | LAW-009/LAW-018: числовые selectors не создают authority; EL1 обеспечивает конечный ceiling и не интерпретирует manifest policy. Static linked-image assurance предназначена только для development; #38 остаётся production trust gate. |
| Порядок зависимостей и честная readiness   | Общая launch-definition routine EL0 сравнивает все требуемые readiness bits до native launch. Readiness — completed native IPC reply от bound receiver; SEND не может подменять RECEIVE/COMMIT/REPLY. Dependency mutation даёт точный DependencyNotRejected; scenario отвергает forged replies и получает timeout для silent worker.                                                                                                        | LAW-043: создание следует аутентифицированным dependencies. Expiry, записанный caller bit и внешне здоровый process не доказывают readiness.                                                                                             |
| Restart identity и ограниченная authority  | Scope требует actual old completion и current token до replacement, закрывает прежние supervisor bindings, освобождает detached old process, затем выдаёт fresh ProcessId и nonwrapping token. Восемь worker credits и один peer credit конечны. Seal уничтожает unused initial grants; retirement отключает invocation. Оба профиля обнаруживают stale-token mutation.                                                                     | LAW-013/LAW-025: replacement не оживляет old namespace и не пополняет authority. Initial launch, replacement и failure observation привязаны к exact instance.                                                                           |
| Owner/root quiescence и retained IPC waits | Registry::checkpoint использует acquired completions двух fixed-affinity owners. Saved waits и continuations сохраняются только для exact ProcessId; endpoints/mailboxes имеют отдельные lifetimes. Wait-identity mutation даёт точный WaitIdentityLost. lifecycle_stop записывает exact-generation owner-local Terminated transition; ASID retirement предшествует frame reuse.                                                            | Detached root сам по себе не разрешает освобождение reachable IPC state. Healthy peers приостанавливаются во время membership changes; independent live admission, migration и wait-any не обещаются.                                    |
| Quota failure и unpublished rollback       | lifecycle_bind транзакционно устанавливает receiver/SEND/feedback handles и закрывает установленные handles при отказе. discard_prepared требует detachment и переводит Prepared непосредственно в retirement без fabricated completion. Настоящий third-grant exhaustion scenario проверяет восстановленные frames, stale identity и дальнейшее продолжение workload; process_protocol host tests проверяют state и identity restrictions. | Неудачное создание не публикует runnable partial child и не расходует успешный instance credit. Host state tests дополняют actual EL0 exhaustion, а не заменяют его.                                                                     |
| Fault containment, timeout и restart storm | Настоящий worker fault после COMMIT даёт EffectUnknown; completion observation различает Faulted, Exited, Terminated и BudgetExpired. EL0 выбирает три retries с counter-clock backoff при remaining native credits. Healthy peer probes завершаются до и после bounded storm.                                                                                                                                                              | Persistent autonomous orchestration и guaranteed peer progress между каждым отказом не заявляются. Silent-service timeout и explicit stop — отдельные outcomes.                                                                          |
| Shutdown под committed load                | Exact service отправляет feedback только после успешного COMMIT; supervisor проверяет его и отвечает до cancellation. Удаление commitment даёт точный CommitNotProven в обоих профилях. Scope retirement и final empty checkpoint дренируют source/mailbox ownership; исполнение проверяет bitmap 255, actual supervisor exit, released CPU owners, zero live processes/domains и restored physical pages.                                  | LAW-043: effect-unknown не означает rollback. Admission прекращается до termination; failed quiescence/watchdog карантинирует fixture и никогда не разрешает reclaim.                                                                    |

## Семантическое соответствие EN/RU

Проверены полные пары supervisor contract и ADR-0026 относительно supervision.rs, supervision_workload.rs, supervision_workload.S, lifecycle binding/retirement в process.rs, scheduler checkpoint/termination, process_protocol tests и exact failure runner. Обе локали сохраняют initial-grant provenance, selectors/tokens, version/operation values, initialized register replies, finite quotas/credits, seal, actual completion kinds, copy/wait retention, quiescence, backoff/deadlines, commitment outcomes и excluded scope. Утверждение о peer probes между worker failures исправлено в обеих локалях на фактические probes до и после storm. Это semantic review Codex, а не выдуманная независимая human linguistic assessment.

## Актуальность evidence и следующий gate

CI run [37433732057](https://github.com/lifeFedorovAlexey/KOLVRT/actions/runs/37433732057) исполнил final PR head `692b024968c4a1a6c5c524b73e2978f1e60f4d94`, merged как reviewed base выше. Все planned runtime source digests совпадают с current tree. Local aggregate verifier проверил shard coverage, source digests, profile/features, pinned compiler/QEMU arguments и build/run ELF identity. Независимое изучение artifacts дополнительно вычислило hash каждого ELF, проверило обе inventories из 125 имён и оба supervision results, а также нашло каждый registered negative witness. Один successful CI job не использовался как доказательство завершения.

Прежние timer и stripped-PROD failures сохраняются как historical evidence. Final main отделяет shared absolute workload deadline от existing coordination interval для remote completion publication; focused deadline-join control остаётся в полной matrix из 140 задач. Успешная bounded current-source matrix не доказывает universal race-freedom и не позволяет считать каждый historical failure исправленным по предположению.

Physical ARM остаётся UNKNOWN. Production trust принадлежит #38. Persistent counter service, standalone ELF applications, userspace selftests и их system acceptance принадлежат #28 и не реализованы этой проверкой. Будущую архитектуру нужно выводить заново из текущих инвариантов; приёмка не замораживает раннюю реализацию.

[English original](../../../../docs/architecture/supervision-phase36-acceptance-review.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.supervision.acceptance-review",
  "kind": "security-analysis",
  "summary": "Приёмка ограниченной Phase 3.6 и semantic review EN/RU по исходникам, отдельно от persistent services и production trust.",
  "relationships": [
    {
      "type": "related_to",
      "to": "kolvrt.services.supervision",
      "scope": "Bounded lifecycle readiness and source-matched QEMU evidence; no physical ARM or stable ABI acceptance."
    }
  ]
}
```
