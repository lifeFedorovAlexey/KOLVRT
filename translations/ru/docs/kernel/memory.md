# Владение памятью и отображения

Безопасный bitmap pool в `kernel-core` имеет одного владельца, проверяемые границы и счётчик свободных единиц. Физический экземпляр содержит 65 536 единиц по 4 KiB с bitmap размером 8 KiB. Загрузочная RAM до конца ядра, DTB, стек, таблицы и firmware reservations исключаются до выделения. Выделенная RAM обнуляется до публикации. Некопируемый Frame с закрытым адресом несёт владение; Mapping заимствует его. Освобождение отклоняет сохранённые динамические отображения, включая забытый mapping guard.

## Модель адресов

TTBR0 охватывает 39-битное identity-пространство; TTBR1 отключён. RAM имеет страницы по 4 KiB: text — RO executable, rodata — RO NX, остальная RAM — RW NX. Динамические отображения данных занимают 0x80000000–0x801fffff, 512 слотов. Регионы устройств используют разреженные блоки Device-nGnRnE по 2 MiB и NX. Покрытие блока устройства может включать соседние регистры; это платформенное окно EL1, а не граница защиты userspace.

Map отклоняет неверное выравнивание, W+X, исполняемые динамические данные, алиасы ядра, адреса вне диапазона и занятые слоты. Unmap сохраняет retiring frame charge, очищает дескриптор, завершает local TLBI и ждёт reader quiescence CPU1 вместе с подтверждёнными local TLBI/barriers до возврата владения pool. Полная инвалидация имеет измеримую стоимость; адресная требует отдельного ревью. Привилегированный identity-алиас сохраняется; владение определяет доступ после освобождения.

## Heap

[Фундамент EL0](el0.md) дополнительно удерживает UserSpace allocations из шести страниц. Заимствованные frames и постоянные per-space charges защищают lifetime tables/code/data/stack даже при забывании guard. Roots immutable во время admission, user aliases не дают доступа к kernel mappings, а оба CPU восстанавливают native root с local TLBI до снятия charges. Native mapping mutation завершается отказом при активном user batch; это ограниченное admission rule, а не постоянное CPU0 ownership для будущих workloads.

Heap постоянно владеет 16 выровненными физическими страницами: 64 KiB, квант выделения 64 байта, bitmap 128 байт. Выделение ограничено, проверяет выравнивание и возвращает null при исчерпании. Lock защищает метаданные; обработчики IRQ не входят в allocator. Освобождение требует исходного живого указателя и точного Layout согласно GlobalAlloc.

Выделение непрерывного диапазона может отказать из-за фрагментации даже при положительном счётчике свободных единиц. Квантование теряет до 63 байт на округлённое выделение; выравнивание добавляет фрагментацию. Этот ограниченный фундамент не объявляется самым быстрым allocator для будущих нагрузок. [Ревью решений](../architecture/implementation-review.md) сравнивает альтернативы. Тесты проверяют исчерпание, повторное использование, коллизии, настоящую трансляцию и faults прав доступа.

## Multicore retirement

Ownership Physical/Frame нельзя передавать или разделять; affinity checks сохраняют allocation и изменение PTE за CPU0. Это граница текущего workload, а не постоянная allocation policy. Существующая блокировка heap metadata поддерживает обычный код обоих CPU. Новые mappings отклоняются при pending retirement. Забытый Retirement guard сохраняет charge; release отклоняется даже после очистки PTE. Table locks освобождаются до ожидания. Timeout/failure является fatal и не разрешает reuse. [SMP контракт](smp.md) определяет publication, reader assumptions и ограничения identity alias.

[Контракт image Phase 2](routing.md) расширяет private stacks до четырёх pages и удерживает bounded RX payload pages в том же borrowed/charged UserSpace lifetime. Firmware или user input не выбирают unchecked physical image address; construction проверяет allocation/page extents до publication.
