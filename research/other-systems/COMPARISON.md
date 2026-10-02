# Другие системы: идеи, ограничения и применимость

Дата: 2026-10-02. Это документальное сравнение; ни одна из систем локально не собиралась
и не benchmarked. «Результат» ниже означает результат, описанный primary source. Отсутствие
найденного failure report не означает отсутствие провалов. Metadata: [ledger](../sources/other-sources.json).

## Fuchsia / Zircon / Starnix

Starnix реализует Linux ABI в Fuchsia userspace program и переводит syscalls к Fuchsia
subsystems; native Fuchsia продолжает использовать собственные interfaces. Это реальный
архитектурный пример отделения personality от native kernel. [Starnix overview](https://fuchsia.dev/fuchsia-src/concepts/starnix).

RFC рассматривает исполнение немодифицированных Linux programs; объём эмуляции включает
процессную и файловую семантику, а не только номера syscalls. [RFC-0082](https://fuchsia.dev/fuchsia-src/contribute/governance/rfcs/0082_starnix).
Полезная идея для KOLVRT: отдельный semantic owner personality. Ограничение: boundary не
устраняет state coupling, а ошибки personality всё ещё воздействуют на её consumers.
Из источников нельзя вывести покрытие всех Linux programs или преимущество скорости.
Конкретный failure postmortem и стоимость syscall transport остаются вопросами Phase 0.2.
Не копировать Zircon object model без анализа нужных KOLVRT primitives.

## Redox OS

Официальная книга описывает Rust microkernel, userspace drivers/services и scheme IPC.
Она также прямо описывает пересмотр system-service interface и эволюцию security design;
это пример того, что Rust не завершает архитектурное проектирование. [Redox goals, official mirror](https://github.com/redox-os/book/blob/master/src/our-goals.md).

Полезная идея: resource protocols между kernel и сервисами. Результат, доступный этому
исследованию: документированная работающая структура проекта, а не проверенная нами
совместимость конкретного hardware. Ограничение: текущая схема и планы не равны immutable
ABI. Изменение планов не следует называть security failure без postmortem. Для KOLVRT
применим принцип явного IPC contract; namespace/file-like model не принимается автоматически.
Нужно исследовать реальные scheme migration regressions и расходы recovery.

## seL4

seL4 публикует конкретные verified configurations и наборы доказанных свойств для них.
Объём assurance зависит от configuration/architecture; нельзя распространить его по
названию проекта на произвольные drivers и applications. [Verified configurations](https://docs.sel4.systems/projects/sel4/verified-configurations.html).

Полезная идея: proof boundary и explicit capability delegation. Реальный результат —
опубликованная configuration-specific verification, не подтверждённый KOLVRT proof.
Ограничение для нас: KOLVRT Rust/toolchain/driver assumptions потребуют собственных proofs.
Обобщённый «провал seL4» источником не установлен. Применить discipline invariant ledger,
не обещать formal verification всего ядра. Список trusted hardware/compiler assumptions
и стоимость поддержания proofs нужно изучить отдельно до выбора verification plan.

## Theseus OS

Проект исследует Rust intralingual design и перенос управления ресурсами в type system;
заменяемость компонентов во время исполнения — заявленная цель. [Theseus book](https://www.theseus-os.com/Theseus/book/),
[safe-language principles](https://www.theseus-os.com/Theseus/book/design/idea.html).

Полезная идея: ownership и state structure как часть interface, а не только calling
convention. Ограничение: typed code replacement не даёт автоматически semantic state
conversion или isolation от unsafe code. Результат в нашем scope — опубликованный
исследовательский design и implementation project; чисел performance здесь не приводим.
Не установлено доказательство произвольного безопасного live swap всех subsystems.
Для KOLVRT: quiescence и state migration должны быть явны, даже если Rust типы совпадают.
Нужен разбор конкретного failure/recovery experiment из papers.

## FreeBSD Linuxulator

Историческая architecture article описывает per-executable ABI dispatch через sysentvec.
Она содержит устаревшие i386/2.6-era детали, поэтому не является описанием всех текущих
ports. [FreeBSD Linux emulation article](https://docs.freebsd.org/en/articles/linux-emulation/).

Конкретный инженерный случай: Q2 2023 report сообщает, что старый kern_alternate_path
неправильно обрабатывал абсолютные symlink targets; alternate ABI root перенесли в
name lookup facility. Это показывает конфликт между изоляцией pathname translation и
полнотой общей path semantics. [Project report](https://www.freebsd.org/status/report-2023-04-2023-06/linuxulator/).

Идея: executable personality binding. Ограничение: compatibility может требовать
нового native primitive. KOLVRT должен обосновывать такой primitive универсальным
контрактом (например, explicit lookup root), а не флагом «если Linux». Не копировать
kernel-resident placement без TCB и latency анализа. Текущий compat coverage требует
отдельного pinned conformance run.

## WSL1

Microsoft описывает Pico processes и kernel-mode providers, переводящие Linux syscalls
в NT operations; при отсутствии прямого mapping provider реализует дополнительную
семантику. Fork — пример более сложной операции. Источник относится к исходной WSL
архитектуре 2016 года, не WSL2. [WSL overview](https://learn.microsoft.com/en-us/archive/blogs/wsl/windows-subsystem-for-linux-overview).

Полезная идея: subsystem identity при dispatch, независимость native host API.
Результат: запуск немодифицированных ELF64 Linux binaries описан разработчиками.
Ограничение: translation не всегда one-to-one, а kernel-mode provider остаётся privileged.
Нельзя объявить переход к другой архитектуре доказательством полного провала WSL1 по
одному overview. Для KOLVRT применима честная декларация невыразимых operations;
конкретные filesystem/signal fidelity gaps и benchmark evidence — отдельный research item.

## Asterinas

Проект документирует Rust kernel с Linux ABI и OSTD как основу safe OS development;
книга отмечает раннюю стадию. [Asterinas book](https://asterinas.github.io/book/).

Полезная идея: малый reviewed foundation для safe clients. Отличие от KOLVRT:
Linux ABI у Asterinas — центральная цель, а у KOLVRT — optional personality.
Результат в scope исследования — доступный проект и описанные interfaces; заявленную
efficiency/security нельзя превращать в независимо подтверждённые числа. Провалы
конкретной boundary abstraction этим источником не установлены. Следует проверить
OSTD soundness assumptions и реальные bug fixes, не импортируя Linux-oriented core design.

## Выводы KOLVRT

Принять: отдельные semantic owners, capability-aware boundaries, typed lifetimes,
configuration-specific claims и evidence для performance. Оставить открытыми: placement,
native process primitives, state migration format и объём formal verification.
[ADR-0008](../../docs/adr/0008-placement.md) не выдаёт выбор monolithic/microkernel за
завершённый только потому, что один проект выглядит ближе по философии.
