/**
 * Require all mandatory partitioned jobs, including the aggregate.
 * A host-only waiver requires explicit skips; unexpected failures cannot be hidden.
 * @param {object} jobs GitHub needs results indexed by declared job ID.
 * @param {boolean} kernelRequired True unless the validated impact step explicitly waives it.
 * @throws {Error} A mandatory job is missing, failed, cancelled or skipped.
 */
function requireSuccess(jobs, kernelRequired = true) {
  const required = !kernelRequired
    ? ["static", "host"]
    : [
        "static",
        "host",
        "kernel-dev",
        "kernel-prod",
        "matrix",
        "routing",
        "asid",
        "native",
        "evidence",
      ];
  if (!kernelRequired) {
    for (const id of [
      "kernel-dev",
      "kernel-prod",
      "matrix",
      "routing",
      "asid",
      "native",
      "evidence",
    ])
      if (jobs[id]?.result !== "skipped")
        throw new Error(`Unexpected workload outcome in host-only run: ${id}`);
  }
  for (const id of required)
    if (jobs[id]?.result !== "success")
      throw new Error(`Required job failed or was skipped: ${id}`);
}
module.exports = { requireSuccess };
if (require.main === module) {
  try {
    requireSuccess(
      JSON.parse(process.env.RESULTS),
      process.env.KERNEL_REQUIRED !== "false",
    );
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
