const fs = require("node:fs");
const path = require("node:path");
/**
 * Sum executed wall observations separately by kind and retain failure outcomes.
 * Nested categories overlap; totals are not exclusive compiler CPU time.
 * @param {object[]} records Schema-v1 per-process observations.
 * @returns {object} Category counts/durations and explicit attribution limits.
 */
function summarize(records) {
  const groups = {};
  for (const record of records) {
    if (
      record.schema_version !== 1 ||
      typeof record.kind !== "string" ||
      !Number.isFinite(record.wall_seconds) ||
      record.wall_seconds < 0 ||
      record.observation !== "executed"
    )
      throw new Error("Invalid timing observation");
    const group = (groups[record.kind] ||= {
      observations: 0,
      wall_seconds: 0,
      outcomes: {},
    });
    group.observations++;
    group.wall_seconds += record.wall_seconds;
    group.outcomes[record.outcome] = (group.outcomes[record.outcome] || 0) + 1;
  }
  return {
    schema_version: 1,
    groups,
    compiler_exclusive_seconds: null,
    attribution:
      "Nested groups overlap; do not sum matrix/host wall time with contained cargo-build or QEMU observations. cargo-build is Cargo invocation wall time, not exclusive rustc CPU time.",
  };
}
/**
 * Read JSONL observations for one job and write its summary without treating missing CPU time as zero.
 */
function main(directory) {
  const records = fs
    .readdirSync(directory)
    .filter((name) => name.endsWith(".jsonl"))
    .flatMap((name) =>
      fs
        .readFileSync(path.join(directory, name), "utf8")
        .split(/\r?\n/)
        .filter((line) => line.trim())
        .map((line) => JSON.parse(line)),
    );
  const result = summarize(records);
  fs.writeFileSync(
    path.join(directory, "observations.json"),
    JSON.stringify(result, null, 2) + "\n",
  );
  if (process.env.GITHUB_STEP_SUMMARY)
    fs.appendFileSync(
      process.env.GITHUB_STEP_SUMMARY,
      "```json\n" + JSON.stringify(result, null, 2) + "\n```\n",
    );
}
module.exports = { summarize };
if (require.main === module) {
  try {
    main(process.argv[2]);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
