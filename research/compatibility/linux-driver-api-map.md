# Pinned Linux driver API observations

Document status: CURRENT
Document scope: executed lexical extraction from three pinned source files; no driver execution or complete dependency analysis.
Status reference: [Observation artifact](linux-driver-api-observations.json)

## Corpus and acquisition

Linux v6.12 commit adc218676eef25575469234709c2d87185ca223a is pinned. The deliberate convenience sample has [igb](https://github.com/torvalds/linux/blob/adc218676eef25575469234709c2d87185ca223a/drivers/net/ethernet/intel/igb/igb_main.c), [USB storage](https://github.com/torvalds/linux/blob/adc218676eef25575469234709c2d87185ca223a/drivers/usb/storage/usb.c) and [VirtIO block](https://github.com/torvalds/linux/blob/adc218676eef25575469234709c2d87185ca223a/drivers/block/virtio_blk.c). They provide different transport/class paths, not a representative Linux population or a hardware relevance ranking.

Retrieve each file from the exact commit, keeping basenames igb_main.c, usb.c and virtio_blk.c in SOURCE_DIRECTORY. For each upstream path, the GitHub contents API accepts ref=adc218676eef25575469234709c2d87185ca223a and Accept: application/vnd.github.raw+json. The extractor checks pinned SHA-256 after UTF-8 decoding, BOM removal and CRLF-to-LF normalization; substitute or oversized inputs fail. No upstream source files or personal acquisition paths are embedded in the artifact.

```text
node scripts/analyze-linux-driver-apis.cjs SOURCE_DIRECTORY
node --test scripts/tests/linux-driver-apis.test.cjs
```

The [extractor](../../scripts/analyze-linux-driver-apis.cjs) strips comments/string/character literals while retaining line positions, then observes exact identifiers from a finite reviewed vocabulary. [Tests](../../scripts/tests/linux-driver-apis.test.cjs) cover comments/strings/longer identifiers, inactive preprocessor branches, bounded locators and substituted/oversized files. Each observation retains source path/URL, normalized digest, lexical occurrence count and up to 32 first line locators. A truncated locator list is explicit.

## Observed family references

| File         | Families with lexical references                                                                  |
| ------------ | ------------------------------------------------------------------------------------------------- |
| igb_main.c   | allocation, locking, IRQ, deferred work, DMA, PCI, power management, device model, ioctl, network |
| usb.c        | allocation, locking, deferred work, USB                                                           |
| virtio_blk.c | allocation, locking, deferred work, sysfs, block                                                  |

All three files contain references in allocation, locking and deferred-work vocabulary. This is observed source reach within this sample, not compatible-driver reach. Zero firmware references means none of the selected firmware identifiers was observed in these translation units; indirect/conditional requirements remain unknown.

## Dependency and evidence limits

No preprocessing, configuration selection, macro expansion, generated code, transitive includes/helpers, call graph or runtime execution occurs. Declarations, assignments and inactive branches are lexical references; counts are not API calls. A missing identifier does not prove an absent semantic dependency.

Driver → API observation → semantic hypothesis → COST-L remains an evidence chain to review. Allocation references motivate inspecting COST-L-0001; sysfs/ioctl references motivate specific attribute/command review for COST-L-0002/0003. No automatic confirmed edge is emitted. Named supported consumers remain empty until exact behavior, native difference and scope are established.

Further work in #45/#55 needs a documented configured corpus, transitive dependency evidence, callback/lifetime/shared-state review and false-positive/false-negative audits. The [taxonomy](taxonomy.md) and [adapter method](high-leverage-adapters.md) retain these limits. Neither source reach nor hash verification proves a driver boots or runs safely.

[Russian translation](../../translations/ru/research/compatibility/linux-driver-api-map.md)
