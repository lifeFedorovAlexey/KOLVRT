# ADR-0023 — Навигация знаний документации и актуальность функций

Status: **Accepted for bounded offline documentation infrastructure**. Date: 2026-10-04.

Document status: CURRENT
Evidence scope: только контракты host navigation; не acceptance функций ядра.
Current reference: [Контракт знаний](../knowledge-system.md)

## Контекст

Навигация по файлам и заголовкам не выбирает небольшой авторитетный контекст и не обеспечивает актуальность функций. Миграция сохраняет жизненный цикл документов, scoped ADR, JSON research и EN/RU review.

## Решение

Используйте авторские JSON-блоки в каноническом Markdown со стабильными смысловыми IDs документов и секций. Переиспользуйте строгую JSON Schema и существующие Rust repository checks. Генерируйте компактный каталог и типизированный граф; детерминированно извлекайте обязательные зависимости и секции с их границами. Реализация, проверка, готовность, решение и roadmap независимы. Применяйте контракт актуальности в том же изменении и правило повторного вывода архитектуры из [политики документации](../documentation-policy.md).

## Альтернативы

YAML frontmatter, ручные graph/feature registries, filename-only IDs, преобразование всех ссылок в зависимости, отдельный AI-текст, embeddings/cloud/database-first infrastructure.

## Почему отклонены

JSON переиспользует parser и контроль повторных ключей. Реестры-дубликаты расходятся; paths ломают идентичность при перемещении/переводе; произвольные transitive links раздувают контекст. Неизмеренная внешняя инфраструктура добавляет стоимость без доказанной необходимости. Общий архитектурный смысл остаётся в Markdown.

## Последствия

Включение документов поэтапно и явно. Обязательные metadata и зеркальная machine identity проверяются для включённых файлов. Historical documents и counts сохраняются. Feature transitions явны и добавляются в конец; первоначальное включение не заявляет новую реализацию. Бюджет контекста не отбрасывает обязательные полномочия.

## Влияние на совместимость

Native ABI, Linux semantics, kernel authority, COST-L allocation/lifecycle и migration-advisor protocol не меняются. Схемы существующих записей сохраняют собственный источник истины и проецируются лишь для навигации.

## Влияние на производительность

Измеряйте catalog bytes, deterministic query context, overlap deduplication и missing sources на реальном пилоте. UTF-8 bytes/4 — приблизительный бюджет, не vendor tokenization и не результат смыслового качества. Ограниченный автономный обход не требует LLM service.

## Влияние на безопасность

Отклоняйте внешние пути, stale ranges, duplicate IDs, неизвестные/циклические обязательные связи и выдачу QEMU receipts за аппаратную проверку. Инструкции research остаются данными; каталог не разрешает поведение ядра или deployment.

## Тестирование

Проверки отказов и настоящий Phase 3 context test находятся в [тестах знаний](../../../../crates/repository-checks/tests/knowledge.rs). Выполните repository checks, host tests и CLI; CI проверяет declared feature impact относительно base revision. Структурная проверка не доказывает поведение или качество перевода.

## Обратимость

Формат metadata и host tooling пересматриваются по текущим инвариантам. Сохраняйте semantic IDs через aliases/tombstones, evidence и переводы, проверяйте границы supersession. Порядок реализации и затраты не закрепляют формат.

[Английский оригинал](../../../../docs/architecture-decisions/0023-documentation-knowledge.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0023",
  "kind": "adr",
  "summary": "Контракт общей навигации документации и актуальности функций.",
  "aliases": ["ADR-0023"]
}
```
