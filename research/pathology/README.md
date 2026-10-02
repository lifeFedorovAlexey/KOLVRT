# Pathology database

Одна запись — один JSON `KOL-PATH-NNNN.json`. Числа не переиспользуются после удаления;
case можно supersede с сохранением истории Git. Schema: [case.schema.json](../../schemas/pathology/case.schema.json).
Таблица: [CASE_INDEX](../../docs/research/CASE_INDEX.md).

Pathology — название базы инженерных случаев, не verdict «всё в Linux неправильно».
Записи включают необходимые hardware constraints, полезные redesign и accepted tradeoffs.
В `category` можно несколько признаков; `kolvrt_decision` строго одно из шести решений.
`NATIVE_FIX` для нового KOLVRT означает preventive native design, не уже исправленный KOLVRT bug.
`COMPAT_ONLY` не требует немедленно реализовать module без подтверждённого consumer.
`HARDWARE_TRANSLATION` может быть необходим для native target.

`status=ANALYZED_WITH_OPEN_QUESTIONS` означает законченный документальный разбор с
перечисленными ограничениями. Не означает reproducer executed или complete git archaeology.
`confidence=MEDIUM` отражает источники плюс непроверенную применимость к ещё не существующему
KOLVRT. HIGH требовал бы дополнительной исторической проверки/воспроизведения релевантного
поведения. Нельзя повышать confidence за сам факт прохождения schema validation.

`first_known_version` может обозначать первое подтверждённое изменение/альтернативу,
что явно написано в строке; это не обязательно introducing version самого старого бага.
Неизвестная introducing version = null + open question. `date` источника не подменяется
датой research. `linux_current_solution` — решение, подтверждённое прочитанным источником,
а не assertion, что просмотрена каждая последующая версия Linux.

## Соответствие 13 вопросам задания

| Вопрос | Поля |
|---|---|
| Что произошло? | observable_behavior, root_cause |
| Когда? | historical_context, linux_versions, first_known_version, sources |
| Почему? | original_reason |
| Тогдашние ограничения | constraints_then |
| Сохранились ли? | constraints_now |
| Как классифицировать? | category; root_cause различает bug/tradeoff/necessity |
| Последствия изменения | change_consequences, compatibility_dependency |
| Ответ Linux | linux_current_solution |
| Можно ли лучше? | improvement_assessment, remaining_problem |
| Ответ KOLVRT | kolvrt_decision |
| Native semantics | kolvrt_native_semantics |
| Нужен ли compat? | compatibility_required, compatibility_scope |
| Нужно ли вообще? | decision + migration_strategy, особенно NOT_APPLICABLE |

`evidence` связывает исторические поля с source IDs. Security/performance/complexity/hardware
impact — техническая оценка описанного механизма; прогнозы KOLVRT не являются измерениями
Linux или цитатами maintainer. Проектные решения, tests и benchmarks — авторские требования.
При отсутствии applicable benchmark следует записать причину, не пустой список.

Изменение записи: проверить source, уточнить locator/provenance, обновить decision и
confidence, добавить migration/test implication, выполнить validator и обновить report.
Автоматическая проверка не подтверждает правдивость URL contents и не заменяет review.
