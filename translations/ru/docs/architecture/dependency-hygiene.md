# Гигиена зависимостей

Document status: CURRENT
Evidence scope: host Cargo metadata, source audit and dependency policy enforcement; no kernel behavior claim.
Current reference: [Dependency checker](../../../../scripts/dependency-audit.cjs)

<a name="kolvrt-dependency-hygiene"></a>

## Политика и реализация

### Граница ядра

Production EL1/TCB допускает ровно `kolvrt-kernel → kernel-core`, без внешних Cargo crates. У обоих crates нет build/dev dependencies. Проверка отвергает любое дополнительное объявление, включая optional dependencies, переименованные пакеты, target predicates и пути к host crates. Инвариант объявлений охватывает все комбинации features без экспоненциального перебора и без зависимости от workspace feature unification. Rust `core` и compiler-builtins остаются частью доверия к компилятору; ноль внешних Cargo crates не означает отсутствие compiler/runtime code или измеренный размер TCB в байтах.

Исключение требует отдельного принятого архитектурного решения: необходимость, небольшая собственная альтернатива, размер/closure, unsafe, maintenance, лицензия, security history, влияние на TCB и удаление/замена. Удобства недостаточно. Сейчас исключения не поддерживаются. Принятое изменение должно явно обновить проверку и rejection controls. Порядок реализации не определяет архитектуру.

### Host supply chain

[Retention policy](../../../../dependencies/policy.json) хранит requirement, назначение и причину сохранения для каждой пары consumer/crate/dependency-kind. Новые прямые зависимости и изменения requirement ломают проверку до рассмотрения решения. Генерируемый inventory добавляет версию, лицензию, source, features, direct/transitive status, closure и reachability. Routing/demo dependencies явно находятся вне production EL1; demo EL0 code не помечается как исключительно host-only. Path dependencies вне workspace считаются внешними и запрещены текущей source policy, допускающей только crates.io. Шесть ранее не имевших лицензии workspace crates помечены `publish = false`; это не назначает лицензию и предотвращает случайную публикацию. Лицензия private workspace crates пропускается, но их внешние зависимости проверяются полностью.

`deny.toml` допускает только crates.io sources и явные лицензии (включая MIT-0), запрещает внешние wildcard versions, OpenSSL и RustSec/yanked findings, сообщает о дубликатах версий. Локальные unpublished path dependencies допустимы. Advisory exemptions отсутствуют. Dependency count не является целью оптимизации или фиксированной квотой. Любое расширение graph помечается для review, включая добавления, замаскированные удалениями при одинаковом итоговом количестве.

Dependabot проверяет GitHub Actions, Cargo и npm еженедельно по понедельникам в 06:00/06:15/06:30 Europe/Moscow, ставит тег dependencies и открывает не более пяти PR на ecosystem. Прямые Rust updates требуют пересмотра retention requirement; автоматический merge не настроен. Конфигурация начинает работать после интеграции в default branch.

### Evidence и проверки

Запустить `npm run check:dependencies` или `node scripts/dependency-audit.cjs check`. Последнюю команду запускать с `--supply-chain` после установки `cargo-deny` 0.19.9. Скрипт использует встроенные средства Node и существующие Cargo metadata; новая зависимость kernel или host crate не появляется. `npm run check` включает boundary/inventory/rejection tests. CI добавляет все четыре cargo-deny checks на push, PR и еженедельное обновление advisories.

Kernel foundation выполняет проверки границы зависимостей один раз в обязательном static workload для push в main, PR и ручных запусков. Старый последовательный режим удалён, поэтому успешный baseline job не может обойти gate разделённых host/kernel evidence. Отдельный workflow Dependency hygiene по-прежнему обеспечивает аудит supply chain.

`target/dependencies/inventory.json` содержит default и all-workspace-features graphs для каждого workspace crate, normal/build/dev edges и target predicates, внешние closures каждой прямой зависимости, compiler/lock/policy provenance, дубликаты версий и supply-chain diagnostics. Счётчики исключают корневой пакет и различают уникальные имена crates и разрешённые версии пакетов. Это консервативные workspace-unified/all-target graphs, а не isolated consumer feature counts или реальные размеры linked binaries. Пропущенная или незавершённая supply check имеет явный status и null metrics, без выдуманных нулей. При проверке сохраняются RustSec database commit и raw diagnostics.

