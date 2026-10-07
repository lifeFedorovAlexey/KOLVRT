"use strict";
const fs = require("node:fs");
const net = require("node:net");
const cp = require("node:child_process");
const crypto = require("node:crypto");
const assert = require("node:assert/strict");

const sha = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const MAGIC = 0x4b4f4c5652543701n.toString();
const WATCHDOG_MS = 30000;

/** Check the actual production fault/replacement/client ledger, in order. */
function validateRecovery(events, fault, completed = false) {
  assert.equal(fault.cpu, 1);
  assert.equal(fault.exceptionLevel, 0);
  assert.equal(fault.operation, 3);
  assert.equal(fault.repliesBeforeFault, 1);
  assert.equal(fault.injectedPc, 0);
  assert(
    !events.some(
      (e) =>
        ["panic", "fatal"].includes(e.event) ||
        (e.event === "native-boot" &&
          (!completed || e.status !== "complete")) ||
        e.status === "fail",
    ),
  );
  const matches = (predicate) =>
    events.flatMap((event, index) =>
      predicate(event) ? [{ event, index }] : [],
    );
  const root = matches((e) => e.event === "native-root-image");
  assert.equal(root.length, 1);
  assert.equal(root[0].event.format, "elf64");
  const images = matches((e) => e.event === "native-image" && e.selector === 0);
  assert.equal(images.length, 2);
  assert.deepEqual(
    images.map((e) => [e.event.format, e.event.generation]),
    [
      ["elf64", 1],
      ["elf64", 2],
    ],
  );
  const faulted = matches(
    (e) =>
      e.event === "native-process-terminal" &&
      e.owner === 1 &&
      e.generation === 1,
  );
  assert.equal(faulted.length, 1);
  assert.equal(faulted[0].event.state, 3);
  assert.equal(faulted[0].event.generation, 1);
  const completion = matches(
    (e) =>
      e.event === "native-completion" && e.selector === 0 && e.generation === 1,
  );
  assert.equal(completion.length, 1);
  assert.equal(completion[0].event.kind, "fault");
  assert.equal(completion[0].event.code, 0x20); // Actual lower-EL instruction abort.
  assert.equal(completion[0].event.generation, 1);
  const clientImages = matches(
    (e) => e.event === "native-image" && e.selector === 1,
  );
  assert.equal(clientImages.length, 1);
  const clients = matches(
    (e) => e.event === "native-completion" && e.selector === 1,
  );
  assert.equal(clients.length, 1);
  assert.equal(clients[0].event.kind, "exit");
  assert.equal(clients[0].event.code, 0);
  const progress = matches((e) =>
    completed
      ? e.event === "native-user-report" && e.words?.length === 7
      : e.event === "native-report-progress" && e.words?.length === 6,
  );
  assert.equal(progress.length, 1);
  const words = progress[0].event.words;
  assert.equal(String(words[0]), MAGIC);
  assert.deepEqual(
    words.slice(1),
    completed ? [4, 3, 12, 3, 2, 3] : [2, 12, 3, 0, 1],
  );
  assert(
    images[0].index < faulted[0].index &&
      faulted[0].index < completion[0].index,
  );
  assert(
    completion[0].index < images[1].index && images[1].index < clients[0].index,
  );
  assert(clients[0].index < progress[0].index);
  if (completed) {
    const stopped = matches(
      (e) =>
        e.event === "native-completion" &&
        e.selector === 0 &&
        e.generation === 2,
    );
    assert.equal(stopped.length, 1);
    assert.equal(stopped[0].event.kind, "terminated");
    assert.equal(stopped[0].event.code, 0);
    assert(
      clients[0].index < stopped[0].index &&
        stopped[0].index < progress[0].index,
    );
    const ends = matches((e) => e.event === "native-boot");
    assert.equal(ends.length, 1);
    const end = ends[0];
    assert.equal(end.event.status, "complete");
    assert.equal(end.event.root_exit, 0);
    assert.equal(end.event.owners_released, true);
    assert.equal(end.event.frames_restored, true);
    assert.equal(end.event.live_processes, 0);
    assert.equal(end.event.live_domains, 0);
    assert(progress[0].index < end.index);
  }
  assert.equal(
    matches((e) => e.event === "native-completion" && e.selector === 0).length,
    completed ? 2 : 1,
  );
  assert.equal(
    matches((e) => e.event === "native-process-terminal" && e.owner === 1)
      .length,
    completed ? 2 : 1,
  );
  return {
    production_supervisor_tested: true,
    service_crash_restarted: true,
    fresh_process_generation: 2,
    fresh_binding_completed: true,
    old_binding_rejected: true,
    restart_initial_value: 0,
    counter_final: 12,
    client_exit: "success",
    guest_shutdown: completed,
    resource_leaks: completed ? 0 : "NOT_MEASURED_ON_FORCED_VM_STOP",
    scope:
      "post-binding instruction abort before the first client ADD commit; actual production supervisor/client/service",
    limitations: [
      "External QEMU debug fault, not spontaneous hardware failure",
      "No startup-crash or post-commit replay acceptance",
      ...(completed
        ? []
        : [
            "Persistent runtime is stopped by the host, not graceful guest shutdown",
          ]),
      "No performance or physical ARM evidence",
    ],
  };
}

