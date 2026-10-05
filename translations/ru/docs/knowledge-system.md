# Система знаний документации

Document status: CURRENT
Evidence scope: детерминированная автономная навигация и пилот Phase 3; выводы о проверке ядра и оборудования не делаются.
Current reference: [Политика документации](documentation-policy.md)

<a name="kolvrt-docs-navigation"></a>

## Функция навигации

Автономный инструмент реализован в ограниченных границах: строгие metadata, извлечение по ID/locale, каталог/граф, явные зависимости, impact и детерминированный context. Выполненный пилот проверяет обозначенный запрос и границы; общее качество semantic search не заявляется.

## Полномочия и владение

Канонический английский Markdown содержит архитектурный смысл и авторские метаданные навигации. Русские версии являются проверенными локализациями тех же сущностей. Существующие JSON COST-L и KOL-PATH сохраняют полномочия для структурированных записей. Каталог и граф — воспроизводимые навигационные проекции. Код определяет фактическое поведение; принятые законы и ADR задают обязательства. Расхождение требует исправления дефекта или проверенного решения.

Метаданные обязательны только для включённых документов. [Список включения](../../../docs/knowledge-enrollment.json) задаёт поэтапную проверку покрытия; каталог перечисляет оставшиеся документы без выдуманного статуса CURRENT. Файлы не перемещаются и не удаляются ради миграции.

## Метаданные и стабильная идентичность

Используйте единственный буквальный HTML-комментарий с маркером `knowledge`, сразу за ним — JSON-блок, вне примеров. [Закрытая схема](../../../schemas/knowledge.schema.json) использует существующий строгий разбор без повторных ключей и JSON Schema Draft 2020-12. Обязательны schema_version, id, kind, summary. Tags/read_when, aliases, depends_on, типизированные relationships и выбранные units необязательны. Unit также требует явного anchor. Document status, Evidence scope, Current reference остаются единственными авторскими полями жизненного цикла документа; название берётся из заголовка.

Смысловые IDs состоят из строчных ASCII-сегментов с цифрами и дефисами, разделённых точками; максимум 160 символов. Перемещение файла и переименование заголовка сохраняют смысл и ID. Настоящая замена получает новый ID и связь supersedes с указанием области. Alias указывает непосредственно на единственный канонический ID, не перекрывает другие IDs, не образует цепочки и циклы. IDs не переиспользуются. Сохранённые REMOVED/SUPERSEDED units служат историческими записями. Адаптеры ADR, законов, COST-L и cases сохраняют внешние IDs как aliases и не выделяют новые записи.

Каждая функция имеет ровно один канонический unit. Заголовок секции задаёт название; границы, ограничения, исходники, issues, ADRs, следующий этап, acceptance, verification и история переходов принадлежат этому unit. Локализации сохраняют машинные поля; summary и условия чтения могут переводиться. Полный смысл контракта сохраняется в зеркальном поясняющем тексте.

## Состояния функций и актуальность

Состояния реализации: PLANNED, RESEARCH, EXPERIMENTAL, BOUNDED_IMPLEMENTED, IMPLEMENTED, SUPERSEDED, REMOVED. PLANNED и RESEARCH могут сменять друг друга; оба переходят в EXPERIMENTAL. Принятые доказательства ограниченного поведения разрешают BOUNDED_IMPLEMENTED; выполнение полного заявленного контракта разрешает IMPLEMENTED. Сокращение гарантий требует явного проверенного понижения. Любое нетерминальное состояние может заменяться или удаляться после разрешения обязательств. Терминальные IDs не воскресают молча; новый смысл требует нового ID и явной связи.

Первый переход UNRECORDED фиксирует ранее проверенное состояние, не приписывая реализацию каталогу. Последующие переходы непрерывны, добавляются в конец и проверяются относительно базы CI. События перехода к реализации содержат реальные acceptance paths. Код без acceptance остаётся EXPERIMENTAL. Удалённые исходники могут отсутствовать; история и ID сохраняются.

Проверка задаётся по окружениям: UNKNOWN, VERIFIED, FAILED, STALE, NOT_APPLICABLE с причинами. VERIFIED содержит точный digest результата и границы исходников, ревизии, профиля, платформы. Исторические receipts сохраняют исходные границы; текущий VERIFIED дополнительно требует совпадения digests объявленных исходников. Непроверенное оборудование остаётся UNKNOWN. Изменение значимых входов требует оценки применимости и STALE/новых результатов; отдельная правка документации не создаёт поведенческую регрессию. Готовность UNKNOWN/NOT_READY/READY независима; READY требует отдельного acceptance. Фаза дорожной карты задаёт порядок гарантий, не статус.

