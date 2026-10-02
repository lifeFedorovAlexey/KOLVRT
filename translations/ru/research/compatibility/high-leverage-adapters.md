# Исследование compatibility adapters с широким охватом

Document status: CURRENT
Document scope: начальный observed source reach и метод решений; без working adapters или coverage percentages.
Status reference: [Сохранённые наблюдения](../../../../research/compatibility/linux-driver-api-observations.json)

## Фактическое начальное наблюдение

[Закреплённый sample из трёх файлов](linux-driver-api-map.md) содержит следующие observed lexical family references. Counts относятся к source files в выбранном sample, а не к executed или compatible drivers.

| Family                                                       | Files с observed references | Значение                                                                           |
| ------------------------------------------------------------ | --------------------------- | ---------------------------------------------------------------------------------- |
| Allocation, locking, deferred work                           | По 3                        | Общие investigation candidates; semantics/configuration closure не разрешены       |
| IRQ, DMA, PCI, USB                                           | По 1                        | Device/transport authority и lifetime требуют отдельных contracts                  |
| Power management, device model, sysfs, ioctl, network, block | По 1                        | Конкретные source references; не выводить самостоятельные reusable adapters        |
| Firmware                                                     | 0                           | Выбранный symbol не наблюдается; indirect requirements и broader corpus неизвестны |

Результат поддерживает первоочередную проверку общих allocation/context/deferred-work assumptions. Он не показывает, что три adapters включают эти drivers. Малый direct DMA reach особенно обманчив для USB/VirtIO paths, чьи helpers и transport layers находятся вне sample.

## Метод исследования и stop conditions

1. Закрепить Linux revision, architectures/configurations, driver selection и exclusions. Включить representative hardware/classes и disconfirming cases; указать convenience-sample bias.
2. Построить direct и transitive API/callback/state dependencies. Соотнести реальные observable semantic differences с reviewed COST-L causes; одного family name недостаточно.
3. Для каждого candidate adapter доказать требуемый semantic closure, shared-state identity, supported operation/layout versions и узкую native authorization/effect boundary.
4. Указывать potential reach только для consumers с установленными полными prerequisites. Сохранять incomplete/conditional edges; любой будущий proportion сопровождается denominator и corpus scope.
5. Сравнивать hardware relevance, ARM64 applicability, security/TCB, complexity/API churn, measured-or-unknown runtime cost, maintenance, migration/support и native alternatives как отдельные dimensions.
6. Прекращать compatibility work при authority expansion, необходимости Linux semantics в native EL1, отсутствии bounded progress или невозможности установить IRQ/DMA quiescence/reset. Пересмотреть native rewrite, deferral или unsupported status до роста второго Linux kernel.

Weighted security/debt score и выдуманные 10/20/40-family percentages не выдаются. Actual hardware popularity и enabled IOMMU/device containment остаются UNKNOWN; source IDs сами по себе их не устанавливают. Hardware requirement с источниками может требовать native support, не являясь software debt.

## Текущие решения и оставшаяся работа

Allocation-context COST-L-0001 — research candidate, а не implemented adapter proposal. Context-coupled locking/deferred work остаются dossiers для разделения по actual state/progress assumptions. DMA ownership — counterexample blanket legacy criticism: upstream guide уже задаёт map/unmap lifetime и barriers. Сохранить [решения по seeds](taxonomy.md).

[Критерии adapter/native и разобранные случаи VirtIO/USB](adapter-versus-native.md) дают начальное исследование #57, явные причины отклонения и условия пересмотра отложенной реализации. Issue #55 отвечает за confirmed driver/debt graph edges. #45 остаётся открытым для configured/transitive и historical analysis. #59 рассматривает host grouping/crash containment после явного определения native service/device contracts; его prototype здесь не разрешён.

Benchmark contracts переиспользуют #49/#50 и существующий migration advisor. Сравнивать одинаковые useful results, authority, outcomes и lifetime; сохранять raw provenance и честно более быструю compatibility. Наблюдаемые source counts не являются costs. Linux Driver Host, driver port, physical inventory или runtime compatibility не реализованы.

[Английский оригинал](../../../../research/compatibility/high-leverage-adapters.md)
