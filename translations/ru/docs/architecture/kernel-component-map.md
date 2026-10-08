# Карта компонентов ядра

Document status: CURRENT
Evidence scope: логические ответственности и выбранные взаимодействия по исходному коду; без проверки исполнения или приёмки производительности отдельных компонентов.
Current reference: [Нативная модель](native-model.md); [Component Arena](component-arena.md)

Это представление версии 2 описывает ответственности текущего кода и выбранные взаимодействия. Оно определяет группировку ниже; связанные production-контракты сохраняют силу. Документ не подтверждает проверку исполнения, приёмку реализации просмотрщика или производительность компонентов. [Нативная модель](native-model.md) и [Component Arena](component-arena.md) задают архитектурные и измерительные границы.

## Граница подсчёта

Представление содержит 13 функциональных групп EL1 и два узла production-служб EL0. Это явно выбранная логическая граница подсчёта, а не внутренне присущее ядру число компонентов, исходных файлов или функций. Группа не означает новый ABI, изолированное адресное пространство, отдельно заменяемый бинарный модуль или выдачу полномочий. Тестовые actors не являются production-узлами.

Исходные файлы намеренно пересекаются: диспетчеризация scheduler участвует в планировании, ожиданиях и наблюдениях CLOCK; загрузочный и платформенный код также публикуют идентичность устройств. Принадлежность не означает исключительное владение или аддитивный размер кода либо стоимость. Политика supervisor в EL0 отделена от обеспечения жизненного цикла в EL1, включённого в группу процессов.

## Семантика взаимодействий

Выбранные рёбра подтверждены исходным кодом; это не полный граф зависимостей и не трасса вызовов исполнения. `call` описывает диспетчеризацию или вызов; `data` — передаваемую или предоставляемую информацию; `authority` — существующую границу авторизации; `lifetime` — обязанности удержания, завершения или безопасного повторного использования. Направление следует описанному взаимодействию и не обязательно прямому вызову функции Rust. Рёбра EL0 абстрагируют SDK и путь входа через исключение. Ребро authority не создаёт новых разрешений. Циклы допустимы.

## Метрики и актуальность

Неизмеренные стоимости узлов, частоты и веса рёбер имеют значения UNKNOWN или NOT_MEASURED, никогда ноль. Данные CLOCK описывают полный пользовательский интервал с частичными наблюдениями окон исполнения; распределять этот интервал между группами нельзя. Измерения пути IPC также не устанавливают аддитивные стоимости отдельных узлов. Покрытие не является скоростью. DEV, PROD, QEMU и физическая машина сохраняют разные области применимости свидетельств.

Терминальное и интерактивное представления используют одну каноническую машинную модель. Оба обязаны сохранять идентичности узлов, уровни, состав, виды рёбер, свидетельства и неизвестные значения. Проверка ссылок устанавливает существование ID и путей, но не проверяет исполнение. `source_files` связывает проверенный снимок исходников значениями SHA-256 после нормализации LF. Несовпадение делает снимок STALE; просмотрщик не должен молча показывать его как актуальный. `reviewed_base` указывает проверенную базовую ревизию, а не утверждает, что последующие изменения рабочей копии входят в её commit. Изменения исходников требуют повторного review и обновления представления при изменении ответственностей или взаимодействий; изменение контракта группировки требует новой версии модели.

## Логические группы

<a name="kolvrt-kernel-view-boot"></a>

### Загрузка и платформа (EL1)

Инициализация выбранной платформы, допуск начального исполнения и ограниченный доступ к MMIO-регистрам.

<a name="kolvrt-kernel-view-memory"></a>

### Память и адресные пространства (EL1)

Владение физическими кадрами, отображениями и временем жизни ASID.

<a name="kolvrt-kernel-view-interrupts"></a>

### Исключения и прерывания (EL1)

Сохранение архитектурного контекста и диспетчеризация синхронных исключений и прерываний.

<a name="kolvrt-kernel-view-smp"></a>

### SMP и состояние CPU (EL1)

Координация вторичных CPU, IPI и подтверждений инвалидации с acquire-семантикой.

<a name="kolvrt-kernel-view-scheduler"></a>

### Планирование и владение задачами (EL1)

Владение состоянием готовых задач, диспетчеризацией и публикацией завершения.

<a name="kolvrt-kernel-view-process"></a>

### Процессы, образы и жизненный цикл (EL1)

Допуск образов и обеспечение создания, завершения, освобождения процессов и механизмов жизненного цикла.

<a name="kolvrt-kernel-view-user-copy"></a>

### Граница копирования пользователя (EL1)

Проверка пользовательских диапазонов и копирование через защищённую границу памяти пользователя.

<a name="kolvrt-kernel-view-authority"></a>

### Имена, права и ресурсные полномочия (EL1)

Разрешение локальных handles, проверка прав и учёт ресурсов доменов.

<a name="kolvrt-kernel-view-wait"></a>

### Ожидание, пробуждение и синхронизация (EL1)

Поддержка идентичности ожиданий и согласование пробуждений с состоянием задач.

<a name="kolvrt-kernel-view-ipc"></a>

### Ограниченный IPC (EL1)

Обеспечение ограниченных запросов, передачи payload, завершения и отмены.

<a name="kolvrt-kernel-view-clock"></a>

### Время и наблюдения текущей задачи (EL1)

Предоставление времени и уже учтённых окон исполнения текущей задачи.

<a name="kolvrt-kernel-view-devices"></a>

### Неизменяемая идентичность устройств (EL1)

Проверка неизменяемых дескрипторов устройств платформы без выдачи полномочий на устройства.

<a name="kolvrt-kernel-view-diagnostics"></a>

### Диагностика и загрузочная консоль (EL1)

Вывод ограниченной диагностики через зарезервированную загрузочную консоль.

<a name="kolvrt-kernel-view-supervisor"></a>

### Нативный supervisor (EL0)

Применение production-политики supervision служб через публичные интерфейсы жизненного цикла и IPC.

<a name="kolvrt-kernel-view-counter-service"></a>

### Служба счётчика (EL0)

Выполнение production-запросов счётчика и публикация ответов.

## Внутренний состав

Каждый логический элемент имеет стабильный идентификатор внутри родительского узла. Это локальные идентичности модели, а не новые идентификаторы ABI или полномочий. Необязательный список `member_relations` использует те же виды взаимодействий и свидетельства исходного кода, что и корневые рёбра; обе его конечные точки находятся внутри родителя. Версия 2 фиксирует 55 логических элементов и 56 выбранных внутренних связей во всех 15 группах. Восемь связей соединяют семь элементов времени и наблюдений. Подписи различают путь диспетчеризации CLOCK, обработку CAPTURE/READ_WINDOW и вспомогательную функцию deadline; это выбранные связи исходного кода, а не утверждение, что все они выполняются при CLOCK.

Группа ожидания дополнительно фиксирует пять проверенных связей между четырьмя механизмами. Ветка WAIT_OWN_EVENT, доступная только в STEP, использует объединяющие события защёлки; это не обычный путь ожидания IPC. Проверка входа в scheduler запрещает обычные блокировки при доступе по владению, не выдумывая вызов защёлки к блокировке. Для IPC отложенный опрос источника публикует удерживаемые последовательности пробуждения, а затем уведомляет удалённые CPU после освобождения разрешений endpoint. Владелец проверяет точные сохранённые WaitKey, поколение процесса и заблокированное состояние перед READY; устаревшие пробуждения подтверждаются без перевода задачи в готовое состояние. Подтверждение источника удаляет удерживаемое пробуждение endpoint до очистки подтверждения mailbox. READY на этом пути пробуждения перезапускает syscall и не утверждает завершение RPC. Завершение копирования проходит отдельным путём scheduler.

Каждая группа теперь имеет проверенные по исходникам внутренние связи без заявления об исчерпывающем покрытии. Группа загрузки и платформы также предоставляет ограниченный доступ к регистрам: контроллер прерываний и опрашивающий writer консоли вызывают её механизм MMIO через явно сохранённые межгрупповые границы. Отсутствие ребра означает отсутствие выбранного проверенного взаимодействия, а не доказательство отсутствия взаимодействий. Стабильные ID элементов остаются непрозрачными идентичностями модели при уточнении отображаемой роли. Измеренная атрибуция стоимости внутренним элементам отсутствует: исторические интервалы CLOCK остаются привязаны к пути родительского узла и не распределяются между его дочерними элементами. Добавление идентичности и состава элементов не меняет корневую границу подсчёта из 13 групп EL1 и двух служб EL0 или её 30 выбранных корневых взаимодействий.

