const fs = require("node:fs");
const crypto = require("node:crypto");
const { spawnSync } = require("node:child_process");
const derived = new Set([
  "README.md",
  "translations/ru/README.md",
  "docs/index.md",
  "translations/ru/docs/index.md",
  "docs/catalog.json",
  "docs/knowledge-graph.json",
  "translations/manifest.json",
  "research/results/documentation-knowledge-pilot.json",
  "docs/implementation-impact.json",
]);
function isHostOnly(node) {
  const environments =
    node.feature?.verification?.map((v) => v.environment) || [];
  return (
    environments.some(
      (value) => value.startsWith("host") || value === "windows-cim",
    ) &&
    !environments.some(
      (value) =>
        value.startsWith("qemu") ||
        (value === "physical-arm64" &&
          node.feature.verification.find((v) => v.environment === value)
            .state !== "NOT_APPLICABLE"),
    )
  );
}
function decision(changed, plan, graph, baseSourcesMatch) {
  if (!baseSourcesMatch)
    return {
      kernel_required: true,
      reason:
        "Kernel/build/runner exact-source inventory changed or the reviewed base is unavailable.",
    };
  const nodes = Object.values(graph.nodes);
  const ci = graph.nodes["kolvrt.ci.performance"];
  const critical = new Set([
    ...(ci?.feature?.sources || []),
    ...(ci?.feature?.acceptance || []),
  ]);
  for (const file of changed) {
    if (critical.has(file))
      return {
        kernel_required: true,
        reason: `CI verification input changed: ${file}`,
      };
    if (derived.has(file)) continue;
    const owners = nodes.filter(
      (node) =>
        node.feature?.sources?.includes(file) ||
        Object.values(node.locations || {}).some(
          (location) => location.path === file,
        ),
    );
    if (!owners.length || owners.some((node) => !isHostOnly(node)))
      return {
        kernel_required: true,
        reason: `Kernel authority or unclassified impact requires the full matrix: ${file}`,
      };
    const impacted = new Set(
      owners.flatMap((node) => [node.id, node.document]),
    );
    let progress = true;
    while (progress) {
      progress = false;
      for (const edge of graph.edges) {
        if (
          edge.type === "depends_on" &&
          impacted.has(edge.to) &&
          !impacted.has(edge.from)
        ) {
          impacted.add(edge.from);
          progress = true;
        }
      }
    }
    if (
      nodes.some(
        (node) =>
          impacted.has(node.id) &&
          (node.id === "kolvrt.ci.performance" ||
            node.id === "doc.kolvrt.ci.performance" ||
            (node.feature && !isHostOnly(node))),
      )
    )
      return {
        kernel_required: true,
        reason: `Declared prerequisite impact reaches kernel/CI authority: ${file}`,
      };
  }
  return {
    kernel_required: false,
    reason:
      "Every kernel/build/runner input matches the reviewed base; changed inputs are derived navigation or declared host-only units with no kernel prerequisite impact.",
    source_inventory: plan.source_files,
  };
}
function main() {
  const [base, planFile, destination] = process.argv.slice(2);
  const plan = JSON.parse(fs.readFileSync(planFile, "utf8"));
  const graph = JSON.parse(
    fs.readFileSync("docs/knowledge-graph.json", "utf8"),
  );
  let baseGraphAvailable = false;
  // Both accepted base and current ownership/dependencies govern scheduling.
  if (/^[a-f0-9]{40}$/.test(base || "")) {
    const prior = spawnSync(
      "git",
      ["show", `${base}:docs/knowledge-graph.json`],
      { encoding: "utf8", maxBuffer: 8 * 1024 * 1024 },
    );
    if (prior.status === 0) {
      const old = JSON.parse(prior.stdout);
      baseGraphAvailable = true;
      for (const [id, node] of Object.entries(old.nodes))
        graph.nodes[`base:${id}`] = node;
      graph.edges.push(...old.edges);
    }
  }
  let changed = [],
    match =
      baseGraphAvailable &&
      /^[a-f0-9]{40}$/.test(base || "") &&
      !/^0+$/.test(base);
  if (match) {
    const diff = spawnSync("git", ["diff", "--name-only", base, "HEAD"], {
      encoding: "utf8",
    });
    match = diff.status === 0;
    changed = diff.stdout.trim().split(/\r?\n/).filter(Boolean);
    for (const source of plan.source_files) {
      const original = spawnSync("git", ["show", `${base}:${source.path}`]);
      if (original.status !== 0) {
        match = false;
        break;
      }
      const bytes = source.sha256_lf
        ? original.stdout.toString("utf8").replaceAll("\r\n", "\n")
        : original.stdout;
      if (
        crypto.createHash("sha256").update(bytes).digest("hex") !==
        (source.sha256_lf || source.sha256)
      ) {
        match = false;
        break;
      }
    }
  }
  const result = decision(changed, plan, graph, match);
  fs.writeFileSync(
    destination,
    JSON.stringify(
      { schema_version: 1, reviewed_base: base || null, ...result },
      null,
      2,
    ) + "\n",
  );
  if (process.env.GITHUB_OUTPUT)
    fs.appendFileSync(
      process.env.GITHUB_OUTPUT,
      `kernel_required=${result.kernel_required}\n`,
    );
  console.log(JSON.stringify(result));
}
module.exports = { decision };
if (require.main === module) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
