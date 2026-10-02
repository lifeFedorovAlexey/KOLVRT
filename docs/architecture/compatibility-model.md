# Compatibility model v0.1

Compatibility module — самостоятельный adapter внешней семантики на текущий native
contract. Его manifest содержит:

| Поле | Значение |
|---|---|
| behavior_id, semantic_version | Неизменяемое описание поддерживаемого поведения |
| implementation_digest | Точный исполняемый artifact; patch update не меняет незаметно semantics |
| owner, source_cases | Ответственность и обоснование из pathology |
| input_protocol, native_contract | Поддерживаемые диапазоны, marshalling и errors |
| required_rights, resource_limits | Минимальные grants, память, pins, queues |
| state_domain, dependencies | Совместно владеемое состояние и transitively required modules |
| security_boundary | Process isolation либо явно указанное выполнение в privileged domain |
| metrics_id, consumer_identity | Атрибуция, lifetime counters, retention policy |
| lifecycle, migration, removal | Drain, state conversion, rollback и критерии удаления |

Semantic version сравнивается по explicit compatibility matrix, не по предположению
«больше — лучше». Решение resolver закрепляет точную пару version/digest. Две версии
могут жить одновременно только при изолированном state либо доказанном interop.
Цикл dependencies, unsupported operation и конфликтный route — явная ошибка до запуска.

Native-only build исключает software compat modules вместе с их dependencies и проходит
тот же native contract suite. Это требование к будущему build graph, не существующая
kernel build: Phase 0.1 не содержит Cargo kernel crates.

## Quarantine и hardware

Legacy layouts, errno rules, syscall tables, process-wide locks и старые protocol state
machines принадлежат adapters. Нельзя добавлять в native scheduler, MM, VFS, HAL или
driver `if old_application` ради них. Adapter не может расширять grants или выключать
memory protection. Не всё переводится: Linux fork, signals, credentials, shared fd и
futex state могут требовать целой personality domain.

Hardware translation отдельно: старый DT/ACPI frontend преобразует входной descriptor;
erratum module может преобразовывать codegen или операции platform backend. Если без
workaround железо неправильно, удаление workaround одновременно исключает affected target.
Это не нарушение native-only software build. Для Cortex-A53 843419 независимый динамически
выгружаемый process shim вообще непригоден: [case 11](../../research/pathology/KOL-PATH-0011.json).

## Bug-compat lifecycle

1. Root cause, исправленный native contract и regression test фиксируются вместе.
2. Определяется реальный consumer старой семантики; гипотетическая совместимость не повод писать модуль.
3. Security review проверяет выразимость старого поведения без ослабления native invariants.
4. Безопасное поведение получает отдельный bug-compat ID, fixture и opt-in manifest.
5. Consumers получают migration guide и A/B measurement с semantic oracle.
6. Module становится deprecated только с явным support window и уведомлением consumers.
7. Удаление требует нуля declared зависимостей в support scope, достаточного observation
   coverage, отсутствия in-flight state и успешного native-only build.

Отсутствие recent calls не равно отсутствию consumers: offline packages, dormant paths и
recovery tools должны учитываться. Registry хранит definition и tombstone даже после
удаления implementation, чтобы старый manifest получал понятную ошибку.

Пример жизненного цикла — модель, не реализованный loader:
`Available -> Bound -> Draining -> Unbound -> Removable`.
Unload во время Bound/Draining запрещён. Ошибка миграции возвращает старый route только
до irreversible side effect; после него требуются recovery/explicit failure, не blind retry.