Внутренние представления продолжают корневой граф через границу выбранного блока. Все 30 корневых связей сохраняют идентичность через 41 проверенных пар `member_endpoints`, присоединяя входящие и исходящие связи к реально участвующим механизмам. Составная корневая связь может иметь несколько ветвей в подробном представлении; это не увеличивает число корневых связей. Карточки внешних групп открывают связанную группу. Входящие внешние компоненты расположены над внутренними механизмами, исходящие — под ними; двунаправленная группа присутствует на обеих границах. Подсказка каждой трассы использует единый компактный формат: источник → получатель и описание взаимодействия; дополнительные пояснения удалённого контекста показываются только при закреплении. Краткие подписи действий обозначают связи, а подробные подсказки сохраняют полное взаимодействие и свидетельства. Это выбранное покрытие исходного кода, а не исчерпывающий граф вызовов или измеренная трасса исполнения.

## Каноническая машинная модель

Машинные идентификаторы и подписи ниже совпадают в обеих языковых версиях.

<!-- arena-architecture -->

```json
{
  "schema_version": 1,
  "version": "2",
  "reviewed_base": "8df8f2d",
  "source_files": {
    "apps/native-apps/src/counter-service.rs": "635fca7cd71c649dd20a773e214ed49f3752ccc0a0f5b9241d82c39ebcb9ee1c",
    "apps/native-apps/src/supervision.rs": "deababd624234c547c23d5d4d2e1034b401bebaaeb515bea34095a052669dda6",
    "apps/native-apps/src/supervisor.rs": "7d65f92ce1283bbecb2dfd5e5a88b13cbeae289fa5ddd2a4b8e44b6fe1cb6c99",
    "apps/native-runtime/src/lib.rs": "27a2e5a3c0a1bd0f8836661cb6ea50d185fa62e4292cb9c639e5152724bfa2c1",
    "crates/kernel-core/src/device.rs": "8e4e073e258ebc0376d169b3b638a5bb2820dc8b2fe2162335a4d895e60e7f66",
    "crates/kernel-core/src/domain.rs": "dad82df93e0beaf252017dd6ee5cb02fe2bb3b815ba64f1bf911e2d77de78788",
    "crates/kernel-core/src/elf.rs": "faa97534ec072f53c773012af327345dde9a35cece294ac1e436dd7399a79e58",
    "crates/kernel-core/src/execution.rs": "1b66287b7111d0fd319f8439e051bcc8658ef11fd8d156002afed213bf52933c",
    "crates/kernel-core/src/handles.rs": "6effe77cfae8e5b64f35e7d2eeb4a4052d2dbe01666204bad06cd64a9af43cd2",
    "crates/kernel-core/src/ipc.rs": "9cd0ca58b7504db9487d6862a811502032ecb36f3cbb3cad3e3b15441dad97f4",
    "crates/kernel-core/src/ipc/mailbox.rs": "fee25e70388e3e2d2750c871537c1851d906fd786dd1efc7db300fb43c60076c",
    "crates/kernel-core/src/memory.rs": "0505ba3442c15da22f87b77b6365698588d3ca8c0db65d67e4bf130e232ceb58",
    "crates/kernel-core/src/platform.rs": "4423248c501cef4d50bc229fc6c97fa13eafbdea3632d037eb2b6a78f3fcd1a9",
    "crates/kernel-core/src/process.rs": "1067a28c3b51d65994a4ae5fe43e5e4d2f3303cc8c24c6709c22b52531312983",
    "crates/kernel-core/src/scheduling/ownership.rs": "41376235734055e9824655e6c5565b3224364f8796146dd917c53f1e784936ea",
    "crates/kernel-core/src/supervision.rs": "96d0ccd94f13e4d1e47720e01c308b3c9e8dd37c216ed05078fc0c4ecefb7ca6",
    "crates/kernel-core/src/time.rs": "308484a5d494f89849224ec0687368f8420712215739418249e30e49a104c403",
    "crates/kernel-core/src/user_copy.rs": "3fec5bdf29e979923cc3dcda503e1fcea2627cb17291090a045344a6bc0909da",
    "crates/kernel-core/src/wait.rs": "c9b1a7f133f9eb2794b8396abec06f791d3fd6b0c850207ee6402f8b910f7332",
    "crates/kernel-core/src/window.rs": "9c27386bd51db4c9db01151926a023bb7fbe5f8668c4d08bef753b257971f92f",
    "crates/kernel/src/arch/aarch64/entry.S": "5df5e13ad91b815d43d31838238d13ce91032e818bd40e7ffcb411e819c1ac1c",
    "crates/kernel/src/arch/aarch64/mod.rs": "e54606a21a0152921041ddab5fef0ede92fbcc6f436026acac4770b1455e5e2e",
    "crates/kernel/src/asid.rs": "7216e67ee636e5df1679728ee0911b90fd77bc0a76371dd090b300d8d205cd7e",
    "crates/kernel/src/diagnostics/mod.rs": "b03821bb2ff3a75672c6f5033eaf5182dc10acf45105db0ac806b15435fc19d4",
    "crates/kernel/src/execution.rs": "a22679e860a69e7dfde7845fe0b882ec6039701a20272901c1b1d957aa108d1f",
    "crates/kernel/src/hal/mod.rs": "4cd1c37db0bc76b990e919f2c2673d84385266793642d23519f417d8a841e731",
    "crates/kernel/src/handles.rs": "53f114a1ba01ac43f3e8e5e3af85bf5e2ce47b08049b241578e42c9d7c539db2",
    "crates/kernel/src/interrupt/mod.rs": "1ed289dca58d3e0d2bdc107279d871c61d7e9152f24602a380b0886a527d1a2c",
    "crates/kernel/src/ipc.rs": "4d8942e9ad2836b10b2bc62e98a6e35ce17befae305fe1276f5d9bd35af1e933",
    "crates/kernel/src/ipc/deferred.rs": "2dd4e5bddd94a02e05d743508c4e453fe175d990e5550f40ab21d70138d91ba4",
    "crates/kernel/src/ipc/native.rs": "bcf6931bd66b9648110bd35be16d6358eda9478a901328c49c7ce42b089d87ac",
    "crates/kernel/src/ipc/storage.rs": "f4b296d6e9bb07efd78fffbd6aca55c2f740460b22395860585f8e42fe1b9991",
    "crates/kernel/src/main.rs": "669b1dd09ebf4c69c5412d593ae751f690e661b95a7034bfd1c2bcc469846fac",
    "crates/kernel/src/memory/mod.rs": "7b30429ef9fcc21fe3bf8eb77778078288f81051a732c1a6cb48de568cc519f0",
    "crates/kernel/src/native_boot.rs": "4fb5e1b86ed624d4cb2c7fb2f1161262a16b618f0fef1ea7e4c0d9a0d04c4ed8",
    "crates/kernel/src/percpu.rs": "5910cf7b2ba96d8812bb390249ea70bc8e016cc8554aeea2b8ed07b6b2c39185",
    "crates/kernel/src/platform/mod.rs": "bfb5f120aabbb9a776ff9a61bb1ac1d9b974a4663e94265454d9afc635f9c7d4",
    "crates/kernel/src/process.rs": "cd560c0b57d78d3ed169b0c5d50a13f0a245cd0a811a39d50456ac9478829810",
    "crates/kernel/src/scheduler/local.rs": "0e295809eab3d48636ae39b0d5ab3e10ea37d1ec63361a005afcf5b028727429",
    "crates/kernel/src/scheduler/mod.rs": "cd9b1aee50697e7a3e69ee3650a754731325d00d3315332c549873f2c68d6c43",
    "crates/kernel/src/scheduler/task.rs": "82295a88f8943ee273ef718dd5823c24db9923b253f734a5089f0b98015581df",
    "crates/kernel/src/security.rs": "4a46f6eaf4d6dd5882aae5c1bb002f3b4451d40fc1bfbef1c1c5eb153fa96806",
    "crates/kernel/src/smp.rs": "5be7c210395aa8b4fb08677928fcd33add7113cb46ace17bbd5f7fb4bb9bbaf3",
    "crates/kernel/src/supervision.rs": "b665f610da6512e79c90a71994547c9ba793d1f8751a095bf3c515d93c00567a",
    "crates/kernel/src/sync/mod.rs": "4425454d73f2cb1b355ae34891612f3480d8463cadcc02babb034179700547da",
    "crates/kernel/src/time/mod.rs": "cefff7cdc9c810edcf5f6024dd62f78df68d01fa2338741257a6a1c9d557c69a",
    "crates/kernel/src/user_copy.rs": "796bda906820e7b9944b7512b4adcd3502c491599f83e8622933555950fd6ae0"
  },
  "nodes": [
    {
      "id": "kolvrt.kernel.view.boot",
      "label": "Boot and platform",
      "layer": "EL1",
      "responsibility": "Initialize the selected platform and provide bounded platform-register access.",
      "members": [
        {
          "name": "Boot admission sequence",
          "sources": ["crates/kernel/src/main.rs"],
          "id": "kolvrt.kernel.view.boot.boot-admission-sequence"
        },
        {
          "name": "Selected platform initialization",
          "sources": ["crates/kernel/src/platform/mod.rs"],
          "id": "kolvrt.kernel.view.boot.selected-platform-initialization"
        },
        {
          "name": "Bounded MMIO access",
          "sources": ["crates/kernel/src/hal/mod.rs"],
          "id": "kolvrt.kernel.view.boot.hardware-abstraction-selection"
        },
        {
          "name": "Platform description decoding",
          "sources": ["crates/kernel-core/src/platform.rs"],
          "id": "kolvrt.kernel.view.boot.platform-description-decoding"
        }
      ],
      "contracts": ["adr.0010", "adr.0005"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.boot.boot-admission-sequence",
          "to": "kolvrt.kernel.view.boot.selected-platform-initialization",
          "kind": "call",
          "label": "kernel_main obtains the validated boot description through discover_boot",
          "evidence": [
            "crates/kernel/src/main.rs",
            "crates/kernel/src/platform/mod.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.boot.selected-platform-initialization",
          "to": "kolvrt.kernel.view.boot.platform-description-decoding",
          "kind": "call",
          "label": "discover_boot bounds the DTB slice, calls discover, and verifies platform constraints",
          "evidence": [
            "crates/kernel/src/platform/mod.rs",
            "crates/kernel-core/src/platform.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.memory",
      "label": "Memory and address spaces",
      "layer": "EL1",
      "responsibility": "Own physical frames, mappings and ASID lifetimes.",
      "members": [
        {
          "name": "Physical and virtual memory ownership",
          "sources": ["crates/kernel/src/memory/mod.rs"],
          "id": "kolvrt.kernel.view.memory.physical-and-virtual-memory-ownership"
        },
        {
          "name": "Frame allocation state",
          "sources": ["crates/kernel-core/src/memory.rs"],
          "id": "kolvrt.kernel.view.memory.frame-allocation-state"
        },
        {
          "name": "Address-space identifier lifecycle",
          "sources": ["crates/kernel/src/asid.rs"],
          "id": "kolvrt.kernel.view.memory.address-space-identifier-lifecycle"
        }
      ],
      "contracts": ["adr.0010", "kolvrt.process.asid"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.memory.physical-and-virtual-memory-ownership",
          "to": "kolvrt.kernel.view.memory.frame-allocation-state",
          "kind": "call",
          "label": "Physical initializes and reserves Pool extents, then allocates aligned frame ranges",
          "evidence": [
            "crates/kernel/src/memory/mod.rs",
            "crates/kernel-core/src/memory.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.memory.physical-and-virtual-memory-ownership",
          "to": "kolvrt.kernel.view.memory.frame-allocation-state",
          "kind": "lifetime",
          "label": "Physical::release returns frames to Pool only after mapping and retained-charge checks",
          "evidence": [
            "crates/kernel/src/memory/mod.rs",
            "crates/kernel-core/src/memory.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.memory.physical-and-virtual-memory-ownership",
          "to": "kolvrt.kernel.view.memory.address-space-identifier-lifecycle",
          "kind": "call",
          "label": "OwnedUserSpace::new allocates an owner-bound ASID lease for the process slot",
          "evidence": [
            "crates/kernel/src/memory/mod.rs",
            "crates/kernel/src/asid.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.memory.physical-and-virtual-memory-ownership",
          "to": "kolvrt.kernel.view.memory.address-space-identifier-lifecycle",
          "kind": "lifetime",
          "label": "reclaim and unpublished rollback release the ASID lease before returning owned frames",
          "evidence": [
            "crates/kernel/src/memory/mod.rs",
            "crates/kernel/src/asid.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.interrupts",
      "label": "Exceptions and interrupts",
      "layer": "EL1",
      "responsibility": "Save architectural context and dispatch synchronous exceptions and interrupts.",
      "members": [
        {
          "name": "Architectural context save and restore",
          "sources": ["crates/kernel/src/arch/aarch64/entry.S"],
          "id": "kolvrt.kernel.view.interrupts.architectural-context-save-and-restore"
        },
        {
          "name": "Vector installation and architectural IRQ operations",
          "sources": ["crates/kernel/src/arch/aarch64/mod.rs"],
          "id": "kolvrt.kernel.view.interrupts.vector-installation-and-exception-dispatch"
        },
        {
          "name": "Interrupt-controller dispatch",
          "sources": ["crates/kernel/src/interrupt/mod.rs"],
          "id": "kolvrt.kernel.view.interrupts.interrupt-controller-dispatch"
        }
      ],
      "contracts": ["adr.0014", "adr.0010"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.interrupts.vector-installation-and-exception-dispatch",
          "to": "kolvrt.kernel.view.interrupts.architectural-context-save-and-restore",
          "kind": "data",
          "label": "vectors installs the assembly vector-table address in VBAR_EL1",
          "evidence": [
            "crates/kernel/src/arch/aarch64/mod.rs",
            "crates/kernel/src/arch/aarch64/entry.S"
          ]
        },
        {
          "from": "kolvrt.kernel.view.interrupts.architectural-context-save-and-restore",
          "to": "kolvrt.kernel.view.interrupts.interrupt-controller-dispatch",
          "kind": "call",
          "label": "IRQ vector entries save context and call interrupt_entry",
          "evidence": [
            "crates/kernel/src/arch/aarch64/entry.S",
            "crates/kernel/src/interrupt/mod.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.interrupts.interrupt-controller-dispatch",
          "to": "kolvrt.kernel.view.interrupts.vector-installation-and-exception-dispatch",
          "kind": "call",
          "label": "interrupt_entry reads acknowledge and calls end_irq for serviced interrupts",
          "evidence": [
            "crates/kernel/src/interrupt/mod.rs",
            "crates/kernel/src/arch/aarch64/mod.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.smp",
      "label": "SMP and per-CPU state",
      "layer": "EL1",
      "responsibility": "Coordinate secondary CPUs, IPIs and acquired invalidation acknowledgements.",
      "members": [
        {
          "name": "Secondary CPU and invalidation coordination",
          "sources": ["crates/kernel/src/smp.rs"],
          "id": "kolvrt.kernel.view.smp.secondary-cpu-and-invalidation-coordination"
        },
        {
          "name": "Per-CPU identity and local state",
          "sources": ["crates/kernel/src/percpu.rs"],
          "id": "kolvrt.kernel.view.smp.per-cpu-identity-and-local-state"
        }
      ],
      "contracts": ["adr.0012"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.smp.secondary-cpu-and-invalidation-coordination",
          "to": "kolvrt.kernel.view.smp.per-cpu-identity-and-local-state",
          "kind": "data",
          "label": "ping reads the target CPU affinity for SGI delivery",
          "evidence": [
            "crates/kernel/src/smp.rs",
            "crates/kernel/src/percpu.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.smp.secondary-cpu-and-invalidation-coordination",
          "to": "kolvrt.kernel.view.smp.per-cpu-identity-and-local-state",
          "kind": "data",
          "label": "secondary_main publishes ONLINE, post-invalidation tlb_ack and final QUIESCENT state",
          "evidence": [
            "crates/kernel/src/smp.rs",
            "crates/kernel/src/percpu.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.smp.secondary-cpu-and-invalidation-coordination",
          "to": "kolvrt.kernel.view.smp.per-cpu-identity-and-local-state",
          "kind": "lifetime",
          "label": "request_invalidation and acknowledged acquire remote state and acknowledgement before retirement completes",
          "evidence": [
            "crates/kernel/src/smp.rs",
            "crates/kernel/src/percpu.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.scheduler",
      "label": "Scheduling and task ownership",
      "layer": "EL1",
      "responsibility": "Own runnable task state, dispatch and completion publication.",
      "members": [
        {
          "name": "Native-call and scheduling dispatch",
          "sources": ["crates/kernel/src/scheduler/mod.rs"],
          "id": "kolvrt.kernel.view.scheduler.native-call-and-scheduling-dispatch"
        },
        {
          "name": "Task context and completion state",
          "sources": ["crates/kernel/src/scheduler/task.rs"],
          "id": "kolvrt.kernel.view.scheduler.task-context-and-completion-state"
        },
        {
          "name": "Guarded per-CPU scheduler storage",
          "sources": ["crates/kernel/src/scheduler/local.rs"],
          "id": "kolvrt.kernel.view.scheduler.guarded-per-cpu-scheduler-storage"
        },
        {
          "name": "Scheduler-storage ownership admission",
          "sources": ["crates/kernel-core/src/scheduling/ownership.rs"],
          "id": "kolvrt.kernel.view.scheduler.task-ownership-transitions"
        }
      ],
      "contracts": ["doc.kolvrt.kernel.scheduler", "adr.0016"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.scheduler.native-call-and-scheduling-dispatch",
          "to": "kolvrt.kernel.view.scheduler.guarded-per-cpu-scheduler-storage",
          "kind": "call",
          "label": "Dispatch enters Local::with for owned queue mutation and Local::inspect after completion",
          "evidence": [
            "crates/kernel/src/scheduler/mod.rs",
            "crates/kernel/src/scheduler/local.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.scheduler.guarded-per-cpu-scheduler-storage",
          "to": "kolvrt.kernel.view.scheduler.task-ownership-transitions",
          "kind": "call",
          "label": "Local gates prepare, publish, start, mutate, inspect and complete through Ownership",
          "evidence": [
            "crates/kernel/src/scheduler/local.rs",
            "crates/kernel-core/src/scheduling/ownership.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.scheduler.guarded-per-cpu-scheduler-storage",
          "to": "kolvrt.kernel.view.scheduler.task-context-and-completion-state",
          "kind": "lifetime",
          "label": "Local retains State task contexts behind exclusive ownership-scoped access",
          "evidence": [
            "crates/kernel/src/scheduler/local.rs",
            "crates/kernel/src/scheduler/mod.rs",
            "crates/kernel/src/scheduler/task.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.scheduler.native-call-and-scheduling-dispatch",
          "to": "kolvrt.kernel.view.scheduler.task-context-and-completion-state",
          "kind": "call",
          "label": "deferred_local publishes exact IPC readiness through Task::publish_ipc_ready",
          "evidence": [
            "crates/kernel/src/scheduler/mod.rs",
            "crates/kernel/src/scheduler/task.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.process",
      "label": "Processes, images and lifecycle",
      "layer": "EL1",
      "responsibility": "Admit images and enforce process creation, completion, reclamation and lifecycle mechanisms.",
      "members": [
        {
          "name": "Process admission and resource lifecycle",
          "sources": ["crates/kernel/src/process.rs"],
          "id": "kolvrt.kernel.view.process.process-admission-and-resource-lifecycle"
        },
        {
          "name": "Process registry transitions",
          "sources": ["crates/kernel-core/src/process.rs"],
          "id": "kolvrt.kernel.view.process.process-registry-transitions"
        },
        {
          "name": "ELF image validation",
          "sources": ["crates/kernel-core/src/elf.rs"],
          "id": "kolvrt.kernel.view.process.elf-image-validation"
        },
        {
          "name": "Granted lifecycle operation enforcement",
          "sources": ["crates/kernel/src/supervision.rs"],
          "id": "kolvrt.kernel.view.process.granted-lifecycle-operation-enforcement"
        },
        {
          "name": "Initial native admission and completion reporting",
          "sources": ["crates/kernel/src/native_boot.rs"],
          "id": "kolvrt.kernel.view.process.initial-native-admission-and-completion-reporting"
        },
        {
          "name": "Immutable bootstrap grant validation",
          "sources": ["crates/kernel-core/src/supervision.rs"],
          "id": "kolvrt.kernel.view.process.immutable-bootstrap-grant-validation"
        }
      ],
      "contracts": [
        "kolvrt.process.lifecycle",
        "kolvrt.services.supervision",
        "kolvrt.apps.native-elf"
      ],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.process.process-admission-and-resource-lifecycle",
          "to": "kolvrt.kernel.view.process.process-registry-transitions",
          "kind": "call",
          "label": "Registry reserves, prepares, admits and reclaims through Table transitions",
          "evidence": [
            "crates/kernel/src/process.rs",
            "crates/kernel-core/src/process.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.process.process-admission-and-resource-lifecycle",
          "to": "kolvrt.kernel.view.process.elf-image-validation",
          "kind": "call",
          "label": "Image admission parses ELF and checks the accepted load segment",
          "evidence": [
            "crates/kernel/src/process.rs",
            "crates/kernel-core/src/elf.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.process.granted-lifecycle-operation-enforcement",
          "to": "kolvrt.kernel.view.process.process-admission-and-resource-lifecycle",
          "kind": "authority",
          "label": "Granted lifecycle commands create, bind, start and stop process instances",
          "evidence": [
            "crates/kernel/src/supervision.rs",
            "crates/kernel/src/process.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.process.granted-lifecycle-operation-enforcement",
          "to": "kolvrt.kernel.view.process.immutable-bootstrap-grant-validation",
          "kind": "call",
          "label": "Scope installation validates immutable bootstrap grants",
          "evidence": [
            "crates/kernel/src/supervision.rs",
            "crates/kernel-core/src/supervision.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.process.initial-native-admission-and-completion-reporting",
          "to": "kolvrt.kernel.view.process.process-admission-and-resource-lifecycle",
          "kind": "call",
          "label": "Native boot creates the root and acquires registry completion",
          "evidence": [
            "crates/kernel/src/native_boot.rs",
            "crates/kernel/src/process.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.process.initial-native-admission-and-completion-reporting",
          "to": "kolvrt.kernel.view.process.granted-lifecycle-operation-enforcement",
          "kind": "call",
          "label": "Native boot installs the grant scope and services captured lifecycle commands",
          "evidence": [
            "crates/kernel/src/native_boot.rs",
            "crates/kernel/src/supervision.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.user-copy",
      "label": "User-copy boundary",
      "layer": "EL1",
      "responsibility": "Validate user ranges and copy through the protected user-memory boundary.",
      "members": [
        {
          "name": "Protected user-memory copy boundary",
          "sources": ["crates/kernel/src/user_copy.rs"],
          "id": "kolvrt.kernel.view.user-copy.protected-user-memory-copy-boundary"
        },
        {
          "name": "User range and byte-limit validation",
          "sources": ["crates/kernel-core/src/user_copy.rs"],
          "id": "kolvrt.kernel.view.user-copy.user-range-and-snapshot-validation"
        }
      ],
      "contracts": ["kolvrt.memory.user-copy"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.user-copy.protected-user-memory-copy-boundary",
          "to": "kolvrt.kernel.view.user-copy.user-range-and-snapshot-validation",
          "kind": "call",
          "label": "Access validation checks bounded user ranges before translation and copying",
          "evidence": [
            "crates/kernel/src/user_copy.rs",
            "crates/kernel-core/src/user_copy.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.authority",
      "label": "Names, rights and resource authority",
      "layer": "EL1",
      "responsibility": "Resolve local handles, enforce rights and account domain resources.",
      "members": [
        {
          "name": "Local handle resolution",
          "sources": ["crates/kernel/src/handles.rs"],
          "id": "kolvrt.kernel.view.authority.local-handle-resolution"
        },
        {
          "name": "Event notification authority and charging",
          "sources": ["crates/kernel/src/security.rs"],
          "id": "kolvrt.kernel.view.authority.capability-and-domain-enforcement"
        },
        {
          "name": "Generational handle namespace",
          "sources": ["crates/kernel-core/src/handles.rs"],
          "id": "kolvrt.kernel.view.authority.generational-handle-namespace"
        },
        {
          "name": "Domain rights and resource accounting",
          "sources": ["crates/kernel-core/src/domain.rs"],
          "id": "kolvrt.kernel.view.authority.domain-rights-and-resource-accounting"
        }
      ],
      "contracts": [
        "kolvrt.handles.local",
        "kolvrt.security.capability-revocation",
        "kolvrt.security.domains"
      ],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.authority.local-handle-resolution",
          "to": "kolvrt.kernel.view.authority.generational-handle-namespace",
          "kind": "call",
          "label": "Copied local handle requests resolve, transfer and close namespace entries",
          "evidence": [
            "crates/kernel/src/handles.rs",
            "crates/kernel-core/src/handles.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.authority.capability-and-domain-enforcement",
          "to": "kolvrt.kernel.view.authority.generational-handle-namespace",
          "kind": "authority",
          "label": "Event notification validates the caller namespace and admits or revokes signal authority",
          "evidence": [
            "crates/kernel/src/security.rs",
            "crates/kernel-core/src/handles.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.authority.capability-and-domain-enforcement",
          "to": "kolvrt.kernel.view.authority.domain-rights-and-resource-accounting",
          "kind": "call",
          "label": "Accepted event work charges consumer requests, service requests and queue capacity",
          "evidence": [
            "crates/kernel/src/security.rs",
            "crates/kernel-core/src/domain.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.authority.generational-handle-namespace",
          "to": "kolvrt.kernel.view.authority.domain-rights-and-resource-accounting",
          "kind": "authority",
          "label": "Namespace domain binding validates the owner and charges retained handles",
          "evidence": [
            "crates/kernel-core/src/handles.rs",
            "crates/kernel-core/src/domain.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.wait",
      "label": "Wait, wake and synchronization",
      "layer": "EL1",
      "responsibility": "Maintain wait identities and coordinate wakeups with task state.",
      "members": [
        {
          "name": "Coalescing event latches and retained signal admission",
          "sources": ["crates/kernel-core/src/wait.rs"],
          "id": "kolvrt.kernel.view.wait.wait-identity-and-terminal-transitions"
        },
        {
          "name": "Ordinary-lock exclusion and scheduler entry guard",
          "sources": ["crates/kernel/src/sync/mod.rs"],
          "id": "kolvrt.kernel.view.wait.kernel-synchronization-primitives"
        },
        {
          "name": "Task wait and wake integration",
          "sources": [
            "crates/kernel/src/scheduler/mod.rs",
            "crates/kernel/src/scheduler/local.rs",
            "crates/kernel/src/scheduler/task.rs"
          ],
          "id": "kolvrt.kernel.view.wait.task-wait-and-wake-integration"
        },
        {
          "id": "kolvrt.kernel.view.wait.retained-ipc-wake-publication-and-acknowledgement",
          "name": "Retained IPC wake publication and acknowledgement",
          "sources": [
            "crates/kernel-core/src/ipc/mailbox.rs",
            "crates/kernel/src/ipc/deferred.rs"
          ]
        }
      ],
      "contracts": ["doc.kolvrt.kernel.wait", "kolvrt.ipc.wait"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.wait.task-wait-and-wake-integration",
          "to": "kolvrt.kernel.view.wait.wait-identity-and-terminal-transitions",
          "kind": "call",
          "label": "STEP WAIT_OWN_EVENT registers then rechecks pending notification",
          "evidence": [
            "crates/kernel/src/scheduler/mod.rs",
            "crates/kernel-core/src/wait.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.wait.task-wait-and-wake-integration",
          "to": "kolvrt.kernel.view.wait.kernel-synchronization-primitives",
          "kind": "call",
          "label": "Local ownership entry asserts no ordinary lock is held",
          "evidence": [
            "crates/kernel/src/scheduler/local.rs",
            "crates/kernel/src/sync/mod.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.wait.task-wait-and-wake-integration",
          "to": "kolvrt.kernel.view.wait.retained-ipc-wake-publication-and-acknowledgement",
          "kind": "call",
          "label": "deferred_local consumes the exact wake after applying or rejecting it",
          "evidence": [
            "crates/kernel/src/scheduler/mod.rs",
            "crates/kernel-core/src/ipc/mailbox.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.wait.retained-ipc-wake-publication-and-acknowledgement",
          "to": "kolvrt.kernel.view.wait.task-wait-and-wake-integration",
          "kind": "data",
          "label": "pending exposes sequence for saved WaitKey and process-generation validation",
          "evidence": [
            "crates/kernel-core/src/ipc/mailbox.rs",
            "crates/kernel/src/scheduler/mod.rs",
            "crates/kernel/src/scheduler/task.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.wait.task-wait-and-wake-integration",
          "to": "kolvrt.kernel.view.wait.retained-ipc-wake-publication-and-acknowledgement",
          "kind": "call",
          "label": "deferred_local polls retained wake publication and source acknowledgements",
          "evidence": [
            "crates/kernel/src/scheduler/mod.rs",
            "crates/kernel/src/ipc/deferred.rs",
            "crates/kernel-core/src/ipc/mailbox.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.ipc",
      "label": "Bounded IPC",
      "layer": "EL1",
      "responsibility": "Enforce bounded requests, payload transfer, completion and cancellation.",
      "members": [
        {
          "name": "Bounded endpoint and request state machine",
          "sources": ["crates/kernel-core/src/ipc.rs"],
          "id": "kolvrt.kernel.view.ipc.bounded-endpoint-and-request-state-machine"
        },
        {
          "name": "IPC runtime integration",
          "sources": ["crates/kernel/src/ipc.rs"],
          "id": "kolvrt.kernel.view.ipc.ipc-runtime-integration"
        },
        {
          "name": "Endpoint and request storage",
          "sources": ["crates/kernel/src/ipc/storage.rs"],
          "id": "kolvrt.kernel.view.ipc.endpoint-and-request-storage"
        },
        {
          "name": "Admission, payload copy and completion",
          "sources": ["crates/kernel/src/ipc/native.rs"],
          "id": "kolvrt.kernel.view.ipc.admission-payload-copy-and-completion"
        },
        {
          "name": "Deferred wake and owner continuation",
          "sources": ["crates/kernel/src/ipc/deferred.rs"],
          "id": "kolvrt.kernel.view.ipc.deferred-wake-and-owner-continuation"
        }
      ],
      "contracts": ["kolvrt.ipc.transport", "kolvrt.ipc.request"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.ipc.ipc-runtime-integration",
          "to": "kolvrt.kernel.view.ipc.endpoint-and-request-storage",
          "kind": "call",
          "label": "Trusted endpoint creation and rollback enter endpoint-local storage",
          "evidence": [
            "crates/kernel/src/ipc.rs",
            "crates/kernel/src/ipc/storage.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.ipc.ipc-runtime-integration",
          "to": "kolvrt.kernel.view.ipc.deferred-wake-and-owner-continuation",
          "kind": "lifetime",
          "label": "Reap requires actual deferred mailbox quiescence",
          "evidence": [
            "crates/kernel/src/ipc.rs",
            "crates/kernel/src/ipc/deferred.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.ipc.endpoint-and-request-storage",
          "to": "kolvrt.kernel.view.ipc.bounded-endpoint-and-request-state-machine",
          "kind": "lifetime",
          "label": "Endpoint cells retain concrete state under acquired nonblocking permits",
          "evidence": [
            "crates/kernel/src/ipc/storage.rs",
            "crates/kernel-core/src/ipc.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.ipc.admission-payload-copy-and-completion",
          "to": "kolvrt.kernel.view.ipc.endpoint-and-request-storage",
          "kind": "call",
          "label": "Native continuations resolve exact receipts and access endpoints through storage permits",
          "evidence": [
            "crates/kernel/src/ipc/native.rs",
            "crates/kernel/src/ipc/storage.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.ipc.admission-payload-copy-and-completion",
          "to": "kolvrt.kernel.view.ipc.bounded-endpoint-and-request-state-machine",
          "kind": "call",
          "label": "Native execution submits, receives, commits, replies and collects through Endpoint methods",
          "evidence": [
            "crates/kernel/src/ipc/native.rs",
            "crates/kernel-core/src/ipc.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.ipc.deferred-wake-and-owner-continuation",
          "to": "kolvrt.kernel.view.ipc.endpoint-and-request-storage",
          "kind": "call",
          "label": "Deferred polling visits endpoint cells outside scheduler storage",
          "evidence": [
            "crates/kernel/src/ipc/deferred.rs",
            "crates/kernel/src/ipc/storage.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.ipc.deferred-wake-and-owner-continuation",
          "to": "kolvrt.kernel.view.ipc.bounded-endpoint-and-request-state-machine",
          "kind": "call",
          "label": "Deferred polling expires requests and publishes or acknowledges retained wakes",
          "evidence": [
            "crates/kernel/src/ipc/deferred.rs",
            "crates/kernel-core/src/ipc.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.clock",
      "label": "Time and own-task observations",
      "layer": "EL1",
      "responsibility": "Expose time and already-accounted current-task execution windows.",
      "members": [
        {
          "name": "Counter reads and deadline arithmetic",
          "sources": [
            "crates/kernel/src/time/mod.rs",
            "crates/kernel/src/arch/aarch64/mod.rs"
          ],
          "id": "kolvrt.kernel.view.clock.counter"
        },
        {
          "name": "Own-task execution observation service",
          "sources": ["crates/kernel/src/execution.rs"],
          "id": "kolvrt.kernel.view.clock.observation"
        },
        {
          "name": "Time value arithmetic",
          "sources": ["crates/kernel-core/src/time.rs"],
          "id": "kolvrt.kernel.view.clock.time-arithmetic"
        },
        {
          "name": "Checked immutable window reduction",
          "sources": ["crates/kernel-core/src/window.rs"],
          "id": "kolvrt.kernel.view.clock.window"
        },
        {
          "name": "Execution observation ABI and report bounds",
          "sources": ["crates/kernel-core/src/execution.rs"],
          "id": "kolvrt.kernel.view.clock.abi"
        },
        {
          "name": "Own-task native-call dispatch",
          "sources": ["crates/kernel/src/scheduler/mod.rs"],
          "id": "kolvrt.kernel.view.clock.dispatch"
        },
        {
          "name": "Current-task accumulated observations",
          "sources": ["crates/kernel/src/scheduler/task.rs"],
          "id": "kolvrt.kernel.view.clock.task-state"
        }
      ],
      "contracts": ["kolvrt.clock.query", "kolvrt.clock.query.api"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.clock.dispatch",
          "to": "kolvrt.kernel.view.clock.counter",
          "kind": "call",
          "label": "CLOCK reads ticks and counter frequency",
          "evidence": [
            "crates/kernel/src/scheduler/mod.rs",
            "crates/kernel/src/arch/aarch64/mod.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.clock.dispatch",
          "to": "kolvrt.kernel.view.clock.task-state",
          "kind": "data",
          "label": "CLOCK reads accumulated task counters into x2 and x3",
          "evidence": [
            "crates/kernel/src/scheduler/mod.rs",
            "crates/kernel/src/scheduler/task.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.clock.dispatch",
          "to": "kolvrt.kernel.view.clock.abi",
          "kind": "data",
          "label": "Decode the native CLOCK operation identifier",
          "evidence": [
            "crates/kernel/src/scheduler/mod.rs",
            "crates/kernel-core/src/execution.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.clock.dispatch",
          "to": "kolvrt.kernel.view.clock.observation",
          "kind": "call",
          "label": "Dispatch CAPTURE and READ_WINDOW through Observations::call",
          "evidence": [
            "crates/kernel/src/scheduler/mod.rs",
            "crates/kernel/src/execution.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.clock.observation",
          "to": "kolvrt.kernel.view.clock.window",
          "kind": "call",
          "label": "READ_WINDOW validates captured span with window::checked",
          "evidence": [
            "crates/kernel/src/execution.rs",
            "crates/kernel-core/src/window.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.clock.observation",
          "to": "kolvrt.kernel.view.clock.abi",
          "kind": "data",
          "label": "Decode CAPTURE/READ_WINDOW and initialize bounded replies",
          "evidence": [
            "crates/kernel/src/execution.rs",
            "crates/kernel-core/src/execution.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.clock.task-state",
          "to": "kolvrt.kernel.view.clock.observation",
          "kind": "lifetime",
          "label": "Task state owns embedded Observations",
          "evidence": [
            "crates/kernel/src/scheduler/task.rs",
            "crates/kernel/src/execution.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.clock.counter",
          "to": "kolvrt.kernel.view.clock.time-arithmetic",
          "kind": "call",
          "label": "deadline_after converts durations through duration_ticks",
          "evidence": [
            "crates/kernel/src/time/mod.rs",
            "crates/kernel-core/src/time.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.devices",
      "label": "Immutable device identity",
      "layer": "EL1",
      "responsibility": "Validate immutable platform device descriptors without granting device authority.",
      "members": [
        {
          "name": "Immutable descriptor validation",
          "sources": ["crates/kernel-core/src/device.rs"],
          "id": "kolvrt.kernel.view.devices.immutable-descriptor-validation"
        },
        {
          "name": "Platform identity extraction",
          "sources": ["crates/kernel-core/src/platform.rs"],
          "id": "kolvrt.kernel.view.devices.platform-identity-extraction"
        },
        {
          "name": "Boot descriptor publication",
          "sources": ["crates/kernel/src/main.rs"],
          "id": "kolvrt.kernel.view.devices.boot-descriptor-publication"
        }
      ],
      "contracts": ["kolvrt.devices.observations"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.devices.boot-descriptor-publication",
          "to": "kolvrt.kernel.view.devices.platform-identity-extraction",
          "kind": "call",
          "label": "discover_boot obtains the validated platform description through discover",
          "evidence": [
            "crates/kernel/src/main.rs",
            "crates/kernel/src/platform/mod.rs",
            "crates/kernel-core/src/platform.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.devices.platform-identity-extraction",
          "to": "kolvrt.kernel.view.devices.immutable-descriptor-validation",
          "kind": "call",
          "label": "After complete FDT validation, discover constructs the immutable console snapshot with Descriptor::validated",
          "evidence": [
            "crates/kernel-core/src/platform.rs",
            "crates/kernel-core/src/device.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.devices.boot-descriptor-publication",
          "to": "kolvrt.kernel.view.devices.immutable-descriptor-validation",
          "kind": "call",
          "label": "Boot reads boot_console, observes its identity and resolves the descriptor before publishing console fields",
          "evidence": [
            "crates/kernel/src/main.rs",
            "crates/kernel-core/src/device.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.diagnostics",
      "label": "Diagnostics and boot console",
      "layer": "EL1",
      "responsibility": "Emit bounded diagnostics through the reserved boot console.",
      "members": [
        {
          "id": "kolvrt.kernel.view.diagnostics.record-framing",
          "name": "Diagnostic record framing",
          "sources": ["crates/kernel/src/diagnostics/mod.rs"]
        },
        {
          "id": "kolvrt.kernel.view.diagnostics.bounded-event-serialization-and-console-output",
          "name": "Bounded diagnostic output admission",
          "sources": ["crates/kernel/src/diagnostics/mod.rs"]
        },
        {
          "id": "kolvrt.kernel.view.diagnostics.polled-console-writer",
          "name": "Bounded polled console writer",
          "sources": ["crates/kernel/src/diagnostics/mod.rs"]
        }
      ],
      "contracts": ["adr.0003", "adr.0010"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.diagnostics.record-framing",
          "to": "kolvrt.kernel.view.diagnostics.bounded-event-serialization-and-console-output",
          "kind": "call",
          "label": "status, boot_banner and enabled machine event framing submit format arguments to print",
          "evidence": ["crates/kernel/src/diagnostics/mod.rs"]
        },
        {
          "from": "kolvrt.kernel.view.diagnostics.bounded-event-serialization-and-console-output",
          "to": "kolvrt.kernel.view.diagnostics.polled-console-writer",
          "kind": "call",
          "label": "print invokes Writer only after secondary-CPU rejection and bounded Length validation",
          "evidence": ["crates/kernel/src/diagnostics/mod.rs"]
        },
        {
          "from": "kolvrt.kernel.view.diagnostics.polled-console-writer",
          "to": "kolvrt.kernel.view.diagnostics.bounded-event-serialization-and-console-output",
          "kind": "data",
          "label": "Writer returns fmt::Error for unavailable UART or exhausted polling; print increments DROPPED",
          "evidence": ["crates/kernel/src/diagnostics/mod.rs"]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.supervisor",
      "label": "Native supervisor",
      "layer": "EL0",
      "responsibility": "Apply production service supervision policy through public lifecycle and IPC interfaces.",
      "members": [
        {
          "name": "Service policy and readiness coordination",
          "sources": ["apps/native-apps/src/supervisor.rs"],
          "id": "kolvrt.kernel.view.supervisor.service-policy-and-readiness-coordination"
        },
        {
          "name": "Production recovery operation",
          "sources": ["apps/native-apps/src/supervision.rs"],
          "id": "kolvrt.kernel.view.supervisor.production-recovery-operation"
        }
      ],
      "contracts": ["kolvrt.services.supervision", "kolvrt.apps.native-elf"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.supervisor.service-policy-and-readiness-coordination",
          "to": "kolvrt.kernel.view.supervisor.production-recovery-operation",
          "kind": "call",
          "label": "recover delegates to recover_service, which requires observed terminal state before replacement",
          "evidence": [
            "apps/native-apps/src/supervisor.rs",
            "apps/native-apps/src/supervision.rs"
          ]
        }
      ]
    },
    {
      "id": "kolvrt.kernel.view.counter-service",
      "label": "Counter service",
      "layer": "EL0",
      "responsibility": "Execute production counter requests and publish replies.",
      "members": [
        {
          "name": "Counter request execution",
          "sources": ["apps/native-apps/src/counter-service.rs"],
          "id": "kolvrt.kernel.view.counter-service.counter-request-execution"
        },
        {
          "name": "Public native ABI call interface",
          "sources": ["apps/native-runtime/src/lib.rs"],
          "id": "kolvrt.kernel.view.counter-service.public-native-abi-call-interface"
        },
        {
          "id": "kolvrt.kernel.view.counter-service.counter-codec-and-private-state",
          "name": "Counter codec and private state",
          "sources": ["apps/native-runtime/src/lib.rs"]
        }
      ],
      "contracts": ["kolvrt.apps.native-elf"],
      "member_relations": [
        {
          "from": "kolvrt.kernel.view.counter-service.counter-request-execution",
          "to": "kolvrt.kernel.view.counter-service.public-native-abi-call-interface",
          "kind": "call",
          "label": "The service receives, commits and replies through native::call operations 2, 3 and 4",
          "evidence": [
            "apps/native-apps/src/counter-service.rs",
            "apps/native-runtime/src/lib.rs"
          ]
        },
        {
          "from": "kolvrt.kernel.view.counter-service.counter-request-execution",
          "to": "kolvrt.kernel.view.counter-service.counter-codec-and-private-state",
          "kind": "call",
          "label": "Decode the exact 16-byte CounterRequest, apply it to private Counter state and encode counter_reply",
          "evidence": [
            "apps/native-apps/src/counter-service.rs",
            "apps/native-runtime/src/lib.rs"
          ]
        }
      ]
    }
  ],
  "edges": [
    {
      "to": "kolvrt.kernel.view.memory",
      "kind": "call",
      "label": "Initialize physical memory, MMU and heap",
      "evidence": ["crates/kernel/src/main.rs"],
      "from": "kolvrt.kernel.view.boot",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.boot.boot-admission-sequence",
          "to_member": "kolvrt.kernel.view.memory.physical-and-virtual-memory-ownership"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.interrupts",
      "kind": "call",
      "label": "Install exception vectors and interrupt controller",
      "evidence": ["crates/kernel/src/main.rs"],
      "from": "kolvrt.kernel.view.boot",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.boot.boot-admission-sequence",
          "to_member": "kolvrt.kernel.view.interrupts.vector-installation-and-exception-dispatch"
        },
        {
          "from_member": "kolvrt.kernel.view.boot.boot-admission-sequence",
          "to_member": "kolvrt.kernel.view.interrupts.interrupt-controller-dispatch"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.smp",
      "kind": "call",
      "label": "Initialize per-CPU state and start secondary CPUs",
      "evidence": ["crates/kernel/src/main.rs"],
      "from": "kolvrt.kernel.view.boot",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.boot.boot-admission-sequence",
          "to_member": "kolvrt.kernel.view.smp.per-cpu-identity-and-local-state"
        },
        {
          "from_member": "kolvrt.kernel.view.boot.boot-admission-sequence",
          "to_member": "kolvrt.kernel.view.smp.secondary-cpu-and-invalidation-coordination"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.process",
      "kind": "call",
      "label": "Admit initial native images",
      "evidence": [
        "crates/kernel/src/main.rs",
        "crates/kernel/src/native_boot.rs"
      ],
      "from": "kolvrt.kernel.view.boot",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.boot.boot-admission-sequence",
          "to_member": "kolvrt.kernel.view.process.initial-native-admission-and-completion-reporting"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.devices",
      "kind": "call",
      "label": "Validate and publish reserved console descriptor",
      "evidence": [
        "crates/kernel/src/main.rs",
        "crates/kernel-core/src/device.rs"
      ],
      "from": "kolvrt.kernel.view.boot",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.boot.boot-admission-sequence",
          "to_member": "kolvrt.kernel.view.devices.immutable-descriptor-validation"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.diagnostics",
      "kind": "data",
      "label": "Validated reserved console address configures diagnostics",
      "evidence": [
        "crates/kernel/src/main.rs",
        "crates/kernel/src/diagnostics/mod.rs"
      ],
      "from": "kolvrt.kernel.view.devices",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.devices.boot-descriptor-publication",
          "to_member": "kolvrt.kernel.view.diagnostics.polled-console-writer"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.diagnostics",
      "kind": "call",
      "label": "Initialize console and emit boot diagnostics",
      "evidence": ["crates/kernel/src/main.rs"],
      "from": "kolvrt.kernel.view.boot",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.boot.boot-admission-sequence",
          "to_member": "kolvrt.kernel.view.diagnostics.polled-console-writer"
        },
        {
          "from_member": "kolvrt.kernel.view.boot.boot-admission-sequence",
          "to_member": "kolvrt.kernel.view.diagnostics.record-framing"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.scheduler",
      "kind": "call",
      "label": "Dispatch timer interrupt to scheduler",
      "evidence": ["crates/kernel/src/interrupt/mod.rs"],
      "from": "kolvrt.kernel.view.interrupts",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.interrupts.interrupt-controller-dispatch",
          "to_member": "kolvrt.kernel.view.scheduler.native-call-and-scheduling-dispatch"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.smp",
      "kind": "call",
      "label": "Dispatch inter-processor interrupt",
      "evidence": ["crates/kernel/src/interrupt/mod.rs"],
      "from": "kolvrt.kernel.view.interrupts",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.interrupts.interrupt-controller-dispatch",
          "to_member": "kolvrt.kernel.view.smp.secondary-cpu-and-invalidation-coordination"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.smp",
      "kind": "lifetime",
      "label": "Require acquired invalidation acknowledgements before reuse",
      "evidence": [
        "crates/kernel/src/memory/mod.rs",
        "crates/kernel/src/smp.rs"
      ],
      "from": "kolvrt.kernel.view.memory",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.memory.physical-and-virtual-memory-ownership",
          "to_member": "kolvrt.kernel.view.smp.secondary-cpu-and-invalidation-coordination"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.memory",
      "kind": "lifetime",
      "label": "Own and release process user-space frames",
      "evidence": ["crates/kernel/src/process.rs"],
      "from": "kolvrt.kernel.view.process",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.process.process-admission-and-resource-lifecycle",
          "to_member": "kolvrt.kernel.view.memory.physical-and-virtual-memory-ownership"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.scheduler",
      "kind": "call",
      "label": "Admit task contexts and acquire completion",
      "evidence": ["crates/kernel/src/process.rs"],
      "from": "kolvrt.kernel.view.process",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.process.process-admission-and-resource-lifecycle",
          "to_member": "kolvrt.kernel.view.scheduler.native-call-and-scheduling-dispatch"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.authority",
      "kind": "authority",
      "label": "Enforce domain charges and local namespace limits",
      "evidence": ["crates/kernel/src/process.rs"],
      "from": "kolvrt.kernel.view.process",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.process.process-admission-and-resource-lifecycle",
          "to_member": "kolvrt.kernel.view.authority.domain-rights-and-resource-accounting"
        },
        {
          "from_member": "kolvrt.kernel.view.process.process-admission-and-resource-lifecycle",
          "to_member": "kolvrt.kernel.view.authority.generational-handle-namespace"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.ipc",
      "kind": "authority",
      "label": "Create endpoints and senders under existing grants",
      "evidence": [
        "crates/kernel/src/process.rs",
        "crates/kernel/src/supervision.rs"
      ],
      "from": "kolvrt.kernel.view.process",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.process.process-admission-and-resource-lifecycle",
          "to_member": "kolvrt.kernel.view.ipc.ipc-runtime-integration"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.ipc",
      "kind": "call",
      "label": "Dispatch prepare, execute, copy and finish operations",
      "evidence": ["crates/kernel/src/scheduler/mod.rs"],
      "from": "kolvrt.kernel.view.scheduler",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.scheduler.native-call-and-scheduling-dispatch",
          "to_member": "kolvrt.kernel.view.ipc.admission-payload-copy-and-completion"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.authority",
      "kind": "authority",
      "label": "Resolve current-task handle and security operations",
      "evidence": ["crates/kernel/src/scheduler/mod.rs"],
      "from": "kolvrt.kernel.view.scheduler",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.scheduler.native-call-and-scheduling-dispatch",
          "to_member": "kolvrt.kernel.view.authority.local-handle-resolution"
        },
        {
          "from_member": "kolvrt.kernel.view.scheduler.native-call-and-scheduling-dispatch",
          "to_member": "kolvrt.kernel.view.authority.capability-and-domain-enforcement"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.clock",
      "kind": "data",
      "label": "Publish own-task counters through CLOCK",
      "evidence": [
        "crates/kernel/src/scheduler/mod.rs",
        "crates/kernel/src/scheduler/task.rs"
      ],
      "from": "kolvrt.kernel.view.scheduler",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.scheduler.task-context-and-completion-state",
          "to_member": "kolvrt.kernel.view.clock.dispatch"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.wait",
      "kind": "call",
      "label": "Integrate event waits and wakeups with task state",
      "evidence": ["crates/kernel/src/scheduler/mod.rs"],
      "from": "kolvrt.kernel.view.scheduler",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.scheduler.native-call-and-scheduling-dispatch",
          "to_member": "kolvrt.kernel.view.wait.wait-identity-and-terminal-transitions"
        },
        {
          "from_member": "kolvrt.kernel.view.scheduler.native-call-and-scheduling-dispatch",
          "to_member": "kolvrt.kernel.view.wait.retained-ipc-wake-publication-and-acknowledgement"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.user-copy",
      "kind": "data",
      "label": "Snapshot and copy initialized user payload bytes",
      "evidence": ["crates/kernel/src/ipc/native.rs"],
      "from": "kolvrt.kernel.view.ipc",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.ipc.admission-payload-copy-and-completion",
          "to_member": "kolvrt.kernel.view.user-copy.protected-user-memory-copy-boundary"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.authority",
      "kind": "authority",
      "label": "Check namespace rights and charge retained requests",
      "evidence": [
        "crates/kernel/src/ipc/native.rs",
        "crates/kernel-core/src/ipc.rs"
      ],
      "from": "kolvrt.kernel.view.ipc",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.ipc.admission-payload-copy-and-completion",
          "to_member": "kolvrt.kernel.view.authority.generational-handle-namespace"
        },
        {
          "from_member": "kolvrt.kernel.view.ipc.bounded-endpoint-and-request-state-machine",
          "to_member": "kolvrt.kernel.view.authority.domain-rights-and-resource-accounting"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.wait",
      "kind": "lifetime",
      "label": "Retain exact wait identities until terminal wakeup",
      "evidence": [
        "crates/kernel/src/ipc/native.rs",
        "crates/kernel/src/ipc/deferred.rs",
        "crates/kernel/src/scheduler/mod.rs"
      ],
      "from": "kolvrt.kernel.view.ipc",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.ipc.admission-payload-copy-and-completion",
          "to_member": "kolvrt.kernel.view.wait.task-wait-and-wake-integration"
        },
        {
          "from_member": "kolvrt.kernel.view.ipc.deferred-wake-and-owner-continuation",
          "to_member": "kolvrt.kernel.view.wait.retained-ipc-wake-publication-and-acknowledgement"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.smp",
      "kind": "call",
      "label": "Ping owner CPU for deferred continuations",
      "evidence": ["crates/kernel/src/ipc/deferred.rs"],
      "from": "kolvrt.kernel.view.ipc",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.ipc.deferred-wake-and-owner-continuation",
          "to_member": "kolvrt.kernel.view.smp.secondary-cpu-and-invalidation-coordination"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.memory",
      "kind": "data",
      "label": "Use protected user-address range boundaries",
      "evidence": ["crates/kernel/src/user_copy.rs"],
      "from": "kolvrt.kernel.view.user-copy",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.user-copy.protected-user-memory-copy-boundary",
          "to_member": "kolvrt.kernel.view.memory.physical-and-virtual-memory-ownership"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.scheduler",
      "kind": "call",
      "label": "Dispatch scheduler IPI and secondary polling",
      "evidence": ["crates/kernel/src/smp.rs"],
      "from": "kolvrt.kernel.view.smp",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.smp.secondary-cpu-and-invalidation-coordination",
          "to_member": "kolvrt.kernel.view.scheduler.native-call-and-scheduling-dispatch"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.process",
      "kind": "authority",
      "label": "Request lifecycle operations within immutable grants",
      "evidence": [
        "apps/native-apps/src/supervisor.rs",
        "crates/kernel/src/supervision.rs"
      ],
      "from": "kolvrt.kernel.view.supervisor",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.supervisor.service-policy-and-readiness-coordination",
          "to_member": "kolvrt.kernel.view.process.granted-lifecycle-operation-enforcement"
        },
        {
          "from_member": "kolvrt.kernel.view.supervisor.production-recovery-operation",
          "to_member": "kolvrt.kernel.view.process.granted-lifecycle-operation-enforcement"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.ipc",
      "kind": "call",
      "label": "Issue readiness and service control requests",
      "evidence": [
        "apps/native-apps/src/supervisor.rs",
        "apps/native-apps/src/supervision.rs"
      ],
      "from": "kolvrt.kernel.view.supervisor",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.supervisor.service-policy-and-readiness-coordination",
          "to_member": "kolvrt.kernel.view.ipc.admission-payload-copy-and-completion"
        },
        {
          "from_member": "kolvrt.kernel.view.supervisor.production-recovery-operation",
          "to_member": "kolvrt.kernel.view.ipc.admission-payload-copy-and-completion"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.ipc",
      "kind": "call",
      "label": "Receive, commit and reply through native runtime",
      "evidence": [
        "apps/native-apps/src/counter-service.rs",
        "apps/native-runtime/src/lib.rs"
      ],
      "from": "kolvrt.kernel.view.counter-service",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.counter-service.public-native-abi-call-interface",
          "to_member": "kolvrt.kernel.view.ipc.admission-payload-copy-and-completion"
        }
      ]
    },
    {
      "to": "kolvrt.kernel.view.diagnostics",
      "kind": "data",
      "label": "Publish lifecycle and generic report events",
      "evidence": [
        "crates/kernel/src/native_boot.rs",
        "crates/kernel/src/supervision.rs"
      ],
      "from": "kolvrt.kernel.view.process",
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.process.initial-native-admission-and-completion-reporting",
          "to_member": "kolvrt.kernel.view.diagnostics.record-framing"
        },
        {
          "from_member": "kolvrt.kernel.view.process.granted-lifecycle-operation-enforcement",
          "to_member": "kolvrt.kernel.view.diagnostics.record-framing"
        }
      ]
    },
    {
      "from": "kolvrt.kernel.view.interrupts",
      "to": "kolvrt.kernel.view.boot",
      "kind": "call",
      "label": "Access interrupt-controller registers through bounded MMIO",
      "evidence": [
        "crates/kernel/src/interrupt/mod.rs",
        "crates/kernel/src/hal/mod.rs"
      ],
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.interrupts.interrupt-controller-dispatch",
          "to_member": "kolvrt.kernel.view.boot.hardware-abstraction-selection"
        }
      ]
    },
    {
      "from": "kolvrt.kernel.view.diagnostics",
      "to": "kolvrt.kernel.view.boot",
      "kind": "call",
      "label": "Write console bytes through bounded MMIO",
      "evidence": [
        "crates/kernel/src/diagnostics/mod.rs",
        "crates/kernel/src/hal/mod.rs"
      ],
      "member_endpoints": [
        {
          "from_member": "kolvrt.kernel.view.diagnostics.polled-console-writer",
          "to_member": "kolvrt.kernel.view.boot.hardware-abstraction-selection"
        }
      ]
    }
  ]
}
```

