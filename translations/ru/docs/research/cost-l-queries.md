# Офлайн-запросы COST-L

Document status: CURRENT
Evidence scope: ограниченные host-запросы к двусторонним объявлениям реестра и модулей; production-данные, полнота и принятие человеком остаются открытыми.
Current reference: [Контракт реестра](../architecture/compatibility-debt.md)

<a name="kolvrt-cost-l-offline-queries"></a>

## Контракт запросов

Архитектура повторно выведена из действующих правил identity, authority, support и evidence до расширения реестра. Используются существующий ограниченный [валидатор](../../../../crates/repository-checks/src/cost_l.rs) и закрытые схемы. [Реализация запросов](../../../../crates/repository-checks/src/cost_l_queries.rs) — проекция только для чтения: она не вводит второй реестр и не выдаёт процессу grants, доступ к payload или authority для retirement. Зависимости ядра, политика scheduler и реализация Linux driver не добавляются.

```text
cargo run --locked -p repository-checks -- cost-l show COST-L-0001
cargo run --locked -p repository-checks -- cost-l consumers COST-L-0001 --json
cargo run --locked -p repository-checks -- cost-l deps CONSUMER --json
cargo run --locked -p repository-checks -- cost-l list --directory PATH --manifests MANIFEST_PATH --json
cargo run --locked -p repository-checks -- cost-l list --status CANDIDATE --json
cargo run --locked -p repository-checks -- cost-l top --sort reach --limit 20
cargo run --locked -p repository-checks -- cost-l top --security reclaim --maintenance context --migration native
cargo run --locked -p repository-checks -- cost-l top --sort cpu --measurement-scope "exact workload" --denominator "exact operation" --json
```

Каждый запрос проверяет весь выбранный реестр до вывода в stdout. `--directory PATH` выбирает другой inventory declarations с теми же схемами репозитория и границей evidence references. `show` возвращает scope, modules, consumers, cost dimensions, native decision и support/removal statements выбранного debt. `consumers` возвращает объявленные direct/transitive связи debt; `deps` — те же связи в обратном направлении для точного consumer ID. Module ID/version/scope, offline/recovery/archival kind и support deadline сохраняются для каждой связи. TRANSITIVE — авторская declaration: формат не хранит путь, CLI не вычисляет package dependency closure. Related debts не являются software dependency edges.

У трёх текущих candidates ноль **объявленных** consumers и нет module declarations. Global reach и runtime costs остаются UNKNOWN; это не устанавливает observed driver count, NATIVE classification или support. Software compatibility declarations не позволяют вывести NATIVE в их stated scope; hardware classification остаётся отдельным полем. Стандартный перечень репозитория теперь подключает [объявления модулей](../architecture/compatibility-manifests.md) через существующий валидатор до любого вывода в stdout. Связи долга, модуля, потребителя, версии, области, артефакта и срока поддержки должны совпадать в обе стороны; проверки Cargo, исходников и native-границы сохраняются. Каждая выбранная строка содержит отсортированные `module_declarations`; строки потребителей включают только их точные ссылки на модули. SYNTHETIC-контракты без ссылок на долг учитываются в перечне, но не приписываются исследовательскому кандидату. Runtime inspection не поддерживается. Production-данные, полнота исходников и принятие человеком в #47/#48 остаются открытыми.

Явный `--directory PATH` сохраняет независимость пользовательского реестра; `--manifests PATH` явно подключает ограниченный manifest к выбранному реестру. Используется тот же двусторонний валидатор; Cargo metadata и пути исходников/артефактов разрешаются относительно корня репозитория. Неверный manifest отклоняется даже при ошибке вне выбранной страницы. Отсутствующий manifest не вызывает молчаливый откат к режиму без проверки. Архитектура повторно выведена из LAW-008/009/013/018/035/036/040 и ADR-0007/0013/0015: это офлайн-объявления, а не доказательства исполнения, полномочий, действующей поддержки или безопасного освобождения.

## Независимые измерения и пределы

