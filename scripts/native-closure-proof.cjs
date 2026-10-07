const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const cp = require("node:child_process");
const root = fs.realpathSync(path.join(__dirname, ".."));
const directory = path.join(
  root,
  "target",
  "native-reference-" + crypto.randomUUID(),
);
const members = [
  "crates/kernel",
  "crates/kernel-core",
  "apps/native-runtime",
  "apps/native-apps",
  "tests/native-apps",
];
function run(command, args, cwd) {
  const result = cp.spawnSync(command, args, {
    cwd,
    stdio: "inherit",
    env: process.env,
  });
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error(command + " failed: " + args.join(" "));
}
function files(directory, result = []) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) files(file, result);
    else result.push(file);
  }
  return result;
}
const before = members
  .flatMap((member) => files(path.join(root, member)))
  .map((file) => ({
    file,
    digest: crypto
      .createHash("sha256")
      .update(fs.readFileSync(file))
      .digest("hex"),
  }));
// Build the ordinary host runner once. It is orchestration, outside kernel/app closure.
run("cargo", ["build", "--locked", "-p", "xtask"], root);
const runner = path.join(
  process.env.CARGO_TARGET_DIR || path.join(root, "target"),
  "debug",
  process.platform === "win32" ? "xtask.exe" : "xtask",
);
fs.mkdirSync(directory, { recursive: true });
const references = [];
for (const relative of [...members, ".cargo", "assets", "research/fixtures"]) {
  const original = path.join(root, relative);
  const reference = path.join(directory, relative);
  fs.mkdirSync(path.dirname(reference), { recursive: true });
  fs.symlinkSync(
    original,
    reference,
    process.platform === "win32" ? "junction" : "dir",
  );
  if (fs.realpathSync(reference) !== fs.realpathSync(original))
    throw new Error("source reference changed identity");
  references.push({
    path: relative,
    actual_source: fs.realpathSync(reference),
    source_copied: false,
  });
}
// Only build metadata is written here; all implementation files are original files.
for (const file of ["Cargo.lock", "rust-toolchain.toml"])
  fs.copyFileSync(path.join(root, file), path.join(directory, file));
const workspace = fs.readFileSync(path.join(root, "Cargo.toml"), "utf8");
fs.writeFileSync(
  path.join(directory, "Cargo.toml"),
  "[workspace]\nmembers=" +
    JSON.stringify(members) +
    '\nresolver="3"\n' +
    workspace.slice(workspace.indexOf("[profile.dev]")),
);
const absent = [
  "routing",
  "routing-demo",
  "window-compat",
  "migration-advisor",
  "migration-workbench",
];
for (const name of absent)
  if (fs.existsSync(path.join(directory, "crates", name)))
    throw new Error("unrelated component present: " + name);
const metadata = cp.spawnSync(
  "cargo",
  ["metadata", "--offline", "--format-version", "1"],
  { cwd: directory, encoding: "utf8", env: process.env },
);
if (metadata.status !== 0) throw new Error(metadata.stderr);
const packages = JSON.parse(metadata.stdout).packages;
for (const pkg of packages)
  if (
    !members.some(
      (member) =>
        fs.realpathSync(pkg.manifest_path) ===
        fs.realpathSync(path.join(root, member, "Cargo.toml")),
    )
  )
    throw new Error("external package in native closure: " + pkg.name);
const env = { ...process.env };
delete env.CARGO_TARGET_DIR;
// Host tools belong to the invoking repository, not the restricted guest workspace.
const emulator =
  env.QEMU_AARCH64 ||
  (process.platform === "win32"
    ? ".toolchains/qemu/bin/qemu-system-aarch64.exe"
    : "qemu-system-aarch64");
env.QEMU_AARCH64 =
  !path.isAbsolute(emulator) && /[\\/]/.test(emulator)
    ? path.resolve(root, emulator)
    : emulator;
function native(args) {
  const result = cp.spawnSync(runner, args, {
    cwd: directory,
    stdio: "inherit",
    env,
  });
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error("native scenario failed: " + args.join(" "));
}
run(
  "cargo",
  [
    "test",
    "--locked",
    "-p",
    "native-userspace",
    "-p",
    "native-apps",
    "-p",
    "kernel-core",
  ],
  directory,
);
native(["service-run"]);
native(["lifecycle-test"]);
native(["app-smoke"]);
const executionReceipts = {};
for (const name of [
  "native-counter-client-1-positive.json",
  "native-counter-client-2-positive.json",
  "native-selftest-lifecycle-peer-6-positive.json",
]) {
  const file = path.join(directory, "target/kernel", name);
  if (!fs.existsSync(file))
    throw new Error("runner escaped the restricted workspace: " + name);
  const receipt = JSON.parse(fs.readFileSync(file));
  if (Object.keys(receipt.profiles).sort().join(",") !== "dev,prod")
    throw new Error("both profiles required in restricted workspace");
  if (
    receipt.source_files.some((item) =>
      absent.some((component) =>
        item.path.startsWith("crates/" + component + "/"),
      ),
    )
  )
    throw new Error("unrelated source reached execution");
  for (const value of Object.values(receipt.profiles)) {
    for (const image of Object.values(value.applications))
      if (!fs.existsSync(path.resolve(directory, image.artifact)))
        throw new Error("selected ELF missing from restricted workspace");
  }
  executionReceipts[name] = receipt;
}
for (const item of before)
  if (
    item.digest !==
    crypto.createHash("sha256").update(fs.readFileSync(item.file)).digest("hex")
  )
    throw new Error("production source changed during proof: " + item.file);
const proof = {
  schema_version: 1,
  scope:
    "Native kernel/app dependency and execution closure using the original implementation files through directory references; host runner dependency closure is excluded; not full Phase 3.7 acceptance",
  implementation_copies: 0,
  directory,
  source_references: references,
  execution_workspace: fs.realpathSync(directory),
  host_emulator: env.QEMU_AARCH64,
  execution_receipts: executionReceipts,
  absent_workspace_components: absent,
  native_external_packages: 0,
  scenarios: [
    "production runtime DEV/PROD",
    "public lifecycle/actual counter service DEV/PROD",
    "ordinary completed production session DEV/PROD",
  ],
  hardware: "UNKNOWN",
};
fs.mkdirSync(path.join(root, "target/kernel"), { recursive: true });
fs.writeFileSync(
  path.join(root, "target/kernel/native-closure-proof.json"),
  JSON.stringify(proof, null, 2) + "\n",
);
console.log(
  "Native closure passed using original source files; implementation copies: 0.",
);
