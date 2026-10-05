const { test } = require("node:test");
const assert = require("node:assert/strict");
const {
  boundary,
  graph,
  retention,
  diff,
  supplySummary,
} = require("../dependency-audit.cjs");
function fixture() {
  const core = {
    id: "core",
    name: "kernel-core",
    version: "0.1.0",
    manifest_path: "/repo/crates/core/Cargo.toml",
    source: null,
    dependencies: [],
  };
  const kernel = {
    id: "kernel",
    name: "kolvrt-kernel",
    version: "0.1.0",
    manifest_path: "/repo/crates/kernel/Cargo.toml",
    source: null,
    dependencies: [
      {
        name: "kernel-core",
        kind: null,
        path: "/repo/crates/core",
        source: null,
      },
    ],
  };
  return {
    packages: [core, kernel],
    workspace_members: ["core", "kernel"],
    resolve: {
      nodes: [
        { id: "core", features: [], deps: [] },
        {
          id: "kernel",
          features: [],
          deps: [
            {
              pkg: "core",
              name: "kernel_core",
              dep_kinds: [{ kind: null, target: null }],
            },
          ],
        },
      ],
    },
  };
}
test("TCB declarations reject optional, target-only, renamed, build/dev and disguised path dependencies", () => {
  assert.equal(boundary(fixture()).normal_external_crates, 0);
  for (const d of [
    { name: "serde", optional: true, target: "cfg(unix)" },
    { name: "sha2", kind: "build" },
    { name: "serde", kind: "dev" },
    { name: "kernel-core", path: "/attacker/core", source: null },
    { name: "kernel-core", source: "registry+evil" },
    { name: "host-tool", rename: "kernel-core" },
  ]) {
    const m = fixture();
    m.packages[1].dependencies.push(d);
    assert.throws(() => boundary(m), /forbidden|expected/);
  }
  const m = fixture();
  m.packages[0].dependencies.push({
    name: "host-tool",
    kind: null,
    path: "/repo/crates/host-tool",
  });
  assert.throws(() => boundary(m));
});
test("TCB root cannot be silently removed or normal edge omitted", () => {
  const m = fixture();
  m.workspace_members.pop();
  assert.throws(() => boundary(m), /missing TCB/);
  const n = fixture();
  n.packages[1].dependencies = [];
  assert.throws(() => boundary(n), /expected normal/);
});
test("graph closure deduplicates diamonds and cycles, includes target-specific build edges and source identity", () => {
  const m = fixture();
  for (const name of ["host", "one", "two"])
    m.packages.push({
      id: name,
      name,
      version: "1.0.0",
      source: name === "host" ? null : "registry+test",
      license: "MIT",
      dependencies: [],
    });
  m.workspace_members.push("host");
  m.resolve.nodes.push(
    {
      id: "host",
      features: ["enabled"],
      deps: [
        {
          name: "one",
          pkg: "one",
          dep_kinds: [{ kind: "build", target: "cfg(unix)" }],
        },
        { name: "two", pkg: "two", dep_kinds: [{ kind: null, target: null }] },
      ],
    },
    {
      id: "one",
      features: [],
      deps: [{ name: "two", pkg: "two", dep_kinds: [] }],
    },
    {
      id: "two",
      features: [],
      deps: [{ name: "one", pkg: "one", dep_kinds: [] }],
    },
  );
  const g = graph(m);
  const host = g.crates.find((c) => c.crate === "host");
  assert.equal(host.transitive_count, 2);
  assert.equal(host.external_count, 2);
  assert.equal(g.metrics.workspace_external_package_versions, 2);
  assert.equal(
    g.packages.find((p) => p.crate === "host").dependencies[0].kinds[0].kind,
    "build",
  );
});
test("unreviewed direct dependency and requirement changes fail; no crate-count quota", () => {
  const g = {
    direct_external_dependencies: [
      {
        consumer: "host",
        crate: "crypto",
        kind: "normal",
        requirement: "=2.0",
      },
    ],
  };
  assert.throws(
    () => retention(g, { direct_dependencies: {} }),
    /missing dependency/,
  );
  const policy = {
    direct_dependencies: {
      "host/crypto/normal": {
        purpose: "verify signatures",
        reason_for_retention: "reviewed cryptography",
        requirement: "=1.0",
      },
    },
  };
  assert.throws(() => retention(g, policy), /unreviewed/);
  policy.direct_dependencies["host/crypto/normal"].requirement = "=2.0";
  retention(g, policy);
});
test("diff flags equal-count replacements, growth, features and removed packages", () => {
  const g = {
    packages: [{ id: "one", features: [], dependencies: [] }],
    crates: [{ crate: "host", transitive_count: 1 }],
    direct_external_dependencies: [],
    metrics: { workspace_external_package_versions: 1 },
  };
  const before = {
    schema_version: 1,
    commit: "a",
    graphs: { default: g, all_features: g },
  };
  const after = JSON.parse(JSON.stringify(before));
  after.commit = "b";
  after.graphs.all_features.packages[0].id = "two";
  after.graphs.default.packages[0].features = ["new"];
  after.graphs.all_features.crates[0].transitive_count = 2;
  const text = diff(before, after);
  assert.match(text, /Added: two/);
  assert.match(text, /Removed: one/);
  assert.match(text, /1 → 2 transitive crates — REVIEW GROWTH/);
  assert.match(text, /Changed features/);
});
test("supply-chain metrics use completed checker statistics and never turn missing data into zero", () => {
  const stats = {
    advisories: { errors: 1 },
    bans: { errors: 0 },
    licenses: { errors: 2 },
    sources: { errors: 3 },
  };
  const r = supplySummary(
    [
      { type: "summary", fields: stats },
      {
        type: "diagnostic",
        fields: { message: "RUSTSEC-2025-0001 RUSTSEC-2025-0001" },
      },
    ],
    13,
  );
  assert.equal(r.advisory_count, 1);
  assert.equal(r.license_violations, 2);
  assert.equal(r.source_violations, 3);
  assert.equal(r.status, "failed");
  const missing = supplySummary([], 0);
  assert.equal(missing.status, "incomplete");
  assert.equal(missing.advisory_count, null);
});
test("real Cargo metadata exposes optional and target-specific forbidden dependencies before resolution", () => {
  const fs = require("node:fs");
  const os = require("node:os");
  const path = require("node:path");
  const cp = require("node:child_process");
  const root = fs.mkdtempSync(
    path.join(os.tmpdir(), "kolvrt-dependency-boundary-"),
  );
  try {
    for (const name of ["kernel", "core"]) {
      fs.mkdirSync(path.join(root, name));
      fs.writeFileSync(path.join(root, name, "lib.rs"), "");
    }
    fs.writeFileSync(
      path.join(root, "Cargo.toml"),
      '[workspace]\nmembers=["kernel","core"]\nresolver="3"\n',
    );
    fs.writeFileSync(
      path.join(root, "core/Cargo.toml"),
      '[package]\nname="kernel-core"\nversion="0.1.0"\nedition="2024"\n[lib]\npath="lib.rs"\n',
    );
    const kernel =
      '[package]\nname="kolvrt-kernel"\nversion="0.1.0"\nedition="2024"\n[lib]\npath="lib.rs"\n[dependencies]\nkernel-core={path="../core"}\n';
    for (const mutation of [
      "",
      "[target.'cfg(unix)'.dependencies]\nserde={version=\"1\",optional=true}\n",
      '[build-dependencies]\nserde="1"\n',
      '[dev-dependencies]\nserde="1"\n',
    ]) {
      fs.writeFileSync(path.join(root, "kernel/Cargo.toml"), kernel + mutation);
      const r = cp.spawnSync(
        process.env.CARGO || "cargo",
        ["metadata", "--format-version", "1", "--no-deps", "--all-features"],
        { cwd: root, encoding: "utf8" },
      );
      assert.equal(r.status, 0, r.stderr);
      const m = JSON.parse(r.stdout);
      if (mutation) assert.throws(() => boundary(m), /forbidden/);
      else boundary(m);
    }
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
