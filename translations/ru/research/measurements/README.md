# История воспроизводимых измерений

Измерения включены в тестовый запуск ядра. Каждый успешный cargo xtask test проверяет настоящие samples и пишет target/kernel/measurement.json. Явная опция record дополнительно создаёт новый файл истории в research/measurements/runs; существующая запись не перезаписывается.

## Команды

```text
cargo xtask test --record cpu0-baseline
cargo xtask compare research/measurements/runs/BASELINE.json research/measurements/runs/CANDIDATE.json
```

Используйте настоящие пути записей, которые печатает test command. Запись заново выполняет полную матрицу DEV/PROD и четыре контроля распространения отказов. Изменение исходников во время выполнения отклоняет запись. Git revision, dirty state, нормализованные source hashes, ELF hashes/features/размеры, точная конфигурация emulator и raw samples идентифицируют проверенную реализацию.

## Тесты и сравнения

Тест allocator cursor_preserves_ownership_and_linear_scan_work упражняет настоящий pool с оптимизацией cursor и без неё. Оба варианта обязаны выделить одинаковые принадлежащие им единицы и правильно их освободить. Вариант с cursor обязан просмотреть линейное число единиц; reference пересканирует занятый префикс и показывает квадратичную работу. Exhaustion не должен просматривать дополнительные единицы. Счётчики просмотров существуют только под cfg(test), поэтому native DEV и PROD не платят runtime cost. Это доказывает контракт работы конкретной оптимизации, а не превосходство над free lists или buddy allocators.

Kernel lock measurements содержат scope, units, frequency, прогрев, iterations, raw samples и median/p95/p99. Host пересчитывает quantiles по samples и отклоняет несогласованные записи. Compare требует совпадения emulator arguments/version, CPU scope, accelerator, compiler, target, features, workload, units, frequency, прогрева и iterations. Он сообщает наблюдаемые различия, а не автоматически принимает метод или отклоняет его по произвольному временному порогу.

## Правило принятия

Сохраняйте предыдущие runs при изменении реализации. Собирайте повторные парные runs в сравнимых условиях; проверяйте надёжность, exhaustion, concurrency и tail behavior вместе со скоростью. Связывайте выбранную замену с ADR и её отрицательными tests через [ревью методов](../../../../docs/architecture/implementation-review.md). Одной более быстрой median недостаточно для принятия.

Текущие измерения lock — наблюдения TCG timer с ограниченным разрешением, включая нулевые дельты. Они не доказывают аппаратный throughput или то, что все выбранные алгоритмы ядра самые быстрые. Численные regression budgets требуют стабильной нагрузки и проверенного обоснования; здесь они не выдумываются. Форматирование может менять пробелы JSON, но нельзя переписывать значения samples ради успешного сравнения.
