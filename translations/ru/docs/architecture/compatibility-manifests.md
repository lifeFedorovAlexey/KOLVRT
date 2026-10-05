# Объявления compatibility-модулей

Document status: CURRENT
Evidence scope: ограниченная host/CI-проверка объявлений и синтетические тесты; не loader, поддержка Linux-драйверов или допуск установленных модулей.
Current reference: [Проверка](../../../../crates/repository-checks/src/compat_modules.rs)

<a name="kolvrt-compatibility-manifests"></a>

## Архитектура и полномочия

Проверка следует LAW-008/009/013/018/035/040, ADR-0007 и ADR-0013. Идентификаторы модуля, долга и исключения, версия Cargo-пакета и runtime-полномочия остаются разными понятиями. Ранние COST-L queries не задают архитектуру этого контракта: они читают проверенные исследовательские объявления, а новый gate отдельно проверяет объявленное включение модулей в поставку. Manifest не выдаёт права, не выбирает маршрут, не допускает запрос и не выгружает код. Для native-авторизации и runtime-quiescence нужны собственные механизмы и доказательства.

[Закрытая схема v1](../../../../schemas/compatibility-modules.schema.json) описывает [перечень объявлений](../../../../policy/compatibility-modules.json). Это ограниченная host-проекция модулей, а не второй реестр долга. Каждый модуль фиксирует ID поведения, semantic version, scope, Cargo-пакет/версию/feature, классификацию, состояние поставки, владельца, конечный срок поддержки, хеши исходников, artifact и именованных потребителей. Отдельный счётчик потребителей не хранится: состав выводится из массивов. Semantic versions ограничены числовыми тройками по существующему формату COST-L; версия пакета и хеш исполняемых байтов независимы.

## Поставка и двусторонние связи

`DECLARED` означает объявленное текущее включение, в том числе через optional feature; это не утверждение о runtime-использовании. `RETIRED` сохраняет tombstone с artifact, исходниками, именованными историческими потребителями и связями с долгом. Retired ID нельзя оставлять в текущем Cargo marker. Удаление marker не доказывает отсутствие живых ссылок или безопасность выгрузки. Архивная установка и продление требуют отдельного решения о поддержке и native-авторизации; этот формат их не предоставляет.

Каждое объявление `PRODUCTION` требует хотя бы один применимый существующий COST-L ID и сохранённые байты artifact с совпадающим SHA-256. Каждый долг должен содержать обратную связь с теми же ID/version/scope/artifact. Связи потребителей проверяются в обе стороны: совпадают ID, kind, direct/transitive, scope, deadline и точная ссылка на модуль. Допустимы общий долг и несколько модулей. Срок модуля не может превышать срок поддерживающего долга; сроки именованных потребителей — срок модуля. Владельцы долга и модуля могут различаться: само имя не доказывает полномочия или фактическое владение. Текущая production-поставка требует долг ACTIVE или DEPRECATED; retired tombstone — RETIRING или RETIRED. Исследовательский кандидат не становится production-поддержкой автоматически.

Потребители могут включать явно названные offline recovery и архивные пакеты; отсутствие наблюдаемых вызовов не отменяет объявленные обязательства. Перечень не устанавливает часы, runtime-счётчики или полноту наблюдения потребителей. Gate сравнивает объявленные календарные сроки; существующая проверка исключений контролирует наступившие software review/support deadlines. Здесь не реализованы прекращение допуска при истечении срока или доказательство поддержки приложения сегодня.

Объявление `SYNTHETIC` требует объяснения, пустых ссылок на production-долг и отсутствия production-artifact. Текущие данные содержат только три существующих fixture-контракта `window-compat`: inclusive, counted и empty-first. Их semantic ID/version уже используются routing. Срок поддержки — политика fixtures, не обещание ABI. Empty-first ссылается на EXC-0001: проверяются его граница исходников и software-deadline. COST-L-0001…0003 остаются исследовательскими кандидатами без выдуманных модулей, потребителей, Linux-истории или измеренных затрат.

## Cargo и граница исходников