`list` по умолчанию сортирует по ID; `top` — по убыванию declared reach. При равенстве используются stable IDs и содержимое связи. `--status`/`--category` — точные фильтры; `--security`, `--maintenance` и `--migration` независимо ищут в соответствующих authored statements, не придумывая severity или difficulty scores. Пустая фильтрованная выборка не доказывает global absence.

Metric sorting принимает cpu, latency, memory, copies, allocations, context_switches и throughput. Требуются точные observation scope и denominator; несовместимые units отклоняются. Соответствующие MEASURED значения идут перед UNKNOWN/out-of-scope; исходные provenance/coverage/reason каждой метрики сохраняются. Измеренный ноль остаётся нулём. Убывающий throughput — сортировка rate, а не debt-cost ranking. Сортировка описательная: она не доказывает сопоставимость машин, равную работу, causal attribution или превосходство. Combined security/debt score отсутствует.

По умолчанию страница содержит до 20 строк; допустимо 1–64. `--offset` и `pagination.next_offset` явно показывают оставшиеся строки. Итоговый stdout, включая human output, ограничен 1 MiB; превышение требует меньшей страницы и возвращает nonzero без partial stdout. Input limits сохраняют существующий контракт 4096 records/64 KiB на record. Malformed/reserved/unknown IDs, неизвестные consumers, неподдержанные commands/options, повторные options, неверные pages, несовместимые metric dimensions и противоречивые records возвращают nonzero. Проекция не содержит live state, usable grants или request payloads.

`--json` возвращает machine schema version 2 с command/selector, inventory scope/coverage/gaps, независимыми ranking/filter selection, pagination и rows. Human output включает text states и UNKNOWN без обязательного цвета. Версия 2 добавляет `module_declarations` каждой строки, число проверенных manifest-модулей (null, если manifest не выбран) и явные состояния интеграции `VALIDATED_RECIPROCAL_DECLARATIONS` / `NOT_SELECTED_CUSTOM_REGISTRY`. Человекочитаемый вывод показывает классификацию, состояние поставки, пакет/версию, область и срок поддержки рядом с каждой связью. Host CLI output независим от будущего runtime wire ABI.

## Доказательства и следующий этап

[End-to-end tests](../../../../crates/repository-checks/tests/cost_l_queries.rs) исполняют настоящий binary с текущими research records и явно синтетическими реестрами. Проверяются оба направления, shared modules, declared transitive/offline связи, partial coverage, measured zero и UNKNOWN, exact-scope ranking, deterministic pagination и nonzero/empty-stdout rejection для malformed или inconsistent inputs. [Сохранённое host evidence](../../../../research/results/issue48-cost-l-queries.json) связывает проверки и current-registry query с точными source digests. Synthetic consumer не становится supported deployment.

[Доказательство интеграции manifests](../../../../research/results/issue48-manifest-queries.json) сохраняет семь настоящих query-тестов и тринадцать manifest-тестов с точными исходниками. Объединённые CLI fixtures проверяют связи многие-ко-многим, версии модулей, транзитивных offline-потребителей, все пять команд, детерминированный порядок, страницы, человекочитаемый/JSON-вывод и отклонение неверных version/scope/artifact/deadline/retirement, отсутствующих связей и повторных JSON-ключей до stdout. Сохранённые артефакты намеренно содержат неисполняемые байты, а не поддерживаемые установки. Исторические доказательства не изменяются.

Проверить реальные production-данные и полноту исходников/features в #47, затем получить смысловую и полную EN/RU-проверку человеком до закрытия #48. Объявленные TRANSITIVE-пути всё ещё не являются восстановленным замыканием зависимостей пакетов; глобальные runtime-покрытие и затраты остаются UNKNOWN. Более широкие research/lifecycle acceptance #45/#46 и runtime gates #49–#52 остаются отдельными.

[English original](../../../../docs/research/cost-l-queries.md)

<!-- knowledge -->

