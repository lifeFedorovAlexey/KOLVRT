# Ограниченные наблюдения native-устройств

Document status: CURRENT
Evidence scope: контракт дескриптора Phase 4.0; реализация и свидетельства выполнения точных исходников учитываются отдельно ниже.
Current reference: [Решение о наблюдениях устройств](../architecture-decisions/0027-device-observations.md)

<a name="kolvrt-devices-observations"></a>

## Наблюдения устройств

Первое устройство — PL011, уже обнаруживаемое из загрузочного DTB закреплённой конфигурации QEMU virt. Его неизменяемый дескриптор содержит одну проверенную физическую область регистров, один GIC SPI с уровневым срабатыванием по высокому уровню, ссылку на контроллер прерываний и явное резервирование для загрузочной консоли. Обнаружение проверяет поддерживаемую топологию корневого уровня, ширину ячеек, точное количество ресурсов, границы с проверкой арифметики, отсутствие пересечений и связь с контроллером. Неподдерживаемые конфигурации явно отклоняются; это не универсальный преобразователь шин firmware. Байты DTB относятся к существующей доверенной границе неизменяемых загрузочных данных. Проверка структуры не обеспечивает криптографическую аутентификацию firmware.

Дескриптор описывает оборудование. Он не разрешает MMIO и не выдаёт IRQ capability. Владельцем остаётся загрузочная консоль; драйвер EL0 не может получить устройство, просто предъявив его дескриптор. Привязка драйверов Phase 4 должна отдельно обосновать передачу консоли либо выбрать другое устройство, проверить grants и обеспечить соблюдение их жизненного цикла. Политика размещения драйверов остаётся вне EL1. Для этих безопасных ограниченных проверок данных не требуется новая привилегированная точка входа.

## Идентичность и жизненный цикл

Идентичность устройства — поколение в области владельца, отдельное от наблюдаемых байтов и будущей идентичности привязки. Доверенный владелец наблюдений задаёт ненулевую область, которую нельзя повторно использовать, пока могут сохраняться старые предъявляемые ссылки. Firmware не выбирает эту область. Поколения возрастают без переполнения; исчерпание запрещает дальнейшую публикацию. Удаление наблюдения делает его ссылки недействительными. Повторная публикация идентичных физических наблюдений создаёт новое поколение и никогда не восстанавливает старую ссылку или полномочия. Ссылки из другой области, неизвестные или устаревшие поколения и изменённые наблюдения отклоняются тем же production-методом проверки.

Загрузочный адаптер имеет одного владельца и публикует одно наблюдение при загрузке. Ограниченные методы жизненного цикла задают аннулирование идентичности; они не реализуют аппаратный hotplug, остановку драйверов или отзыв разрешений. Выделение областей между будущими сервисами, замена драйверов и протоколы повторной привязки относятся к отдельным проверенным контрактам. Идентичность не является постоянным идентификатором всей машины или замороженным внешним ABI.

## Границы и исключения

Этот срез содержит одно наблюдение PL011, одну область регистров и один SPI. DMA, произвольные шины, динамическое обнаружение, удаление во время работы, отображения MMIO в EL0, grants доставки IRQ и универсальный объект Device не входят в область. Существующая защита памяти и загрузочные отображения обеспечиваются независимо. DEV и PROD используют одинаковые проверки обнаружения. Этот контракт дескриптора не подтверждает физический ARM64 или выполнение драйверов во время работы.

## Проверка

Host-тесты вызывают настоящие методы обнаружения и идентичности с допустимыми и недопустимыми входами. Отказы охватывают некорректные кодировки ресурсов и прерываний, запрещённую топологию, переполнение, пересечение, несоответствие контроллера, поддельные наблюдения, устаревшую идентичность и исчерпание поколения. Тесты не копируют и не повреждают production-реализации. Настоящая загрузка DEV и PROD должна выполнить обнаружение по DTB, предоставленному QEMU; одних host-фикстур недостаточно для подтверждения этого выполнения. Перед закрытием issue #79 необходимы свидетельства для текущих исходников и независимое архитектурное и EN/RU-ревью.

[Английский оригинал](../../../../docs/kernel/devices.md)

[Приёмка Phase 4.0](../../../../research/results/device-phase40-reviewed.json) фиксирует 98 host-тестов kernel-core и один doctest, 13 проверок output validator и полную matrix из 144 обязательств: 32 реальных запуска и 112 использований свежего evidence в том же прогоне. Обе suites DEV/PROD прошли по 147 проверок; в каждой suite и обычной загрузке проверено ровно одно наблюдение PL011 с MMIO 0x09000000/0x1000 и IRQ 33. Независимое архитектурное/code-ревью и полное EN/RU-ревью завершены. READY относится только к ограниченному контракту дескриптора #79, без готовности драйверов или физического оборудования.

