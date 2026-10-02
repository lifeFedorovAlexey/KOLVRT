# Vision — Phase 0.1

KOLVRT начинает с контрактов, а не с boot stub. Источник истины — текущая native
specification. Совместимость является явно выбранным versioned поведением вокруг неё.
Существование shim не разрешает обходить native memory safety, authorization, ownership
или resource accounting. Невыразимое безопасно поведение отклоняется явно.

Основная платформа: AArch64, 64-bit only, EL1, MMU, SMP, GICv3, ARM Generic Timer,
Device Tree, UART, VirtIO на QEMU virt. PCIe рассматривается как расширение транспорта;
ACPI и x86_64 — последующие порты, не предпосылки первого запуска.

Совместимость не глобальный режим. Разные executable, process, driver, device и API
families одновременно могут иметь разные routes. Минимальная единица смены маршрута
определяется общим состоянием и протоколом, а не удобством UI.

Native core не содержит legacy syscall numbers, старые layouts и условные ветки по
версии приложения. Compatibility modules имеют identity, version, owner, consumers,
метрики, ограничения, migration/removal contract. Native-only build не зависит от них.
Аппаратная необходимость отличается от программного legacy: CPU erratum может быть
обязателен для корректной работы даже полностью native build на affected CPU.

Linux policy защищает реально работающих пользователей от регрессий; KOLVRT не
объявляет эту цель ошибкой. Предлагаемое отличие — изоляция сохраняемой семантики и
видимость цены, а не обещание совместимости без затрат. См. [Linux regression policy](https://docs.kernel.org/process/handling-regressions.html)
и [внутренние интерфейсы Linux](https://docs.kernel.org/process/stable-api-nonsense.html).

Ошибку самого KOLVRT исправляют в native specification/implementation вместе с
regression test. Если безопасное старое поведение действительно нужно consumer,
оно получает bug-compat ID и migration plan. Ошибка, нарушающая isolation, не может
вернуться как привилегия shim. Наличие потребителей измеряется по declared dependencies
и наблюдениям; ноль вызовов за короткое окно не доказывает отсутствие зависимости.

Цели Phase 0.1: структурированные cases, источники, проверяемые laws, модели и ADR.
Не цели: kernel implementation, Linux binary compatibility implementation, driver port,
выбор scheduler по вкусу, performance claims без измерений, полная историческая
реконструкция каждой подсистемы. [Backlog](research/PHASE_0_2.md) перечисляет следующие доказательства.

Все MUST/обязан/запрещено в architecture documents — нормативные требования к будущим
реализациям. Документы не утверждают, что runtime эти требования уже выполняет.
