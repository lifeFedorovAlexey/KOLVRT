const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const roots = [
  "target/debug",
  "target/aarch64-unknown-none",
  "target/route-tools/debug",
  "target/ci-tools/cargo-deny/bin",
];

function inventory(workspace) {
  const entries = {};
  function walk(relative) {
    const absolute = path.resolve(workspace, relative);
    const boundary = path.resolve(workspace) + path.sep;
    if (!absolute.startsWith(boundary))
      throw new Error("Cache path outside workspace");
    if (!fs.existsSync(absolute)) return;
    const stat = fs.lstatSync(absolute);
    if (stat.isSymbolicLink()) throw new Error("Cache symlink rejected");
    if (stat.isDirectory()) {
      for (const name of fs.readdirSync(absolute).sort())
        walk(`${relative}/${name}`);
    } else if (stat.isFile()) {
      entries[relative] = crypto
        .createHash("sha256")
        .update(fs.readFileSync(absolute))
        .digest("hex");
    } else throw new Error("Unsupported cache entry");
  }
  for (const root of roots) walk(root);
  return entries;
}

function verify(expected, actual) {
  if (
    expected.schema_version !== 1 ||
    !expected.files ||
    typeof expected.files !== "object"
  )
    throw new Error("Invalid cache inventory");
  const names = Object.keys(expected.files).sort();
  const actualNames = Object.keys(actual).sort();
  if (
    JSON.stringify(names) !== JSON.stringify(actualNames) ||
    names.some((name) => expected.files[name] !== actual[name])
  )
    throw new Error(
      "Compiled cache integrity mismatch; rebuild using a fresh cache key",
    );
}

function main() {
  const [mode, manifest] = process.argv.slice(2);
  if (!["record", "verify"].includes(mode) || !manifest)
    throw new Error("Usage: cache-integrity.cjs record|verify MANIFEST");
  const files = inventory(process.cwd());
  if (mode === "verify")
    verify(JSON.parse(fs.readFileSync(manifest, "utf8")), files);
  else
    fs.writeFileSync(
      manifest,
      JSON.stringify({ schema_version: 1, files }, null, 2) + "\n",
    );
}

module.exports = { inventory, verify };
if (require.main === module) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