CI сравнивает фактический PR base SHA, применяя текущий checker к историческим manifests, публикует `before → after` в PR check summary и загружает evidence с commit в имени на 90 дней. Write token и автоматический PR comment не нужны; исторические audit scripts не выполняются. Скачиваемые artifacts дают недавнюю историю по commit; для длительного хранения архивировать их отдельно до истечения срока. Сохранённый [main baseline](../../../../dependencies/baseline-main.json) остаётся неизменяемым evidence main `599fa24a3278297f9fc9614b0882dd4afc98d423`; это graph evidence, а не историческое утверждение, что текущая supply policy уже существовала.

После интеграции #92 дополнительно сохранён [актуальный main snapshot](../../../../dependencies/baselines/0a3a3d69ff2381b6ec922682af03c1fd13e41a23.json) для `0a3a3d69ff2381b6ec922682af03c1fd13e41a23`. Первый baseline не изменён; новая host-process-metrics/serde dependency включена в retention policy. Новых внешних package versions нет.

### Первый аудит тяжёлых зависимостей

[Machine-readable audit](../../../../dependencies/heavy-audit.json) хранит pinned-source hashes, source bytes, лексические unsafe token occurrences, features, лицензии, security review, влияние на TCB и условия удаления. Source bytes/token counts включают cfg-disabled code и комментарии; это не compiled size и не доказательство безопасности. Maintenance review фиксирует pinned version/source и проверенный RustSec snapshot, а не утверждает, что crate самая новая или не имеет уязвимостей.

| Dependency          | Decision        | Transitive package versions | Reason                                                                                                   |
| ------------------- | --------------- | --------------------------- | -------------------------------------------------------------------------------------------------------- |
| jsonschema 0.33.0   | KEEP            | 98 → 98                     | Draft 2020-12 validator; HTTP/file resolution уже отключены; reqwest отсутствует.                        |
| ed25519-dalek 2.2.0 | KEEP            | 34 → 34                     | Strict provenance verification и fixture signatures; сохраняются zeroization и проверенная криптография. |
| quanta 0.13.0       | REDUCE FEATURES | 28 → 28                     | Отключены неиспользуемые mock/flaky_tests; сохранена семантика calibrated clock.                         |

Baseline root source sizes составляют соответственно 671 464 / 111 014 / 63 273 байт; лексические unsafe occurrences — 0 / 5 / 6. Транзитивно достижимый код может содержать unsafe, даже если в корневом crate его нет. Kernel external crates остаётся ноль. Workspace содержит восемь прямых внешних имён crates и 130 внешних package versions (128 имён); `getrandom` и `syn` имеют по две версии. Завершённая проверка 2026-10-05 нашла ноль применимых advisories и ноль license/source violations по сохранённому RustSec database snapshot. Это evidence с ограниченной областью, а не сертификация.

