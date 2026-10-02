# ADR-0016 — Обязанности scheduler и enforced per-CPU ownership

Status: **Accepted for bounded Phase 3.0**. Date: 2026-10-02.

## Контекст

[Issue #16](https://github.com/lifeFedorovAlexey/KOLVRT/issues/16) и [issue #17](https://github.com/lifeFedorovAlexey/KOLVRT/issues/17) требуют decomposition и enforced ownership перед dynamic processes. Review использует актуальный main и [ADR-0015](0015-el0-versioned-routing.md), а не исторический commit issue. Контракт UnsafeCell только в комментарии, смешанные fixture fields и raw setup/inspection не могут безопасно служить расширяемой lifecycle boundary. LAW-013, LAW-018, LAW-035, LAW-041 и LAW-043 сохраняются; новые law или privilege exception не добавляются.

## Решение

Разделить чистый selection и безопасный atomic ownership, kernel runtime state, AArch64 frame/trap boundary и trusted bootstrap verification. Инкапсулировать storage за проверками наблюдаемых CPU/IRQ/phase/generation, nonblocking exclusive permits и higher-ranked closures. Сохранить immutable task definitions приватными, копировать completed results и читать reports ограниченными chunks. Постоянные per-CPU slots переживают workloads; generation заменяется только через проверенную quiescent setup. Сохранить fixed affinity и существующий per-task running-owner CAS. Обеспечить оба направления ordinary-lock/scheduler access и запретить перенос guard. [Контракт](../kernel/scheduler.md) задаёт фактические границы.

## Альтернативы

Сохранить UnsafeCell access с контрактом только в комментарии; добавить global scheduler lock; выдавать mutable guard references, переживающие user entry; сразу использовать универсальный lifecycle object; реализовать migration/work stealing; немедленно перенести всю scheduling policy в будущий EL0 service.

## Почему отклонены

Комментарии не отклоняют stale/foreign access. Global lock скрывает ownership и вводит IRQ progress dependencies. Ссылки, удерживаемые при exception re-entry, нарушают aliasing. Универсальный object или migration требуют посторонних lifecycle proofs и меняют разрешённую нагрузку. Размещение service policy требует будущего native process/IPC foundation; этот этап не допускает новый policy service в EL1.

## Последствия

Нагрузка восьми процессов сохраняет требуемые faults, survivor progress, stacks, registers и reclamation. Tick publication переносится в EL0 fixture через существующий own-slices call; marker/fault verification выполняется после completion. Старые timing samples не относятся к эквивалентной instruction workload. Static queue capacity отделена от fixture count. Второй native session проверяет reset/reuse без reboot. Dynamic per-process lifecycle, vacant slots и независимые process generations остаются работой #20; queue generations не являются ABI user handles.

## Влияние на совместимость

Native dependency closure остаётся kernel/kernel-core. Routing и translation остаются в optional EL0 image с прежними native observation semantics и отдельным regression evidence. Scheduler не импортирует routing, adapter, profile, capability или IPC policy.

## Влияние на производительность

Каждый краткий storage access добавляет проверяемые atomics и наблюдения CPU/IRQ; это correctness enforcement, а не утверждение об ускорении. Contention wait или global queue lock не добавляются. Context preservation, full local TLBI и типизированный quantum не меняются. Results копируются без выделения полного набора evidence buffers; ограниченные report chunks избегают исчерпания фиксированного heap 64 KiB. Host/QEMU evidence не является сравнением самого быстрого метода.

## Влияние на безопасность

По [admission policy](../architecture/kernel-admission-policy.md) это сужает существующее privileged enforcement, а не добавляет EL1 service. Protected current-task attribution, exception return, timer preemption и TTBR/TLBI требуют trusted privileged mechanisms; EL0 service не может установить trusted return/root или подтвердить другой kernel slot. Allocation, images, budgets и fixture expectations остаются bootstrap choices, а не разрешённой постоянной service policy. Native issuer остаётся trusted bootstrap; scope — текущая fixed-affinity task, без delegation или stable grant API. Immutable roots переживают оба CPU completion и local invalidation; timeout не разрешает reuse. Нарушение privileged ownership фатально. Compiler/firmware/QEMU trust и непроверенный silicon остаются явными. [Threat model](../security/threat-model.md) фиксирует одинаковую границу DEV/PROD.

## Тестирование

Host tests проверяют wrong role, unmasked access, re-entry/concurrent exclusion, stale generation, live reset/read, duplicate start, reader/reset exclusion и generation exhaustion. Compile-time context assertions и source guards сохраняют разделение. DEV/PROD требуют прежние 53 tests и generation reuse, non-test boots, eleven original failure controls и fifteen ownership controls в обоих profiles. Должны пройти реальная routing regression и native-only dependency/build checks; [результаты](../../../../research/results/kernel-phase3.json) привязывают claims к точным sources. Unsafe inventory остаётся средством аудита, а не доказательством.

## Обратимость

Следующий #20 расширит owned boundary per-process identities, vacant/admitted/terminal transitions, fallible rollback, persistent admission и acknowledged reclamation. Он должен сохранять permit lifetimes и доказывать новых callers/phases, а не экспортировать storage. CPU0 — текущий ограниченный allocator/coordinator, а не постоянное ограничение будущей admission architecture. Migration, IRQ nesting, ASID reuse и новые privileged responsibilities требуют отдельных решений. Завершение milestone не начинает #20, user-copy или IPC.

[English original](../../../../docs/architecture-decisions/0016-scheduler-ownership.md)