/** Find public IPC SVC instruction boundaries in the original executable ELF. */
function ipcBreakpoints(bytes) {
  assert.equal(bytes.subarray(0, 4).toString("hex"), "7f454c46");
  assert.equal(bytes[4], 2);
  assert.equal(bytes[5], 1);
  assert.equal(bytes.readUInt16LE(18), 183);
  const points = [];
  for (let i = 0; i < bytes.readUInt16LE(56); i++) {
    const p = Number(bytes.readBigUInt64LE(32)) + i * bytes.readUInt16LE(54);
    if (bytes.readUInt32LE(p) !== 1 || !(bytes.readUInt32LE(p + 4) & 1))
      continue;
    const offset = Number(bytes.readBigUInt64LE(p + 8));
    const address = Number(bytes.readBigUInt64LE(p + 16));
    const size = Number(bytes.readBigUInt64LE(p + 32));
    for (let n = 0; n + 4 <= size; n += 4) {
      if (bytes.readUInt32LE(offset + n) === 0xd4001401)
        points.push(address + n);
    }
  }
  assert(points.length > 0, "no public IPC SVC boundary in production service");
  return points;
}

/** Minimal serialized GDB remote client; all time limits belong to the host. */
class Remote {
  constructor(socket, deadline) {
    this.socket = socket;
    this.deadline = deadline;
    this.buffer = "";
    socket.setNoDelay(true);
    socket.on("data", (bytes) => {
      try {
        this.receive(bytes);
      } catch (error) {
        this.fail(error);
      }
    });
    socket.on("error", (error) => this.fail(error));
    socket.on("close", () => this.fail(new Error("debug connection closed")));
  }
  fail(error) {
    if (this.pending) {
      clearTimeout(this.pending.timer);
      this.pending.reject(error);
      this.pending = undefined;
    }
  }
  receive(bytes) {
    this.buffer += bytes.toString("latin1");
    for (;;) {
      const begin = this.buffer.indexOf("$");
      if (begin < 0) {
        this.buffer = "";
        return;
      }
      const end = this.buffer.indexOf("#", begin);
      if (end < 0 || this.buffer.length < end + 3) return;
      const encoded = this.buffer.slice(begin + 1, end);
      const sum = [...Buffer.from(encoded, "latin1")].reduce(
        (a, b) => (a + b) & 255,
        0,
      );
      assert.equal(
        sum,
        parseInt(this.buffer.slice(end + 1, end + 3), 16),
        "RSP checksum",
      );
      this.buffer = this.buffer.slice(end + 3);
      this.socket.write("+");
      let decoded = "";
      for (let i = 0; i < encoded.length; i++) {
        if (encoded[i] === "}")
          decoded += String.fromCharCode(encoded.charCodeAt(++i) ^ 32);
        else if (encoded[i] === "*")
          decoded += decoded.at(-1).repeat(encoded.charCodeAt(++i) - 29);
        else decoded += encoded[i];
      }
      assert(this.pending, "unsolicited debug response");
      const pending = this.pending;
      this.pending = undefined;
      clearTimeout(pending.timer);
      pending.resolve(decoded);
    }
  }
  command(body) {
    assert(!this.pending, "overlapping debug commands");
    const remaining = this.deadline - Date.now();
    assert(remaining > 0, "external crash scenario watchdog expired");
    return new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => this.fail(new Error("external debug watchdog expired")),
        remaining,
      );
      this.pending = { resolve, reject, timer };
      const sum = [...Buffer.from(body)].reduce((a, b) => (a + b) & 255, 0);
      this.socket.write(`$${body}#${sum.toString(16).padStart(2, "0")}`);
    });
  }
  async ok(body) {
    assert.equal(await this.command(body), "OK", body);
  }
  async xml(name) {
    let xml = "";
    for (;;) {
      const part = await this.command(
        `qXfer:features:read:${name}:${Buffer.byteLength(xml).toString(16)},fff`,
      );
      assert(["l", "m"].includes(part[0]));
      xml += part.slice(1);
      if (part[0] === "l") return xml;
    }
  }
  async registers() {
    const xml = await this.xml("aarch64-core.xml"),
      regs = new Map();
    let next = 0;
    for (const [, attributes] of xml.matchAll(/<reg\s+([^>]+)>/g)) {
      const fields = Object.fromEntries(
        [...attributes.matchAll(/(\w+)="([^"]*)"/g)].map((m) => [m[1], m[2]]),
      );
      if (fields.regnum !== undefined) next = Number(fields.regnum);
      regs.set(fields.name, {
        number: next++,
        bytes: Number(fields.bitsize) / 8,
      });
    }
    for (const name of ["x0", "pc", "cpsr"])
      assert(regs.has(name), "missing register " + name);
    this.regs = regs;
  }
  async read(name) {
    const reg = this.regs.get(name),
      bytes = Buffer.from(
        await this.command("p" + reg.number.toString(16)),
        "hex",
      );
    assert.equal(bytes.length, reg.bytes);
    return reg.bytes === 8
      ? Number(bytes.readBigUInt64LE())
      : bytes.readUInt32LE();
  }
  async faultPc() {
    const reg = this.regs.get("pc");
    await this.ok(`P${reg.number.toString(16)}=${"00".repeat(reg.bytes)}`);
    assert.equal(await this.read("pc"), 0);
  }
}

