# Проверка committed-crash fixture и обновление приёмки Phase 3.7

Document status: HISTORICAL MILESTONE
Evidence scope: проверка исходников, кода, архитектуры и полного соответствия EN/RU, выполненная Codex по запросу сопровождающего для реализации 43af402b185afab82177713ed3076595d7e648a6, основы bf54960925559d8925c24b6d40e557e98af08f41 и независимо проверенного текущего CI. Независимое человеческое одобрение, физический ARM и приёмка производительности не заявляются.
Current reference: [Native-приложения](../kernel/native-applications.md)

## Решение и точное исправление

Обновить [прежнюю ограниченную приёмку первого сценария](native-phase37-acceptance-review.md) для исправленных исходников. Это исправление не меняет production SDK, ELF-приложения, loader, переходы IPC, scheduler и поведение recovery. ABI остаётся экспериментальным; готовность к production и доверие образам отделены.

[Сбой 333726b](../../../../research/results/native-phase37-333726b-failure.json) сохранён: stage 1002, операция Submit=1, Expired=16, coverage 15. Исходник определяет payload operation 2 — запрос committed-crash, а не readiness probe. Его короткий readiness deadline разрешал expiry до admission, поэтому тест не мог дойти до настоящего COMMIT/fault, который должен был проверить. Native DEV/PROD прошли независимо в том неуспешном полном запуске; эти результаты не разрешали объявлять foundation зелёным.

Assembly fixture теперь выбирает максимальный допустимый absolute deadline для operation 2, как уже делал для отдельного сценария Cancel. Это вход без намеренного expiry для проверки подтверждённого отказа, а не новый production timeout или принятие результата Expired. Readiness operation 1 сохраняет /8, включая намеренный expiry молчащего сервиса. Настоящие COMMIT, EffectUnknown, completion Faulted, свежая identity, отказ старым ссылкам, bitmap 255 и acquired освобождение ресурсов остаются обязательными. Production special case, задержки, повторы до зелёного результата и изменения копий исходников не добавляются.

## Выполненные проверки и review

[Полный CI 37571776305](https://github.com/lifeFedorovAlexey/KOLVRT/actions/runs/37571776305) прошёл все четыре shard и каждый обязательный workload, evidence и foundation на 43af402b185afab82177713ed3076595d7e648a6. Dependency hygiene прошёл отдельно. [Новый receipt](../../../../research/results/native-phase37-43af402.json) независимо проверяет 144/144 выполненных задач, совпадение ELF сборки и запуска, 151 входной исходник и восемь неизменяемых исходных артефактов. Входы плана гостя отделены от семи проверенных workflow/tooling входов checkout. Native DEV/PROD снова подтверждают постоянное обслуживание запросов и настоящий production crash/recovery/fresh binding/counter 12 с последующим штатным shutdown без утечек. Исправленное доказательство выполнения в ограниченном workspace прошло.

Три локальных повтора DEV+PROD прошли все 147 checks/profile. npm run check, docs generate/pilot, запись полного проверенного EN/RU перевода, source-bound impact/check-change относительно указанной основы и проверки diff прошли. Изменённый выбор deadline проверен по разным assertions readiness, committed-crash и cancellation и принятой семантике IPC expiry. Оба языка сохраняют одинаковые входы, точные результаты, исторические failures и исключения. Прежняя проверка исходников, кода и архитектуры остаётся применимой к неизменённому production-коду; эта review покрывает новое изменение fixture и свежесть evidence.

## Сохранённые ограничения

Покрытие сохраняет четыре ordinary, десять legacy-failure, двадцать один invariant, девяносто три negative-input и шестнадцать mixed задач. Положительные проверки TLBI/IRQ/ASID/shootdown не доказывают обнаружение пропущенной операции. Обновление не заявляет исправление каждого исторического finite-readiness expiry, всеобщие гарантии latency, startup-crash/post-commit replay, сохранность данных на диске, доверие образам, замороженный ABI, физический ARM или допустимость изменений производительности. Прежние успешные и неуспешные receipts сохраняют свой точный исходный scope.

[Английский оригинал](../../../../docs/architecture/native-phase37-fixture-review.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.native-applications.fixture-review",
  "kind": "security-analysis",
  "summary": "Проверка исправления входа committed-crash теста по исходникам и обновление evidence ограниченного Phase 3.7.",
  "relationships": [
    {
      "type": "related_to",
      "to": "kolvrt.apps.native-elf",
      "scope": "Renewed exact-source full CI after fixture correction; unchanged production code, no hardware/performance acceptance."
    }
  ]
}
```
