# Загрузка native-ядра

Текущая реализация загружает самостоятельное ядро EL1 с двумя активными CPU, без userspace, scheduler и compatibility paths. [Решение о фундаменте](../architecture-decisions/0010-kernel-foundation.md) определяет принятые границы.

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

После инициализации UART оба профиля выводят ASCII-эмблему проекта и имя KOLVRT. Исходный PNG пользователя сохранён в assets/branding/logo.png; assets/branding/boot-logo.txt включён как immutable text. Загрузка не декодирует изображение и не выделяет память heap для баннера. Человекочитаемый вывод предшествует структурированным test events.

## Проверка

Запустите `cargo xtask test`. Загрузочные образы DEV и PROD действительно выполняют выделение, отображение, доступ к памяти, снятие отображения, освобождение и доставку таймерного IRQ перед публикацией структурированного события загрузки. Отсутствие событий, panic, fatal exception или timeout завершают host command ошибкой. См. [тестирование](testing.md) и [unsafe-границы](unsafe.md).

[SMP протокол](smp.md) добавляет cleaned release/acquire publication, secondary root activation, local interrupts с соответствующей affinity и подтверждённый retirement. Оба boot profile требуют настоящего secondary IPI и подтверждённого CPU_OFF до сообщения boot success.