[Английский оригинал](../../../../docs/architecture/kernel-component-map.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.architecture.kernel-component-map",
  "kind": "subsystem-contract",
  "summary": "Source-derived logical view of kernel responsibilities, EL0 services and typed interactions.",
  "depends_on": ["adr.0010", "adr.0008", "doc.kolvrt.architecture.arena"],
  "units": [
    {
      "id": "kolvrt.kernel.view.boot",
      "anchor": "kolvrt-kernel-view-boot",
      "kind": "subsystem-contract",
      "summary": "View grouping: Boot and platform.",
      "depends_on": ["adr.0010", "adr.0005"]
    },
    {
      "id": "kolvrt.kernel.view.memory",
      "anchor": "kolvrt-kernel-view-memory",
      "kind": "subsystem-contract",
      "summary": "View grouping: Memory and address spaces.",
      "depends_on": ["adr.0010", "kolvrt.process.asid"]
    },
    {
      "id": "kolvrt.kernel.view.interrupts",
      "anchor": "kolvrt-kernel-view-interrupts",
      "kind": "subsystem-contract",
      "summary": "View grouping: Exceptions and interrupts.",
      "depends_on": ["adr.0014", "adr.0010"]
    },
    {
      "id": "kolvrt.kernel.view.smp",
      "anchor": "kolvrt-kernel-view-smp",
      "kind": "subsystem-contract",
      "summary": "View grouping: SMP and per-CPU state.",
      "depends_on": ["adr.0012"]
    },
    {
      "id": "kolvrt.kernel.view.scheduler",
      "anchor": "kolvrt-kernel-view-scheduler",
      "kind": "subsystem-contract",
      "summary": "View grouping: Scheduling and task ownership.",
      "depends_on": ["doc.kolvrt.kernel.scheduler", "adr.0016"]
    },
    {
      "id": "kolvrt.kernel.view.process",
      "anchor": "kolvrt-kernel-view-process",
      "kind": "subsystem-contract",
      "summary": "View grouping: Processes, images and lifecycle.",
      "depends_on": [
        "kolvrt.process.lifecycle",
        "kolvrt.services.supervision",
        "kolvrt.apps.native-elf"
      ]
    },
    {
      "id": "kolvrt.kernel.view.user-copy",
      "anchor": "kolvrt-kernel-view-user-copy",
      "kind": "subsystem-contract",
      "summary": "View grouping: User-copy boundary.",
      "depends_on": ["kolvrt.memory.user-copy"]
    },
    {
      "id": "kolvrt.kernel.view.authority",
      "anchor": "kolvrt-kernel-view-authority",
      "kind": "subsystem-contract",
      "summary": "View grouping: Names, rights and resource authority.",
      "depends_on": [
        "kolvrt.handles.local",
        "kolvrt.security.capability-revocation",
        "kolvrt.security.domains"
      ]
    },
    {
      "id": "kolvrt.kernel.view.wait",
      "anchor": "kolvrt-kernel-view-wait",
      "kind": "subsystem-contract",
      "summary": "View grouping: Wait, wake and synchronization.",
      "depends_on": ["doc.kolvrt.kernel.wait", "kolvrt.ipc.wait"]
    },
    {
      "id": "kolvrt.kernel.view.ipc",
      "anchor": "kolvrt-kernel-view-ipc",
      "kind": "subsystem-contract",
      "summary": "View grouping: Bounded IPC.",
      "depends_on": ["kolvrt.ipc.transport", "kolvrt.ipc.request"]
    },
    {
      "id": "kolvrt.kernel.view.clock",
      "anchor": "kolvrt-kernel-view-clock",
      "kind": "subsystem-contract",
      "summary": "View grouping: Time and own-task observations.",
      "depends_on": ["kolvrt.clock.query", "kolvrt.clock.query.api"]
    },
    {
      "id": "kolvrt.kernel.view.devices",
      "anchor": "kolvrt-kernel-view-devices",
      "kind": "subsystem-contract",
      "summary": "View grouping: Immutable device identity.",
      "depends_on": ["kolvrt.devices.observations"]
    },
    {
      "id": "kolvrt.kernel.view.diagnostics",
      "anchor": "kolvrt-kernel-view-diagnostics",
      "kind": "subsystem-contract",
      "summary": "View grouping: Diagnostics and boot console.",
      "depends_on": ["adr.0003", "adr.0010"]
    },
    {
      "id": "kolvrt.kernel.view.supervisor",
      "anchor": "kolvrt-kernel-view-supervisor",
      "kind": "subsystem-contract",
      "summary": "View grouping: Native supervisor.",
      "depends_on": ["kolvrt.services.supervision", "kolvrt.apps.native-elf"]
    },
    {
      "id": "kolvrt.kernel.view.counter-service",
      "anchor": "kolvrt-kernel-view-counter-service",
      "kind": "subsystem-contract",
      "summary": "View grouping: Counter service.",
      "depends_on": ["kolvrt.apps.native-elf"]
    }
  ]
}
```
