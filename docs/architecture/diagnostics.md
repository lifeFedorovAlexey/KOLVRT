# Dependency diagnostics: vector вместо искусственного score

Единственное число не определяет степень зависимости: редкий compat call может быть
необходим для boot, а миллионы дешёвых переводов могут занимать мало CPU. Call share,
CPU share, memory и возможность удалить module — разные свойства. Не суммировать их
с произвольными весами и не называть качество приложения.

Для каждого consumer, route generation и observation window показывать:

- Declared required modules и version/digest, transitive dependency graph, active bindings,
  unique consumer count и consumers с пока ненаблюдавшимися paths.
- `C_native` и `C_compat`: внешние admitted API operations по входному binding; completed,
  failed, cancelled и in-flight отдельно. Calls adapter -> native backend не считаются
  новыми внешними native calls.
- `compat_call_share = C_compat / (C_native + C_compat)`; при нулевом знаменателе unknown.
  Native share — complement только при полной классификации выбранного scope.
- Exclusive CPU time по execution domain: compat adapter, native backend, consumer,
  unattributed. Compat CPU share имеет явно выбранный denominator, например всё
  attributable service CPU; native backend под compat учитывается как native backend,
  но request route остаётся compat. Это не «доля native приложения».
- Wall latency по request route, throughput, allocations/bytes, private/shared/pinned
  footprint, copies/bytes, context switches, lock contention, translations, serialization
  и deserialization counts/bytes, syscall/API family frequency.
- Coverage, sampling rate, dropped events, disabled counters, missing asynchronous
  attribution и observation scope. Unknown не превращается в ноль.

Асинхронные work items наследуют causal request/module IDs; shared work attribution
указывает метод распределения или остаётся unattributed. Exclusive spans предотвращают
double-count nested CPU; wall intervals разных requests могут перекрываться и не суммируются
как время всего процесса. Вывод counters не раскрывает user payload или чужие credentials.

## Текстовые состояния

Это описательные labels поверх вектора, не универсальная ранжирующая шкала. Scope всегда
назван: package declared dependency, process observed workload либо device binding.

| Label | Доказательство | Цветовая подсказка |
|---|---|---|
| LEGACY | Consumer объявляет только legacy entry contracts, native contract не заявлен | brown |
| COMPAT | Все используемые в полном scope entry families требуют adapter; native entry возможен вне scope | red |
| MIXED | В scope есть native и compat routes; нижеописанный migration budget не задан/не выполнен | yellow |
| MOSTLY_NATIVE | MIXED и выполнен явно заданный consumer migration budget по calls/CPU/dependencies с coverage | light green |
| NATIVE | В объявленном support scope нет прямых/транзитивных software compat dependencies; coverage достаточен | green |

Приоритет классификации: недостаточные сведения -> `UNKNOWN` как дополнительный diagnostic
status; затем NATIVE, затем LEGACY по manifest, затем COMPAT, затем MOSTLY_NATIVE либо MIXED.
UNKNOWN не замещает пять обязательных состояний, а запрещает ложный вывод при missing data.
Для fully stripped PROD возможен только declared-state label с явной пометкой, без
выдуманных dynamic shares. MOSTLY_NATIVE требует опубликованных thresholds и rationale
конкретного migration plan; общесистемного «95 баллов» не существует.

Hardware errata вынесены в отдельную ось `hardware_translations`: native consumer на
неидеальном CPU не становится LEGACY приложением. Loaded, bound и called module counts
различаются. Даже zero observed compat calls при declared dependency не дают NATIVE.
Все цвета сопровождаются label, числом/единицей, scope и confidence. Faster compat path
показывается без штрафа score. [Benchmark methodology](benchmarking.md) определяет сравнение.
