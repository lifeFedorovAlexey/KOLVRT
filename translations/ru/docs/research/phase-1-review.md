# Ревью самостоятельного фундамента Phase 1

Завершённый программный фундамент — самостоятельное ядро EL1 с одним активным CPU. SMP явно отложен в рамках разрешённого пользователем предыдущего законченного уровня. Scheduler, userspace и compatibility не реализованы и не запланированы автоматически.

## Свидетельства Definition of Done

| Требование            | Свидетельство                                                         |
| --------------------- | --------------------------------------------------------------------- |
| Сборка clean checkout | Локальный Git checkout кандидата; locked build и kernel matrix прошли |
| Загрузка QEMU AArch64 | Boot images DEV и PROD выполнены                                      |
| EL1                   | CurrentEL прочитан и проверен в ядре                                  |
| UART                  | Настоящая идентичность PL011 registers и serial output на host        |
| Exceptions            | Probes BRK, data abort и instruction abort                            |
| Physical allocator    | Reservation, allocation, free, reuse и exhaustion                     |
| MMU                   | Настоящие translation, unmap, RO и NX faults                          |
| Kernel heap           | Настоящие Box/Vec access и exhaustion                                 |
| Timer IRQ             | Physical timer delivery, masking и rearming                           |
| In-kernel harness     | По 23 именованных теста на профиль                                    |
| Failure propagation   | Четыре отрицательные host commands завершаются кодом 1                |
| DEV                   | Diagnostic core build и настоящая загрузка                            |
| PROD                  | Тот же оптимизированный core; boot features пусты                     |
| Native isolation      | Native dependency closure содержит только kernel/core/runtime         |
| Unsafe audit          | 48 lexical locations, один assembly file, три runtime artifacts       |
| Документация          | Английские/русские контракты и ревью реализации                       |
| CI/local command      | Локальный npm run check прошёл; remote CI настроен, не запускался     |

[Машинные свидетельства](../../../../research/results/kernel-foundation.json) сохраняют source hashes, ELF identities, точные настройки emulator, measurements и идентичность clean candidate. [Unsafe inventory](../../../../research/results/kernel-unsafe-audit.json) сохраняет реальные locations. Lexical count включает declarations и attributes; это не число unsafe blocks и не доказательство безопасности.

## Репозиторий и загрузка

```text
crates/
  kernel-core/      safe bounded algorithms and boot-description parser
  kernel/          arch/aarch64, platform, hal, memory, interrupt, sync, time, diagnostics
  xtask/           build, QEMU runner, failure controls, audit
  native-protocol-model/
  native-state-models/
  repository-checks/
docs/kernel/       boot, memory, exceptions, interrupts, time, smp, testing, unsafe
research/          cases, sources, results, requests, fixtures
scripts/           pinned QEMU setup
.github/workflows/ kernel checks
translations/ru/   mirrored documentation
```

ELF entry → EL1 → проверенное boot description → UART → vectors → physical discovery/reservations → allocator → page tables/MMU → heap → GICv3 → physical timer IRQ → memory validation → test harness или boot validation → PSCI shutdown. [Подробности загрузки](../kernel/boot.md) и [команды](../kernel/testing.md) описывают точные предположения.

## Результаты и размеры

Прошли 45 host tests. DEV и PROD прошли по 23 настоящих in-kernel tests. Отдельные boot images прошли. Контроли assertion, panic, второго owner и retained mapping вернули host exit 1. State-model evidence и negative controls Phase 0 также воспроизведены; они остаются отдельными от выполнения ядра. Детерминированный тест allocator также проверяет эквивалентность владения и линейное сканирование относительно пересканирующего reference. [История измерений](../../research/measurements/README.md) сохраняет настоящие samples и отклоняет несогласованные сравнения.

| Boot image | ELF bytes | PT_LOAD file bytes | PT_LOAD memory bytes |
| ---------- | --------: | -----------------: | -------------------: |
| DEV        |   1611120 |              95571 |               914771 |
| PROD       |    266368 |              37484 |               852588 |

ELF сохраняет debug information. Абсолютные пути исходников влияют на debug bytes и полный ELF hash между checkouts; подтверждается воспроизводимость сборки и выполнения, а не побитовая идентичность binary. Числа относятся к сохранённым workspace artifacts; размеры ELF другого checkout могут немного отличаться. Timing samples — TCG counter ticks, а не аппаратный throughput и не сравнительное превосходство алгоритмов.

## Оставшиеся архитектурные вопросы

SMP требует состояния каждого CPU, affinity discovery и подтверждённого remote retirement. Вторая платформа и настоящая аппаратура требуют независимых контрактов и tests. Растущие нагрузки heap требуют сравнения allocators; адресный TLBI требует данных корректности и производительности. Полный exception-stack unwinding и обработка stack overflow остаются будущей работой. Закрытая идентичность Frame обеспечена visibility, но отдельный compile-fail fixture остаётся пробелом проверки. Лицензия проекта по-прежнему требует решения владельца.

[Ревью методов](../architecture/implementation-review.md) фиксирует альтернативы, ограничения надёжности и проверки исторических ошибок до будущего принятия. Известный механизм отказа не допускается как временная архитектура.
