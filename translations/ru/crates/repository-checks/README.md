# Проверки исследования

Эта программа командной строки на Rust проверяет исследовательские данные и документацию. Она не исполняет ядро и не устанавливает историческую истину. Рабочая область закрепляет компилятор основной системы в `rust-toolchain.toml`, а зависимости — в `Cargo.lock`. Python не требуется.

Выполнять команды из корня репозитория:

```text
cargo test --locked
cargo run --locked -p repository-checks -- check
cargo run --locked -p repository-checks -- validate
cargo run --locked -p repository-checks -- check-cost-l
cargo run --locked -p repository-checks -- check-docs
cargo run --locked -p repository-checks -- report --check
cargo run --locked -p repository-checks -- check-translations
```

`check` выполняет все проверки. `validate` применяет JSON Schema Draft 2020-12 с проверкой форматов, строгим отклонением повторяющихся ключей и согласованностью записей. `check-docs` проверяет локальные ссылки, сведения об источниках, обязательства законов и разделы архитектурных решений. Число законов не имеет нижней или верхней квоты; множество должно быть непустым, с уникальными идентификаторами и полными обязательствами. `report --check` отклоняет устаревшие английский или русский указатели.

После изменения случая проверить соответствующую двуязычную запись и её `source_sha256` в `research/sources/case-index-text.json`. Выполнить `report` без `--check`, чтобы заново сформировать оба указателя. Формирование не утверждает переводы. Проверить каждую изменённую пару документов и явно записать её:

```text
cargo run --locked -p repository-checks -- record-translation ru docs/research/case-index.md
```

Команда записывает только выбранную существующую пару; она не подтверждает смысл. Затем снова выполнить `check`. Включать все обновлённые файлы в один коммит. Не записывать контрольные суммы лишь ради подавления ошибки устаревшего перевода.

Первая сборка загружает зависимости. Последующие проверки могут работать с параметром Cargo `--offline`, когда зависимости уже сохранены. Во время проверки внешние источники не загружаются. В Windows вариант Rust для MSVC также требует инструментария сборки C++. Локальная установка репозитория может задавать `CARGO_HOME` и `RUSTUP_HOME` внутри игнорируемого каталога `.toolchains` и явно вызывать его исполняемый файл Cargo; менять общесистемный PATH не требуется. Эти инструменты основной системы отделены от будущего инструментария ядра ARM64.

`check` и `validate` также проверяют ограниченные COST-L allocation ledger и records. `check-cost-l` выполняет только эту проверку и принимает `--directory PATH` для rejection fixtures. [Контракт реестра](../../docs/architecture/compatibility-debt.md) описывает lifecycle, конечный support, неизвестные observations и retained artifact digests. Успех устанавливает offline declaration consistency, но не Linux driver support, native authority, historical truth или runtime quiescence.

[Английский оригинал](../../../../crates/repository-checks/README.md)
