# ADR-0023 — One-process domains и scoped Event grants

Статус: **Принято для bounded Phase 3.4**. Дата: 2026-10-04.

Document status: CURRENT
Evidence scope: generation-aware one-process domains, fixed-affinity notification services и retained terminal outcomes на двух QEMU CPU.
Current reference: [Domain contract](../../../../docs/kernel/domains.md)

## Контекст

Private address spaces и reference identity не разрешают effects. Issue #24 требует explicit grants и revoke/admission serialization; issue #25 требует domain lifetime и budgets, включая teardown во время accepted work. LAW-005 требует privileged enforcement necessity, а не только performance. Существующие handles или ранние IPC fixtures не определяют последующую архитектуру.

Это расширяет [ADR-0022](0022-native-event-grants-and-revocation.md): minimum defaults Event SEND и Completion NONE, запрет Completion REVOKE и synchronous revoke API сохранены. SharedEvent сериализует synchronous и retained deferred admission на одном atomic gate; pending latch — отдельное storage, а не второй authority ledger. Уже принятый deferred effect публикуется через private completion method даже после revoke. Прямой synchronous signal через service-bound grant запрещён, чтобы исключить обход consumer/service accounting.

## Решение

В этом subset один процесс принадлежит одному domain. Trusted bootstrap передаёт explicit grants и limits; process runtime применяет identity, memory ownership, handle/request/queue charges и closing. Product policy, initial grant/limit selection и orchestration принадлежат trusted EL0 policy/supervisor layer; текущие static launchers — bootstrap fixtures, а не policy interpreter. Domain metadata использует fixed atomic pool из 32 cells с nonwrapping generation и retained references.

Event grant связывает один immutable target и точный service ProcessId. SEND, TRANSFER и REVOKE — отдельные rights. Delegation ослабляет rights и списывает receiver budget. Target aliases разделяют единый revoke/admission gate. Admission до revoke удерживает work; последующий admission отклоняется. Close не равен revoke. Universal object hierarchy, magic root handle и generic invoke не добавляются.

Конкретный enforcement reader — same-CPU EL0 notification service с одним queue cell. Accepted work удерживает consumer request и service queue/request charges. Service может завершить только admitted consumer target независимо от private grants. Process/service generation и request sequence связывают каждый terminal receipt. Sender teardown прекращает new admissions; accepted work может пережить reclaim private pages sender. Service failure публикует cancellation before effect и освобождает ownership один раз.

Experimental notification encoding не является архитектурным авторитетом для general IPC. До реализации #26/#27 queue topology, waits, cancellation, outcomes и policy installation заново выводятся из current invariants; этот pilot должен быть переработан или удалён, если ограничивает такой дизайн.

## Альтернативы

Использовать только private roots и integer labels; авторизовать каждую operation через EL0 policy interpreter; выделять domain lifetime через Arc; сохранять ранний queue ABI потому, что его tests уже проходят; создать universal capability/object layer.

## Почему отклонены

Labels и roots не применяют resource authority или quotas. Untrusted callers не могут проверять собственные grants, mapping isolation и lifetime; эти narrow checks требуют EL1, а выбор policy не требует. Final Arc drop может взять ordinary heap lock внутри masked scheduler ownership, нарушая accepted lock contract; fixed atomic pool исключает этот путь и обрабатывает exhaustion. Ранее потраченные усилия не обосновывают ABI или topology. Named Event reader не обосновывает universal abstraction.

## Последствия

Domain fault/exhaustion ограничивается caller. Exact identity и closing checks предшествуют authority admission. Physical memory освобождается только после scheduler detachment; copied accepted requests не удерживают user pointers. Pool reuse ждёт final retained release. Unknown rights, absent grants, spoofed scope, stale service binding и quota failure не могут незаметно перейти к другому effect.

## Влияние на совместимость

Reference lookup/close/attenuated transfer остаются bounded. Scoped issued Event grants добавляют REVOKE=4; minimum reference defaults остаются Event SEND / Completion NONE. Новый notification contract имеет статус EXPERIMENTAL без ABI-FREEZE. General IPC, multi-process domains, migration и immediate revoke/drain не поддерживаются.

## Влияние на производительность

Admission/terminal paths используют bounded owned state и atomics без heap allocation/deallocation или ordinary locks. Existing exact-source copy/handle benchmarks и kernel footprint сохраняются вместе с matrix. Speedup и silicon throughput не заявляются; последующие изменения contention либо topology требуют новых измерений.

## Влияние на безопасность

Kernel enforcement необходим для trusted current identity, page ownership, linear reference retention, explicit grants и resource charges. Альтернативный EL0 layer выбирает policy и может запрашивать installed limits/grants через будущий trusted installation contract; native enforcement он не обходит. AI/routing/package decisions не входят в mechanism. Domain и request generations запрещают stale rebinding; service-private authority не заменяет consumer scope.

## Проверки

Actual EL0 tests проверяют оба CPU, отдельные zero budgets, receiver handle exhaustion, unknown rights/missing grants, spoofed scope/foreign memory, attenuation, cross-CPU revoke/admission, fault containment, restart и request outcomes. Step execution reclaim sender до завершения retained request и наблюдает service-fault cancellation из surviving caller. Пять mutations отключают реальные identity, budget, closing, revoke либо scope checks; DEV/PROD обязаны показать failed machine records и nonzero host status. Host checks, architecture Clippy, native-only и routing regressions дополняют QEMU execution; physical ARM64 evidence остаётся отдельным.

## Обратимость

Public/stable ABI не заморожен. Более поздний IPC/supervisor reader обязан заново проверить implementation против accepted laws и decisions, включая fixed-affinity и single-process assumptions. Ранняя реализация, затраченные усилия и совместимость с текущими tests недостаточны для сохранения ограничивающего дизайна.

[Английский оригинал](../../../../docs/architecture-decisions/0023-domains-and-scoped-grants.md)
