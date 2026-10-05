const fs = require("node:fs");
const path = require("node:path");

/**
 * Convert valid ordered API timestamps to wall seconds; unavailable/reversed data stays null.
 */
function seconds(start, end) {
  const value = (Date.parse(end) - Date.parse(start)) / 1000;
  return Number.isFinite(value) && value >= 0 ? value : null;
}

/**
 * Sum job durations along the longest declared dependency path, excluding runner queues.
 * @param {Object<string, number>} durations Nonnegative duration by job ID.
 * @param {Object<string, string[]>} dependencies Required predecessors by job ID.
 * @returns {number} Longest declared path duration.
 * @throws {Error} Missing durations or dependency cycles.
 */
function criticalPath(durations, dependencies) {
  const memo = new Map(),
    active = new Set();
  /**
   * Memoize one dependency-path duration while detecting cycles in the active traversal.
   */
  function visit(id) {
    if (memo.has(id)) return memo.get(id);
    if (active.has(id)) throw new Error("CI dependency cycle");
    if (!Number.isFinite(durations[id]) || durations[id] < 0)
      throw new Error("Missing CI job duration");
    active.add(id);
    const value =
      durations[id] + Math.max(0, ...(dependencies[id] || []).map(visit));
    active.delete(id);
    memo.set(id, value);
    return value;
  }
  return Math.max(0, ...Object.keys(durations).map(visit));
}

/**
 * Retain completed-run identities, step outcomes and available wall times.
 * Nested Cargo/QEMU attribution cannot be recovered from API timestamps alone.
 */
function snapshot(run, jobs) {
  if (run.status !== "completed") throw new Error("Run must be completed");
  const steps = jobs.flatMap((job) =>
    (job.steps || []).map((step) => ({
      job: job.name,
      runner_labels: job.labels || [],
      name: step.name,
      number: step.number,
      conclusion: step.conclusion,
      wall_seconds:
        step.conclusion === "skipped"
          ? null
          : seconds(step.started_at, step.completed_at),
    })),
  );
  const starts = jobs.map((job) => Date.parse(job.started_at));
  const ends = jobs.map((job) => Date.parse(job.completed_at));
  const valid = jobs.length && [...starts, ...ends].every(Number.isFinite);
  return {
    run_id: run.id,
    run_attempt: run.run_attempt,
    head_sha: run.head_sha,
    workflow_id: run.workflow_id,
    event: run.event,
    conclusion: run.conclusion,
    execution_span_seconds: valid
      ? (Math.max(...ends) - Math.min(...starts)) / 1000
      : null,
    steps,
    cargo_compilation_seconds: null,
    qemu_execution_seconds: null,
    check_inventory_id: null, // API job names do not attest the source-bound task inventory.
    cache_outcomes:
      "See per-workload job.json artifacts; UNKNOWN in API timestamps",
    declared_dag_seconds: dagDuration(jobs),
  };
}

/**
 * Interpret only the known serial/sharded foundation graphs; unfamiliar shapes stay null.
 */
function dagDuration(jobs) {
  const completed = jobs.filter((job) => job.conclusion !== "skipped");
  const durations = {};
  for (const job of completed) {
    const id = job.name
      .split(" / ")[0]
      .replace(/^matrix \((\d+)\)$/, "matrix-$1");
    const value = seconds(job.started_at, job.completed_at);
    if (value === null || id in durations) return null;
    durations[id] = value;
  }
  if (Object.keys(durations).length === 1 && "foundation" in durations)
    return durations.foundation;
  const shards = Object.keys(durations).filter((id) => /^matrix-\d+$/.test(id));
  const parallel = [
    "host",
    "kernel-dev",
    "kernel-prod",
    "routing",
    "asid",
    ...(shards.length ? shards : ["matrix"]),
  ];
  if (
    !("static" in durations) ||
    !("foundation" in durations) ||
    parallel.some((id) => !(id in durations))
  )
    return null;
  const dependencies = Object.fromEntries(
    parallel.map((id) => [id, ["static"]]),
  );
  dependencies.foundation = ["static", ...parallel];
  if ("evidence" in durations) {
    dependencies.evidence = shards.length ? shards : ["matrix"];
    dependencies.foundation.push("evidence");
  }
  return criticalPath(durations, dependencies);
}

