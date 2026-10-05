/* Dependency graph evidence and architectural boundary checks; no external JS packages. */
const fs = require("node:fs");
const path = require("node:path");
const cp = require("node:child_process");
const crypto = require("node:crypto");

const KERNEL = ["kolvrt-kernel", "kernel-core"];
const read = (file) =>
  JSON.parse(fs.readFileSync(file, "utf8").replace(/^\uFEFF/, ""));
const sorted = (items) => [...new Set(items)].sort();
const digest = (text) => crypto.createHash("sha256").update(text).digest("hex");
function run(command, args, root) {
  const result = cp.spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error || result.status !== 0)
    throw new Error(
      `${command} ${args.join(" ")} failed: ${result.error || result.stderr}`,
    );
  return result.stdout.trim();
}
function metadata(root, allFeatures) {
  return JSON.parse(
    run(
      process.env.CARGO || "cargo",
      [
        "metadata",
        "--locked",
        "--format-version",
        "1",
        ...(allFeatures ? ["--all-features"] : []),
      ],
      root,
    ),
  );
}
function boundary(m) {
  const errors = [];
  const packages = m.packages.filter((p) => m.workspace_members.includes(p.id));
  for (const name of KERNEL) {
    const p = packages.find((p) => p.name === name);
    if (!p) {
      errors.push(`missing TCB member ${name}`);
      continue;
    }
    const expected = name === "kolvrt-kernel" ? ["kernel-core"] : [];
    const normal = p.dependencies.filter((d) => !d.kind);
    if (
      JSON.stringify(sorted(normal.map((d) => d.name))) !==
      JSON.stringify(expected)
    )
      errors.push(
        `${name}: expected normal boundary ${expected.join(",") || "empty"}`,
      );
    for (const d of p.dependencies) {
      // Declaration checks cover disabled features and every target, independently of resolver unification.
      const target = d.target || "all targets";
      const kind = d.kind || "normal";
      const internal = packages.find(
        (candidate) =>
          candidate.name === d.name &&
          d.path &&
          path.resolve(path.dirname(candidate.manifest_path)) ===
            path.resolve(d.path),
      );
      if (
        kind !== "normal" ||
        !internal ||
        !expected.includes(d.name) ||
        d.source
      )
        errors.push(
          `${name}: forbidden ${kind} dependency ${d.name} (${target}, optional=${d.optional})`,
        );
    }
  }
  if (errors.length) throw new Error(errors.join("\n"));
  return {
    members: KERNEL,
    normal_external_crates: 0,
    build_external_crates: 0,
    dev_external_crates: 0,
    coverage:
      "all declared dependency kinds, optional flags and target predicates; no exceptions",
  };
}
function graph(m) {
  const members = new Set(m.workspace_members);
  const packages = new Map(m.packages.map((p) => [p.id, p]));
  const key = (id) => {
    const p = packages.get(id);
    if (!p) throw new Error(`unresolved package ${id}`);
    return `${p.name}@${p.version}#${p.source || (members.has(id) ? "workspace" : "external-path")}`;
  };
  const nodes = new Map(m.resolve.nodes.map((n) => [n.id, n]));
  function closure(id) {
    const seen = new Set();
    const pending = (nodes.get(id)?.deps || []).map((d) => d.pkg);
    while (pending.length) {
      const next = pending.pop();
      if (next === id || seen.has(next)) continue;
      seen.add(next);
      for (const d of nodes.get(next)?.deps || []) pending.push(d.pkg);
    }
    return [...seen];
  }
  const external = (ids) =>
    sorted(ids.filter((id) => !members.has(id)).map(key));
  const packageInventory = [...nodes.values()]
    .map((n) => {
      const p = packages.get(n.id);
      return {
        id: key(n.id),
        crate: p.name,
        version: p.version,
        source: p.source || (members.has(n.id) ? "workspace" : "external-path"),
        license: p.license,
        workspace: members.has(n.id),
        features: sorted(n.features),
        dependencies: n.deps
          .map((d) => ({ id: key(d.pkg), alias: d.name, kinds: d.dep_kinds }))
          .sort((a, b) => a.id.localeCompare(b.id)),
      };
    })
    .sort((a, b) => a.id.localeCompare(b.id));
  const crates = m.workspace_members
    .map((id) => {
      const p = packages.get(id);
      const direct = sorted((nodes.get(id)?.deps || []).map((d) => key(d.pkg)));
      const reachable = closure(id);
      return {
        crate: p.name,
        id: key(id),
        features: sorted(nodes.get(id)?.features || []),
        direct_dependencies: direct,
        transitive_dependencies: sorted(reachable.map(key)),
        direct_count: direct.length,
        transitive_count: reachable.length,
        external_dependencies: external(reachable),
        external_count: external(reachable).length,
      };
    })
    .sort((a, b) => a.crate.localeCompare(b.crate));
  const direct = [];
  for (const id of m.workspace_members) {
    const consumer = packages.get(id);
    for (const d of consumer.dependencies) {
      const matches = (nodes.get(id)?.deps || []).filter(
        (edge) =>
          edge.name === (d.rename || d.name).replace(/-/g, "_") &&
          edge.dep_kinds.some(
            (kind) => kind.kind === d.kind && kind.target === d.target,
          ),
      );
      for (const edge of matches) {
        const p = packages.get(edge.pkg);
        if (members.has(p.id)) continue;
        direct.push({
          consumer: consumer.name,
          crate: p.name,
          version: p.version,
          requirement: d.req,
          kind: d.kind || "normal",
          target: d.target,
          optional: d.optional,
          default_features: d.uses_default_features,
          requested_features: d.features,
          resolved_features: nodes.get(p.id).features,
          source: p.source || "external-path",
          license: p.license,
          direct: true,
          reachability: KERNEL.includes(consumer.name)
            ? "kernel-reachable"
            : consumer.name === "routing" || consumer.name === "routing-demo"
              ? "host-tool/EL0-demo; outside production EL1"
              : "host-only",
          transitive_dependencies: external(closure(p.id)),
          transitive_count: external(closure(p.id)).length,
        });
      }
    }
  }
  direct.sort((a, b) =>
    `${a.consumer}/${a.crate}/${a.kind}/${a.target}`.localeCompare(
      `${b.consumer}/${b.crate}/${b.kind}/${b.target}`,
    ),
  );
  const versions = {};
  for (const p of packageInventory.filter((p) => !p.workspace))
    (versions[p.crate] ||= []).push(p.version);
  return {
    packages: packageInventory,
    crates,
    direct_external_dependencies: direct,
    metrics: {
      kernel_external_crates: 0,
      kernel_tcb_external_code: {
        third_party_cargo_crates: 0,
        compiler_runtime:
          "trusted Rust core/compiler-builtins; not counted as external Cargo crates or measured bytes",
      },
      workspace_direct_external_crates: sorted(direct.map((d) => d.crate))
        .length,
      workspace_external_package_versions: packageInventory.filter(
        (p) => !p.workspace,
      ).length,
      workspace_transitive_external_crates: sorted(
        packageInventory.filter((p) => !p.workspace).map((p) => p.crate),
      ).length,
      duplicated_crate_versions: Object.fromEntries(
        Object.entries(versions)
          .filter(([, v]) => sorted(v).length > 1)
          .map(([k, v]) => [k, sorted(v)]),
      ),
    },
  };
}
function retention(g, policy) {
  const used = new Set();
  for (const d of g.direct_external_dependencies) {
    const id = `${d.consumer}/${d.crate}/${d.kind}`;
    const entry = policy.direct_dependencies[id];
    if (!entry?.purpose?.trim() || !entry?.reason_for_retention?.trim())
      throw new Error(`missing dependency retention decision: ${id}`);
    if (entry.requirement !== d.requirement)
      throw new Error(
        `unreviewed dependency requirement: ${id}: ${entry.requirement} -> ${d.requirement}`,
      );
    Object.assign(d, {
      purpose: entry.purpose,
      reason_for_retention: entry.reason_for_retention,
    });
    used.add(id);
  }
  const stale = Object.keys(policy.direct_dependencies).filter(
    (id) => !used.has(id),
  );
  if (stale.length)
    throw new Error(`stale retention decisions: ${stale.join(", ")}`);
}
function diff(before, after) {
  if (before.schema_version !== 1 || after.schema_version !== 1)
    throw new Error("incompatible dependency evidence schema");
  const lines = [
    "# Dependency graph diff",
    "",
    `Before: ${before.commit}`,
    `After: ${after.commit}`,
    "",
    "Counts describe package versions, exclude the root, and are not optimization targets.",
    "",
  ];
  let changes = 0;
  for (const mode of ["default", "all_features"]) {
    const oldGraph = before.graphs[mode];
    const newGraph = after.graphs[mode];
    const oldIds = oldGraph.packages.map((p) => p.id);
    const newIds = newGraph.packages.map((p) => p.id);
    const added = newIds.filter((id) => !oldIds.includes(id));
    const removed = oldIds.filter((id) => !newIds.includes(id));
    lines.push(
      `## ${mode}`,
      "",
      `External package versions: ${oldGraph.metrics.workspace_external_package_versions} → ${newGraph.metrics.workspace_external_package_versions}`,
      "",
    );
    for (const id of added) lines.push(`- Added: ${id}`);
    for (const id of removed) lines.push(`- Removed: ${id}`);
    changes += added.length + removed.length;
    const rows = (g) => [
      ...g.crates.map((c) => ({ id: c.crate, count: c.transitive_count })),
      ...g.direct_external_dependencies.map((d) => ({
        id: `${d.consumer}/${d.crate}/${d.kind}/${d.target || "all"}`,
        count: d.transitive_count,
      })),
    ];
    const oldRows = new Map(rows(oldGraph).map((r) => [r.id, r.count]));
    for (const row of rows(newGraph)) {
      const count = oldRows.get(row.id);
      if (count !== row.count) {
        lines.push(
          `- ${row.id}: ${count ?? "absent"} → ${row.count} transitive crates${row.count > (count ?? 0) ? " — REVIEW GROWTH" : ""}`,
        );
        changes++;
      }
    }
    const oldPackages = new Map(oldGraph.packages.map((p) => [p.id, p]));
    for (const p of newGraph.packages) {
      const old = oldPackages.get(p.id);
      if (old && JSON.stringify(old) !== JSON.stringify(p)) {
        lines.push(`- Changed features/edges/license: ${p.id}`);
        changes++;
      }
    }
    lines.push("");
  }
  if (!changes) lines.push("No graph changes.");
  return lines.join("\n") + "\n";
}
function supplySummary(records, status) {
  const stats = records.find((r) => r.type === "summary")?.fields;
  // Fail closed if cargo-deny changed format or failed before completing every check.
  if (
    !stats ||
    !["advisories", "bans", "licenses", "sources"].every((k) =>
      Number.isInteger(stats[k]?.errors),
    )
  )
    return {
      status: "incomplete",
      exit_code: status,
      advisory_count: null,
      license_violations: null,
      source_violations: null,
      diagnostics: records,
    };
  const ids = sorted(
    JSON.stringify(records).match(/RUSTSEC-\d{4}-\d{4}/g) || [],
  );
  return {
    status:
      status === 0 && Object.values(stats).every((s) => s.errors === 0)
        ? "passed"
        : "failed",
    exit_code: status,
    advisory_ids: ids,
    advisory_count: stats.advisories.errors > ids.length ? null : ids.length,
    license_violations: stats.licenses.errors,
    source_violations: stats.sources.errors,
    check_statistics: stats,
    diagnostics: records,
  };
}
function supply(root, output) {
  const result = cp.spawnSync(
    process.env.CARGO || "cargo",
    [
      "deny",
      "--locked",
      "--all-features",
      "--format",
      "json",
      "check",
      "--show-stats",
    ],
    { cwd: root, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  );
  fs.writeFileSync(
    path.join(output, "cargo-deny.jsonl"),
    (result.stdout || "") + (result.stderr || ""),
  );
  if (result.error || result.status === null)
    throw new Error(`cargo-deny unavailable: ${result.error}`);
  const records = ((result.stdout || "") + (result.stderr || ""))
    .split("\n")
    .filter((s) => s.trim())
    .map((s) => {
      try {
        return JSON.parse(s);
      } catch {
        return { unparsed: s };
      }
    });
  const summary = supplySummary(records, result.status);
  summary.tool_version = run(
    process.env.CARGO || "cargo",
    ["deny", "--version"],
    root,
  );
  const dbRoot = path.join(
    process.env.CARGO_HOME || path.join(require("node:os").homedir(), ".cargo"),
    "advisory-dbs",
  );
  summary.advisory_database_commits = fs.existsSync(dbRoot)
    ? fs
        .readdirSync(dbRoot)
        .filter((name) => fs.existsSync(path.join(dbRoot, name, ".git")))
        .map((name) => ({
          database: name,
          commit: run(
            "git",
            ["-C", path.join(dbRoot, name), "rev-parse", "HEAD"],
            root,
          ),
        }))
    : [];
  return summary;
}
function audit(root, output, withSupply) {
  fs.mkdirSync(output, { recursive: true });
  const policy = read(path.join(root, "dependencies/policy.json"));
  const defaults = metadata(root, false);
  const all = metadata(root, true);
  const boundaryResult = boundary(all);
  const graphs = { default: graph(defaults), all_features: graph(all) };
  retention(graphs.all_features, policy);
  for (const d of graphs.default.direct_external_dependencies) {
    const decision =
      policy.direct_dependencies[`${d.consumer}/${d.crate}/${d.kind}`];
    Object.assign(d, {
      purpose: decision.purpose,
      reason_for_retention: decision.reason_for_retention,
    });
  }
  const evidence = {
    schema_version: 1,
    commit: run("git", ["rev-parse", "HEAD"], root),
    tracked_changes: run(
      "git",
      ["status", "--porcelain", "--untracked-files=no"],
      root,
    )
      .split("\n")
      .filter(Boolean),
    cargo_lock_sha256: digest(fs.readFileSync(path.join(root, "Cargo.lock"))),
    policy_sha256: digest(
      fs.readFileSync(path.join(root, "dependencies/policy.json")),
    ),
    deny_config_sha256: digest(fs.readFileSync(path.join(root, "deny.toml"))),
    source_files: sorted([
      "Cargo.toml",
      "Cargo.lock",
      "rust-toolchain.toml",
      ".cargo/config.toml",
      "dependencies/policy.json",
      "deny.toml",
      "scripts/dependency-audit.cjs",
      ...all.packages
        .filter((p) => all.workspace_members.includes(p.id))
        .map((p) => path.relative(root, p.manifest_path).replace(/\\/g, "/")),
    ]).map((file) => ({
      path: file,
      sha256_lf: digest(
        fs.readFileSync(path.join(root, file), "utf8").replace(/\r\n/g, "\n"),
      ),
    })),
    cargo_version: run(process.env.CARGO || "cargo", ["--version"], root),
    scope:
      "Cargo metadata, all target predicates, workspace-unified default and all-workspace-features graphs, all dependency kinds; not per-consumer isolated feature counts",
    boundary: boundaryResult,
    graphs,
    supply_chain: withSupply
      ? supply(root, output)
      : {
          status: "not-run",
          advisory_count: null,
          license_violations: null,
          source_violations: null,
        },
  };
  fs.writeFileSync(
    path.join(output, "inventory.json"),
    JSON.stringify(evidence, null, 2) + "\n",
  );
  const baseline = path.join(root, "dependencies/baseline-main.json");
  const metrics = graphs.all_features.metrics;
  const report = [
    "# Dependency evidence",
    "",
    `Commit: ${evidence.commit}`,
    `Tracked changes: ${evidence.tracked_changes.length} (see source digests in inventory.json)`,
    "",
    `Kernel external Cargo crates: ${metrics.kernel_external_crates}`,
    `Workspace direct external crate names: ${metrics.workspace_direct_external_crates}`,
    `Workspace external package versions: ${metrics.workspace_external_package_versions}`,
    `Workspace transitive external crate names: ${metrics.workspace_transitive_external_crates}`,
    `Duplicate versions: ${JSON.stringify(metrics.duplicated_crate_versions)}`,
    `Supply chain: ${evidence.supply_chain.status}; advisories: ${evidence.supply_chain.advisory_count ?? "not measured"}; license violations: ${evidence.supply_chain.license_violations ?? "not measured"}; source violations: ${evidence.supply_chain.source_violations ?? "not measured"}`,
    "",
    "| Consumer | Direct dependency | Transitive external package versions |",
    "| --- | --- | --- |",
    ...graphs.all_features.direct_external_dependencies.map(
      (d) =>
        `| ${d.consumer} (${d.kind}) | ${d.crate} ${d.version} | ${d.transitive_count} |`,
    ),
    "",
    "Workspace-unified all-target counts exclude roots. Compiler runtime remains trusted; these counts do not measure compiled TCB bytes. No count quota.",
  ];
  fs.writeFileSync(path.join(output, "report.md"), report.join("\n") + "\n");
  if (fs.existsSync(baseline))
    fs.writeFileSync(
      path.join(output, "diff.md"),
      diff(read(baseline), evidence),
    );
  console.log(
    JSON.stringify(
      {
        boundary: boundaryResult,
        metrics: graphs.all_features.metrics,
        supply_chain: {
          status: evidence.supply_chain.status,
          advisory_count: evidence.supply_chain.advisory_count,
          license_violations: evidence.supply_chain.license_violations,
          source_violations: evidence.supply_chain.source_violations,
        },
      },
      null,
      2,
    ),
  );
  if (withSupply && evidence.supply_chain.status !== "passed")
    throw new Error(
      "supply-chain checks failed; see inventory.json and cargo-deny.jsonl",
    );
  return evidence;
}
if (require.main === module) {
  try {
    const [command, ...args] = process.argv.slice(2);
    if (command === "diff" && args.length === 3)
      fs.writeFileSync(args[2], diff(read(args[0]), read(args[1])));
    else if (
      command === "check" &&
      args.length <= 2 &&
      args.every((a) => a === "--supply-chain" || !a.startsWith("--"))
    )
      audit(
        process.cwd(),
        path.resolve(
          args.find((a) => a !== "--supply-chain") || "target/dependencies",
        ),
        args.includes("--supply-chain"),
      );
    else
      throw new Error(
        "usage: node scripts/dependency-audit.cjs check [OUTPUT] [--supply-chain] | diff BEFORE AFTER OUTPUT",
      );
  } catch (e) {
    console.error(e.message);
    process.exitCode = 1;
  }
}
module.exports = { boundary, graph, retention, diff, audit, supplySummary };
