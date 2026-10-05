const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
const { spawnSync } = require("node:child_process");
const directory = path.resolve("target/ci-build-reuse");
if (!directory.startsWith(path.resolve("target") + path.sep))
  throw new Error("Build proof boundary mismatch");
const observations = [];
function run(args) {
  const start = process.hrtime.bigint();
  const result = spawnSync(process.env.CARGO || "cargo", args, {
    stdio: "inherit",
  });
  if (result.status !== 0) throw new Error("Build reuse proof command failed");
  return Number(process.hrtime.bigint() - start) / 1e9;
}
function clean() {
  run(["clean", "--target-dir", directory]);
}
function build(label, prod, negative = false) {
  const features = [
    "kernel-tests",
    "machine-events",
    ...(!prod ? ["diagnostics"] : []),
    ...(negative ? ["negative-test"] : []),
  ];
  const seconds = run([
    "build",
    "--locked",
    "-p",
    "kolvrt-kernel",
    "--target",
    "aarch64-unknown-none",
    "--target-dir",
    directory,
    "--no-default-features",
    "--features",
    features.join(","),
    ...(prod ? ["--release"] : []),
  ]);
  const elf = fs.readFileSync(
    path.join(
      directory,
      "aarch64-unknown-none",
      prod ? "release" : "debug",
      "kolvrt-kernel",
    ),
  );
  const sha256 = crypto.createHash("sha256").update(elf).digest("hex");
  observations.push({
    label,
    profile: prod ? "prod" : "dev",
    features,
    elf_sha256: sha256,
    cargo_build_wall_seconds: seconds,
  });
  return sha256;
}
try {
  clean();
  const dev = build("cold-dev", false);
  if (build("warm-dev", false) !== dev)
    throw new Error("Warm DEV hash mismatch");
  if (build("negative-dev", false, true) === dev)
    throw new Error("Negative control contaminated positive build");
  if (build("dev-after-negative", false) !== dev)
    throw new Error("Positive DEV hash changed after negative build");
  const prod = build("cold-prod", true);
  if (build("warm-prod", true) !== prod)
    throw new Error("Warm PROD hash mismatch");
  clean();
  if (
    build("clean-dev-again", false) !== dev ||
    build("clean-prod-again", true) !== prod
  )
    throw new Error("Clean rebuild identity mismatch");
  fs.writeFileSync(
    "target/ci-build-reuse-proof.json",
    JSON.stringify(
      {
        schema_version: 1,
        scope:
          "Named DEV/PROD positive builds and one DEV negative feature; same paths, compiler and source. Not a proof for every feature, cross-machine reproducibility, cache service or QEMU reuse.",
        observations,
        correctness: "passed",
      },
      null,
      2,
    ) + "\n",
  );
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