Каждое проверенное изменение функции ОБЯЗАНО в том же изменении обновить её канонический статус, границы, acceptance/применимость evidence, ограничения и следующий этап, перегенерировать каталог/граф и затронутые публичные обзоры, проверить полные EN/RU пары и пройти проверки. Это относится к добавлению, частичной/экспериментальной реализации, ограничениям, удалению, замене, аппаратной проверке и готовности. PR не завершён, пока публичный статус заведомо ложен. Закрытие issue требует acceptance в обозначенных границах либо явно исследовательского результата.

Порядок реализации НЕ определяет архитектуру. Перед использованием ранней функции как основы следующего этапа архитектура заново выводится из текущих Kernel Laws, принятых ADR, инвариантов и модели authority/lifetime. Противоречащий, ограничивающий или преждевременно закрепляющий код нужно переработать, заменить или удалить. Наличие реализации, затраченные усилия, тесты и стоимость переделки не являются архитектурным доказательством.

## Секции, граф и бюджет контекста

Явные именованные anchors стоят перед заголовками. Диапазон включает поддерево до следующего заголовка того же или более высокого уровня, вводную область статуса/evidence и вступления родительских секций. Заголовки внутри примеров не учитываются. Диапазоны генерируются с нумерацией от 1 включительно и привязкой к нормализованному hash и локализации. Устаревшие, повторные или отсутствующие anchors отклоняются. Перекрывающиеся секции и общие зависимости объединяются; метаданные исключаются из показываемого контекста.

Закрытые типы связей: depends_on, related_to, supersedes, implemented_by, validated_by, motivates, contrasts_with, blocked_by, cost_l, source_of_truth. Зависимости и supersession ацикличны; обычные отношения могут образовывать циклы. Supersession требует границ. implemented_by/validated_by указывают на файлы; остальные связи разрешают knowledge IDs. cost_l указывает на debt record. CURRENT dependencies не используют заменённые полномочия. Не превращайте все Markdown-ссылки в prerequisites и не выводите семантические зависимости из лексических наблюдений.

Каталог содержит компактные навигационные записи, aliases и список невключённых документов. Граф содержит locations, hashes, типизированные связи и структурированные метаданные без Markdown-текстов. Семантические связи принадлежат каноническим декларациям; backlinks производны. Embeddings, cloud, database и vendor tokenizer не нужны. Лимиты: 1 MiB на авторский документ, 8192 nodes, 4096 query bytes, 4 MiB rendered context. Пересматривайте эти защитные ограничения при измеренном росте.

## Команды и проверки

```text
cargo xtask docs context "проверь отзыв полномочий" --locale ru --budget-bytes 262144
cargo xtask docs generate
cargo xtask docs generate --check
cargo xtask docs find dma
cargo xtask docs show kolvrt.handles.identity
cargo xtask docs show kolvrt.handles.identity --locale ru
cargo xtask docs deps kolvrt.security.capability-revocation
cargo xtask docs related cost-l.0001
cargo xtask docs impact kolvrt.handles.identity
cargo xtask docs context "review Phase 3.4 capability revocation" --budget-bytes 131072
cargo xtask docs check-change origin/main
```

Фасад xtask вызывает существующий repository-checks, использующий общую библиотеку. Context детерминированно ранжирует ID/tags/read conditions/title/summary, выбирает до трёх кандидатов с оценкой не ниже 60% лучшего результата и полное обязательное замыкание. Результат объясняет выбор, пробелы, bytes и приблизительные ceil(bytes/4) единицы. Превышение бюджета явно завершает команду ошибкой; обязательные зависимости не отбрасываются. Сузьте запрос. Related edges раскрываются по необходимости. Impact следует объявленным обратным связям; это влияние на документацию, а не исчерпывающий анализ кода.

Основная проверка отклоняет ошибки schema/IDs/aliases/edges/cycles, отсутствие обязательных metadata, устаревшие outputs, отсутствующие sources/anchors, ложные feature transitions, изменение receipt digest, устаревшие исходники VERIFIED и расхождение машинных EN/RU полей. Проверка относительно CI base дополнительно отвергает переписанную историю и изменения реализации/сборки без impact-деклараций, привязанных к digests. Явные объяснения no-impact допускают сохранение canonical контракта без правки; истинность оценивает reviewer. Существование внешних issues проверяется авторизованными GitHub tools; автономный инструмент лишь записывает IDs без вывода о существовании/закрытии. Сетевые ошибки означают UNKNOWN.

## Миграция и пределы доказательств

Stage A задаёт schema/policy/checks; B мигрирует processes/user-copy/handles и ограниченные capability/domain acceptance; C добавляет law/ADR navigation и производные README rows; D проецирует COST-L/cases; E добавляет Linux mechanisms и навигацию hardware/security/benchmark/advisor; F поэтапно включает остальные historical docs. Каждый этап сохраняет человеческую навигацию, доказательства и переводы. Пилот измеряет выбранный контекст вместе с catalog overhead, не доказывая достаточность источников для всех будущих задач.

