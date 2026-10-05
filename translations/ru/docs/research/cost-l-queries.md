# Офлайн-запросы COST-L

Document status: CURRENT
Evidence scope: ограниченные host-запросы к проверенным declarations реестра; issue #48 остаётся открытой до production-manifest integration после #47.
Current reference: [Контракт реестра](../architecture/compatibility-debt.md)

<a name="kolvrt-cost-l-offline-queries"></a>

## Контракт запросов

Архитектура повторно выведена из действующих правил identity, authority, support и evidence до расширения реестра. Используются существующий ограниченный [валидатор](../../../../crates/repository-checks/src/cost_l.rs) и закрытые схемы. [Реализация запросов](../../../../crates/repository-checks/src/cost_l_queries.rs) — проекция только для чтения: она не вводит второй реестр и не выдаёт процессу grants, доступ к payload или authority для retirement. Зависимости ядра, политика scheduler и реализация Linux driver не добавляются.

```text
cargo run --locked -p repository-checks -- cost-l show COST-L-0001
cargo run --locked -p repository-checks -- cost-l consumers COST-L-0001 --json
cargo run --locked -p repository-checks -- cost-l deps CONSUMER --json
cargo run --locked -p repository-checks -- cost-l list --status CANDIDATE --json
cargo run --locked -p repository-checks -- cost-l top --sort reach --limit 20
cargo run --locked -p repository-checks -- cost-l top --security reclaim --maintenance context --migration native
cargo run --locked -p repository-checks -- cost-l top --sort cpu --measurement-scope "exact workload" --denominator "exact operation" --json
```

Каждый запрос проверяет весь выбранный реестр до вывода в stdout. `--directory PATH` выбирает другой inventory declarations с теми же схемами репозитория и границей evidence references. `show` возвращает scope, modules, consumers, cost dimensions, native decision и support/removal statements выбранного debt. `consumers` возвращает объявленные direct/transitive связи debt; `deps` — те же связи в обратном направлении для точного consumer ID. Module ID/version/scope, offline/recovery/archival kind и support deadline сохраняются для каждой связи. TRANSITIVE — авторская declaration: формат не хранит путь, CLI не вычисляет package dependency closure. Related debts не являются software dependency edges.

У трёх текущих candidates ноль **объявленных** consumers и нет module declarations. Global reach и runtime costs остаются UNKNOWN; это не устанавливает observed driver count, NATIVE classification или support. Software compatibility declarations не позволяют вывести NATIVE в их stated scope; hardware classification остаётся отдельным полем. Production manifests и их bidirectional validation недоступны до #47. Runtime inspection не поддерживается. Поэтому полный acceptance #48 остаётся открытым.

## Независимые измерения и пределы

`list` по умолчанию сортирует по ID; `top` — по убыванию declared reach. При равенстве используются stable IDs и содержимое связи. `--status`/`--category` — точные фильтры; `--security`, `--maintenance` и `--migration` независимо ищут в соответствующих authored statements, не придумывая severity или difficulty scores. Пустая фильтрованная выборка не доказывает global absence.

Metric sorting принимает cpu, latency, memory, copies, allocations, context_switches и throughput. Требуются точные observation scope и denominator; несовместимые units отклоняются. Соответствующие MEASURED значения идут перед UNKNOWN/out-of-scope; исходные provenance/coverage/reason каждой метрики сохраняются. Измеренный ноль остаётся нулём. Убывающий throughput — сортировка rate, а не debt-cost ranking. Сортировка описательная: она не доказывает сопоставимость машин, равную работу, causal attribution или превосходство. Combined security/debt score отсутствует.

По умолчанию страница содержит до 20 строк; допустимо 1–64. `--offset` и `pagination.next_offset` явно показывают оставшиеся строки. Итоговый stdout, включая human output, ограничен 1 MiB; превышение требует меньшей страницы и возвращает nonzero без partial stdout. Input limits сохраняют существующий контракт 4096 records/64 KiB на record. Malformed/reserved/unknown IDs, неизвестные consumers, неподдержанные commands/options, повторные options, неверные pages, несовместимые metric dimensions и противоречивые records возвращают nonzero. Проекция не содержит live state, usable grants или request payloads.

`--json` возвращает machine schema version 1 с command/selector, inventory scope/coverage/gaps, независимыми ranking/filter selection, pagination и rows. Human output включает text states и UNKNOWN без обязательного цвета. Host CLI output независим от будущего runtime wire ABI.

## Доказательства и следующий этап

[End-to-end tests](../../../../crates/repository-checks/tests/cost_l_queries.rs) исполняют настоящий binary с текущими research records и явно синтетическими реестрами. Проверяются оба направления, shared modules, declared transitive/offline связи, partial coverage, measured zero и UNKNOWN, exact-scope ranking, deterministic pagination и nonzero/empty-stdout rejection для malformed или inconsistent inputs. [Сохранённое host evidence](../../../../research/results/issue48-cost-l-queries.json) связывает проверки и current-registry query с точными source digests. Synthetic consumer не становится supported deployment.

Интегрировать независимо reviewed production manifests после #47, затем повторить bidirectional completeness и version/scope/retirement checks до закрытия #48. Повторно вывести архитектуру из актуальных инвариантов; текущие projections и tests не должны закреплять будущую manifest/consumer model. Broader research/lifecycle acceptance #45/#46 и runtime gates #49–#52 остаются отдельными.

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
      "depends_on": ["law.009", "law.036", "law.040", "adr.0007", "adr.0008"],
      "feature": {
        "verification": [
          {
            "receipt_sha256": "bdc6a41c6c16ff647bbb83acabb48b74b530dbffc0259ccdc94c09bbde77010b",
            "receipt": "research/results/issue48-cost-l-arena-rebased.json",
            "scope": "Bounded host query semantics only; no production consumers or runtime costs.",
            "reason": "Executed five existing COST-L CLI tests after main rebase and Arena dispatcher integration; exact-source host evidence only.",
            "state": "VERIFIED",
            "environment": "host-process"
          },
          {
            "environment": "physical-arm64",
            "reason": "This is read-only offline host tooling; no physical kernel behavior is claimed.",
            "state": "NOT_APPLICABLE"
          }
        ],
        "adrs": ["adr.0007", "adr.0008"],
        "readiness": "NOT_READY",
        "roadmap_gate": "COST-L host tooling #48",
        "limitations": [
          "Direct/transitive relations are declarations without reconstructed package paths; current global coverage is UNKNOWN. #47 production-manifest integration and full #48 acceptance remain open."
        ],
        "acceptance": [
          "research/results/issue48-cost-l-queries.json",
          "research/results/issue48-cost-l-arena-dispatch.json",
          "research/results/issue48-cost-l-arena-rebased.json"
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
        "implementation_scope": "Host queries over the existing validated registry declarations; no production-manifest or runtime integration.",
        "implementation": "BOUNDED_IMPLEMENTED",
        "next_gate": "Review and integrate #47 production manifests with bidirectional completeness/version/scope/retirement validation before closing #48.",
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
          "research/cost-l/COST-L-0003.json"
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
