# A/B migration benchmark methodology

Здесь только методология. **Benchmark runs и результаты отсутствуют.** Native не считается
быстрее заранее; artificial delays, неравные quotas и отключение safety только для одного
пути запрещены. Более быстрый compat path — результат, требующий анализа native path.

## Предусловия

Перед запуском сформулировать workload, correctness oracle, semantic differences, primary
metric, нагрузку и stopping rule. Если native и compat дают разные observable effects,
сначала определить эквивалентный полезный результат и отдельно оценить стоимость translation.
Сравнение разных guarantees, например durable и buffered write, не является A/B одной задачи.

Оба пути получают одинаковые inputs, resource limits, scheduling domains, memory layout
policy и device setup. Fixtures сбрасываются между paired runs. Shadow execution не
дублирует реальные writes, messages или payments: replay в независимых snapshots.
Порядок A/B и B/A чередуется или randomized с сохранённым seed. Не запускать одновременно
на одном contended device, если сравнивается isolated latency.

Записывать exact kernel/adapter digests, route manifests, Rust/compiler/linker flags,
QEMU machine/CPU/version/accelerator или hardware stepping/firmware, RAM, SMP/affinity,
frequency governor, thermal state, interrupt placement, background load и instrumentation.
QEMU throughput не переносится на silicon без измерения. CPU pinning не применяется
только к одной стороне. Cold-start и steady-state — отдельные эксперименты.

## Sampling и статистика

Warm-up criterion задаётся до запуска: например стабилизация нескольких окон с заданным
допуском и максимальным числом окон. Сохранять warm-up samples с marker, но не смешивать
их со steady-state distribution. Нестабильный warm-up даёт invalid run, а не удобный tail cut.

Начальная экспериментальная политика: минимум 30 независимых paired runs и для оценки
p99 минимум 10000 completions на сторону внутри заявленного steady-state scope. Это
начальные lower bounds, не гарантия статистической точности. Pilot оценивает autocorrelation
и необходимый budget до основного эксперимента. Если tail samples/independent runs
недостаточно, p99/CI обозначить inconclusive. Stopping rule запрещено менять после взгляда
на победителя; sequential testing требует заранее описанного метода.

Отчёт включает n, run duration, median, p95, p99, mean, sample variance и standard deviation.
Для percentile использовать заранее закреплённый estimator, например nearest-rank
`sorted[ceil(p*n)-1]`; не менять estimator между сторонами. Для confidence intervals
использовать paired/bootstrap по независимым runs или blocks, а не считать каждый
correlated request независимым. Сохранять и absolute difference, и ratio, и uncertainty.
При baseline=0 ratio undefined. Не усреднять per-run p99 как будто это pooled p99:
показывать distribution per-run quantiles и отдельно явно названный pooled estimator.

Timeouts, errors, dropped requests и unfinished operations не исключаются из denominator.
Публиковать их counts и latency censoring; если tail censored, честный p99 может быть
только lower bound. Не скрывать outliers; исключения допустимы только по predeclared
invalid-run criteria с причиной и полными raw samples. Multiple workloads публикуются
полностью; summary weights фиксируются заранее. Regression margins задаются по product
requirements, а не подгоняются под measured variance.

## Метрики

| Метрика | Определение |
|---|---|
| Wall latency | admission -> terminal outcome; queue time и service time отдельно |
| Throughput | successful useful units / wall seconds; errors рядом |
| CPU | exclusive thread/kernel CPU и attributable deferred work; cycles отдельно |
| RAM | private resident, shared mappings, pinned bytes, high-water; не суммировать shared pages повторно |
| Allocations | count, allocated bytes, peak live bytes; allocator и размерный профиль |
| Context switches | voluntary/involuntary; scheduler scope фиксирован |
| Copies | bytes + count по явно instrumented copy boundary |
| Locks | acquisition count, contended count, wait time distribution |
| Translations | adapter entries, conversion count, bytes serialized/deserialized |
| PMU | cycles, instructions, cache/TLB misses если доступны; event encoding, multiplexing и sampling error |
| Observer overhead | matched runs instrumentation on/off и lost-event counters |

Нельзя вычитать noisy observer overhead и выдавать результат за точное измерение.
Показывать наблюдаемую разницу и uncertainty. Unsupported counter = unavailable.
Разделять adapter cost, shared native backend cost и end-to-end cost; их нельзя смешивать
в единственный «compat slowdown». Requirements конкретных workloads есть в JSON cases.

Артефакт будущего run: machine-readable manifest, workload digest, oracle result,
raw samples, rejected-run log, aggregate script version и итоговый report. Реализация
benchmark runner начинается только после появления двух настоящих сравниваемых paths.