/**
 * Describe the exact workflow/event and ordered job-step/runner graph for repeat comparison.
 */
function signature(run) {
  return JSON.stringify([
    run.workflow_id,
    run.event,
    run.steps.map((step) => [step.job, step.name, step.runner_labels]),
  ]);
}

/**
 * Compare against at least three distinct successful compatible baseline runs.
 * Mapped graph comparison is descriptive and explicitly scoped; it is not paired speed inference.
 * @param {object} current Completed current-run snapshot.
 * @param {object[]} baselines Retained baseline snapshots.
 * @param {boolean} sameSource Require identical source SHA for repeat runs.
 * @param {boolean} mappedInventory Allow the authored full-inventory graph mapping.
 * @returns {object} Comparison state, baseline identities and median/delta when available.
 */
function compare(
  current,
  baselines,
  sameSource = false,
  mappedInventory = false,
) {
  const eligible = baselines.filter(
    (run) =>
      run.run_id !== current.run_id &&
      run.conclusion === "success" &&
      run.execution_span_seconds !== null &&
      ((signature(run) === signature(current) &&
        (run.head_sha === current.head_sha ||
          (run.check_inventory_id !== null &&
            run.check_inventory_id !== undefined &&
            run.check_inventory_id === current.check_inventory_id))) ||
        (mappedInventory &&
          run.workflow_id === current.workflow_id &&
          ["push", "pull_request", "workflow_dispatch"].includes(run.event) &&
          ["push", "pull_request", "workflow_dispatch"].includes(
            current.event,
          ) &&
          run.check_inventory_id === "kernel-130-v1" &&
          current.check_inventory_id === "kernel-130-v1")) &&
      (!sameSource || run.head_sha === current.head_sha),
  );
  const unique = [
    ...new Map(eligible.map((run) => [run.run_id, run])).values(),
  ];
  if (unique.length < 3)
    return { state: "INSUFFICIENT_BASELINE", runs: unique.length };
  const values = unique
    .map((run) => run.execution_span_seconds)
    .sort((a, b) => a - b);
  const middle = Math.floor(values.length / 2);
  const median =
    values.length % 2
      ? values[middle]
      : (values[middle - 1] + values[middle]) / 2;
  return {
    state: current.conclusion === "success" ? "COMPARABLE" : "CURRENT_FAILED",
    baseline_run_ids: unique.map((run) => run.run_id),
    comparison_mapping: mappedInventory
      ? "Authored kernel-130-v1 inventory mapping; descriptive wall comparison, not paired inference"
      : "same job/step graph",
    baseline_head_shas: [...new Set(unique.map((run) => run.head_sha))],
    baseline_median_seconds: median,
    delta_seconds:
      current.execution_span_seconds === null
        ? null
        : current.execution_span_seconds - median,
  };
}

/**
 * Render the top five wall-time steps and attribution limits, escaping API-provided labels.
 */
