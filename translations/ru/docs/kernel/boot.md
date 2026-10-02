# Загрузка native-ядра

Консоль и события следуют [политике вывода](../architecture/output-policy.md). Обычные образы показывают завершённые этапы с единицами без сырого JSON. Structured evidence включается явно и сохраняется раннером в log/result files.

Текущая реализация загружает самостоятельное ядро EL1 с двумя участвующими CPU и исполняет ограниченный [EL0 scheduling workload](el0.md). Compatibility остаётся отключённым. [Решение о фундаменте](../architecture-decisions/0010-kernel-foundation.md) фиксирует предыдущий этап EL1.

QEMU 10.1.0 использует `virt-10.1,gic-version=3,virtualization=on,its=off,dtb-randomness=off`, Cortex-A57, TCG, два настроенных CPU и 256 MiB RAM. CPU0 и CPU1 исполняют native EL1 code; CPU1 запускается через PSCI CPU_ON с отдельным stack. Обнаружение платформы изолировано в `crates/kernel/src/platform`; архитектурные регистры и векторы находятся в `arch/aarch64`. Имена каталогов описывают обязанности и не означают импорт другой ОС.

## Последовательность

```text
ELF entry → mask IRQ → EL2 to EL1h if necessary → stack and BSS
→ immutable DTB validation → PL011 UART → exception vectors
→ reserved-memory discovery → physical pool → page tables and MMU
→ heap → GICv3 CPU0 interface → physical timer IRQ
→ memory validation → kernel tests or boot validation → PSCI SYSTEM_OFF
```

ELF начинается с 0x40200000, оставляя до 2 MiB у начала RAM 0x40000000 для DTB QEMU. Стек размером 256 KiB выровнен; таблица векторов выровнена на 2 KiB. Сегменты ELF PT_LOAD и права страниц обеспечивают W^X. QEMU загружает ELF напрямую; Linux boot protocol, таблица syscall и исходники Linux не включены.

После инициализации UART оба профиля выводят ASCII-эмблему проекта и имя KOLVRT. Исходный PNG пользователя сохранён в assets/branding/logo.png; assets/branding/boot-logo.txt включён как immutable text. Загрузка не декодирует изображение и не выделяет память heap для баннера. После эмблемы идут человекочитаемые этапы. Версионированные машинные записи появляются только при явном включении.

## Проверка

Запустите `cargo xtask test`. Загрузочные образы DEV и PROD действительно выполняют выделение, отображение, доступ к памяти, снятие отображения, освобождение и доставку таймерного IRQ перед публикацией структурированного события загрузки. Отсутствие событий, panic, fatal exception или timeout завершают host command ошибкой. См. [тестирование](testing.md) и [unsafe-границы](unsafe.md).

[SMP протокол](smp.md) добавляет cleaned release/acquire publication, secondary root activation, local interrupts с соответствующей affinity и подтверждённый retirement. Оба boot profile требуют настоящего secondary IPI и подтверждённого CPU_OFF до сообщения boot success.

В boot event `active_cpus` означает число CPU, участвовавших в boot validation; это не снимок online CPU в момент публикации события. `secondary_shutdown_verified: true` выводится только после того, как `smp::shutdown()` наблюдает quiescent state secondary и ответ OFF от PSCI AFFINITY_INFO. Поэтому два участвовавших CPU и проверенное отключение secondary описывают последовательные этапы. Прежнее поле `secondary_off` в сохранённых записях Phase 1.1 имело тот же смысл; исторические измерения не переписываются.

Явно выбранный opaque [EL0 routing payload](routing.md) исполняется после неизменного foundation workload и до verified secondary shutdown. Default native image исключает его; boot code не содержит adapter/version dispatch.