Linux research использует [шаблон механизма](../research/linux/mechanism-template.md); первичные источники фиксируют version/commit/path/configuration и review date. COST-L lifecycle, native decision, driver observations, compatibility modules и feature states различаются. Инструмент не собирает и не публикует личный hardware inventory. Используйте существующие research issues и provenance gates.

[Английский оригинал](../../../docs/knowledge-system.md)

### Проверка точных исходников и влияние изменений реализации

VERIFIED требует, чтобы каждый объявленный исходник feature встретился ровно один раз в `source_files` receipt с совпадающим `sha256_lf` после нормализации LF. Дубликаты, неверные digests, отсутствие покрытия или изменение исходника проваливают CI. До слияния укажите STALE с причиной либо предоставьте новое evidence с совпадающими исходниками. Исторические receipts неизменяемы; их digest проверяется и при STALE. Область реализации и исторический acceptance независимы от текущей проверки. После успешной проверки README не может публиковать устаревший VERIFIED.

Для каждого PR с изменениями реализации или входов сборки заполните [декларацию влияния](../../../docs/implementation-impact.json) относительно точного base commit для review. Её [закрытая схема](../../../schemas/implementation-impact.schema.json) связывает каждый изменённый файл с LF hashes до и после изменения (null при создании/удалении). Обязательная граница включает все файлы в crates, scripts, .cargo и .github/workflows, корневые Cargo/package manifests и locks, build.rs/rust-toolchain.toml и все объявленные исходники feature. Новые и удалённые файлы учитываются, включая untracked при локальной проверке. Другие корни реализации остаются обязанностью ручного enrollment review; checker не распознаёт произвольную новую функциональность по смыслу.

Классифицируйте каждый файл как new-feature, existing-feature или no-feature-impact с объяснением. New-feature требует нового canonical feature, которому принадлежит исходник. Для изменений известных исходников также нужен один disposition feature: semantic-change, evidence-change или no-impact. Semantic-change требует содержательного обновления canonical документа; whitespace-touch недостаточен. Evidence-change требует изменения evidence metadata. No-impact допускает изменение исходника без правки контракта после review объяснения, но никогда не обходит проверку точных исходников. Это утверждение для review, а не автоматическое доказательство эквивалентности. Публикуйте dispositions в описании PR и выводе CI; reviewer оценивает причины и смысл EN/RU. Rebase, правка исходников или продвижение base инвалидируют декларацию и требуют нового review. Файл хранит запись review для изменения, сохраняемую историей Git, а не второй реестр feature.

Не закрывайте infrastructure issue, пока обязательное смысловое или языковое review ещё ожидается. Успешные checks устанавливают механическую согласованность, а не завершение review.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.docs.knowledge",
  "kind": "policy",
  "summary": "Контракт общих авторских метаданных и детерминированной навигации.",
  "units": [
    {
      "id": "kolvrt.docs.navigation",
      "anchor": "kolvrt-docs-navigation",
      "kind": "feature",
      "summary": "Автономный поиск по stable ID, извлечение в границах и проверка графа.",
      "tags": ["documentation", "catalog", "navigation"],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "Offline metadata validation, Unicode EN/RU context, exact-source verification freshness, prerequisite closure and review-visible per-change implementation impact declarations.",
        "sources": [
          "crates/repository-checks/src/knowledge.rs",
          "crates/repository-checks/src/knowledge/impact.rs",
          "crates/repository-checks/tests/knowledge.rs",
          "schemas/implementation-impact.schema.json"
        ],
        "acceptance": ["research/results/documentation-knowledge-pilot.json"],
        "issues": [74],
        "adrs": ["adr.0024"],
        "limitations": [
          "No automatic semantic-completeness proof, no-impact truth proof, LLM search or all-doc migration; arbitrary implementation roots outside the declared CI boundary still require human enrollment review."
        ],
        "next_gate": "Review coverage on additional tasks before enrolling further domains or adding semantic search.",
        "verification": [
          {
            "environment": "host-process",
            "state": "VERIFIED",
            "reason": "Executed named EN/RU offline context pilots with current declared source digests; semantic sufficiency remains review judgment.",
            "receipt": "research/results/documentation-knowledge-pilot.json",
            "receipt_sha256": "e2e0100d711cd863cc2f7e3f91af44c01fff35b83f1970018df2dceb6b302bd2",
            "scope": "Named Phase 3.4 revocation queries and mandatory prerequisites, Unicode Russian retrieval, explicit planned gaps and reverse impact on this exact-source host tool; no kernel execution, universal retrieval quality or physical hardware claim."
          },
          {
            "environment": "physical-arm64",
            "state": "NOT_APPLICABLE",
            "reason": "Offline host navigation has no physical kernel verification claim."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Introduce the bounded offline navigation implementation.",
            "acceptance": []
          },
          {
            "from": "EXPERIMENTAL",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "Actual retrieval closure and explicit-gap pilot plus rejection tests.",
            "acceptance": [
              "research/results/documentation-knowledge-pilot.json"
            ]
          }
        ]
      }
    }
  ]
}
```
