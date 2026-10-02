# Реестр первичных источников

Дата обращения ко всем записям: 2026-10-02. URL прочитаны через web retrieval; вторичные статьи не обосновывают архитектурные выводы. Изложения — пересказ, а не воспроизведение документации. Даты обхода поиска не являются датами публикации. Для изменяемых страниц нет воспроизводимого commit, если он явно не указан; не считать их гарантиями всего выпуска.

| ID  | Первичный источник                                                                                                    | Издатель, редакция и область даты                             |
| --- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| K01 | [CE 1.2 overview](https://support.kaspersky.com/help/KCE/1.2/en-US/overview.htm)                                      | Kaspersky; CE 1.2; undated HTML                               |
| K02 | [CE architecture](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_architecture.htm)                         | Kaspersky; CE 1.2; undated HTML                               |
| K03 | [CE IPC control](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_ipc_control.htm)                           | Kaspersky; CE 1.2; undated HTML                               |
| K04 | [CE resource access](https://support.kaspersky.com/help/KCE/1.2/en-US/overview_resource_acces_control.htm)            | Kaspersky; CE 1.2; undated HTML                               |
| K05 | [CE component specifications](https://support.kaspersky.com/help/KCE/1.2/en-US/ice.htm)                               | Kaspersky; CE 1.2; undated HTML                               |
| K06 | [CE IDL](https://support.kaspersky.com/help/KCE/1.2/en-US/ice_idl.htm)                                                | Kaspersky; CE 1.2; undated HTML                               |
| K07 | [CE guide](https://support.kaspersky.com/help/KCE/1.2/en-US/KasperskyOS-CE-en-US.pdf)                                 | Kaspersky; CE 1.2; copyright 2024; printed pp. 246–249        |
| K08 | [Developer FAQ](https://os.kaspersky.com/faq-general/)                                                                | Kaspersky; living/unversioned; date not stated                |
| K09 | [Historical security policies](https://support.kaspersky.com/kos/educationkitbeta0.1/en-us/security_policies.htm)     | Kaspersky; Education Kit Beta 0.1; reviewed 2020-05-19        |
| S01 | [seL4 capabilities](https://docs.sel4.systems/Tutorials/capabilities.html)                                            | seL4 project; living tutorial; version not pinned             |
| S02 | [seL4 IPC](https://docs.sel4.systems/Tutorials/ipc.html)                                                              | seL4 project; living tutorial; version not pinned             |
| S03 | [Verified configurations](https://docs.sel4.systems/projects/sel4/verified-configurations.html)                       | seL4 project; living configuration matrix                     |
| S04 | [Proof assumptions](https://sel4.systems/Verification/assumptions.html)                                               | seL4 project; living verification scope                       |
| F01 | [Zircon handles](https://fuchsia.dev/fuchsia-src/concepts/kernel/handles)                                             | Fuchsia project; living documentation                         |
| F02 | [Zircon rights](https://fuchsia.dev/fuchsia-src/concepts/kernel/rights)                                               | Fuchsia project; updated 2025-03-22                           |
| F03 | [Driver framework DFv2](https://fuchsia.dev/fuchsia-src/concepts/drivers/driver_framework)                            | Fuchsia project; living documentation                         |
| F04 | [IOMMU create](https://fuchsia.dev/reference/syscalls/iommu_create)                                                   | Fuchsia project; updated 2025-12-05                           |
| F05 | [BTI create](https://fuchsia.dev/reference/syscalls/bti_create)                                                       | Fuchsia project; updated 2025-03-04                           |
| F06 | [Starnix RFC-0082](https://fuchsia.dev/fuchsia-src/contribute/governance/rfcs/0082_starnix)                           | Fuchsia project; historical design RFC, 2021                  |
| F07 | [Starnix UAPI implementation](https://fuchsia.dev/fuchsia-src/concepts/starnix/kernel)                                | Fuchsia project; living implementation description            |
| R01 | [Redox goals and security design](https://raw.githubusercontent.com/redox-os/book/master/src/our-goals.md)            | Redox project book; mutable master, no commit pin             |
| L01 | [Linux LSM development](https://docs.kernel.org/security/lsm-development.html)                                        | Linux project; living documentation, not a kernel release pin |
| L02 | [Linux DMA guide](https://docs.kernel.org/core-api/dma-api-howto.html)                                                | Linux project; living documentation, not a kernel release pin |
| C01 | [Capsicum paper](https://www.cl.cam.ac.uk/research/security/capsicum/papers/2010usenix-security-capsicum-website.pdf) | Watson/Anderson/Laurie/Kennaway; USENIX Security 2010         |

## Места утверждений и ограничения

При завершающей проверке 2026-10-02 повторно прочитаны K02–K06, страницы политики K07,
обсуждение DMA K08, S02–S04, F03/F04, R01 и L01. CE 1.2 описывает совместные проверки
OCap и политики, включая оба направления IPC; это не подтверждает трактовку только выдачи прав.
Матрица конфигураций seL4 теперь указывает целостность/доступность и конфиденциальность AArch64,
исключая из этой области трансляцию устройств и функции boot/debug. Доказательства seL4
не переносятся на KOLVRT. Указанная книга Redox всё ещё описывает namespaces/UID/GID
и планируемый переход к capabilities. Это документальные наблюдения, а не повторённые сборки.

K01/K02 обосновывают обзор и привилегированные механизмы; K03 — проверки IPC; K04 — сочетание полномочий; K05/K06 — генерацию интерфейсов; K07/K09 — состояния политики; K08 — условную DMA-модель. S01/S02 — capabilities и IPC; S03/S04 — границы доказательств. F01–F07 — handles, driver hosts, IOMMU и размещение совместимости. R01 описывает документированную namespace/identity-модель Redox и планируемый переход к capabilities, а не завершённое универсальное объектное обеспечение. L01/L02 — hooks и различие адресов DMA. C01 — контрпример отождествлению capabilities и микроядерного размещения.

## Недоступные свидетельства

Не получены сторонняя сборка/запуск, аудит исходников коммерческого ядра, сопоставимые privileged LOC, IPC benchmark, враждебный DMA-запуск и доказательство масштабирования общего назначения. Угаданные URL CE с ошибкой retrieval исключены. Сайт книги Redox вернул 403; прочитано raw-зеркало книги официального проекта. Термины Education Kit явно исторические. Публичные маркетинговые обещания не превращены в гарантии KOLVRT. Будущие эксперименты отдельно закрепляют редакции исходников и аппаратные конфигурации.

[Английский оригинал](../../../../../research/other-systems/kasperskyos/sources.md)
