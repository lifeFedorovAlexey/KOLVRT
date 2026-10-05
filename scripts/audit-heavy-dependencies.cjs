// Source review evidence complements cargo-deny; token counts are not an unsafe safety proof.
const fs = require("node:fs");
const path = require("node:path");
const cp = require("node:child_process");
const crypto = require("node:crypto");
const r = cp.spawnSync(
  process.env.CARGO || "cargo",
  ["metadata", "--locked", "--format-version", "1", "--all-features"],
  { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
);
if (r.error || r.status !== 0) throw new Error(r.error || r.stderr);
const m = JSON.parse(r.stdout);
const inventory = JSON.parse(
  fs.readFileSync("target/dependencies/inventory.json", "utf8"),
);
const baseline = JSON.parse(
  fs.readFileSync("dependencies/baseline-main.json", "utf8"),
);
const decisions = {
  jsonschema: {
    decision: "KEEP",
    reason:
      "Draft 2020-12 reference resolution and schema validation semantics are not a safe small custom implementation. Default HTTP/file retrieval features were already disabled on main; no reqwest in the resolved graph. Keep validating repository/security schema contracts.",
    removal:
      "Requires replacement validator proving current positive/negative schema fixtures and Draft 2020-12 behavior; do not replace based on count.",
    maintenance:
      "Pinned 0.33.0; no assertion of latest version. Run RustSec on every PR and weekly. Upgrade needs schema compatibility tests.",
  },
  "ed25519-dalek": {
    decision: "KEEP",
    reason:
      "Strict Ed25519 verification authenticates provenance; fixture signing requires rand_core. Retain zeroize and fast precomputed tables: removing them needs secret-lifetime and timing evidence, not a crate count argument. No custom cryptography.",
    removal:
      "Requires interoperable signature vectors, strict rejection semantics, key/entropy/zeroization review and provenance rejection tests.",
    maintenance:
      "Pinned 2.2.0; current RustSec snapshot has no matching advisory. Historical vulnerability review is below, not proof of absence.",
  },
  quanta: {
    decision: "REDUCE FEATURES",
    reason:
      "Only Clock::new/now/delta are used for calibrated host timing. mock and flaky_tests APIs are not used by KOLVRT and are now disabled with default-features=false. Dependency closure remains 28: no artificial count reduction claim.",
    removal:
      "Switching to std::time::Instant requires a new measurement mechanism/profile and overhead/resolution comparison; existing timing evidence is bound to quanta semantics.",
    maintenance:
      "Pinned 0.13.0; keep production clock implementation and test actual workbench behavior without default features.",
  },
};
const audits = [];
for (const [name, decision] of Object.entries(decisions)) {
  const p = m.packages.find((p) => p.name === name);
  const dir = path.dirname(p.manifest_path);
  const files = [];
  function walk(d) {
    for (const entry of fs.readdirSync(d, { withFileTypes: true })) {
      const full = path.join(d, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (entry.name.endsWith(".rs")) files.push(full);
    }
  }
  walk(path.join(dir, "src"));
  const source = files.sort().map((file) => {
    const bytes = fs.readFileSync(file);
    return {
      path: path.relative(dir, file).replace(/\\/g, "/"),
      bytes: bytes.length,
      sha256: crypto.createHash("sha256").update(bytes).digest("hex"),
      unsafe_token_occurrences: (
        bytes.toString("utf8").match(/\bunsafe\b/g) || []
      ).length,
    };
  });
  const direct =
    inventory.graphs.all_features.direct_external_dependencies.filter(
      (d) => d.crate === name,
    );
  const old = baseline.graphs.all_features.direct_external_dependencies.filter(
    (d) => d.crate === name,
  );
  const history =
    name === "ed25519-dalek"
      ? [
          {
            id: "RUSTSEC-2022-0093",
            patched: ">=2",
            pinned_version: "2.2.0",
            source: "https://rustsec.org/advisories/RUSTSEC-2022-0093.html",
            review:
              "Historical signing oracle vulnerability; pinned v2 API and SigningKey avoid decoupled keypair input. hazmat feature remains disabled.",
          },
          {
            id: "RUSTSEC-2024-0344",
            crate: "curve25519-dalek",
            pinned_version: "4.1.3",
            source: "https://rustsec.org/advisories/RUSTSEC-2024-0344.html",
            review:
              "Transitive scalar timing history; applicability checked by retained RustSec snapshot.",
          },
        ]
      : [];
  audits.push({
    crate: name,
    version: p.version,
    license: p.license,
    source: p.source,
    repository: p.repository,
    rust_version: p.rust_version,
    ...decision,
    features_before: old[0].resolved_features,
    features_after: direct[0].resolved_features,
    transitive_count_before: old[0].transitive_count,
    transitive_count_after: direct[0].transitive_count,
    consumers: direct.map((d) => d.consumer),
    reachability: "outside production kernel EL1/TCB; host-only",
    source_files: source,
    source_bytes: source.reduce((sum, f) => sum + f.bytes, 0),
    unsafe_token_occurrences: source.reduce(
      (sum, f) => sum + f.unsafe_token_occurrences,
      0,
    ),
    unsafe_review:
      "Lexical occurrences include comments/tests/cfg-disabled code; not executed unsafe count or compiled size. Review includes transitives in machine inventory; zero-token root does not mean zero unsafe in closure.",
    security_history: history,
    security_snapshot: {
      status: inventory.supply_chain.status,
      advisory_count: inventory.supply_chain.advisory_count,
      advisory_database_commits:
        inventory.supply_chain.advisory_database_commits,
    },
    tcb_impact:
      "No third-party code in production kernel; dependency declarations remain confined to host consumers.",
  });
}
fs.writeFileSync(
  "dependencies/heavy-audit.json",
  JSON.stringify(
    {
      schema_version: 1,
      source_commit: baseline.commit,
      cargo_lock_sha256: baseline.cargo_lock_sha256,
      audit_date: "2026-10-05",
      scope:
        "Pinned source review with measured workspace-unified all-target closure; changed quanta features evaluated in implementation worktree",
      audits,
    },
    null,
    2,
  ) + "\n",
);
