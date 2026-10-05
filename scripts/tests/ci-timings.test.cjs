const test = require("node:test");
const assert = require("node:assert/strict");
const { snapshot, compare, markdown, collect } = require("../ci-timings.cjs");

function fixture(id = 4, sha = "abc") {
  return snapshot(
    {
      id,
      head_sha: sha,
      workflow_id: 1,
      run_attempt: 1,
      event: "push",
      status: "completed",
      conclusion: "success",
    },
    [
      {
        name: "kernel",
        labels: ["windows-2022"],
        started_at: "2026-10-05T10:00:00Z",
        completed_at: "2026-10-05T10:02:00Z",
        steps: [
          {
            name: "matrix",
            number: 1,
            conclusion: "success",
            started_at: "2026-10-05T10:00:00Z",
            completed_at: "2026-10-05T10:01:00Z",
          },
          { name: "skipped", number: 2, conclusion: "skipped" },
        ],
      },
    ],
  );
}

test("retains step timings and unknown nested observations", () => {
  const current = fixture();
  assert.equal(current.execution_span_seconds, 120);
  assert.equal(current.steps[0].wall_seconds, 60);
  assert.equal(current.steps[1].wall_seconds, null);
  assert.equal(current.qemu_execution_seconds, null);
  assert.equal(current.cargo_compilation_seconds, null);
});

test("three unique compatible successful runs required; failed and changed runs excluded", () => {
  const current = fixture();
  const baselines = [fixture(1), fixture(2), fixture(3)];
  assert.equal(compare(current, baselines, true).state, "COMPARABLE");
  assert.equal(
    compare(current, [baselines[0], baselines[0], baselines[1]]).state,
    "INSUFFICIENT_BASELINE",
  );
  baselines[0].conclusion = "failure";
  baselines[1].steps[0].runner_labels = ["ubuntu-24.04"];
  baselines[2].head_sha = "changed";
  assert.equal(compare(current, baselines, true).runs, 0);
});

test("explicit before/after comparison admits changed source and retains identities", () => {
  const current = fixture(4, "after");
  const comparison = compare(current, [fixture(1), fixture(2), fixture(3)]);
  assert.deepEqual(comparison.baseline_head_shas, ["abc"]);
  assert.equal(comparison.delta_seconds, 0);
  current.conclusion = "failure";
  assert.equal(
    compare(current, [fixture(1), fixture(2), fixture(3)]).state,
    "CURRENT_FAILED",
  );
});

test("summary escapes API-provided names", () => {
  const current = fixture();
  current.steps[0].name = "evil|\n<script>";
  const result = markdown({ current, comparison: compare(current, []) });
  assert.ok(!result.includes("<script>"));
  assert.ok(result.includes("INSUFFICIENT_BASELINE"));
});

test("API error fails collection without emitting credentials", async () => {
  await assert.rejects(
    collect("owner/repo", 42, "secret", async () => ({
      ok: false,
      status: 403,
    })),
    /403/,
  );
  await assert.rejects(collect("invalid", 42, "secret"), /Invalid repository/);
});

test("critical path uses the longest parallel branch and rejects cycles or missing jobs", () => {
  const { criticalPath } = require("../ci-timings.cjs");
  assert.equal(
    criticalPath(
      { static: 2, host: 3, matrix: 10, gate: 1 },
      { host: ["static"], matrix: ["static"], gate: ["host", "matrix"] },
    ),
    13,
  );
  assert.throws(
    () => criticalPath({ a: 1, b: 2 }, { a: ["b"], b: ["a"] }),
    /cycle/,
  );
  assert.throws(() => criticalPath({ a: 1 }, { a: ["missing"] }), /Missing/);
});

test("matching shard topology cannot certify historical workload equivalence", () => {
  const job = (name, steps = []) => ({
    name,
    labels: ["windows-2022"],
    conclusion: "success",
    started_at: "2026-10-05T10:00:00Z",
    completed_at: "2026-10-05T10:01:00Z",
    steps: steps.map((name) => ({ name, conclusion: "success" })),
  });
  const run = (id) => ({
    id,
    name: "Kernel foundation",
    head_sha: `source-${id}`,
    workflow_id: 1,
    run_attempt: 1,
    event: "push",
    status: "completed",
    conclusion: "success",
  });
  const oldSteps = [
    "Run npm run check",
    "Routing model contracts",
    "Production model and absent switching",
    "Architecture lint",
    "Production architecture lint",
    "Real kernel matrix and host failure propagation",
    "Real EL0 routing matrix and failure propagation",
    "Real 16-bit ASID baseline comparison",
  ];
  const baselines = [1, 2, 3].map((id) =>
    snapshot(run(id), [job("foundation", oldSteps)]),
  );
  const current = snapshot(
    run(4),
    [
      "static",
      "host",
      "kernel-dev",
      "kernel-prod",
      "routing",
      "asid",
      "evidence",
      "foundation",
      "matrix (0)",
      "matrix (1)",
      "matrix (2)",
      "matrix (3)",
    ].map((name) => job(`${name} / run`, ["Execute required workload"])),
  );
  assert.equal(current.check_inventory_id, null);
  assert.equal(
    compare(current, baselines, false, true).state,
    "INSUFFICIENT_BASELINE",
  );
});