Runner теперь требует ровно одно корректное наблюдение перед приёмкой свежего выполнения с машинным evidence, включая streaming native runtime. Отсутствие, повтор или некорректные поля вызывают отказ; совместимость исторического reader не ослабляет эту проверку. [Первоначальная приёмка](../../../../research/results/device-phase40.json) сохранена с прежними исходниками.

## Интеграция пилота CLOCK

Паспорт CLOCK повторно использует executor свежего machine-mode запуска, поэтому до успешного завершения собственного oracle обязан опубликовать то же проверенное наблюдение устройства. Выбор другого внешнего root не обходит discovery и не создаёт device grant. Изменение исходников общего runner делает предыдущий receipt историческим для той ревизии runner; реализация descriptor и принятый ограниченный scope не меняются.

## Интеграция issue 33

Общий путь исполнения Arena для CLOCK и IPC pilot по-прежнему требует свежего проверенного boot-наблюдения устройства для каждого фактического запуска. Сборка IPC report из частей не заменяет это наблюдение и не подтверждает новый драйвер или grants. Приёмка дескриптора сохраняет прежние границы; изменённые общие исходники требуют новой current-source проверки.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.kernel.devices",
  "kind": "subsystem-contract",
  "summary": "Ограниченные native-наблюдения PL011 и идентичность без переполнения в области владельца, отдельные от grants устройства.",
  "units": [
    {
      "id": "kolvrt.devices.observations",
      "anchor": "kolvrt-devices-observations",
      "kind": "feature",
      "summary": "Проверенный дескриптор загрузочной консоли и ссылки на наблюдения с безопасными поколениями.",
      "depends_on": ["law.031", "law.035", "adr.0008"],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "One root-level non-DMA PL011 discovered from the pinned boot DTB, immutable bounded observations, reserved boot-console ownership and owner-local identity validation.",
        "sources": [
          "crates/kernel-core/src/device.rs",
          "crates/kernel-core/src/lib.rs",
          "crates/kernel-core/src/platform.rs",
          "crates/kernel-core/tests/device_contract.rs",
          "crates/kernel-core/tests/boot_description.rs",
          "crates/kernel/src/main.rs",
          "crates/kernel/src/platform/mod.rs",
          "crates/xtask/src/output.rs",
          "crates/xtask/src/main.rs",
          "crates/xtask/src/native_apps.rs"
        ],
        "acceptance": [
          "research/results/device-phase40.json",
          "research/results/device-phase40-reviewed.json"
        ],
        "issues": [79],
        "adrs": ["adr.0027"],
        "limitations": [
          "No driver binding, MMIO/IRQ grants, DMA, hotplug, global scope allocator or physical ARM64 acceptance; boot console remains reserved."
        ],
        "next_gate": "Issue #80 must derive explicit binding/grants and any console handoff; no driver execution is claimed by this completed descriptor slice.",
        "verification": [
          {
            "environment": "qemu-arm64",
            "state": "STALE",
            "reason": "Historical bounded acceptance is retained; issue33 changes shared execution/report sources and requires new exact-source regression evidence before current verification.",
            "scope": "Pinned QEMU 10.1.0 virt/cortex-a57/TCG DEV/PROD; descriptor discovery and fresh observation gate, not driver or physical readiness.",
            "receipt": "research/measurements/runs/1791408804253-phase40-clock-passport-488919492db0.json",
            "receipt_sha256": "1e6b5dfd336c6a36e1aff2e57dcfa5d1d2c3d7601a90ddbcfec198b2a970eb84"
          },
          {
            "environment": "physical-arm64",
            "state": "UNKNOWN",
            "reason": "No physical device execution evidence."
          }
        ],
        "readiness": "READY",
        "roadmap_gate": "Phase 4.0",
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "EXPERIMENTAL",
            "reason": "Introduce the bounded descriptor implementation; execution acceptance remains separate."
          },
          {
            "from": "EXPERIMENTAL",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "Independent architecture/code and EN/RU review accepted the bounded contract; real DEV/PROD discovery and full 144-obligation matrix passed.",
            "acceptance": ["research/results/device-phase40.json"]
          }
        ],
        "readiness_acceptance": [
          "research/results/device-phase40-reviewed.json"
        ]
      }
    }
  ]
}
```