Каждый объявленный пакет перечисляет ID/version в `[package.metadata.kolvrt].compatibility_modules`. Для каждого marker нужен один совпадающий текущий manifest, а для manifest — marker, точная версия пакета и существующая feature. В источники входят Cargo manifest и входные файлы всех Cargo targets с SHA-256 по нормализованным LF. Полнота вспомогательных исходников и build inputs требует review: входные файлы не доказывают покрытие всех транзитивных байтов. Хеш сохранённого artifact идентифицирует байты, а не выполнение, честность производителя или соответствие исходникам.

Native roots закреплены в коде: `kolvrt-kernel` и `kernel-core`; правкой перечня их нельзя убрать. Locked offline `cargo metadata --no-deps` предоставляет реальные разобранные объявления. Консервативный обход включает normal/build/dev, optional, target-specific и renamed edges, даже выключенные. Достижение compatibility-пакета или external/unresolved-зависимости завершает проверку ошибкой. Optional compatibility-зависимость в native root запрещена: выключенный код не становится допустимой архитектурой ядра. Это инвариант объявлений для всех features, не доказательство linker/runtime-изоляции. Отдельные native-only routing tests остаются обязательными; gate не объявляет любой host/EL0-пакет native root.

Cargo metadata не распознаёт смысл неотмеченного adapter. Review должен выявить compatibility-пакеты и поставить markers; совместное отсутствие объявления и marker не покрывается структурной проверкой. Metadata создаётся из проверяемого checkout, а не принимается от недоверенного caller как runtime-policy. Будущие форматы нужно заново выводить из принятых границ полномочий и lifetime, а не сохранять раннее представление ради удобства.

## Ограничения и проверки

Перечень ограничен 64 KiB, 128 модулями, 32 ссылками на долги/исключения на модуль, 16 исходными файлами и 128 потребителями на модуль. Схема отклоняет неизвестные поля, неверные даты/версии, пустые ID и чрезмерные значения. Строгий JSON parser отклоняет повторяющиеся ключи. Повтор module/consumer ID отклоняется даже при разных остальных полях. Файловые ссылки должны оставаться внутри checkout. Чтение artifact ограничено 64 MiB с дополнительной проверкой роста; Cargo metadata — 16 MiB после завершения доверенного дочернего процесса. Это host input limits, не runtime-бюджеты и не защита от враждебного Cargo executable/build configuration.

```text
cargo run --locked -p repository-checks -- check-compatibility
cargo test --locked -p repository-checks --test compat_modules
```

Обычные команды `check` и `validate` выполняют gate, поэтому он входит в существующий CI репозитория. Пользовательские каталоги COST-L queries остаются самостоятельными синтетическими/исследовательскими входами; manifests не подмешиваются в их вывод. Production-интеграция manifests с queries остаётся отдельным шагом acceptance #48.

Тринадцать тестов проверяют текущий синтетический перечень и настоящий CLI; изолированные production-shaped и retirement fixtures; many-to-many связи; отсутствие обоснования, dangling relations, несовпадение artifact/source/version/scope; offline consumers/deadlines; границы схемы; срок EXC; Cargo markers/features; запрет native optional/renamed/target/build/dev edges. Временный настоящий Cargo workspace проверяет разбор выключенных renamed target declarations. Artifacts production-shaped fixtures намеренно содержат неисполняемые байты: успех не подтверждает поддерживаемый драйвер или исполняемый модуль. [Доказательство на точных исходниках](../../../../research/results/issue47-compatibility-manifests.json) фиксирует этот host scope.

Оставшиеся gates #47: reviewed production-данные module/debt/consumer при появлении реальной поддержки, полнота source/features и owner/semantic review. Runtime-retirement, distribution/archival policy и effective-authority enforcement здесь не реализованы. Исследование/история #46 и production-полнота queries #48 остаются открытыми. Этот ограниченный checker не закрывает задачи и не устанавливает production readiness.

[English document](../../../../docs/architecture/compatibility-manifests.md)

<!-- knowledge -->

