# ADR-0021 — Жизненный цикл AArch64 ASID при фиксированном CPU

Статус: **Принято для ограниченного набора process roots**. Дата: 2026-10-03.

Document status: CURRENT
Evidence scope: два CPU с фиксированным affinity, приватные immutable process roots, AArch64 QEMU и объявленный ASID-zero fallback; migration и выполнение shared root не реализованы.
Current reference: [Адресные пространства EL0](../../../../docs/kernel/el0.md)

## Контекст

Фундамент EL0/process назначал ASID zero каждому root и выполнял полный local TLBI после каждого переключения TTBR0. Это простой correctness baseline, но он инвалидировал translations, не относящиеся к уходящему процессу. Повторное использование ASID безопасно только после того, как предыдущий владелец прекратил использовать tag, а все CPU, способные удержать translation, завершили invalidation до выдачи tag заново. Жизненный цикл процессов уже требует fixed affinity и ждёт scheduler detachment до освобождения frames.

## Решение

- Зарезервировать ASID zero для native root и явного full-flush fallback. Прочитать `ID_AA64MMFR0_EL1.ASIDBits`: использовать аппаратную 8-битную namespace либо установить `TCR_EL1.AS` и использовать 16 бит, если encoding поддерживается. Неизвестная кодировка отключает tagged mode.
- Каждому owned process root назначать ненулевой lease с ASID, монотонным software epoch, закреплённым owner CPU и allocator slot. ASID должен быть уникален среди одновременно живых roots одного CPU; аппаратная namespace ASID локальна для CPU, поэтому одинаковые числа могут одновременно использоваться на двух CPU.
- При tagged root обычный TTBR0 switch записывает root и ASID без TLBI. Возврат к native root с ASID zero в tagged mode также сохраняет независимые translations.
- Отображения процесса остаются неизменными после admission. При terminal completion фиксированный owner CPU выполняет `DSB ISH; TLBI ASIDE1; DSB ISH; ISB`; только после завершения этой local invalidation lease можно пометить retired. Acquired scheduler completion и detachment предшествуют освобождению frames/tables и повторной выдаче ASID. Transactional creation rollback может сразу вернуть lease только до публикации root.
- Сохранить запуск каждого root на одном закреплённом CPU. Root не может быть resident на нескольких CPU в этой области. Для будущей поддержки shared root или migration потребуются residency tracking и подтверждённая remote invalidation до retirement/reuse; сами ASID migration не обеспечивают.
- Сохранить ASID-zero/full-local-flush mode как для неподдерживаемого hardware, так и как явный measurement build `asid-baseline`.

## Альтернативы

- Оставлять full `VMALLE1` после каждого switch.
- Повторно выдавать tag без retirement acknowledgement или полагаться на то, что allocator случайно вернёт те же frames.
- Выполнять inner-shareable remote TLBI при каждом локальном process exit, несмотря на fixed-affinity контракт.
- Считать ASID глобальным process identity или разрешением мигрировать root.

## Почему отклонены

Полная invalidation отбрасывает translations других живых roots. Повторное использование без завершённой invalidation прежнего владельца позволяет stale VA-to-PA translation попасть в новый процесс. Совпадение адреса frame не гарантируется архитектурой. Remote invalidation не требуется, если root закреплён ровно за одним CPU; добавление её без такого reader увеличило бы SMP-контракт. ASID — translation tag, а не process identity, ownership или authority.

## Последствия

Каждый живой process root получает собственную non-global translation namespace на owner CPU. Terminal root локально инвалидируется до повторного использования lease и backing frames. Существующие общие kernel mappings не меняются; private user leaves помечены not-global. Ограниченный scheduler сохраняет текущий completion barrier, fixed affinity и отсутствие пользовательской мутации mappings. При неизвестном ASID encoding остаётся исходное поведение с full flush.

## Влияние на совместимость

Распределитель ASID применяется только к корням EL0-процессов. Корень ядра сохраняет ASID 0, а для неизвестной или неподдерживаемой ширины аппаратного ASID остаётся прежний контракт ASID 0 с полной инвалидацией. Изменение не затрагивает legacy-маршрутизацию, native authority, формат образа процесса и закрепление за CPU.

## Влияние на производительность

Benchmark issue #18 выполняет восемь counterbalanced QEMU-пар: по 32 process create/run/reclaim цикла на каждом CPU. В [issue18-asid-measurements.json](../../../../research/results/issue18-asid-measurements.json) сохраняются timer ticks, TTBR switches, full и ASID-scoped TLBI counts, reuse tags, build/run metadata и точная source inventory. Отчёт стандартного Cortex-A57 фиксирует реальные 16-битные ASID; [отчёт QEMU max](../../../../research/results/issue18-asid-measurements-16bit.json) фиксирует реальные 16-битные ASID на тех же исходниках. CI повторяет 16-битное парное сравнение и проверяет сообщённую аппаратную ширину. Реализованный 8-битный fallback не имеет отдельного отчёта эмулятора. Положительная парная разница означает преимущество tagged mode. Это только QEMU TCG observations; они не доказывают throughput или ускорение на silicon.

## Влияние на безопасность

Epoch в каждом root lease отклоняет stale software retirement. Allocator не разрешает повторное использование, пока owner CPU не завершил ASID invalidation и coordinator не получил scheduler quiescence. Для forced exhaustion test pool ограничивается четырьмя ненулевыми tags на CPU. Same-VA tests удерживают старые physical frames, чтобы пропущенная invalidation не прошла из-за совпавшего allocator address. `--asid-reuse-control` пропускает TLBI и обязан провалить тест. Эти проверки не доказывают physical weak-memory behavior или remote invalidation для не реализованного shared-root дизайна.

## Проверки

DEV/PROD matrix проверяет same-VA isolation, exhaustion на обоих CPU, повторное использование generation-safe tag, stale-translation rejection, scheduler detachment и физическую reclamation. Focused control без invalidation обязан провалить `asid_reuse_requires_invalidation` по наблюдаемому числу retirement TLBI каждого CPU, а не по compile-time feature flag. Аппаратное чтение TTBR проверяет tag при каждом tagged activation; и 8-битные, и 16-битные tags начинаются с бита 48. QEMU TCG сохраняет isolation при пропуске invalidation; negative control проверяет, что обязательная retirement operation действительно выполнена. `cargo xtask asid-bench` сохраняет парные tagged и ASID-zero/full-flush измерения. Также обязательны repository host checks, Clippy, formatting и git diff checks. QEMU не заменяет hardware review до заявлений о скорости на устройстве.

## Обратимость

Allocator и switch contract приватны для ограниченного kernel process runtime. До увеличения числа CPU, совместного использования roots, мутации process mappings или migration следует пересмотреть residency и remote shootdown. ASID-zero baseline остаётся доступным для сравнения и fallback.

[Английский оригинал](../../../../docs/architecture-decisions/0021-asid-lifecycle.md)
