# Закреплённые наблюдения Linux driver API

Document status: CURRENT
Evidence scope: выполненное lexical extraction из трёх pinned source files; без исполнения драйверов или полного dependency analysis.
Current reference: [Артефакт наблюдений](../../../../research/compatibility/linux-driver-api-observations.json)

## Corpus и получение источников

Закреплён Linux v6.12 commit adc218676eef25575469234709c2d87185ca223a. Намеренный convenience sample включает [igb](https://github.com/torvalds/linux/blob/adc218676eef25575469234709c2d87185ca223a/drivers/net/ethernet/intel/igb/igb_main.c), [USB storage](https://github.com/torvalds/linux/blob/adc218676eef25575469234709c2d87185ca223a/drivers/usb/storage/usb.c) и [VirtIO block](https://github.com/torvalds/linux/blob/adc218676eef25575469234709c2d87185ca223a/drivers/block/virtio_blk.c). Они дают разные transport/class paths, но не representative Linux population или hardware relevance ranking.

Получить каждый файл из точного commit, сохранив basenames igb_main.c, usb.c и virtio_blk.c в SOURCE_DIRECTORY. Для каждого upstream path GitHub contents API принимает ref=adc218676eef25575469234709c2d87185ca223a и Accept: application/vnd.github.raw+json. Extractor проверяет pinned SHA-256 после UTF-8 decoding, удаления BOM и CRLF-to-LF normalization; подменённые или чрезмерные inputs отклоняются. Upstream source files и личные acquisition paths не включаются в artifact.

```text
node scripts/analyze-linux-driver-apis.cjs SOURCE_DIRECTORY
node --test scripts/tests/linux-driver-apis.test.cjs
```

[Extractor](../../../../scripts/analyze-linux-driver-apis.cjs) удаляет comments/string/character literals, сохраняя line positions, затем наблюдает точные identifiers из конечного проверяемого vocabulary. [Тесты](../../../../scripts/tests/linux-driver-apis.test.cjs) проверяют comments/strings/longer identifiers, inactive preprocessor branches, bounded locators и подменённые/чрезмерные files. Каждое observation сохраняет source path/URL, normalized digest, lexical occurrence count и до 32 первых line locators. Усечение locator list отмечается явно.

## Наблюдаемые family references

| File         | Families с lexical references                                                                     |
| ------------ | ------------------------------------------------------------------------------------------------- |
| igb_main.c   | allocation, locking, IRQ, deferred work, DMA, PCI, power management, device model, ioctl, network |
| usb.c        | allocation, locking, deferred work, USB                                                           |
| virtio_blk.c | allocation, locking, deferred work, sysfs, block                                                  |

Все три файла содержат references из vocabulary allocation, locking и deferred work. Это наблюдаемый source reach в данном sample, а не compatible-driver reach. Ноль firmware references означает отсутствие выбранных firmware identifiers в этих translation units; indirect/conditional requirements остаются неизвестными.

## Dependency и границы evidence

Нет preprocessing, configuration selection, macro expansion, generated code, transitive includes/helpers, call graph или runtime execution. Declarations, assignments и inactive branches являются lexical references; counts не обозначают API calls. Отсутствующий identifier не доказывает отсутствие semantic dependency.

Driver → API observation → semantic hypothesis → COST-L остаётся evidence chain, требующим review. Allocation references мотивируют проверку COST-L-0001; sysfs/ioctl references — конкретных attributes/commands для COST-L-0002/0003. Автоматические confirmed edges не выдаются. Named supported consumers остаются пустыми до установления exact behavior, native difference и scope.

Дальнейшая работа #45/#55 требует documented configured corpus, transitive dependency evidence, callback/lifetime/shared-state review и false-positive/false-negative audits. [Taxonomy](taxonomy.md) и [adapter method](high-leverage-adapters.md) сохраняют эти ограничения. Ни source reach, ни hash verification не доказывают, что driver загружается или безопасно работает.

[Английский оригинал](../../../../research/compatibility/linux-driver-api-map.md)