```json
{
  "units": [
    {
      "summary": "Ограниченные объявления compatibility-модулей и двусторонние проверки COST-L/Cargo без runtime-полномочий.",
      "feature": {
        "next_gate": "Review real production declarations and source/feature completeness; integrate full #48 queries without promoting synthetic evidence.",
        "adrs": ["adr.0007", "adr.0013", "adr.0015"],
        "implementation_scope": "Closed module inventory, reciprocal debt/consumer/version/scope checks, source/artifact/EXC/support identities and Cargo-parsed native declaration closure, with existing synthetic window contracts only.",
        "readiness": "NOT_READY",
        "issues": [47, 46, 48],
        "transitions": [
          {
            "reason": "First bounded manifest gate with synthetic current inventory and exact-source host controls; production/runtime gates remain open.",
            "acceptance": [
              "research/results/issue47-compatibility-manifests.json"
            ],
            "from": "UNRECORDED",
            "to": "BOUNDED_IMPLEMENTED"
          }
        ],
        "verification": [
          {
            "scope": "Host consistency and declaration boundary only; production-shaped artifacts are non-executable test bytes, not supported modules.",
            "reason": "Thirteen host regressions, actual CLI and real Cargo declaration fixture passed on these exact source bytes.",
            "state": "VERIFIED",
            "receipt_sha256": "a2cdd955c319fc90449689133da0eb28c1d1a9da993513c3aab0822a244c26be",
            "receipt": "research/results/issue47-compatibility-manifests.json",
            "environment": "host-process"
          },
          {
            "reason": "Host declaration checking does not verify kernel, driver or physical execution.",
            "state": "NOT_APPLICABLE",
            "environment": "physical-arm64"
          }
        ],
        "roadmap_gate": "COST-L module/CI declarations #47",
        "sources": [
          "Cargo.lock",
          "Cargo.toml",
          "crates/host-process-metrics/Cargo.toml",
          "crates/kernel-core/Cargo.toml",
          "crates/kernel/Cargo.toml",
          "crates/migration-advisor/Cargo.toml",
          "crates/migration-workbench/Cargo.toml",
          "crates/native-protocol-model/Cargo.toml",
          "crates/native-state-models/Cargo.toml",
          "crates/repository-checks/Cargo.toml",
          "crates/repository-checks/src/compat_modules.rs",
          "crates/repository-checks/src/cost_l.rs",
          "crates/repository-checks/src/lib.rs",
          "crates/repository-checks/src/main.rs",
          "crates/repository-checks/tests/compat_modules.rs",
          "crates/routing-demo/Cargo.toml",
          "crates/routing/Cargo.toml",
          "crates/window-compat/Cargo.toml",
          "crates/window-compat/src/lib.rs",
          "crates/window-compat/tests/protocols.rs",
          "crates/xtask/Cargo.toml",
          "policy/compatibility-modules.json",
          "policy/exceptions.json",
          "rust-toolchain.toml",
          "schemas/compatibility-modules.schema.json"
        ],
        "acceptance": ["research/results/issue47-compatibility-manifests.json"],
        "implementation": "BOUNDED_IMPLEMENTED",
        "limitations": [
          "Unmarked adapters and auxiliary source completeness require review; no production module/driver data, loader, live consumers, admission or runtime retirement proof. Current Linux debts remain research-only."
        ]
      },
      "depends_on": [
        "law.008",
        "law.009",
        "law.013",
        "law.018",
        "law.035",
        "law.040",
        "adr.0007",
        "adr.0013",
        "adr.0015"
      ],
      "tags": [
        "compatibility",
        "debt",
        "module",
        "manifest",
        "cargo",
        "retirement"
      ],
      "anchor": "kolvrt-compatibility-manifests",
      "id": "kolvrt.compatibility.manifests",
      "kind": "feature"
    }
  ],
  "kind": "subsystem-contract",
  "id": "doc.kolvrt.architecture.compatibility-manifests",
  "schema_version": 1,
  "summary": "Ограниченные объявления compatibility-модулей и двусторонние проверки COST-L/Cargo без runtime-полномочий."
}
```
