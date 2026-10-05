# Время и жизненный цикл сеанса verifier

Document status: CURRENT
Evidence scope: ограниченное исправление на машине разработчика для #71; тесты с внедрёнными часами, без подтверждения доверенного времени, живого отзыва ключей и долговечного replay после сбоя хоста.
Current reference: [Реализация проверки происхождения](../../../../crates/migration-advisor/src/provenance.rs)

<a name="kolvrt-verifier-time"></a>

## Временная граница

`SignedArtifactStore::provisioned` принимает настроенный приложением `Arc<dyn TrustedUtcClock>`, а не целочисленный снимок времени при создании. Часы и политика не десериализуются из запроса. CLI использует `SystemUtcClock`; доверие к UTC операционной системы — явное допущение для host-среды, а не промышленная служба аутентифицированного времени.

При создании фиксируется первое наблюдение. Допуск сеанса проверяет текущее время перед проверкой подписанного challenge и после операций с ledger, непосредственно перед публикацией активной привязки. Каждый provisioned-вызов `authenticate` проверяет время при входе и перед успешным возвратом. Обе проверки учитывают срок сеанса, допустимое опережение часов, издателя challenge и срок ключей свидетельств. Сеанс, истекающий в момент T, отвергается в T. Второй проход отбирает уже проверенные неизменяемые attestations и повторно проверяет независимость производителей; подписи и хеши файлов повторно не вычисляются.

Принятый результат действителен на момент последнего наблюдения. Он не даёт права на последующее развёртывание без отдельной проверки актуального допуска и разрешения. Стабильность артефактов остаётся обязанностью владельца хранилища.

## Отказы часов и параллельность

Недоступное время, повреждённый mutex или наблюдение раньше последнего принятого времени этого verifier приводят к отказу. Недоступность или откат навсегда делают экземпляр непригодным; восстановление часов его не оживляет. Нужны новый verifier из внешней конфигурации и новый challenge от издателя. Параллельные наблюдения, полученные не в порядке времени, также консервативно отвергаются; интеграция с параллельными вызовами должна учитывать это ограничение доступности.

