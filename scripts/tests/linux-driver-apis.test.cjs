"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const {
  scanSymbols,
  analyze,
  normalized,
} = require("../analyze-linux-driver-apis.cjs");

test("comments, strings and longer identifiers cannot manufacture API observations", () => {
  const source = [
    "/* kmalloc GFP_ATOMIC */",
    'const char *s = "dma_map_page";',
    "kmalloc(1, GFP_KERNEL); // request_irq",
    "my_kmalloc_wrapper();",
    "spin_lock_irqsave(lock, flags);",
  ].join("\n");
  const result = scanSymbols(source);
  assert.equal(result.allocation[0].symbol, "kmalloc");
  assert.deepEqual(result.allocation[0].first_lines, [3]);
  assert.equal(result.allocation[0].lexical_occurrences, 1);
  assert.equal(result.dma, undefined);
  assert.equal(result.irq, undefined);
  assert.equal(result.locking[0].symbol, "spin_lock_irqsave");
  assert.equal(normalized("\uFEFFa\r\nb"), "a\nb");
});
test("conditional source remains a lexical observation with bounded locators", () => {
  const result = scanSymbols(
    "#if NEVER\n" + "kmalloc(1,GFP_KERNEL);\n".repeat(40) + "#endif",
  );
  assert.equal(result.allocation[0].lexical_occurrences, 40);
  assert.equal(result.allocation[0].first_lines.length, 32);
  assert.equal(result.allocation[0].locators_truncated, true);
});
test("substituted or oversized sources fail before claiming pinned provenance", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "kolvrt-api-map-"));
  try {
    fs.writeFileSync(path.join(dir, "igb_main.c"), "kmalloc(1,GFP_KERNEL);");
    assert.throws(() => analyze(dir), /digest mismatch/);
    fs.writeFileSync(path.join(dir, "igb_main.c"), " ".repeat(1024 * 1024 + 1));
    assert.throws(() => analyze(dir), /bounded regular file/);
    const result = spawnSync(process.execPath, [
      path.join(__dirname, "..", "analyze-linux-driver-apis.cjs"),
      dir,
    ]);
    assert.notEqual(result.status, 0);
    assert.equal(result.stdout.length, 0);
  } finally {
    assert.equal(path.resolve(path.dirname(dir)), path.resolve(os.tmpdir()));
    assert.ok(path.basename(dir).startsWith("kolvrt-api-map-"));
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
