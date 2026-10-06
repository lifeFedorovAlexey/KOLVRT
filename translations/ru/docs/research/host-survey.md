# Локальный сбор сведений для исследований

Document status: CURRENT
Evidence scope: инструмент для Windows с версионированным форматом, синтетические проверки провайдеров, схемы и CLI; реальная инвентаризация не проводилась и не публиковалась.
Current reference: [Формирование и проверка отчёта](../../../../scripts/host-survey.cjs)

<a name="kolvrt-host-survey"></a>

## Область применения и команды

Это инструмент из #53, отдельный от обследования первой физической машины в #58. Windows x86_64 допустим как `research_host`, но не доказывает поддержку загрузки KOLVRT на x86. В каждом отчёте `kolvrt_execution` равен `NOT_RUN`. Такой отчёт не становится проверкой `boot_target`, драйвера или изоляции.

Реализация расширяет существующие исследовательские скрипты Node.js небольшим сборщиком PowerShell. Служба ядра и второй реестр не добавляются. Для сбора на Windows нужны Node.js 18+ и PowerShell 7. Команды не загружают данные в сеть, не открывают issues, не повышают привилегии и не меняют состояние устройств.

```powershell
# Только справка: провайдеры не опрашиваются.
pwsh -NoProfile -File scripts/collect-host-survey.ps1

# Только синтетические данные.
node scripts/host-survey.cjs sanitize research/hardware/fixtures/synthetic-survey-input.json
node scripts/host-survey.cjs validate research/hardware/fixtures/synthetic-survey-report.json

# Явный локальный сбор, который оператор запускает отдельно.
pwsh -NoProfile -File scripts/collect-host-survey.ps1 -Collect > target/host-survey.local.json
node scripts/host-survey.cjs validate target/host-survey.local.json
```

Порядок работы: явно запустить сбор → сформировать отчёт из разрешённых полей → проверить → просмотреть точный текст → отдельно разрешить публикацию. Проверка не означает согласия на публикацию и не доказывает анонимность. Сочетание публичных моделей и кодов может описывать узнаваемую конфигурацию. В рамках этого изменения реальный сбор не запускался.

## Разрешённые поля провайдеров

[Сборщик](../../../../scripts/collect-host-survey.ps1) запрашивает только перечисленные свойства CIM. Полный объект провайдера, исключение, путь экземпляра и вывод диагностических команд не сериализуются.

| Провайдер               | Запрашиваемые свойства                                                                                | Значение в отчёте                                                                                                                     |
| ----------------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `Win32_Processor`       | `Architecture`, `Name`, `NumberOfCores`, `NumberOfLogicalProcessors`, `VirtualizationFirmwareEnabled` | Архитектура, ограниченный текст модели, число ядер и потоков в записи и сведения о виртуализации прошивки                             |
| `Win32_ComputerSystem`  | `TotalPhysicalMemory`, `HypervisorPresent`                                                            | ОЗУ в байтах и сообщённое наличие гипервизора                                                                                         |
| `Win32_PnPEntity`       | `PNPDeviceID`, `PNPClass`, `HardwareID`, `CompatibleID`, фильтр `Present = TRUE`                      | Публичные коды PCI/USB и классы из закрытого списка; коды других шин остаются UNKNOWN                                                 |
| `Win32_PnPSignedDriver` | `DeviceID`, `InfName`, `DriverVersion`                                                                | Имя INF-файла без пути и числовая версия соответствующего драйвера, а не путь к бинарному файлу или подтверждение загруженного модуля |

Значения свойств основаны на документации Microsoft: [Processor](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-processor), [ComputerSystem](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-computersystem), [PnPEntity](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-pnpentity) и [PnPSignedDriver](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/legacy/aa394354%28v%3Dvs.85%29), просмотренной 2026-10-05. Это изменяемая документация, а не неизменяемое подтверждение провайдера или поддержки драйвера.

