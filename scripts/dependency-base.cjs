// Current trusted PR checker evaluates historical manifests; historical audit scripts are not executed.
const fs = require("node:fs");
const path = require("node:path");
const cp = require("node:child_process");
const crypto = require("node:crypto");
const { graph, boundary } = require("./dependency-audit.cjs");
const [root, output] = process.argv.slice(2);
if (!root || !output) throw new Error("usage: dependency-base.cjs ROOT OUTPUT");
function run(command, args) {
  const r = cp.spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  if (r.error || r.status !== 0)
    throw new Error(`${command}: ${r.error || r.stderr}`);
  return r.stdout.trim();
}
const metadata = (all) =>
  JSON.parse(
    run(process.env.CARGO || "cargo", [
      "metadata",
      "--locked",
      "--format-version",
      "1",
      ...(all ? ["--all-features"] : []),
    ]),
  );
const defaults = metadata(false);
const all = metadata(true);
fs.mkdirSync(output, { recursive: true });
fs.writeFileSync(
  path.join(output, "inventory.json"),
  JSON.stringify(
    {
      schema_version: 1,
      commit: run("git", ["rev-parse", "HEAD"]),
      cargo_lock_sha256: crypto
        .createHash("sha256")
        .update(fs.readFileSync(path.join(root, "Cargo.lock")))
        .digest("hex"),
      cargo_version: run(process.env.CARGO || "cargo", ["--version"]),
      source_files: [
        "Cargo.toml",
        "Cargo.lock",
        ...all.packages
          .filter((p) => all.workspace_members.includes(p.id))
          .map((p) => path.relative(root, p.manifest_path).replace(/\\/g, "/")),
      ]
        .sort()
        .map((file) => ({
          path: file,
          sha256_lf: crypto
            .createHash("sha256")
            .update(
              fs
                .readFileSync(path.join(root, file), "utf8")
                .replace(/\r\n/g, "\n"),
            )
            .digest("hex"),
        })),
      scope:
        "Historical base evaluated using current graph schema; no historical retention/supply-chain claim",
      boundary: boundary(all),
      graphs: { default: graph(defaults), all_features: graph(all) },
    },
    null,
    2,
  ) + "\n",
);
