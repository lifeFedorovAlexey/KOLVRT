"use strict";

// Pinned exploratory lexical observations, not a C compiler or compatibility proof.
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const COMMIT = "adc218676eef25575469234709c2d87185ca223a";
const SAMPLES = [
  {
    file: "igb_main.c",
    upstream: "drivers/net/ethernet/intel/igb/igb_main.c",
    digest: "02d70acc747b5dbedb229dad4ec09e0bc1988dda4c43f46dbb914965efa62e6e",
  },
  {
    file: "usb.c",
    upstream: "drivers/usb/storage/usb.c",
    digest: "687610467c65ca47576486cd1510b2fa7be7e40a60f5c2ec379794f311437fca",
  },
  {
    file: "virtio_blk.c",
    upstream: "drivers/block/virtio_blk.c",
    digest: "5fa798b0643db3d0b75af30923e9adaca258249503f3dd9f770db636477ac49f",
  },
];
const FAMILIES = {
  allocation: [
    "kmalloc",
    "kzalloc",
    "kcalloc",
    "kfree",
    "GFP_KERNEL",
    "GFP_ATOMIC",
    "GFP_NOIO",
    "GFP_NOFS",
  ],
  locking: [
    "mutex_lock",
    "mutex_unlock",
    "spin_lock",
    "spin_lock_irqsave",
    "spin_unlock_irqrestore",
  ],
  irq: [
    "request_irq",
    "request_threaded_irq",
    "free_irq",
    "disable_irq",
    "enable_irq",
  ],
  deferred_work: [
    "INIT_WORK",
    "INIT_DELAYED_WORK",
    "schedule_work",
    "queue_work",
    "cancel_work_sync",
    "tasklet_setup",
    "timer_setup",
    "mod_timer",
  ],
  dma: [
    "dma_map_single",
    "dma_map_page",
    "dma_unmap_single",
    "dma_unmap_page",
    "dma_alloc_coherent",
    "dma_free_coherent",
    "dma_set_mask_and_coherent",
  ],
  pci: [
    "pci_enable_device",
    "pci_disable_device",
    "pci_set_master",
    "pci_request_regions",
    "pci_iomap",
  ],
  usb: [
    "usb_alloc_urb",
    "usb_submit_urb",
    "usb_kill_urb",
    "usb_get_dev",
    "usb_put_dev",
  ],
  firmware: ["request_firmware", "release_firmware"],
  power_management: [
    "pm_runtime_get_sync",
    "pm_runtime_put_sync",
    "pm_runtime_enable",
  ],
  device_model: [
    "device_register",
    "device_unregister",
    "get_device",
    "put_device",
    "dev_set_drvdata",
    "dev_get_drvdata",
  ],
  sysfs: [
    "device_create_file",
    "device_remove_file",
    "sysfs_create_group",
    "sysfs_remove_group",
    "DEVICE_ATTR",
    "DEVICE_ATTR_RW",
    "DEVICE_ATTR_RO",
  ],
  ioctl: ["compat_ioctl", "compat_ptr_ioctl", "ndo_do_ioctl", "ndo_eth_ioctl"],
  network: [
    "netif_napi_add",
    "napi_schedule",
    "napi_complete_done",
    "netif_tx_stop_all_queues",
    "register_netdev",
  ],
  block: [
    "blk_mq_alloc_disk",
    "blk_mq_start_request",
    "blk_mq_end_request",
    "blk_mq_free_tag_set",
  ],
};
function normalized(text) {
  return text.replace(/^\uFEFF/, "").replace(/\r\n/g, "\n");
}
function scanSymbols(source) {
  const text = normalized(source);
  // Keep line numbering. Preprocessor branches, macros and declarations are not resolved.
  const lexical = text.replace(
    /\/\*[\s\S]*?\*\/|\/\/[^\n]*|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'/g,
    (match) => match.replace(/[^\n]/g, " "),
  );
  const lines = lexical.split("\n");
  const families = {};
  for (const [family, symbols] of Object.entries(FAMILIES)) {
    const observations = [];
    for (const symbol of symbols) {
      const pattern = new RegExp("\\b" + symbol + "\\b", "g");
      let count = 0;
      let observedLines = 0;
      const locators = [];
      for (let index = 0; index < lines.length; index++) {
        const matches = lines[index].match(pattern);
        if (matches) {
          observedLines++;
          count += matches.length;
          if (locators.length < 32) locators.push(index + 1);
        }
      }
      if (count)
        observations.push({
          symbol,
          lexical_occurrences: count,
          first_lines: locators,
          locators_truncated: observedLines > 32,
        });
    }
    if (observations.length) families[family] = observations;
  }
  return families;
}
function analyze(directory) {
  const samples = SAMPLES.map((sample) => {
    const file = path.join(directory, sample.file);
    const stat = fs.lstatSync(file);
    if (!stat.isFile() || stat.size > 1024 * 1024)
      throw new Error("Source must be a bounded regular file");
    const bytes = fs.readFileSync(file);
    if (bytes.length > 1024 * 1024)
      throw new Error("Source grew beyond input bound");
    const text = normalized(
      new TextDecoder("utf-8", { fatal: true }).decode(bytes),
    );
    const digest = crypto.createHash("sha256").update(text).digest("hex");
    if (digest !== sample.digest)
      throw new Error(
        "Pinned normalized source digest mismatch: " + sample.file,
      );
    return {
      upstream_path: sample.upstream,
      url:
        "https://github.com/torvalds/linux/blob/" +
        COMMIT +
        "/" +
        sample.upstream,
      normalized_source_sha256: digest,
      families: scanSymbols(text),
    };
  });
  return {
    schema_version: 1,
    research_date: "2026-10-03",
    linux_version: "v6.12",
    linux_commit: COMMIT,
    observation_kind: "lexical_source_references",
    corpus_scope:
      "Three intentionally selected network/USB-storage/VirtIO-block translation units; convenience sample, not representative coverage.",
    limitations: [
      "No preprocessing, macro expansion, build configuration, transitive closure or runtime execution.",
      "A reference or declaration is not a call, incompatibility, working adapter or driver support.",
      "No observed reference does not prove absence; listed symbols are an incomplete reviewed vocabulary.",
      "Counts are per-source lexical observations; no compatibility percentage or synthetic runtime cost.",
    ],
    samples,
    family_source_reach: Object.fromEntries(
      Object.keys(FAMILIES).map((family) => [
        family,
        samples.filter((sample) => sample.families[family]).length,
      ]),
    ),
  };
}
module.exports = { scanSymbols, analyze, normalized };
if (require.main === module) {
  try {
    if (process.argv.length !== 3)
      throw new Error(
        "usage: node scripts/analyze-linux-driver-apis.cjs SOURCE_DIRECTORY",
      );
    process.stdout.write(
      JSON.stringify(analyze(process.argv[2]), null, 2) + "\n",
    );
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