Объекты-обёртки CIM и полные PnP-идентификаторы нужны лишь временно, в памяти, чтобы сопоставить устройство и драйвер. Публичные коды PCI/USB извлекаются из аппаратной части перед суффиксом экземпляра; суффиксы не экспортируются и не хешируются. Полные объекты провайдеров и идентификаторы экземпляров остаются в процессе сборщика; выбранный текст модели и драйвера передаётся через закрытый канал в памяти для проверки, без файлов и кеша. Повторные записи одного личного идентификатора объединяются только при совпадении сформированных фактов; при конфликте сведения об устройствах отбрасываются с указанной причиной. Неоднозначное сопоставление драйверов даёт UNKNOWN. Одинаковые публичные коды остаются отдельными наблюдениями PnP: это могут быть разные устройства или интерфейсы.

Сборщик читает данные с правами текущего пользователя. Отказ или нехватка прав дают фиксированный код причины, а не текст исключения. Отсутствующие факты остаются UNKNOWN; личные или неподходящие по формату строки заменяются на `malformed_value`. Это минимизация полей и ограничение строк, а не безошибочное распознавание секрета под видом допустимой модели. Просмотр отчёта остаётся обязательным.

## Закрытый формат и пределы

[Схема v1](../../../../schemas/host-survey.schema.json) задаёт закрытый формат JSON Schema Draft 2020-12. CLI дополнительно проверяет согласованность провайдеров и фактов, пары кодов производителя/устройства и подсистемы PCI, длины кодов классов для каждой шины, псевдонимы внутри отчёта и порядок записей. Повторные ключи JSON, неизвестные поля, неподдерживаемые версии и области применения, а также противоречивые записи отвергаются до вывода.

Факты о машине ограничены моделью и количеством ядер/потоков CPU, ОЗУ, кодами производителя/устройства/подсистемы/класса PCI, кодами производителя/продукта/класса USB и доступными сведениями о драйвере. Для MAC/IP/SSID, имён учётных записей и компьютера, серийных номеров, секретов, личных путей и идентификаторов экземпляров, дампов прошивки и свободного текста ошибок полей нет. Код класса PCI содержит шесть шестнадцатеричных цифр, код совместимого класса USB — две. Отсутствующий код интерфейса не дополняется нулями.

Накопители, сеть, GPU, звук, Bluetooth, HID и другие присутствующие классы PnP определяются без экспорта произвольных имён. `coverage.inventory` всегда равен `PARTIAL`: ненаблюдавшийся класс не означает отсутствия оборудования. Запись устройства — наблюдение, а не подтверждённый физический экземпляр. Псевдонимы вроде `device-0001` создаются заново из отсортированных публичных фактов в каждом отчёте; это не постоянные идентификаторы машины и не права доступа.

Пределы: 1 МиБ входа, 128 КиБ сформированного вывода, 16 уровней вложенности JSON, 32 768 узлов, 64 записи процессоров, 256 записей PnP и 512 записей драйверов. Превышение числа записей провайдера даёт UNKNOWN, а не усечённый список. Превышение размера экспорта атомарно: ненулевой код завершения и пустой stdout. Для каждого запроса CIM задан тайм-аут десять секунд; это не гарантированный общий предел времени для произвольного провайдера.

Сведения о виртуализации не подтверждают наличие IOMMU или включённую защиту DMA отдельного устройства. Проверенного провайдера такой гарантии здесь нет, поэтому IOMMU остаётся UNKNOWN с причиной `unsupported_provider`. На других ОС сбор даёт отчёт о неподдерживаемой платформе, а не догадки.

## Полномочия и свидетельства

Решение следует [контрактам ядра](../architecture/native-model.md), [границам безопасности](../architecture-decisions/0013-security-boundaries.md) и [решению о размещении](../architecture-decisions/0008-placement.md). Наблюдения не дают прав на MMIO/IRQ/DMA, не подтверждают поддержку в Linux и не разрешают исключения совместимости. Метаданные COST-L и заявления о поддержке устройств остаются разными понятиями. Архитектура будущих устройств и драйверов выводится независимо; этот формат не фиксирует ABI ядра или привязку драйвера.

[Тесты](../../../../scripts/tests/host-survey.test.cjs) запускают настоящий CLI и сборщик с синтетическими подменёнными провайдерами CIM. Проверяются личные данные во вложенных полях, пути и сообщения, недоступные провайдеры и права, неверные коды, повторные личные и публичные идентификаторы, неоднозначные драйверы, пустой список, неподдерживаемый сбор, строгий JSON и реальные превышения пределов входа, вывода и записей. [Тест схемы](../../../../crates/repository-checks/tests/host_survey_schema.rs) сверяет настоящий вывод CLI с закрытой схемой и сохранённым синтетическим отчётом; запрещённые поля и вымышленное подтверждение IOMMU должны отвергаться. Оба набора входят в `npm run check`.