Обращения к часам выполняются вне mutex verifier. Провайдер обязан выдавать достоверное продвигающееся UTC либо сообщать об отказе. Проверка порядка наблюдений не обнаруживает злонамеренно замороженные часы и не доказывает подлинность времени. Rust прямо указывает, что [SystemTime не монотонен](https://doc.rust-lang.org/std/time/struct.SystemTime.html). [Правила истечения RFC 7519](https://www.rfc-editor.org/rfc/rfc7519#section-4.1.4) дают близкий первичный пример сравнения срока с текущим временем, а не временем создания; этот собственный протокол подписей не является JWT и не заимствует его полномочия.

Экземпляр допускает не больше одного provisioned-сеанса после проверки challenge. Атомарный флаг допуска устанавливается перед созданием replay-маркера; другой сеанс не может заменить активную привязку или смешать свидетельства разных сеансов внутри одной проверки. Неудачную проверку до этой точки можно повторить; после начала допуска через ledger отказ расходует экземпляр. Неудачная завершающая проверка времени или ключа оставляет маркер challenge и не активирует сеанс. Маркеры не удаляются ради успешного повтора.

## Политика ключей и оставшиеся задачи

Ключи, признаки отзыва/компрометации и домены производителей — неизменяемый снимок политики. Начало и конец действия ключей, включая ключ издателя сеанса, проверяются по текущему времени в течение активного сеанса. Новая политика отзыва или ротации требует вывести старые экземпляры из использования и создать новые из внешней конфигурации; живой отзыв, принудительное прекращение использования и безопасная доставка политики остаются промышленными задачами. Старый экземпляр не объявляется наблюдающим внешнее изменение политики.

Replay по-прежнему ограничен созданием маркера в host-файловой системе ровно один раз. Долговечная фиксация файла и его имени, модель хранилища и сбоев, восстановление повреждений, удаление старых записей и тесты остановки процесса/потери питания остаются открытыми в #71. Промышленный допуск #14 не завершён. Это исправление не заявляет долговечность и не добавляет установку, смену маршрутов, зависимость ядра или механизм IPC.

Rust API конструктора намеренно меняется: вызывающий код передаёт реализацию часов вместо сохранения опасного жизненного цикла снимка. Экспериментальная схема 1 остаётся отдельной. Раннее внедрение не оправдывает сохранение старого интерфейса.

## Границы проверки

[Тесты provisioned verifier](../../../../crates/migration-advisor/tests/provisioned_provenance.rs) используют частные синтетические часы и ключи Ed25519. Проверяются истечение при создании/допуске/аутентификации, будущие сеансы, недоступность и откат часов с необратимым отказом, сроки ключей издателя и производителей, истечение и отказ на завершающей проверке, израсходованные неопубликованные challenges и параллельный допуск разных сеансов. [Тесты миграции](../../../../crates/migration-advisor/tests/migration.rs) сохраняют подписанную цепочку планирования/разрешения и проверки CLI с новым интерфейсом.

[Свидетельство точных исходников](../../../../research/results/issue71-verifier-time.json) фиксирует только host-проверку объявленных байтов. Это не подтверждение аутентифицированного времени, реальной независимости владельцев ключей, физического ARM64 или сохранности после потери питания. Полное закрытие #71 требует перечисленных оставшихся задач.

[Английский оригинал](../../../../docs/security/verifier-time.md)

<!-- knowledge -->

```json
{
  "kind": "subsystem-contract",
  "summary": "Допуск host-verifier по текущим часам с необратимым отказом и неизменяемыми границами сеанса и политики.",
  "units": [
    {
      "summary": "Допуск host-verifier по текущим часам с необратимым отказом и неизменяемыми границами сеанса и политики.",
      "depends_on": [
        "law.009",
        "law.013",
        "law.036",
        "law.040",
        "adr.0013",
        "adr.0008"
      ],
      "tags": ["trust", "clock", "freshness", "session", "migration"],
      "kind": "feature",
      "id": "kolvrt.security.verifier-time",
      "feature": {
        "verification": [
          {
            "scope": "Host verification mechanism with injected fixture clocks; no production authenticated time, live revocation or storage crash guarantees.",
            "receipt_sha256": "fd273b9171688235813dddb0b7e83492b6a63dbd2f405676588bbe7836c68e67",
            "environment": "host-process",
            "receipt": "research/results/issue71-verifier-time.json",
            "reason": "Historical receipt retained after consolidated IPC/dependency integration; current-source revalidation is required for changed declared inputs: crates/migration-advisor/src/provenance.rs, crates/migration-advisor/Cargo.toml, Cargo.lock",
            "state": "STALE"
          },
          {
            "state": "NOT_APPLICABLE",
            "environment": "physical-arm64",
            "reason": "Host verifier mechanics do not establish physical KOLVRT execution or hardware clock trust."
          }
        ],
        "readiness": "NOT_READY",
        "transitions": [
          {
            "acceptance": ["research/results/issue71-verifier-time.json"],
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "First enrolled corrected temporal verifier scope with exact-source host evidence; durable replay and production trust gates remain open.",
            "from": "UNRECORDED"
          }
        ],
        "implementation_scope": "Current-time session/key admission and final-acceptance revalidation, sticky clock failure/rollback rejection and single active provisioned session per host verifier.",
        "next_gate": "Complete #71 durable replay, recovery and live policy/time integration without upgrading synthetic evidence into production trust.",
        "roadmap_gate": "Migration trust #71",
        "issues": [71, 14],
        "sources": [
          "crates/migration-advisor/src/provenance.rs",
          "crates/migration-advisor/src/main.rs",
          "crates/migration-advisor/tests/provisioned_provenance.rs",
          "crates/migration-advisor/tests/migration.rs",
          "crates/migration-advisor/Cargo.toml",
          "Cargo.lock",
          "rust-toolchain.toml"
        ],
        "adrs": ["adr.0013", "adr.0008"],
        "limitations": [
          "Immutable policy snapshot does not observe online revocation. Trusted provider authenticity/progress, storage-crash durability, recovery/pruning and production admission remain open; quantum/kernel execution is not involved."
        ],
        "acceptance": ["research/results/issue71-verifier-time.json"],
        "implementation": "BOUNDED_IMPLEMENTED"
      },
      "anchor": "kolvrt-verifier-time"
    }
  ],
  "id": "doc.kolvrt.security.verifier-time",
  "schema_version": 1
}
```
