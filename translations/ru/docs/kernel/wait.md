# Основа ожидания и пробуждения

Document status: CURRENT
Evidence scope: ограниченный latch собственного события процесса под доверенной bootstrap-координацией; production event loop и публичный IPC transport пока не реализованы.
Current reference: [Условия production scheduling](../architecture/production-scheduler.md)

## Контракт

Экспериментальный SVC `0x55` ожидает собственное событие исполняемого процесса в
пути `Registry::step()`. Защищённая runqueue определяет identity; userspace не
передаёт process ID, адрес события или буфер. Синхронный bootstrap `dispatch()`
не допускает эту операцию.

Регистрация публикует waiter до повторной проверки pending signal. Если сигнал
не потреблён, RUNNING переходит в BLOCKED и возвращает управление native
координатору. Trap и координатор повторно проверяют публикацию до сохранения
blocked state. BLOCKED tasks исключены из выбора, сохраняют контекст и память,
и их нельзя освободить. Шаг без READY task возвращается без исполнения EL0 и
без публикации завершения процесса.

Только доверенная координация CPU0 может вызвать `Registry::signal(id)`. Перед
доступом к событию слота проверяется точное admitted generation. Prepared,
completed и stale identities отклоняются. Потреблённое уведомление переводит
BLOCKED в READY один раз. Pending notifications объединяются; новое уведомление
после потребления сохраняется для следующего ожидания. События не содержат
payload или счётчик сообщений.

Latch закреплённого слота сбрасывается при создании, когда исполнение неактивно
и registry владеет зарезервированным слотом. Внешних retained publisher handles
пока нет. До появления конкурентных native event sources это ограничение
lifecycle потребуется заменить контрактом удержания publisher и revocation.

## Свидетельства и ограничения

[QEMU receipt исходников](../../../../research/results/scheduler-wait-block.json)
содержит 67 проверок на каждый профиль DEV/PROD и 53 отрицательных контроля.
Сценарий ожидания проверяет уведомление до регистрации, объединение уведомлений,
два последовательных ожидания, пробуждение после блокировки, пустую runnable
очередь, запрет reclamation и stale generation после повторного использования
слота. Host tests проверяют гонку публикации с регистрацией и сохранение
уведомлений. Текущий native publisher сериализован координатором; host race
не доказывает реализацию конкурентного native publisher.

Ожидающая EL0-программа выполняет один SVC на ожидание и не потребляет CPU slices
в BLOCKED. Барьер двух CPU и существующий bootstrap polling сохранены.
Production idle/event loop, общие источники ожидания, гонки close/death/shutdown,
deadlines для blocked процессов, IPC и handles с capabilities остаются за
условиями production scheduling plan.

[Английский оригинал](../../../../docs/kernel/wait.md)