[Проверка точных исходников](../../../../research/results/issue53-host-survey.json) относится только к синтетическим экспериментам. Реальный сбор через провайдеры Windows, инвентаризация ARM64, сопоставление с драйверами Linux (#54) и физическое исполнение KOLVRT не проверены. Для отдельно разрешённой публикации действуют [правила личных данных в issues](issue-submissions.md).

[Английский оригинал](../../../../docs/research/host-survey.md)

<!-- knowledge -->

```json
{
  "kind": "subsystem-contract",
  "schema_version": 1,
  "units": [
    {
      "id": "kolvrt.research.host-survey",
      "anchor": "kolvrt-host-survey",
      "depends_on": [
        "law.001",
        "law.009",
        "law.013",
        "law.031",
        "law.036",
        "law.040",
        "adr.0008",
        "adr.0013"
      ],
      "kind": "feature",
      "tags": ["hardware", "survey", "privacy", "research_host"],
      "feature": {
        "verification": [
          {
            "receipt_sha256": "cc07b61d771e202ca0ffc7025fd3070a3c27d179c40704618b3dccb51b896beb",
            "state": "STALE",
            "reason": "Phase 3.7 extends package.json with native QEMU correctness commands. The immutable issue53 synthetic receipt retains its earlier exact-source package integration; framework semantics and hardware-collection boundaries are unchanged, and current-source synthetic acceptance is not inferred from that old receipt.",
            "scope": "Synthetic fixtures and shadowed CIM providers only; no actual hardware collection or kernel execution.",
            "receipt": "research/results/issue53-host-survey.json",
            "environment": "host-process"
          },
          {
            "state": "UNKNOWN",
            "environment": "windows-cim",
            "reason": "No real provider collection has been run; deterministic provider shadows do not establish production hardware observations."
          },
          {
            "state": "NOT_APPLICABLE",
            "environment": "physical-arm64",
            "reason": "This host research framework cannot establish physical KOLVRT kernel/device support."
          }
        ],
        "adrs": ["adr.0008", "adr.0013"],
        "implementation": "BOUNDED_IMPLEMENTED",
        "readiness": "NOT_READY",
        "sources": [
          "scripts/host-survey.cjs",
          "scripts/collect-host-survey.ps1",
          "scripts/tests/host-survey.test.cjs",
          "crates/repository-checks/tests/host_survey_schema.rs",
          "schemas/host-survey.schema.json",
          "research/hardware/fixtures/synthetic-survey-input.json",
          "research/hardware/fixtures/synthetic-survey-report.json",
          "package.json"
        ],
        "next_gate": "Review the exact local report and separately authorize publication before #58 first-machine research; rederive device/binding authority independently before runtime use.",
        "transitions": [
          {
            "acceptance": ["research/results/issue53-host-survey.json"],
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "First local allowlisted framework with exact-source synthetic host acceptance; physical collection and publication remain separate decisions.",
            "from": "UNRECORDED"
          }
        ],
        "roadmap_gate": "COST-L host survey #53",
        "issues": [53],
        "limitations": [
          "Actual Windows-provider collection and real ARM64 inventory have not run. Reports are unauthenticated research observations, not boot_target/driver/DMA guarantees or publication consent. #54 driver mapping and #58 physical first-machine review remain separate."
        ],
        "implementation_scope": "Versioned local Windows research-host collection with a closed allowlist, Node projection/validation, explicit gaps, bounded output and synthetic integration evidence.",
        "acceptance": ["research/results/issue53-host-survey.json"]
      },
      "summary": "Локальные наблюдения об исследовательской машине из разрешённых полей: синтетическая проверка без публикации и полномочий на устройства."
    }
  ],
  "id": "doc.kolvrt.research.host-survey",
  "summary": "Локальные наблюдения об исследовательской машине из разрешённых полей: синтетическая проверка без публикации и полномочий на устройства."
}
```
