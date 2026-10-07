// Diagnostic counterexample only: external host-thread starvation, never a passing acceptance run.
const fs = require("node:fs"),
  cp = require("node:child_process"),
  path = require("node:path"),
  assert = require("node:assert/strict"),
  crypto = require("node:crypto");
const { Remote, port, connect } = require("./native-crash-recovery.cjs");
const pause = (ms) => new Promise((r) => setTimeout(r, ms));
async function bounded(promise, label, milliseconds = 10000) {
  let timer;
  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => {
        timer = setTimeout(
          () => reject(Error(label + " timeout")),
          milliseconds,
        );
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}
function exited(child) {
  // Resolve errors as data immediately, so a later cleanup cannot encounter an
  // unhandled rejection while the debugger is still waiting on another socket.
  return new Promise((resolve) => {
    child.once("error", (error) => resolve({ error }));
    child.once("exit", (code, signal) => resolve({ code, signal }));
  });
}
assert.equal(
  process.platform,
  "win32",
  "diagnostic requires Windows host thread API",
);
const dir = path.resolve("target/kernel/supervision-host-stall");
fs.mkdirSync(dir, { recursive: true });
function hash(b) {
  return crypto.createHash("sha256").update(b).digest("hex");
}
function symbols(b) {
  const sh = Number(b.readBigUInt64LE(40)),
    n = b.readUInt16LE(60),
    size = b.readUInt16LE(58);
  const sections = [];
  for (let i = 0; i < n; i++) {
    const p = sh + i * size;
    sections.push({
      type: b.readUInt32LE(p + 4),
      address: Number(b.readBigUInt64LE(p + 16)),
      offset: Number(b.readBigUInt64LE(p + 24)),
      bytes: Number(b.readBigUInt64LE(p + 32)),
      link: b.readUInt32LE(p + 40),
      stride: Number(b.readBigUInt64LE(p + 56)),
    });
  }
  const out = [];
  for (const s of sections.filter((s) => s.type === 2)) {
    const strings = sections[s.link];
    for (let i = 0; i < s.bytes; i += s.stride) {
      const p = s.offset + i,
        np = strings.offset + b.readUInt32LE(p),
        name = b.toString("utf8", np, b.indexOf(0, np));
      out.push({ name, address: Number(b.readBigUInt64LE(p + 8)) });
    }
  }
  return { out, sections };
}
async function qmp(socket) {
  let buffer = "",
    next = 0;
  const waits = new Map();
  socket.on("data", (chunk) => {
    buffer += chunk;
    for (;;) {
      const p = buffer.indexOf("\n");
      if (p < 0) break;
      const line = buffer.slice(0, p);
      buffer = buffer.slice(p + 1);
      if (!line.trim()) continue;
      const j = JSON.parse(line);
      if (j.id && waits.has(j.id)) {
        waits.get(j.id)(j);
        waits.delete(j.id);
      }
    }
  });
  return async (execute) => {
    const id = ++next;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(Error("QMP timeout")), 5000);
      waits.set(id, (j) => {
        clearTimeout(timer);
        j.error ? reject(Error(JSON.stringify(j.error))) : resolve(j.return);
      });
      socket.write(JSON.stringify({ execute, id }) + "\n");
    });
  };
}
async function main() {
  const inputs = [
    "crates/kernel-core/src/ipc/tests.rs",
    "crates/kernel/src/supervision_workload.S",
    "crates/kernel/src/supervision_workload.rs",
    "tests/system/native-crash-recovery.cjs",
    "tests/system/supervision-host-stall.cjs",
    "tests/system/hold-qemu-thread.ps1",
  ];
  const sourceFiles = Object.fromEntries(
    inputs.map((p) => [
      p,
      hash(Buffer.from(fs.readFileSync(p, "utf8").replace(/\r\n/g, "\n"))),
    ]),
  );
  const record = JSON.parse(
      fs.readFileSync("target/kernel/dev-tests.run.json", "utf8"),
    ),
    args = [...record.arguments],
    elf = args[args.indexOf("-kernel") + 1],
    bytes = fs.readFileSync(elf),
    digest = hash(bytes);
  assert.equal(digest, record.elf_sha256);
  const sy = symbols(bytes),
    start = sy.out.find((s) => s.name === "supervisor_image_start").address,
    end = sy.out.find((s) => s.name === "supervisor_image_end").address,
    section = sy.sections.find(
      (s) => start >= s.address && end <= s.address + s.bytes,
    );
  assert(section);
  const raw = bytes.subarray(
    section.offset + start - section.address,
    section.offset + end - section.address,
  );
  const points = [];
  for (let i = 12; i + 4 <= raw.length; i += 4)
    if (
      raw.readUInt32LE(i) === 0xd4001401 &&
      raw.readUInt32LE(i - 12) === 0xd2800601
    )
      points.push(0x20000000 + i);
  assert.equal(points.length, 1);
  const entry = sy.out.find((s) =>
    s.name.includes("supervision_workload8exercise"),
  );
  assert(entry);
  const gdbPort = await port(),
    qmpPort = await port(),
    deadline = Date.now() + 120000;
  args.push(
    "-S",
    "-gdb",
    `tcp:127.0.0.1:${gdbPort}`,
    "-qmp",
    `tcp:127.0.0.1:${qmpPort},server=on,wait=off`,
  );
  const log = dir + "/readiness-host-stall.log",
    fd = fs.openSync(log, "w");
  const child = cp.spawn(process.env.QEMU_AARCH64, args, {
    windowsHide: true,
    stdio: ["ignore", fd, fd],
  });
  const childClosed = exited(child);
  let remote, socket, held, heldClosed, observed, result;
  const hits = [];
  const errors = [];
  try {
    remote = new Remote(await connect(gdbPort, child, deadline), deadline);
    await remote.command("qSupported");
    await remote.registers();
    await remote.ok(`Z1,${entry.address.toString(16)},4`);
    await remote.command("c");
    assert.equal(await remote.read("pc"), entry.address);
    await remote.ok(`z1,${entry.address.toString(16)},4`);
    await remote.ok(`Z1,${points[0].toString(16)},4`);
    socket = await connect(qmpPort, child, deadline);
    const command = await qmp(socket);
    await command("qmp_capabilities");
    const cpus = await command("query-cpus-fast");
    for (let i = 0; i < 20; i++) {
      const stop = await remote.command("c");
      assert(stop.startsWith("T") || stop.startsWith("S"), stop);
      const thread = stop.match(/thread:([^;]+);/)?.[1];
      assert(thread);
      await remote.ok("Hg" + thread);
      const hit = {
        thread,
        stage: await remote.read("x20"),
        payload: await remote.read("x17"),
        pc: await remote.read("pc"),
      };
      hits.push(hit);
      if (hit.stage !== 1030 || hit.payload !== 1) {
        await remote.ok("z1," + points[0].toString(16) + ",4");
        await remote.ok("Hc" + thread);
        await remote.command("s");
        await remote.ok("Z1," + points[0].toString(16) + ",4");
        await remote.ok("Hc-1");
        continue;
      }
      const currentThread = await remote.command("qC");
      assert.match(currentThread, /^QC[0-9a-f]+$/i);
      assert.equal(parseInt(currentThread.slice(2), 16), 1);
      assert.equal((await remote.read("cpsr")) & 15, 0);
      await remote.ok(`z1,${points[0].toString(16)},4`);
      const cpu = cpus.find((c) => c["cpu-index"] === 0);
      assert(cpu && Number.isInteger(cpu["thread-id"]));
      held = cp.spawn(
        "powershell.exe",
        [
          "-NoProfile",
          "-ExecutionPolicy",
          "Bypass",
          "-File",
          path.join(__dirname, "hold-qemu-thread.ps1"),
          "-QemuPid",
          String(child.pid),
          "-ThreadId",
          String(cpu["thread-id"]),
          "-HoldMs",
          "500",
        ],
        { windowsHide: true, stdio: ["ignore", "pipe", "pipe"] },
      );
      heldClosed = exited(held);
      await bounded(
        new Promise((resolve, reject) => {
          let output = "";
          held.stdout.on("data", (b) => {
            output += b.toString();
            if (output.split(/\r?\n/).includes("HELD")) resolve();
          });
          held.stderr.on("data", (b) => reject(Error(b.toString())));
          heldClosed.then((end) =>
            reject(
              end.error || Error("hold helper exited before HELD: " + end.code),
            ),
          );
        }),
        "hold helper readiness",
      );
      const resumedAt = Date.now();
      await remote.ok("D");
      remote.socket.destroy();
      const end = await bounded(heldClosed, "hold helper completion");
      if (end.error) throw end.error;
      assert.equal(end.code, 0, "hold helper exit");
      for (;;) {
        const logText = fs.readFileSync(log, "utf8");
        const line = logText
          .split(/\r?\n/)
          .find(
            (l) =>
              l.startsWith("@KOLVRT/1 ") && l.includes("supervision-reject"),
          );
        if (line) {
          observed = JSON.parse(line.slice(10));
          break;
        }
        assert(Date.now() < deadline, "no rejection observed");
        await pause(50);
      }
      observed.host_experiment_elapsed_ms = Date.now() - resumedAt;
      break;
    }
    assert(observed, "replacement readiness not reached");
    assert.equal(observed.stage, 1030);
    assert.equal(observed.payload_operation, 1);
    assert.equal(observed.actual, 16);
    assert.equal(observed.operation, 1);
    result = {
      source_commit: cp
        .execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" })
        .trim(),
      worktree_dirty: !!cp
        .execFileSync(
          "git",
          ["status", "--porcelain", "--untracked-files=no"],
          { encoding: "utf8" },
        )
        .trim(),
      source_files_sha256_lf: sourceFiles,
      build: JSON.parse(
        fs.readFileSync("target/kernel/dev-tests-build.json", "utf8"),
      ),
      elf_sha256: digest,
      arguments: args,
      host_fault:
        "Suspend only owned QEMU CPU0 host thread for500ms while VM clock and CPU1 continue",
      acceptance:
        "diagnostic induced rejection, not normal pass and not proof of original CI scheduling cause",
      production_source_or_image_mutation: false,
      hits,
      observed,
    };
  } catch (error) {
    errors.push(error);
  } finally {
    // Keep the original diagnostic failure and every cleanup failure while
    // still attempting each independent cleanup action.
    const cleanup = async (action) => {
      try {
        await action();
      } catch (error) {
        errors.push(error);
      }
    };
    await cleanup(() => remote?.socket.destroy());
    await cleanup(() => socket?.destroy());
    await cleanup(async () => {
      if (heldClosed) {
        const end = await bounded(heldClosed, "hold helper cleanup");
        if (end.error) throw end.error;
        assert.equal(end.code, 0, "hold helper cleanup exit");
      }
    });
    // Retire the owned QEMU even when helper startup or resume failed.
    await cleanup(async () => {
      if (child.pid && child.exitCode === null && child.signalCode === null)
        child.kill();
      const end = await bounded(childClosed, "QEMU cleanup");
      if (end.error) throw end.error;
    });
    // Let the helper resume normally first; forced termination is only fallback
    // after the owned QEMU cleanup attempt.
    await cleanup(async () => {
      if (held?.pid && held.exitCode === null && held.signalCode === null) {
        held.kill();
        const end = await bounded(heldClosed, "hold helper termination");
        if (end.error) throw end.error;
      }
    });
    await cleanup(() => fs.closeSync(fd));
    await cleanup(() => assert.equal(hash(fs.readFileSync(elf)), digest));
    for (const [p, digest] of Object.entries(sourceFiles)) {
      await cleanup(() =>
        assert.equal(
          hash(Buffer.from(fs.readFileSync(p, "utf8").replace(/\r\n/g, "\n"))),
          digest,
          "diagnostic source changed during execution: " + p,
        ),
      );
    }
  }
  if (errors.length) {
    throw new AggregateError(errors, "Host-stall diagnostic or cleanup failed");
  }
  // Publish only after the witness, owned-process cleanup and immutable-input
  // checks have all succeeded. A failing diagnostic must not create a receipt.
  fs.writeFileSync(
    dir + "/readiness-host-stall.json",
    JSON.stringify(result, null, 2) + "\n",
  );
  console.log(JSON.stringify(result.observed));
}
main().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
