const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const { inventory, verify } = require("../cache-integrity.cjs");
const { requireSuccess } = require("../ci-gate.cjs");
const { summarize } = require("../ci-observations.cjs");

test("compiled cache additions, corruption and missing artifacts fail closed", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "kolvrt-cache-"));
  try {
    fs.mkdirSync(path.join(dir, "target/debug"), { recursive: true });
    const file = path.join(dir, "target/debug/example");
    fs.writeFileSync(file, "original");
    const expected = { schema_version: 1, files: inventory(dir) };
    verify(expected, inventory(dir));
    fs.writeFileSync(file, "corruption");
    assert.throws(() => verify(expected, inventory(dir)), /integrity mismatch/);
    fs.unlinkSync(file);
    assert.throws(() => verify(expected, inventory(dir)), /integrity mismatch/);
    fs.writeFileSync(file, "original");
    fs.writeFileSync(path.join(dir, "target/debug/extra"), "unexpected");
    assert.throws(() => verify(expected, inventory(dir)), /integrity mismatch/);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
test("mandatory graph gate rejects failed, cancelled, missing and skipped jobs", () => {
  const jobs = Object.fromEntries(
    [
      "static",
      "host",
      "kernel-dev",
      "kernel-prod",
      "matrix",
      "routing",
      "asid",
      "native",
      "evidence",
    ].map((id) => [id, { result: "success" }]),
  );
  requireSuccess(jobs, true);
  for (const result of ["failure", "cancelled", "skipped", undefined]) {
    for (const id of ["matrix", "native"]) {
      jobs[id] = { result };
      assert.throws(() => requireSuccess(jobs, true), new RegExp(id));
      jobs[id] = { result: "success" };
    }
  }
  assert.throws(
    () => requireSuccess({ baseline: { result: "success" } }),
    /static/,
  );
});
test("nested timing totals stay separate; unknown compiler time is not zero", () => {
  const report = summarize([
    {
      schema_version: 1,
      kind: "matrix",
      wall_seconds: 5,
      outcome: "success",
      observation: "executed",
    },
    {
      schema_version: 1,
      kind: "qemu",
      wall_seconds: 1,
      outcome: "failure",
      observation: "executed",
    },
  ]);
  assert.equal(report.groups.matrix.wall_seconds, 5);
  assert.equal(report.groups.qemu.wall_seconds, 1);
  assert.equal(report.compiler_exclusive_seconds, null);
  assert.throws(
    () =>
      summarize([
        {
          schema_version: 1,
          kind: "qemu",
          wall_seconds: -1,
          observation: "executed",
        },
      ]),
    /Invalid/,
  );
});
test(
  "timed wrapper preserves real child failure and records the failed observation",
  { skip: process.platform !== "win32" },
  () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "kolvrt-timing-"));
    try {
      const quote = (value) => "'" + value.replaceAll("'", "''") + "'";
      const command = `& ${quote(path.resolve(__dirname, "../run-timed.ps1"))} -Kind host -Label rejection -Command ${quote(process.execPath)} -CommandArguments @('-e', 'process.exit(7)'); exit $LASTEXITCODE`;
      const child = spawnSync("pwsh", ["-NoProfile", "-Command", command], {
        env: { ...process.env, CI_TIMING_DIR: dir },
        encoding: "utf8",
      });
      assert.equal(child.status, 7, child.stderr);
      const file = fs.readdirSync(dir).find((name) => name.endsWith(".jsonl"));
      const record = JSON.parse(fs.readFileSync(path.join(dir, file), "utf8"));
      assert.equal(record.outcome, "failure");
      assert.ok(record.wall_seconds > 0);
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  },
);

test("evidence merge rejects missing/duplicate shards, changed sources, features and ELF hashes", () => {
  const { aggregate } = require("../ci-evidence.cjs");
  const expected = {
    schema_version: 1,
    source_files: [],
    qemu_arguments: ["-kernel", "<ELF>"],
    tasks: Array.from({ length: 4 }, (_, i) => ({
      id: `dev-${i}`,
      profile: "dev",
      tests: true,
      feature: null,
    })),
  };
  const receipts = expected.tasks.map((task, i) => ({
    schema_version: 1,
    shard_count: 4,
    shard_index: i,
    plan: expected,
    completed: [
      {
        id: task.id,
        outcome: "passed",
        observation: "executed",
        build: {
          features: ["diagnostics", "kernel-tests", "machine-events"],
          sha256: "a".repeat(64),
          compiler: "1.99.0",
          target: "aarch64-unknown-none",
        },
        run: {
          elf_sha256: "a".repeat(64),
          qemu_version: "QEMU emulator version 10.1.0 test",
          arguments: ["-kernel", "actual.elf"],
        },
      },
    ],
  }));
  assert.equal(aggregate(expected, receipts).completed.length, 4);
  assert.throws(() => aggregate(expected, receipts.slice(1)), /Missing shard/);
  for (const mutate of [
    (r) => (r[1].shard_index = 0),
    (r) => (r[0].plan.source_files = [{ path: "changed" }]),
    (r) => (r[0].completed[0].build.features = []),
    (r) => (r[0].completed[0].run.elf_sha256 = "b".repeat(64)),
    (r) => (r[0].completed = []),
    (r) => (r[0].completed[0].outcome = "failure"),
  ]) {
    const changed = structuredClone(receipts);
    mutate(changed);
    assert.throws(() => aggregate(expected, changed));
  }
});

test("impact scheduling requires exact unchanged kernel inputs and declared host-only ownership", () => {
  const { decision } = require("../ci-impact.cjs");
  const host = {
    id: "host",
    document: "host-doc",
    feature: {
      sources: ["scripts/research.cjs"],
      verification: [
        { environment: "host-process", state: "VERIFIED" },
        { environment: "physical-arm64", state: "NOT_APPLICABLE" },
      ],
    },
  };
  const graph = {
    nodes: {
      host,
      "kolvrt.ci.performance": {
        id: "kolvrt.ci.performance",
        feature: { sources: ["scripts/ci-gate.cjs"] },
      },
    },
    edges: [],
  };
  assert.equal(
    decision(["scripts/research.cjs"], { source_files: [] }, graph, true)
      .kernel_required,
    false,
  );
  assert.equal(
    decision(["scripts/research.cjs"], { source_files: [] }, graph, false)
      .kernel_required,
    true,
  );
  assert.equal(
    decision(["scripts/unknown.cjs"], { source_files: [] }, graph, true)
      .kernel_required,
    true,
  );
  assert.equal(
    decision(["scripts/ci-gate.cjs"], { source_files: [] }, graph, true)
      .kernel_required,
    true,
  );
  graph.edges = [
    { from: "kolvrt.ci.performance", to: "host-doc", type: "depends_on" },
  ];
  assert.equal(
    decision(["scripts/research.cjs"], { source_files: [] }, graph, true)
      .kernel_required,
    true,
  );
});

test("host-only final status requires static/host success and explicit skips for waived kernel jobs", () => {
  const jobs = {
    static: { result: "success" },
    host: { result: "success" },
    ...Object.fromEntries(
      [
        "kernel-dev",
        "kernel-prod",
        "matrix",
        "routing",
        "asid",
        "native",
        "evidence",
      ].map((id) => [id, { result: "skipped" }]),
    ),
  };
  requireSuccess(jobs, false);
  jobs.asid.result = "failure";
  assert.throws(() => requireSuccess(jobs, false), /Unexpected/);
});