```json
{
  "summary": "Bounded offline projections of COST-L declarations, independent dimensions and explicit production/runtime gaps.",
  "schema_version": 1,
  "units": [
    {
      "id": "kolvrt.cost-l.offline-queries",
      "anchor": "kolvrt-cost-l-offline-queries",
      "depends_on": [
        "law.009",
        "law.036",
        "law.040",
        "adr.0007",
        "adr.0008",
        "kolvrt.compatibility.manifests"
      ],
      "feature": {
        "verification": [
          {
            "environment": "host-process",
            "state": "STALE",
            "receipt": "research/results/issue48-manifest-queries.json",
            "receipt_sha256": "c4b07aeddc7c6ae61f463ffbeadfb7b2c65d2ee73871dee39fa2eb1d7294514d",
            "scope": "Bounded host declaration/query consistency with synthetic non-executable production-shaped artifacts only; no observed runtime consumers or costs.",
            "reason": "Phase 3.6 changes shared lifecycle/build/runner sources; historical receipts remain immutable, and their current exact-source applicability is not asserted before new scoped verification."
          },
          {
            "environment": "physical-arm64",
            "reason": "This is read-only offline host tooling; no physical kernel behavior is claimed.",
            "state": "NOT_APPLICABLE"
          }
        ],
        "adrs": ["adr.0007", "adr.0008", "adr.0013", "adr.0015"],
        "readiness": "NOT_READY",
        "roadmap_gate": "COST-L host tooling #48",
        "limitations": [
          "TRANSITIVE relations remain declared without reconstructed package paths; global runtime coverage is UNKNOWN. Production data/source completeness under #47 and semantic/EN-RU human acceptance for #48 remain open."
        ],
        "acceptance": [
          "research/results/issue48-cost-l-queries.json",
          "research/results/issue48-cost-l-arena-dispatch.json",
          "research/results/issue48-cost-l-arena-rebased.json",
          "research/results/issue48-manifest-queries.json"
        ],
        "transitions": [
          {
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "First scoped CLI implementation with exact-source host acceptance; production integration remains gated by #47.",
            "from": "UNRECORDED",
            "acceptance": ["research/results/issue48-cost-l-queries.json"]
          }
        ],
        "issues": [48],
        "implementation_scope": "Bounded host queries over registry and reciprocal module declarations, default repository join and explicit custom-inventory join; no runtime inspection or production deployment.",
        "implementation": "BOUNDED_IMPLEMENTED",
        "next_gate": "Review actual production module data and source/feature completeness under #47; obtain semantic and complete EN/RU human acceptance before closing #48.",
        "sources": [
          "crates/repository-checks/src/cost_l_queries.rs",
          "crates/repository-checks/src/cost_l.rs",
          "crates/repository-checks/src/main.rs",
          "crates/repository-checks/src/lib.rs",
          "crates/repository-checks/tests/cost_l_queries.rs",
          "crates/repository-checks/Cargo.toml",
          "Cargo.lock",
          "rust-toolchain.toml",
          "schemas/cost-l.schema.json",
          "schemas/cost-l-registry.schema.json",
          "research/cost-l/registry.json",
          "research/cost-l/COST-L-0001.json",
          "research/cost-l/COST-L-0002.json",
          "research/cost-l/COST-L-0003.json",
          "Cargo.toml",
          "crates/host-process-metrics/Cargo.toml",
          "crates/kernel-core/Cargo.toml",
          "crates/kernel/Cargo.toml",
          "crates/migration-advisor/Cargo.toml",
          "crates/migration-workbench/Cargo.toml",
          "crates/native-protocol-model/Cargo.toml",
          "crates/native-state-models/Cargo.toml",
          "crates/repository-checks/src/compat_modules.rs",
          "crates/repository-checks/tests/compat_modules.rs",
          "crates/routing-demo/Cargo.toml",
          "crates/routing/Cargo.toml",
          "crates/window-compat/Cargo.toml",
          "crates/window-compat/src/lib.rs",
          "crates/window-compat/tests/protocols.rs",
          "crates/xtask/Cargo.toml",
          "policy/compatibility-modules.json",
          "policy/exceptions.json",
          "schemas/compatibility-modules.schema.json"
        ]
      },
      "summary": "Read-only host COST-L show/consumers/deps/list/top with UNKNOWN, exact-scope sorting and pagination.",
      "kind": "feature"
    }
  ],
  "id": "doc.kolvrt.research.cost-l-queries",
  "kind": "subsystem-contract"
}
```
