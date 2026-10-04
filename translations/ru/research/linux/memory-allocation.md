# Исследование контекста выделения памяти Linux

Document status: CURRENT
Evidence scope: обзор документации Linux v6.12 на adc218676eef25575469234709c2d87185ca223a; без выполнения драйверов, истории происхождения или allocation service KOLVRT.
Current reference: [Существующая кандидатная запись](../../../../research/cost-l/COST-L-0001.json)

<a name="linux-memory-allocation"></a>

## Контекст выделения и reclaim

Первичный источник различает GFP_KERNEL со sleeping/direct reclaim и ограничения контекста. Это мотивирует исследование рекурсивного reclaim и callback/lifetime dependencies; лексические allocation references не устанавливают полное configured dependency closure драйвера. Историческое происхождение остаётся UNKNOWN в кандидатной записи.

Сравнение с KOLVRT: текущие IRQ paths не выделяют память и не берут heap/table locks. Будущая EL0 allocation service обязана явно определить reclaim/failure и native authorization. Native disposition кандидата остаётся PROPOSED; новая архитектура и поддержка Linux drivers не установлены. COST-L-0001 остаётся CANDIDATE, не активной реализацией совместимости.

Источник: [Linux v6.12 memory-allocation.rst](https://github.com/torvalds/linux/blob/adc218676eef25575469234709c2d87185ca223a/Documentation/core-api/memory-allocation.rst), path Documentation/core-api/memory-allocation.rst, locator “Get Free Page flags”, проверено 2026-10-04. Границы — общая документированная политика; driver behavior конкретной architecture/configuration не выполнялось. Дальнейшие lifecycle/concurrency/failure/performance/history исследования следуют шаблону. Измеренная стоимость KOLVRT неизвестна.

Открытые вопросы: исходные introducing commits; представительные configured driver consumers; повторяющаяся семантическая потребность; точные authority/lifetime native service; измеренная стоимость adapter-versus-native. Координируйте существующее COST-L research, не подтверждая кандидата автоматически.

[Английский оригинал](../../../../research/linux/memory-allocation.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.linux.memory.allocation",
  "kind": "linux-mechanism",
  "summary": "Навигация по документу: Исследование контекста выделения памяти Linux. Доказательства имеют указанные границы.",
  "units": [
    {
      "id": "linux.memory.allocation",
      "anchor": "linux-memory-allocation",
      "kind": "linux-mechanism",
      "summary": "Каноническая секция: Контекст выделения и reclaim.",
      "tags": ["linux", "memory", "allocation", "reclaim"],
      "relationships": [
        {
          "type": "motivates",
          "to": "cost-l.0001",
          "confidence": "HYPOTHESIS",
          "scope": "Research candidate, not confirmed driver dependency.",
          "evidence": ["research/cost-l/COST-L-0001.json"]
        }
      ]
    }
  ]
}
```
