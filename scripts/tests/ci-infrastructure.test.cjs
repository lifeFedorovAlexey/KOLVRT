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
      expected_marker: null,
      expected_ipc_test: null,
      flag: null,
      observable_checks: null,
    })),
  };
  const receipts = expected.tasks.map((task, i) => ({
    schema_version: 2,
    shard_count: 4,
    shard_index: i,
    plan: expected,
    completed: [
      {
        id: task.id,
        events: [{ event: "suite", status: "pass" }],
        outcome: "passed",
        observation: "executed",
        build: {
          artifact: "target/kernel/dev-tests.elf",
          features: ["diagnostics", "kernel-tests", "machine-events"],
          sha256: "a".repeat(64),
          compiler: "1.99.0",
          target: "aarch64-unknown-none",
        },
        run: {
          elf_sha256: "a".repeat(64),
          qemu_version: "QEMU emulator version 10.1.0 test",
          arguments: ["-kernel", "target/kernel/dev-tests.elf"],
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

function reuseMatrixFixture(total = 8, count = 2) {
  const expected = {
    schema_version: 1,
    source_files: [{ path: "source", sha256_lf: "c".repeat(64) }],
    qemu_arguments: ["-kernel", "<ELF>"],
    tasks: Array.from({ length: total }, (_, i) => ({
      id: "task-" + i,
      profile: i % 2 ? "prod" : "dev",
      tests: true,
      feature: null,
      expected_marker: null,
      expected_ipc_test: null,
      flag: i < count ? null : "--guard-control",
      observable_checks: i < count ? null : ["guard"],
    })),
  };
  const receipts = Array.from({ length: count }, (_, shard_index) => ({
    schema_version: 2,
    shard_count: count,
    shard_index,
    plan: expected,
    completed: [],
  }));
  expected.tasks.forEach((task, i) => {
    const donor = receipts[i % count].completed[0];
    const result = donor
      ? {
          ...structuredClone(donor),
          id: task.id,
          observation: "reused",
          reused_from: donor.id,
        }
      : {
          id: task.id,
          outcome: "passed",
          observation: "executed",
          events: [
            { event: "test", name: "guard", status: "pass" },
            { event: "suite", status: "pass" },
          ],
          build: {
            artifact: `target/kernel/${task.profile}-tests.elf`,
            features: [
              "machine-events",
              "kernel-tests",
              ...(task.profile === "dev" ? ["diagnostics"] : []),
            ],
            sha256: "a".repeat(64),
            compiler: "1.99.0",
            target: "aarch64-unknown-none",
          },
          run: {
            elf_sha256: "a".repeat(64),
            qemu_version: "QEMU emulator version 10.1.0 test",
            arguments: ["-kernel", `target/kernel/${task.profile}-tests.elf`],
          },
        };
    receipts[i % count].completed.push(result);
  });
  return { expected, receipts, count };
}

test("matrix schema2 preserves every obligation and reports executed versus reused", () => {
  const { aggregate } = require("../ci-evidence.cjs");
  const { expected, receipts } = reuseMatrixFixture(144, 4);
  const result = aggregate(expected, receipts, 4);
  assert.equal(result.schema_version, 2);
  assert.equal(result.completed.length, 144);
  assert.equal(result.executed, 4);
  assert.equal(result.reused, 140);
  assert.deepEqual(
    result.completed.map((x) => x.id),
    expected.tasks.map((x) => x.id),
  );
});

const invalidReuseCases = {
  "unknown reuse policy": (f) => {
    f.receipts[0].reuse_policy = "historical";
  },
  "disabled reuse policy": (f) => {
    f.receipts[0].reuse_policy = "disabled";
  },
  "marker consumer": (f) => {
    f.expected.tasks[2].expected_marker = "failure";
  },
  "IPC failure consumer": (f) => {
    f.expected.tasks[2].expected_ipc_test = "failure";
  },
  "wrong executed kernel path": (f) => {
    f.receipts[0].completed[0].run.arguments[1] = "foreign.elf";
  },
  "wrong matching build and run path": (f) => {
    for (const r of f.receipts[0].completed)
      r.build.artifact = r.run.arguments[1] = "target/kernel/prod-tests.elf";
  },
  "old shard schema": (f) => {
    f.receipts[0].schema_version = 1;
  },
  "missing donor": (f) => {
    f.receipts[0].completed[1].reused_from = "absent";
  },
  "historical donor": (f) => {
    f.receipts[0].completed[1].reused_from = "historical-task";
  },
  "cross-shard donor": (f) => {
    f.receipts[0].completed[1].reused_from = "task-1";
  },
  "future donor": (f) => {
    f.receipts[0].completed[1].reused_from = "task-4";
  },
  "chained donor": (f) => {
    f.receipts[0].completed[2].reused_from = "task-2";
  },
  "self donor": (f) => {
    f.receipts[0].completed[1].reused_from = "task-2";
  },
  "cross-profile": (f) => {
    f.expected.tasks[2].profile = "prod";
    f.receipts[0].completed[1].build.features = [
      "machine-events",
      "kernel-tests",
    ];
  },
  "changed build": (f) => {
    f.receipts[0].completed[1].build.artifact = "different.elf";
  },
  "changed ELF pair": (f) => {
    f.receipts[0].completed[1].build.sha256 =
      f.receipts[0].completed[1].run.elf_sha256 = "b".repeat(64);
  },
  "changed run": (f) => {
    f.receipts[0].completed[1].run.arguments[1] = "different.elf";
  },
  "changed events": (f) => {
    f.receipts[0].completed[1].events.push({ event: "detail", value: 1 });
  },
  "missing events": (f) => {
    delete f.receipts[0].completed[0].events;
  },
  "missing consumer witness": (f) => {
    f.expected.tasks[2].observable_checks = ["absent"];
  },
  "duplicate consumer witness": (f) => {
    for (const r of f.receipts[0].completed)
      r.events.push({ event: "test", name: "guard", status: "pass" });
  },
  "failed witness": (f) => {
    for (const r of f.receipts[0].completed) r.events[0].status = "fail";
  },
  "nonpassing witness": (f) => {
    for (const r of f.receipts[0].completed) r.events[0].status = "skipped";
  },
  "failed donor": (f) => {
    f.receipts[0].completed[0].outcome = "failed";
  },
  "unrelated failed event": (f) => {
    f.receipts[0].completed[0].events.push({ event: "panic", status: "fail" });
  },
  "duplicate suite": (f) => {
    f.receipts[0].completed[0].events.push({ event: "suite", status: "pass" });
  },
  "missing suite": (f) => {
    f.receipts[0].completed[0].events.pop();
  },
  "failed suite": (f) => {
    f.receipts[0].completed[0].events[1].status = "fail";
  },
  "executed names donor": (f) => {
    f.receipts[0].completed[0].reused_from = "task-1";
  },
  "feature-specific consumer": (f) => {
    f.expected.tasks[2].feature = "fatal-input";
    f.receipts[0].completed[1].build.features.push("fatal-input");
  },
  "fatal consumer without witnesses": (f) => {
    f.expected.tasks[2].observable_checks = null;
  },
  "boot consumer": (f) => {
    f.expected.tasks[2].tests = false;
    f.receipts[0].completed[1].build.features = [
      "machine-events",
      "diagnostics",
    ];
  },
  "feature-specific donor": (f) => {
    f.expected.tasks[0].feature = "fatal-input";
    f.receipts[0].completed[0].build.features.push("fatal-input");
  },
  "donor with no positive contract": (f) => {
    f.expected.tasks[0].flag = "--fatal";
  },
  "reordered future executed donor": (f) => {
    const r = f.receipts[0].completed;
    r[1].observation = "executed";
    delete r[1].reused_from;
    r[0].observation = "reused";
    r[0].reused_from = r[1].id;
    [r[0], r[1]] = [r[1], r[0]];
  },
  "missing obligation": (f) => {
    f.receipts[0].completed.pop();
  },
};
for (const [name, mutate] of Object.entries(invalidReuseCases)) {
  test("matrix reuse rejects " + name, () => {
    const { aggregate } = require("../ci-evidence.cjs");
    const fixture = reuseMatrixFixture();
    mutate(fixture);
    assert.throws(() =>
      aggregate(fixture.expected, fixture.receipts, fixture.count),
    );
  });
}

test("schema2 allows executed feature-specific controls without treating them as donors", () => {
  const { aggregate } = require("../ci-evidence.cjs");
  const f = reuseMatrixFixture(2, 2);
  f.expected.tasks[0].feature = "fatal-input";
  f.expected.tasks[0].flag = "--fatal";
  f.receipts[0].completed[0].build.features.push("fatal-input");
  f.receipts[0].completed[0].build.artifact =
    f.receipts[0].completed[0].run.arguments[1] =
      "target/kernel/dev-fatal-input.elf";
  f.receipts[0].completed[0].events = [{ event: "panic", status: "fail" }];
  const result = aggregate(f.expected, f.receipts, f.count);
  assert.equal(result.executed, 2);
  assert.equal(result.reused, 0);
});
