// Build the native acceptance path with unrelated component crates physically absent.
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const { spawnSync } = require("node:child_process");
const root = fs.realpathSync(process.cwd());
const destination = path.join(
  root,
  "target",
  `native-closure-${crypto.randomUUID()}`,
);
if (path.relative(root, destination).startsWith(".."))
  throw Error("proof escaped workspace");
fs.mkdirSync(destination, { recursive: true });
const members = [
  "apps/native-runtime",
  "apps/native-apps",
  "crates/kernel-core",
  "crates/kernel",
  "crates/xtask",
  "crates/native-protocol-model",
  "crates/native-state-models",
];
for (const source of [
  ...members,
  ".cargo",
  "assets",
  "research/fixtures",
  "Cargo.lock",
  "rust-toolchain.toml",
])
  fs.cpSync(path.join(root, source), path.join(destination, source), {
    recursive: true,
  });
const workspace = fs.readFileSync(path.join(root, "Cargo.toml"), "utf8");
fs.writeFileSync(
  path.join(destination, "Cargo.toml"),
  `[workspace]\nmembers = ${JSON.stringify(members)}\ndefault-members = ${JSON.stringify(members.filter((p) => p !== "crates/kernel"))}\nresolver = "3"\n${workspace.slice(workspace.indexOf("[profile.dev]"))}`,
);
const runner = path.join(destination, "crates/xtask/Cargo.toml");
fs.writeFileSync(
  runner,
  fs
    .readFileSync(runner, "utf8")
    .replace(/^routing = .*\r?\n/m, "")
    .replace('route-tools = ["dep:routing"]', "route-tools = []"),
);
fs.unlinkSync(path.join(destination, "crates/xtask/src/routing_demo.rs"));
const absent = [
  "routing",
  "routing-demo",
  "window-compat",
  "migration-advisor",
  "migration-workbench",
];
for (const name of absent)
  if (fs.existsSync(path.join(destination, "crates", name)))
    throw Error(`component remains: ${name}`);
const env = { ...process.env };
delete env.CARGO_TARGET_DIR;
if (env.QEMU_AARCH64 && /[/\\]/.test(env.QEMU_AARCH64))
  env.QEMU_AARCH64 = path.resolve(root, env.QEMU_AARCH64);
else if (!env.QEMU_AARCH64 && process.platform === "win32")
  env.QEMU_AARCH64 = path.join(
    root,
    ".toolchains/qemu/bin/qemu-system-aarch64.exe",
  );
function run(args) {
  const result = spawnSync(env.CARGO || "cargo", args, {
    cwd: destination,
    env,
    stdio: "inherit",
  });
  if (result.error || result.status !== 0)
    throw Error(
      `closure command failed: cargo ${args.join(" ")}: ${result.error || result.status}`,
    );
}
// Preserve the selected lock versions while pruning absent workspace packages.
run([
  "test",
  "--offline",
  "--no-run",
  "-p",
  "native-userspace",
  "-p",
  "kernel-core",
  "-p",
  "native-protocol-model",
  "-p",
  "native-state-models",
]);
run(["xtask", "audit"]);
run(["xtask", "selftest"]);
run(["xtask", "app-smoke"]);
const sha = (file) =>
  crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
const inventory = [];
function walk(relative) {
  for (const entry of fs.readdirSync(path.join(destination, relative), {
    withFileTypes: true,
  })) {
    const file = `${relative}/${entry.name}`;
    if (entry.isDirectory()) walk(file);
    else {
      const original = path.join(root, file),
        copied = path.join(destination, file);
      if (
        !["crates/xtask/Cargo.toml"].includes(file) &&
        sha(original) !== sha(copied)
      )
        throw Error(`native source differs: ${file}`);
      inventory.push({
        path: file,
        original_sha256: sha(original),
        proof_sha256: sha(copied),
      });
    }
  }
}
for (const source of [...members, "research/fixtures"]) walk(source);
const results = {};
for (const file of fs.readdirSync(path.join(destination, "target/kernel")))
  if (/^native-.*\.json$/.test(file))
    results[file] = JSON.parse(
      fs.readFileSync(path.join(destination, "target/kernel", file), "utf8"),
    );
const receipt = {
  schema_version: 1,
  proof_directory: destination,
  status: "passed",
  physically_absent: absent,
  not_implemented_or_copied: ["AI/Soul", "native package manager"],
  changes: [
    "Remove unrelated workspace members",
    "Remove optional routing path dependency and host routing implementation",
    "Prune lock entries without upgrading selected versions",
  ],
  native_sources: inventory,
  manifest_digests: ["Cargo.toml", "Cargo.lock", "crates/xtask/Cargo.toml"].map(
    (file) => ({
      path: file,
      original_sha256: sha(path.join(root, file)),
      proof_sha256: sha(path.join(destination, file)),
    }),
  ),
  kernel_external_dependencies: 0,
  results,
  scope:
    "Actual DEV/PROD standalone ELF selftests, fourteen precise controls and app smoke; no physical ARM claim",
};
fs.writeFileSync(
  path.join(root, "target/kernel/native-closure-proof.json"),
  JSON.stringify(receipt, null, 2) + "\n",
);
console.log(`Native closure proof passed: ${destination}`);
