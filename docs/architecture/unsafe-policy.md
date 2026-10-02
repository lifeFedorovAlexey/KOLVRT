# Unsafe policy

Safe Rust — default. Unsafe разрешён только для конкретного hardware/FFI/memory boundary,
который невозможно выразить существующими safe abstractions. «Быстрее» без измерения и
«удобнее» не основания. Safe code не доказывает отсутствие semantic races, deadlocks,
resource exhaustion или ошибок foreign memory.

Каждый unsafe block, unsafe fn/trait impl, inline assembly и raw-pointer operation имеет
минимальный scope и локальный `SAFETY:` comment со ссылкой на invariant ID. Будущий core
crate использует `#![forbid(unsafe_code)]`, кроме явно одобренных boundary crates; в них
`#![deny(unsafe_op_in_unsafe_fn)]`. Generated code и dependencies входят в аудит, а не
исключаются как «чужие». Unsafe в macro expansion также учитывается.

## Обязательная карточка invariant

| Поле | Требование |
|---|---|
| ID, owner, source locations | Стабильная identity и ответственный reviewer |
| Necessity | Почему safe implementation невозможна на этой границе |
| Preconditions | Alignment, initialization, bounds, provenance, ownership, aliasing |
| Temporal assumptions | Lifetime, interrupt/preemption context, CPU affinity, ordering |
| External agents | DMA, device, firmware, userspace mutation, FFI callbacks |
| Guarantees | Что получает safe caller; полный error/cancel/drop contract |
| Proof boundary | Какие assumptions не проверяются Rust/compiler и чем подтверждены |
| Tests | Negative cases, fault injection, model/litmus/hardware checks |
| Review | Независимый review boundary и повторный review при изменении assumptions |

Нельзя формировать ссылку на volatile/MMIO или mutable user memory как на обычную Rust
memory без доказательства reference validity. Volatile не является atomic и не заменяет
barrier. `Send`/`Sync` impl требует отдельных SMP и interrupt proofs. `MaybeUninit` нельзя
публиковать до инициализации всех observable bytes. Reused buffers заново задают flags.
Drop не освобождает DMA memory, пока device может писать. Panic/unwind policy ещё требует ADR;
никакая cleanup path не может полагаться на недоказанный unwind через interrupt/FFI.

Тестирование по границе: Miri применим только к поддерживаемому host-side коду; concurrency
model checking — к моделируемым interleavings; QEMU — к platform integration; real hardware
и Arm litmus — к ordering/errata. Ни один из них по отдельности не является proof всего ядра.
Инструменты Miri/model checking в Phase 0.1 не запускались и не устанавливались.

Основания: [Rust undefined behavior reference](https://doc.rust-lang.org/reference/behavior-considered-undefined.html),
[Dirty Pipe](../../research/pathology/KOL-PATH-0008.json), [DMA](../../research/pathology/KOL-PATH-0010.json),
[RCU](../../research/pathology/KOL-PATH-0022.json), [epoll lifetime](../../research/pathology/KOL-PATH-0030.json).
Конкретная версия Rust compiler/reference должна быть закреплена перед первым unsafe implementation.
