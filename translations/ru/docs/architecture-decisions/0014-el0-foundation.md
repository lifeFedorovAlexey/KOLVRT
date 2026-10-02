# ADR-0014 — Фундамент EL0 execution с фиксированной affinity

Status: **Принят для ограниченного фундамента**. Date: 2026-10-02.

## Context

После проверенного SMP пользователь разрешил address spaces, переходы EL1/EL0, stacks, context switching, timer scheduling, process isolation и локализацию faults на обоих CPU. IPC, handles, cancellation, service models и security domains явно отложены. [Контракт EL0](../kernel/el0.md) определяет фактический workload; полный native slice не завершён.

## Decision

Использовать независимые per-CPU queues с фиксированной process affinity и atomic executing owner на процесс. Сохранять полный architectural context и разделять между private roots только privileged kernel mappings. ASID zero и завершённый local TLBI при каждом switch исключают преждевременное ASID reuse. Удерживать все space guards, пока оба CPU не восстановят native root и не опубликуют completion. Global scheduler lock, remote address-space mutation и migration не добавляются.

## Alternatives

Только один CPU; global locked queue; migration/work stealing; ASID-tagged targeted invalidation; cooperative switching; lazy SIMD/FP saving.

## Why rejected

Первый вариант не проверяет требуемую SMP boundary. Shared queue добавляет IRQ lock ownership и progress obligations без потребности этого workload в cross-CPU admission. Migration и ASID reuse требуют participant/lifetime protocols за пределами fixed-affinity контракта. Только cooperative execution не ограничивает non-yielding process. Lazy FP требует отдельного trap/ownership handling. Full local invalidation имеет видимую стоимость; утверждения о самом быстром методе нет.

## Consequences

Восемь процессов работают за одну статическую boot session. Dynamic loading/creation, migration и общее process management не поддерживаются. Чистый round-robin selection отделён от architecture mechanisms. Trusted boot fixture выбирает images и budgets; её validation assertions не являются user authority service.

## Compatibility impact

Kernel dependency closure остаётся kernel и kernel-core. Routing/adapter groundwork сохранён и отключён. Foreign semantics, syscall compatibility и version-dependent task behavior не добавляются.

## Performance impact

Выбранный quantum — типизированный Duration 1 ms. Fairness зависит от timer delivery и ограниченных handlers; TCG observations не устанавливают рейтинг latency или throughput. Каждый switch сохраняет весь GPR/SIMD/FP/TLS state и очищает local translations. ASIDs, lazy state или targeted flushes требуют evidence до оптимизации.

## Security impact

ERET/exception vectors, TTBR/TLBI и timer enforcement требуют privileged instructions; EL0 process не может безопасно установить собственные trusted exception return или protection root. Это аргумент privileged necessity по [admission policy](../architecture/kernel-admission-policy.md). Allocation/image selection и round-robin policy — bootstrap choices, а не irreducible global-authority services. Их placement пересматривается при появлении IPC/supervision. Privileged identity aliases остаются частью TCB; DMA или security-domain гарантии не заявляются.

[Arm exception model](https://documentation-service.arm.com/static/67ac57fb091bfc3e0a9479cc), sections 5.1–5.2, описывает vector/stack selection и ERET restoration из SPSR/ELR. [Arm memory management](https://developer.arm.com/-/media/Arm%20Developer%20Community/PDF/Learn%20the%20Architecture/LearnTheArchitecture-MemoryManagement-101811_0100_00_en.pdf), sections 5.2 и 8, обосновывает ASID identity и TLB maintenance. Эти архитектурные источники подтверждают механизмы; фактические correctness evidence даёт workload выбранного ядра.

## Testing

DEV/PROD требуют 53 real tests, настоящее non-test boot execution и одиннадцать negative host controls. EL0 tests проверяют same-VA private data tags, восстановление registers/SIMD/FP/TLS и stack, отказ kernel/foreign memory, RO/NX/guard faults, запрет privileged instruction, выживание peers и возврат frames. Root/context/forgotten-guard controls должны завершаться ошибкой. [Unsafe register](../kernel/unsafe.md) определяет local proof obligations; hardware weak-memory behavior остаётся непроверенным.

## Reversibility

Будущие scheduling domains или EL0 policy services могут заменить static selector только при явных ownership, admission, lifetime, timeout и remote quiescence contracts. Migration/ASID reuse требуют нового проверенного решения. Этап не создаёт stable userspace ABI.

[Английский оригинал](../../../../docs/architecture-decisions/0014-el0-foundation.md)