Историческая [RUSTSEC-2022-0093](https://rustsec.org/advisories/RUSTSEC-2022-0093.html) затрагивает ed25519-dalek до v2; pinned 2.2.0 использует `SigningKey`, strict verification и не включает hazmat feature. Транзитивная [RUSTSEC-2024-0344](https://rustsec.org/advisories/RUSTSEC-2024-0344.html) охвачена текущей RustSec проверкой curve25519-dalek 4.1.3. Замена quanta требует нового timing profile и overhead/resolution evidence; замена jsonschema требует эквивалентных schema rejection tests; собственная криптография не является допустимым способом сократить счётчик.

### Проверка реализации

Rejection controls включают реальные Cargo metadata для optional/target-specific, build и dev dependencies, подменённых путей, host reachability, отсутствующих TCB members, graph cycles/diamonds, замен с одинаковым счётчиком, feature changes, retention changes и незавершённого supply-chain output. Workbench/advisor/schema-validator regression tests проверяют сокращение quanta features. CI обеспечивает live advisory/license/source enforcement и хранит результаты отдельно от kernel behavior evidence.

Оба workflow берут Node 24.21.0 из `.nvmrc`. Локально эту версию устанавливают и включают через nvm: `nvm install 24.21.0`, затем `nvm use 24.21.0`. Checkout 7.0.1, setup-node 7.0.0 и upload-artifact 7.0.1 используют встроенную среду Node 24. Аудит закреплён за `ubuntu-24.04`: будущая смена `ubuntu-latest` не изменит окружение без правки workflow. Ядро проверяется на `windows-2022`. Локальные проверки под Node 24 не подтверждают работу на hosted runner; для восстановления статуса VERIFIED нужны новые доказательства CI на текущих исходниках.

Issue #109 добавляет кэш закреплённого cargo-deny 0.19.9 с проверкой целостности и обязательную проверку версии в Ubuntu. Workflow ядра разделяет static/host checks и выполняет полный matrix из четырёх shards с exact-source агрегацией; npm run check сохраняет все уникальные host validations. Кэши скачиваний и compiled artifacts никогда не разрешают пропускать supply-chain или kernel checks. Поведение кэшей на hosted runners не проверено; исторические dependency receipts остаются STALE.

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.kolvrt.dependencies.hygiene",
  "kind": "policy",
  "summary": "Production TCB dependency boundary and observable host supply chain.",
  "units": [
    {
      "id": "kolvrt.dependencies.hygiene",
      "anchor": "kolvrt-dependency-hygiene",
      "kind": "feature",
      "summary": "All-declaration kernel boundary, feature-expanded inventory, graph deltas and supply-chain checks.",
      "tags": ["dependencies", "supply-chain", "kernel", "tcb"],
      "feature": {
        "implementation": "BOUNDED_IMPLEMENTED",
        "implementation_scope": "Host-only metadata and policy checker; no external production kernel dependency, no count quota.",
        "sources": [
          "scripts/dependency-audit.cjs",
          "scripts/dependency-base.cjs",
          "scripts/audit-heavy-dependencies.cjs",
          "scripts/tests/dependency-audit.test.cjs",
          ".github/workflows/dependencies.yml",
          "dependencies/policy.json",
          "deny.toml",
          "package.json",
          "crates/kernel/Cargo.toml",
          "crates/kernel-core/Cargo.toml",
          "crates/routing/Cargo.toml",
          "crates/routing-demo/Cargo.toml",
          "crates/window-compat/Cargo.toml",
          "crates/xtask/Cargo.toml",
          "crates/migration-workbench/Cargo.toml",
          ".github/dependabot.yml",
          ".github/workflows/kernel.yml",
          ".nvmrc"
        ],
        "acceptance": ["research/results/issue94-dependency-hygiene.json"],
        "issues": [94],
        "adrs": ["adr.0013"],
        "readiness": "NOT_READY",
        "next_gate": "Review EN/RU policy, rerun Node 24 CI on Windows 2022 and Ubuntu 24.04, and review PR-base delta before integration.",
        "limitations": [
          "Workspace-unified all-target closures are conservative, not compiled size or isolated consumer counts. Artifact history expires after 90 days without external archival. No kernel execution claim."
        ],
        "verification": [
          {
            "environment": "host-process",
            "state": "STALE",
            "reason": "Checkpoint publication coordination, counter ordering and native control inventory changed shared sources. Historical receipts retain their exact scope; current-source applicability requires new scoped evidence and semantic/EN-RU review.",
            "scope": "Host dependency policy and compile checks only; no kernel runtime or physical-hardware claim.",
            "receipt": "research/results/issue94-dependency-hygiene.json",
            "receipt_sha256": "a27a01010ef49bbfcb545513c59d932849f91901e562fee57752786d95789b2f"
          },
          {
            "environment": "physical-arm64",
            "state": "NOT_APPLICABLE",
            "reason": "Host dependency checks do not establish kernel behavior."
          }
        ],
        "transitions": [
          {
            "from": "UNRECORDED",
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "First bounded implementation of the dependency hygiene policy, inventory and CI controls.",
            "acceptance": ["research/results/issue94-dependency-hygiene.json"]
          }
        ]
      }
    }
  ]
}
```