const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function port() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const selected = server.address().port;
  await new Promise((resolve) => server.close(resolve));
  return selected;
}
async function connect(selected, child, deadline) {
  for (;;) {
    try {
      return await new Promise((resolve, reject) => {
        const socket = net.createConnection(selected, "127.0.0.1");
        socket.once("connect", () => resolve(socket));
        socket.once("error", (error) => {
          socket.destroy();
          reject(error);
        });
      });
    } catch (error) {
      if (!child.pid || child.exitCode !== null || Date.now() >= deadline)
        throw error;
      await pause(20); // Socket readiness only, never a kernel outcome retry.
    }
  }
}
function eventsAt(path) {
  const text = fs.readFileSync(path, "utf8");
  return text
    .slice(0, text.lastIndexOf("\n") + 1)
    .split("\n")
    .filter((line) => line.startsWith("@KOLVRT/1 "))
    .map((line) =>
      JSON.parse(
        line.slice(10).replace(/("words"\s*:\s*\[\s*)(\d+)/, '$1"$2"'),
      ),
    );
}
async function runProfile(profile, record, emulator) {
  const original = new Map();
  for (const [role, binary] of [
    ["root", "native-supervisor"],
    ["service", "counter-service"],
    ["client", "counter-client"],
  ]) {
    const metadata = record.applications[role],
      bytes = fs.readFileSync(metadata.artifact);
    assert.equal(sha(bytes), metadata.sha256);
    const path = `target/aarch64-unknown-none/${profile === "dev" ? "debug" : "release"}/${binary}`;
    assert.equal(
      sha(fs.readFileSync(path)),
      metadata.sha256,
      "test must use original production " + binary,
    );
    original.set(metadata.artifact, metadata.sha256);
    original.set(path, metadata.sha256);
  }
  const ordinary = record.run || record.result.run,
    args = [...ordinary.arguments];
  const kernel = args[args.indexOf("-kernel") + 1];
  assert(
    kernel && args.includes("-smp") && args[args.indexOf("-smp") + 1] === "2",
  );
  assert.equal(
    sha(fs.readFileSync(kernel)),
    ordinary.kernel_sha256 || ordinary.elf_sha256,
  );
  original.set(kernel, ordinary.kernel_sha256 || ordinary.elf_sha256);
  const points = ipcBreakpoints(
    fs.readFileSync(record.applications.service.artifact),
  );
  const selected = await port(),
    deadline = Date.now() + WATCHDOG_MS;
  args.push("-S", "-gdb", `tcp:127.0.0.1:${selected}`);
  const log = `target/kernel/${profile}-production-crash.log`,
    output = fs.openSync(log, "w");
  const child = cp.spawn(emulator, args, {
    stdio: ["ignore", output, output],
    windowsHide: true,
  });
  const closed = new Promise((resolve) => {
    child.once("error", resolve);
    child.once("close", resolve);
  });
  let remote, fault;
  try {
    remote = new Remote(await connect(selected, child, deadline), deadline);
    assert(
      (await remote.command("qSupported")).includes("qXfer:features:read+"),
    );
    await remote.registers();
    for (const address of points)
      await remote.ok(`Z1,${address.toString(16)},4`);
    let replies = 0;
    for (let hit = 0; hit < 256; hit++) {
      const stop = await remote.command("c"),
        thread = stop.match(/thread:([^;]+);/)?.[1];
      assert(
        thread && stop.startsWith("T05"),
        "unexpected debug stop: " + stop,
      );
      await remote.ok("Hg" + thread);
      const pc = await remote.read("pc"),
        pstate = await remote.read("cpsr");
      if (parseInt(thread, 16) === 2 && (pstate & 15) === 0) {
        assert(points.includes(pc));
        const input = await remote.read("x0"),
          bytes = Buffer.from(
            await remote.command(`m${input.toString(16)},28`),
            "hex",
          );
        assert.equal(bytes.length, 40);
        assert.equal(bytes.readUInt16LE(0), 1);
        const operation = bytes.readUInt16LE(2);
        if (operation === 4) replies++;
        // Readiness GET has replied; the client binding and delivery exist.
        // Stop before its first ADD COMMIT, hence before Counter::apply.
        if (operation === 3 && replies === 1) {
          fault = {
            cpu: 1,
            exceptionLevel: 0,
            operation,
            repliesBeforeFault: replies,
            originalPc: pc,
            injectedPc: 0,
            frame: bytes.toString("hex"),
          };
          for (const address of points)
            await remote.ok(`z1,${address.toString(16)},4`);
          await remote.faultPc();
          await remote.ok("D");
          remote.socket.end();
          break;
        }
      }
      assert(points.includes(pc), "stop outside selected instruction boundary");
      await remote.ok(`z1,${pc.toString(16)},4`);
      await remote.ok("Hc" + thread);
      assert((await remote.command("s")).startsWith("T05"));
      await remote.ok(`Z1,${pc.toString(16)},4`);
      await remote.ok("Hc-1");
    }
    assert(fault, "production service fault boundary missing");
    for (;;) {
      const events = eventsAt(log);
      assert(
        !events.some(
          (e) =>
            ["panic", "fatal"].includes(e.event) ||
            e.status === "fail" ||
            (e.event === "native-boot" && e.status !== "complete"),
        ),
      );
      if (
        events.some((e) => e.event === "native-boot" && e.status === "complete")
      ) {
        const result = validateRecovery(events, fault, true);
        while (child.exitCode === null) {
          assert(
            Date.now() < deadline,
            "guest did not power off after actual reclamation",
          );
          await pause(20);
        }
        assert.equal(child.exitCode, 0);
        return {
          applications: record.applications,
          kernel_sha256: ordinary.kernel_sha256 || ordinary.elf_sha256,
          debugger: {
            arguments: args,
            source: "QEMU GDB remote protocol",
            fault,
          },
          result,
          events,
        };
      }
      assert(
        child.exitCode === null,
        "guest stopped before recovery/reclamation result",
      );
      assert(Date.now() < deadline, "external recovery watchdog expired");
      await pause(20);
    }
  } finally {
    remote?.socket.destroy();
    if (child.exitCode === null) child.kill();
    await closed;
    fs.closeSync(output);
    for (const [path, digest] of original)
      assert.equal(
        sha(fs.readFileSync(path)),
        digest,
        "production artifact changed: " + path,
      );
  }
}
async function main() {
  assert.equal(
    process.argv.length,
    3,
    "usage: node tests/system/native-crash-recovery.cjs RUNTIME_RECEIPT",
  );
  const receipt = JSON.parse(fs.readFileSync(process.argv[2]));
  const emulator = process.env.QEMU_AARCH64 || "qemu-system-aarch64";
  const profiles = {};
  for (const [profile, record] of Object.entries(receipt.profiles)) {
    assert(["dev", "prod"].includes(profile));
    profiles[profile] = await runProfile(profile, record, emulator);
    console.log(`Production supervisor crash-recovery passed (${profile})`);
  }
  assert(Object.keys(profiles).length > 0);
  fs.writeFileSync(
    "target/kernel/native-crash-recovery.json",
    JSON.stringify(
      {
        schema_version: 1,
        source_files: receipt.source_files,
        profiles,
        hardware: "UNKNOWN",
        kernel_or_application_source_mutation: false,
      },
      null,
      2,
    ) + "\n",
  );
}
module.exports = { validateRecovery, ipcBreakpoints };
if (require.main === module)
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