function markdown(report) {
  const escape = (text) => String(text).replace(/[|\r\n`<>]/g, " ");
  const rows = [...report.current.steps]
    .filter((step) => step.wall_seconds !== null)
    .sort((a, b) => b.wall_seconds - a.wall_seconds)
    .slice(0, 5);
  return [
    "# CI timing observations",
    "",
    `Run: ${report.current.run_id}; source: ${escape(report.current.head_sha)}; conclusion: ${escape(report.current.conclusion)}.`,
    `Execution span including inter-job waits: ${report.current.execution_span_seconds ?? "UNKNOWN"} seconds.`,
    `Declared dependency path (job durations, excludes runner queues): ${report.current.declared_dag_seconds ?? "UNKNOWN"} seconds.`,
    `Comparison: ${escape(report.comparison.state)}.`,
    `Baseline median: ${report.comparison.baseline_median_seconds ?? "UNKNOWN"}; delta: ${report.comparison.delta_seconds ?? "UNKNOWN"} seconds.`,
    "",
    "| Job | Step | Wall seconds | Result |",
    "| --- | --- | --- | --- |",
    ...rows.map(
      (step) =>
        `| ${escape(step.job)} | ${escape(step.name)} | ${step.wall_seconds} | ${escape(step.conclusion)} |`,
    ),
    "",
    "All steps are retained in JSON. Step time includes nested builds and runs. Cargo compilation, QEMU-only execution, cache hit/miss and runner queue time remain UNKNOWN; execution span includes waits; declared DAG duration uses known Kernel foundation dependencies and excludes runner queues. Matching labels do not prove equal runner load. No kernel acceptance is inferred.",
    "",
  ].join("\n");
}

/**
 * Fetch completed-run steps and successful same-SHA candidates from GitHub Actions.
 * @param {string} repository Validated OWNER/REPO identity.
 * @param {string|number} runId Numeric run ID.
 * @param {string} token Read-only Actions token, never included in reports.
 * @param {Function} request Injectable fetch-compatible transport.
 * @returns {Promise<object>} Retained current/baseline observations and comparison.
 * @throws {Error} Invalid identities, incomplete runs or failed API requests.
 */
async function collect(repository, runId, token, request = fetch) {
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository))
    throw new Error("Invalid repository");
  if (!/^\d+$/.test(String(runId))) throw new Error("Invalid run ID");
  /**
   * Fetch one validated-repository API endpoint; HTTP failures remain visible and reject collection.
   */
  async function get(endpoint) {
    const response = await request(
      `https://api.github.com/repos/${repository}/${endpoint}`,
      {
        headers: {
          Authorization: `Bearer ${token}`,
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
        },
      },
    );
    if (!response.ok)
      throw new Error(`GitHub timing API failed: ${response.status}`);
    return response.json();
  }
  /**
   * Fetch every page of jobs for the recorded run attempt before producing a snapshot.
   */
  async function load(run) {
    const jobs = [];
    for (let page = 1; ; page++) {
      const result = await get(
        `actions/runs/${run.id}/attempts/${run.run_attempt}/jobs?per_page=100&page=${page}`,
      );
      jobs.push(...result.jobs);
      if (result.jobs.length < 100) break;
    }
    return snapshot(run, jobs);
  }
  const run = await get(`actions/runs/${runId}`);
  const current = await load(run);
  const recent = await get(
    `actions/workflows/${run.workflow_id}/runs?head_sha=${run.head_sha}&status=success&per_page=20`,
  );
  const baselines = [];
  for (const candidate of recent.workflow_runs) {
    if (candidate.id !== run.id && candidate.event === run.event)
      baselines.push(await load(candidate));
  }
  return {
    schema_version: 1,
    current,
    baselines,
    comparison: compare(current, baselines, true),
  };
}

/**
 * Collect reports and optionally compare the retained baseline; write JSON/Markdown and the check summary.
 */
async function main() {
  const [repository, runId, destination, baselineFile] = process.argv.slice(2);
  if (!repository || !runId || !destination || !process.env.GITHUB_TOKEN)
    throw new Error(
      "Usage: ci-timings.cjs OWNER/REPO RUN_ID OUTPUT_DIR [BASELINE_JSON]; GITHUB_TOKEN required",
    );
  const report = await collect(repository, runId, process.env.GITHUB_TOKEN);
  if (baselineFile) {
    const baseline = JSON.parse(fs.readFileSync(baselineFile, "utf8"));
    report.comparison = compare(
      report.current,
      baseline.baselines,
      false,
      true,
    );
  }
  fs.mkdirSync(destination, { recursive: true });
  fs.writeFileSync(
    path.join(destination, "timings.json"),
    JSON.stringify(report, null, 2) + "\n",
  );
  fs.writeFileSync(path.join(destination, "timings.md"), markdown(report));
  if (process.env.GITHUB_STEP_SUMMARY)
    fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, markdown(report));
}

module.exports = { snapshot, compare, markdown, collect, criticalPath };
if (require.main === module)
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
