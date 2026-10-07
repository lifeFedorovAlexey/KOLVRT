# ADR-0027 — Ограниченные native-наблюдения устройств

Status: **Accepted for the bounded Phase 4.0 contract**. Date: 2026-10-07.

Document status: CURRENT
Evidence scope: решение о descriptor и идентичности наблюдения для одного PL011 QEMU virt; независимый acceptance review и evidence выполнения на точных исходниках остаются отдельными.
Current reference: [Native-наблюдения устройств](../kernel/devices.md)

## Контекст

Issue #79 требует native-описания до привязки драйвера. Существующий boot discovery читает область регистров PL011, которой уже пользуется boot console. Это использование не разрешает доступ драйверу EL0 и не обосновывает универсальный менеджер устройств. [LAW-031](../architecture/kernel-laws.md#law-031) требует проверенной трансляции platform input и отвергает непрерывность идентичности, выведенную из одинаковых описаний замены. [LAW-035](../architecture/kernel-laws.md#law-035) сохраняет enforcement в DEV/PROD. [ADR-0008](0008-placement.md) и [admission policy](../architecture/kernel-admission-policy.md) оставляют service policy вне EL1, если конкретный привилегированный инвариант не требует иного.

Закреплённая [реализация QEMU 10.1.0 virt](https://raw.githubusercontent.com/qemu/qemu/v10.1.0/hw/arm/virt.c) задаёт выбранную топологию и firmware description. Runtime parser проверяет фактически переданные байты DTB; константы в host-отчёте не заменяют discovery. Существующее предположение о доверенном неизменяемом boot input остаётся явным и не устанавливает криптографическую подлинность firmware.

## Решение

Хранить один неизменяемый descriptor PL011: одну проверенную MMIO region, один level-high GIC SPI, проверенную ссылку на interrupt controller и reservation BootConsole. Один раз преобразовывать поддержанное подмножество root-level DTB. Отклонять неверные ширины cells, число ресурсов, ranges, overlap, controller references и неподдержанные interrupt encodings. Публиковать закрытый validated snapshot только после полного успешного discovery; последующие изменения публичных platform observations не могут переписать snapshot. Firmware identifiers описывают топологию и не дают authority.

Представлять идентичность наблюдения ненулевым scope доверенного владельца и ненулевой монотонно возрастающей generation. Владелец обязан выделять уникальный scope среди владельцев, чьи claims могут сравниваться, и не сбрасывать или переиспользовать его, пока сохраняются старые claims. Firmware и недоверенные claims не выбирают доверенного владельца. Boot adapter имеет одного владельца со scope 1 в своей boot session и публикует один раз; это не машинный глобальный allocator и не формат persistent identity.

Ограниченное состояние Console допускает одно текущее наблюдение. Дополнительное наблюдение отклоняется до invalidation текущей identity. Invalidation делает старые claims stale; повторная публикация одинаковых байтов descriptor получает новую generation. Исчерпание generation завершается отказом без wraparound или восстановления прежнего наблюдения. Проверка claim требует текущего owner scope, точной generation и точных наблюдаемых resource fields. Успешная проверка доказывает только соответствие claim наблюдению, а не наличие grant.

Descriptor остаётся отдельным от будущей binding identity и явных MMIO/IRQ grants. Boot console сохраняет reservation. Issue #80 должна определить binding, lifetime и возможную передачу console; #29 не может получить это устройство предъявлением совпадающих байтов descriptor. Эти data contracts не добавляют privileged syscall, mapping operation, driver policy или права доступа.

## Альтернативы

Рассмотрены identity, выведенная из физических наблюдений, универсальный менеджер устройств и отдельная тестовая реализация parser.

## Причины отказа от альтернатив

Вывод identity из MMIO addresses или compatible strings возродил бы старые claims после замены и отвергнут. Универсальный bus graph или Device mega-object преждевременно закрепил бы неподдержанную policy и lifecycle obligations без workload и отложен. Отдельная копия parser для тестов не проверяла бы production behavior и отвергнута.

## Последствия

Чистые методы invalidation/republication описывают время жизни наблюдения, а не live hotplug, device reset, driver teardown или permission revocation. Это решение не реализует hardware removal, DMA, произвольную enumeration, grant transfer или автоматическое recovery. Будущие services обязаны заново вывести архитектуру scope allocation и binding, а не наследовать boot singleton как дизайн глобального сервиса.

## Влияние на compatibility

Внутренний формат descriptor и claim не создаёт стабильный публичный ABI или слой совместимости Linux devices. Существующее владение boot console сохраняется. Будущие binding и grants требуют явного контракта, а не признания одинаковых descriptors совместимой authority.

## Влияние на безопасность

Descriptor — безопасные ограниченные данные, созданные существующим boot decoder. Новый привилегированный механизм не допускается; memory mappings и interrupt access обеспечиваются отдельно. Владелец, переиспользующий scope вне указанного контракта, может создать alias identities, поэтому будущая интеграция нескольких владельцев должна обеспечить уникальность scope. Структурная проверка не защищает от скомпрометированной firmware внутри существующей доверенной boot boundary. DEV и PROD выполняют одинаковые discovery checks.

## Влияние на производительность

Validation выполняется при discovery и переходах наблюдения, а не на scheduler или IPC hot path. Улучшение timing, throughput, physical ARM64 или production trust не заявляется.

## Проверка

Тесты импортируют единственную production-реализацию parser и identity. Они покрывают discovery настоящего DTB, усечение, forged или отсутствующие controller associations, повторяющиеся identities/properties, неверные SPI/trigger, точные resource extents, overlap/overflow, изменённые claims, stale generations, разделение scope и исчерпание generation. Production DEV/PROD boot должен исполнять настоящий discovery path на DTB, переданном QEMU. Одни host fixtures не доказывают выполнение в guest или работающий EL0 driver.

Независимое архитектурное/code-ревью Codex и полное смысловое EN/RU-ревью приняли этот ограниченный контракт 2026-10-07. Приёмка выполнения на точных исходниках учитывается отдельно в canonical device feature перед закрытием issue. Исторические receipts Phase 3 остаются историческими; это решение не повышает их verification и не закрывает последующие driver gates.

## Обратимость

Типы descriptor и identity внутренние и не заморожены. Замена может сузить или пересмотреть их representation, сохраняя явное разделение observation/authority, отказ для stale claims и сохранённый evidence. Поддержка будущих hardware или firmware требует отдельного review ограниченного контракта.

[Английский оригинал](../../../../docs/architecture-decisions/0027-device-observations.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "adr.0027",
  "kind": "adr",
  "summary": "Принятые ограниченные наблюдения PL011 с owner-scoped identity без подразумеваемой device authority.",
  "aliases": ["ADR-0027"],
  "depends_on": ["adr.0008", "law.031", "law.035"]
}
```
