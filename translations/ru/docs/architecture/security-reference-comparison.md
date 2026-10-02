# Сравнение систем безопасности

Документальное исследование, 2026-10-02. Нет баллов победителей и сторонних benchmarks. KOLVRT описывает намерения, а не реализованную защиту. [Редакции источников и пределы свидетельств](../../research/other-systems/kasperskyos/sources.md).

## Первичные источники

Linux: [LSM hooks](https://docs.kernel.org/security/lsm-development.html) и [DMA](https://docs.kernel.org/core-api/dma-api-howto.html). KasperskyOS CE 1.2: [архитектура](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_architecture.htm), [IPC](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_ipc_control.htm), [полномочия ресурсов](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_resource_acces_control.htm). seL4: [capabilities](https://docs.sel4.systems/Tutorials/capabilities.html), [IPC](https://docs.sel4.systems/Tutorials/ipc.html), [границы доказательств](https://docs.sel4.systems/projects/sel4/verified-configurations.html). Zircon: [handles](https://fuchsia.dev/fuchsia-src/concepts/kernel/handles), [driver hosts](https://fuchsia.dev/fuchsia-src/concepts/drivers/driver_framework), [RFC Starnix](https://fuchsia.dev/fuchsia-src/contribute/governance/rfcs/0082_starnix). Redox: [официальная книга](https://raw.githubusercontent.com/redox-os/book/master/src/our-goals.md). Источники обосновывают ячейки механизмов; неизвестное означает отсутствие подтверждения здесь.

## Механизмы

| Свойство                 | Linux                        | KasperskyOS CE 1.2    | seL4                         | Fuchsia/Zircon           | Описанный Redox                | Предлагаемый KOLVRT                |
| ------------------------ | ---------------------------- | --------------------- | ---------------------------- | ------------------------ | ------------------------------ | ---------------------------------- |
| Привилегированный размер | Не измерен                   | Не измерен            | Не измерен                   | Не измерен               | Не измерен                     | Учёт цепочки; LOC не измерен       |
| Привилегии драйвера      | Возможны kernel modules      | Обычно user mode      | User drivers                 | User driver hosts        | User services/drivers          | EL0 по умолчанию; решение нагрузки |
| IPC                      | Зависит от подсистемы        | Rendezvous            | Endpoints                    | Channels/handles         | Schemes                        | Bounded frames; транспорт открыт   |
| Полномочия               | Credentials/LSM              | OCap и политика       | Object capabilities          | Handle rights            | Namespaces, UID/GID            | Native grants, область эффектов    |
| Default deny             | Зависит от policy/config     | Явная политика        | Зависит от grant/config      | Зависит от grant/routing | Цель безопасных defaults       | Без native grant — отказ           |
| Capability-модель        | Не равна object caps         | Handle rights         | CSpace/CNode                 | Process-bound handles    | Capability-переход планируется | Узкие типы                         |
| Оценка политики          | Operation hooks              | IPC security module   | Kernel authority/user config | Rights/services          | Scheme/identity проверки       | Выдача и нужные текущие проверки   |
| Домены изоляции          | Процессы; modules privileged | Процессы              | VSpaces/CSpace               | Process/host             | Processes/namespaces           | Настоящие адресные пространства    |
| Локализация отказа       | Зависит от boundary/config   | Зависит от policy/TCB | Зависит от assumption/config | Общий host делит риск    | Здесь не воспроизведена        | Явная область ущерба/TCB           |
| Изоляция compat          | Зависит от механизма         | Здесь неизвестна      | Зависит от нагрузки          | Userspace-проект Starnix | Relibc/ports                   | Отдельная граница доверия          |
| Стоимость runtime policy | Не измерена                  | Не измерена           | Не измерена                  | Не измерена              | Не измерена                    | Требуется эксперимент              |

## Ограничения и выводы

Driver hosts допускают совместное размещение драйверов. [Stub IOMMU](https://fuchsia.dev/reference/syscalls/iommu_create) Zircon удерживает память без аппаратной защиты доступа. [DMA-допущения seL4](https://sel4.systems/Verification/assumptions.html) отделены от CPU-memory доказательств. План capability-перехода Redox не подтверждает завершённую реализацию. [Описание Starnix](https://fuchsia.dev/fuchsia-src/concepts/starnix/kernel) показывает UAPI/user-copy и conformance, а не нужную KOLVRT политику сохранения.

[Исследование Capsicum](https://www.cl.cam.ac.uk/research/security/capsicum/papers/2010usenix-security-capsicum-website.pdf) даёт контрпример отождествлению capabilities и микроядерного размещения. Рекомендовать атрибуцию, ограниченное делегирование и явные допущения; не выводить лучшую скорость/безопасность из названий архитектуры. [Реестр решений](../../research/other-systems/kasperskyos/kolvrt-lessons.md).

[Английский оригинал](../../../../docs/architecture/security-reference-comparison.md)
