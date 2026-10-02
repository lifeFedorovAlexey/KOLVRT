# Вход в исключения и восстановление

Шестнадцать слотов векторов AArch64 направляют синхронные faults, IRQ или fatal paths. Rust выполняется со стеком, выровненным на 16 байт. Входной frame размером 784 байта сохраняет все общие регистры, Q0–Q31, FPCR и FPSR. Закреплённый Rust runtime поддерживает NEON; сохранение только общих регистров повреждало бы прерванный код. Настоящий таймерный тест намеренно перезаписывает SIMD/FP-состояние внутри IRQ и проверяет восстановление.

## Политика faults

PROD не содержит механизма current-EL test recovery. Неожиданные kernel exceptions маскируют IRQ, останавливают таймер, публикуют ошибку при доступном UART и вызывают PSCI shutdown. DEV дополнительно сообщает ESR, FAR и ELR. Сборки kernel-tests допускают current-EL recovery только по точному зарегистрированному PC probe для BRK, data abort или instruction abort. Тесты проверяют класс fault ESR и FAR при настоящей записи в RO, чтении unmapped и выборке инструкции NX.

Lower-EL AArch64 synchronous/IRQ vectors используют frame размером 816 байт, расширяющий базовый полями ELR, SPSR, SP_EL0 и TPIDR_EL0. [EL0 scheduler](el0.md) записывает user fault как terminal process outcome и возобновляет другой runnable process; он не превращает содержимое user registers в privileged authority. Возврат в kernel восстанавливает сохранённый ABI и native root. Compile-time frame assertions и настоящие проверки GPR/SIMD/FP/TLS/stack покрывают этот путь в обоих profiles.

Векторы устанавливаются до тестов защиты памяти. IRQ не вкладываются в этом milestone. Каждый активный CPU имеет собственный постоянный выровненный stack и probe state. Обнаружение stack overflow остаётся отложенным. [Unsafe-инварианты](unsafe.md) описывают предположения ассемблера; [границы SMP](smp.md) определяют область конкурентности.
