const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

function aggregate(expected, receipts, count = 4) {
  if (
    receipts.length !== count ||
    expected.schema_version !== 1 ||
    !Array.isArray(expected.tasks)
  )
    throw new Error("Missing shard or invalid plan");
  const tasks = new Map(expected.tasks.map((task) => [task.id, task]));
  if (tasks.size !== expected.tasks.length || tasks.size === 0)
    throw new Error("Duplicate or empty expected inventory");
  const indices = new Set(),
    completed = new Map();
  for (const receipt of receipts) {
    if (
      receipt.schema_version !== 1 ||
      receipt.shard_count !== count ||
      !Number.isInteger(receipt.shard_index) ||
      receipt.shard_index < 0 ||
      receipt.shard_index >= count ||
      indices.has(receipt.shard_index) ||
      JSON.stringify(receipt.plan) !== JSON.stringify(expected)
    )
      throw new Error("Incompatible source/plan or duplicate shard");
    indices.add(receipt.shard_index);
    for (const result of receipt.completed) {
      const task = tasks.get(result.id);
      const position = expected.tasks.findIndex(
        (item) => item.id === result.id,
      );
      if (
        !task ||
        position % count !== receipt.shard_index ||
        completed.has(result.id) ||
        result.outcome !== "passed" ||
        result.observation !== "executed"
      )
        throw new Error(
          "Missing, duplicate, unexpected or failed matrix observation",
        );
      const features = [
        "machine-events",
        ...(task.profile === "dev" ? ["diagnostics"] : []),
        ...(task.tests ? ["kernel-tests"] : []),
        ...(task.feature ? [task.feature] : []),
      ].sort();
      if (
        JSON.stringify([...result.build.features].sort()) !==
          JSON.stringify(features) ||
        !/^[a-f0-9]{64}$/.test(result.build.sha256) ||
        result.run.elf_sha256 !== result.build.sha256 ||
        result.build.target !== "aarch64-unknown-none" ||
        result.build.compiler !== "1.99.0" ||
        !result.run.qemu_version.startsWith("QEMU emulator version 10.1.0 ") ||
        JSON.stringify(result.run.arguments.slice(0, -1)) !==
          JSON.stringify(expected.qemu_arguments.slice(0, -1))
      )
        throw new Error("Incompatible profile/features/ELF/QEMU configuration");
      completed.set(result.id, result);
    }
  }
  if (completed.size !== tasks.size)
    throw new Error("Incomplete required matrix");
  return {
    schema_version: 1,
    scope:
      "complete exact-source sharded kernel matrix; no physical hardware acceptance",
    plan: expected,
    completed: expected.tasks.map((task) => completed.get(task.id)),
    shards: count,
  };
}

function findReceipts(directory) {
  const files = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...findReceipts(file));
    else if (/^shard-\d+\.json$/.test(entry.name)) files.push(file);
  }
  return files;
}
function read(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}
function main() {
  const [planFile, directory, destination] = process.argv.slice(2);
  const expected = read(planFile);
  for (const source of expected.source_files) {
    const absolute = path.resolve(source.path);
    if (!absolute.startsWith(process.cwd() + path.sep))
      throw new Error("Source outside workspace");
    const bytes = fs.readFileSync(absolute);
    const digest = crypto
      .createHash("sha256")
      .update(
        source.sha256_lf
          ? bytes.toString("utf8").replaceAll("\r\n", "\n")
          : bytes,
      )
      .digest("hex");
    if (digest !== (source.sha256_lf || source.sha256))
      throw new Error("Current-source digest mismatch");
  }
  const result = aggregate(expected, findReceipts(directory).map(read));
  fs.writeFileSync(destination, JSON.stringify(result, null, 2) + "\n");
  console.log(
    `Complete matrix: ${result.completed.length} exact-source executed tasks across ${result.shards} shards`,
  );
}
module.exports = { aggregate };
if (require.main === module) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
