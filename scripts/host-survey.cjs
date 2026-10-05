"use strict";

// Local, unauthenticated research observations. No upload or device authority.
const fs = require("node:fs");
const MAX_INPUT = 1024 * 1024;
const MAX_OUTPUT = 128 * 1024;
const MAX_DEVICES = 256;
const CLASSES = [
  "storage",
  "network",
  "gpu",
  "audio",
  "bluetooth",
  "hid",
  "other",
];
const REASONS = [
  "not_reported",
  "provider_unavailable",
  "permission_denied",
  "unsupported_platform",
  "unsupported_provider",
  "malformed_value",
  "budget_exceeded",
];
const PROVIDERS = [
  "Win32_Processor",
  "Win32_ComputerSystem",
  "Win32_PnPEntity",
  "Win32_PnPSignedDriver",
];
const UNKNOWN = (reason) => ({ status: "UNKNOWN", reason });
const fail = (code) => {
  throw new Error(code);
};
const compare = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

// JSON.parse alone silently accepts duplicate keys. Keep the bounded grammar
// check separate from semantic projection, including inside rejected fields.
function parse(text) {
  if (Buffer.byteLength(text, "utf8") > MAX_INPUT) fail("input_budget");
  let i = 0,
    nodes = 0;
  const space = () => {
    while (/[ \t\r\n]/.test(text[i] || "") && i < text.length) i++;
  };
  function string() {
    const start = i++;
    while (i < text.length) {
      if (text[i] === "\\") {
        i += 2;
        continue;
      }
      if (text[i++] === '"') return JSON.parse(text.slice(start, i));
    }
    fail("invalid_json");
  }
  function value(depth) {
    if (depth > 16 || ++nodes > 32768) fail("structure_budget");
    space();
    if (text[i] === '"') return string();
    if (text[i] === "{") {
      i++;
      space();
      const result = Object.create(null),
        keys = new Set();
      if (text[i] === "}") {
        i++;
        return result;
      }
      for (;;) {
        space();
        if (text[i] !== '"') fail("invalid_json");
        const key = string();
        if (keys.has(key)) fail("duplicate_key");
        keys.add(key);
        space();
        if (text[i++] !== ":") fail("invalid_json");
        result[key] = value(depth + 1);
        space();
        const next = text[i++];
        if (next === "}") return result;
        if (next !== ",") fail("invalid_json");
      }
    }
    if (text[i] === "[") {
      i++;
      space();
      const result = [];
      if (text[i] === "]") {
        i++;
        return result;
      }
      for (;;) {
        result.push(value(depth + 1));
        space();
        const next = text[i++];
        if (next === "]") return result;
        if (next !== ",") fail("invalid_json");
      }
    }
    const start = i;
    while (i < text.length && !/[\s,\]}]/.test(text[i])) i++;
    if (start === i) fail("invalid_json");
    return JSON.parse(text.slice(start, i));
  }
  const result = value(0);
  space();
  if (i !== text.length) fail("invalid_json");
  return result;
}

function shape(value, keys) {
  if (!value || typeof value !== "object" || Array.isArray(value))
    fail("invalid_shape");
  const actual = Object.keys(value);
  if (
    actual.length !== keys.length ||
    actual.some((key) => !keys.includes(key))
  )
    fail("unknown_or_missing_field");
}
function member(value, options) {
  if (!options.includes(value)) fail("invalid_enum");
  return value;
}
const integer = (max) => (value) =>
  Number.isSafeInteger(value) && value > 0 && value <= max;
const hex = (length) => (value) =>
  typeof value === "string" && new RegExp(`^[0-9A-F]{${length}}$`).test(value);
const safeModel = (value) =>
  typeof value === "string" &&
  value.length > 0 &&
  value.length <= 128 &&
  /^[A-Za-z0-9 ()@.,+_-]+$/.test(value) &&
  !/\b(?:\d{1,3}\.){3}\d{1,3}\b|(?:[A-Fa-f0-9]{2}[:-]){5}[A-Fa-f0-9]{2}|password|token|serial|ssid|hostname|username/i.test(
    value,
  ) &&
  (!value.includes("@") || / @ [0-9.]+GHz$/.test(value));
const infName = (value) =>
  typeof value === "string" &&
  /^[A-Za-z0-9][A-Za-z0-9_.-]{0,59}\.inf$/.test(value);
const driverVersion = (value) =>
  typeof value === "string" && /^[0-9]{1,8}(\.[0-9]{1,8}){0,3}$/.test(value);

function observation(input, accepts, sanitize) {
  if (input?.status === "UNKNOWN") {
    shape(input, ["status", "reason"]);
    return UNKNOWN(member(input.reason, REASONS));
  }
  shape(input, ["status", "value"]);
  if (input.status !== "OBSERVED") fail("invalid_observation");
  if (!accepts(input.value)) {
    if (sanitize) return UNKNOWN("malformed_value");
    fail("invalid_observation_value");
  }
  return { status: "OBSERVED", value: input.value };
}

function project(input, sanitize = true) {
  const reportKeys = [
    "schema_version",
    "scope",
    "provenance",
    "system",
    "devices",
  ];
  if (!sanitize) reportKeys.push("coverage", "kolvrt_execution", "publication");
  shape(input, reportKeys);
  if (input.schema_version !== 1 || input.scope !== "research_host")
    fail("unsupported_scope_or_version");
  shape(input.provenance, ["collector", "version", "input_kind", "providers"]);
  if (
    input.provenance.collector !== "kolvrt.windows-cim-survey" ||
    input.provenance.version !== 1
  )
    fail("unsupported_collector");
  const kind = member(input.provenance.input_kind, [
    "WINDOWS_CIM",
    "SYNTHETIC_FIXTURE",
    "UNSUPPORTED_PLATFORM",
  ]);
  if (
    !Array.isArray(input.provenance.providers) ||
    input.provenance.providers.length !== PROVIDERS.length
  )
    fail("provider_coverage");
  const seen = new Set();
  const providers = input.provenance.providers
    .map((entry) => {
      shape(entry, ["name", "outcome"]);
      const name = member(entry.name, PROVIDERS);
      if (seen.has(name)) fail("duplicate_provider");
      seen.add(name);
      return { name, outcome: member(entry.outcome, ["OBSERVED", ...REASONS]) };
    })
    .sort((a, b) => compare(a.name, b.name));
  shape(input.system, [
    "architecture",
    "cpus",
    "ram_bytes",
    "hypervisor_present",
    "iommu",
  ]);
  if (!Array.isArray(input.system.cpus) || input.system.cpus.length > 64)
    fail("cpu_budget");
  const obs = (value, accepts) => observation(value, accepts, sanitize);
  const cpus = input.system.cpus
    .map((cpu) => {
      shape(cpu, [
        "model",
        "cores",
        "logical_processors",
        "firmware_virtualization",
      ]);
      return {
        model: obs(cpu.model, safeModel),
        cores: obs(cpu.cores, integer(4096)),
        logical_processors: obs(cpu.logical_processors, integer(8192)),
        firmware_virtualization: obs(
          cpu.firmware_virtualization,
          (value) => typeof value === "boolean",
        ),
      };
    })
    .sort((a, b) => compare(JSON.stringify(a), JSON.stringify(b)));
  const system = {
    architecture: obs(input.system.architecture, (value) =>
      ["x86_64", "x86", "aarch64", "aarch32"].includes(value),
    ),
    cpus,
    ram_bytes: obs(input.system.ram_bytes, integer(2 ** 50)),
    hypervisor_present: obs(
      input.system.hypervisor_present,
      (value) => typeof value === "boolean",
    ),
    iommu: obs(input.system.iommu, () => false),
  };
  if (input.system.iommu.status !== "UNKNOWN") fail("unsupported_iommu_claim");
  if (!Array.isArray(input.devices) || input.devices.length > MAX_DEVICES)
    fail("device_budget");
  const devices = input.devices.map((device, index) => {
    const keys = ["bus", "category", "ids", "driver"];
    if (!sanitize) keys.push("alias");
    shape(device, keys);
    const bus = member(device.bus, ["PCI", "USB", "OTHER"]);
    const category = member(device.category, CLASSES);
    shape(device.ids, [
      "vendor_id",
      "device_id",
      "subsystem_vendor_id",
      "subsystem_device_id",
      "class_id",
    ]);
    shape(device.driver, ["inf_name", "version"]);
    const ids = {
      vendor_id: obs(device.ids.vendor_id, hex(4)),
      device_id: obs(device.ids.device_id, hex(4)),
      subsystem_vendor_id: obs(device.ids.subsystem_vendor_id, hex(4)),
      subsystem_device_id: obs(device.ids.subsystem_device_id, hex(4)),
      class_id: obs(device.ids.class_id, hex(bus === "USB" ? 2 : 6)),
    };
    if (
      bus === "OTHER" &&
      Object.values(ids).some((value) => value.status !== "UNKNOWN")
    )
      fail("unsupported_bus_ids");
    if (
      bus === "USB" &&
      [ids.subsystem_vendor_id, ids.subsystem_device_id].some(
        (value) => value.status !== "UNKNOWN",
      )
    )
      fail("unsupported_subsystem_ids");
    if (
      (ids.subsystem_vendor_id.status === "OBSERVED") !==
      (ids.subsystem_device_id.status === "OBSERVED")
    )
      fail("partial_subsystem_identity");
    if (
      (ids.vendor_id.status === "OBSERVED") !==
      (ids.device_id.status === "OBSERVED")
    )
      fail("partial_bus_identity");
    if (
      !sanitize &&
      device.alias !== `device-${String(index + 1).padStart(4, "0")}`
    )
      fail("invalid_alias");
    return {
      bus,
      category,
      ids,
      driver: {
        inf_name: obs(device.driver.inf_name, infName),
        version: obs(device.driver.version, driverVersion),
      },
    };
  });
  const observed = (value) => value.status === "OBSERVED";
  const providerObserved = (name) =>
    providers.find((provider) => provider.name === name).outcome === "OBSERVED";
  if (
    !providerObserved("Win32_Processor") &&
    (cpus.length || observed(system.architecture))
  )
    fail("inconsistent_provider");
  if (
    !providerObserved("Win32_ComputerSystem") &&
    [system.ram_bytes, system.hypervisor_present].some(observed)
  )
    fail("inconsistent_provider");
  if (!providerObserved("Win32_PnPEntity") && devices.length)
    fail("inconsistent_provider");
  if (
    !providerObserved("Win32_PnPSignedDriver") &&
    devices.some((device) => Object.values(device.driver).some(observed))
  )
    fail("inconsistent_provider");
  if (
    kind === "UNSUPPORTED_PLATFORM" &&
    providers.some((provider) => provider.outcome !== "unsupported_platform")
  )
    fail("inconsistent_provider");
  devices.sort((a, b) => compare(JSON.stringify(a), JSON.stringify(b)));
  const aliased = devices.map((device, index) => ({
    alias: `device-${String(index + 1).padStart(4, "0")}`,
    ...device,
  }));
  const coverage = {
    inventory: "PARTIAL",
    reason: "pnp_observation_is_not_exhaustive",
    unobserved_classes: CLASSES.filter(
      (category) => !devices.some((device) => device.category === category),
    ),
  };
  const result = {
    schema_version: 1,
    scope: "research_host",
    provenance: {
      collector: "kolvrt.windows-cim-survey",
      version: 1,
      input_kind: kind,
      providers,
    },
    system,
    devices: aliased,
    coverage,
    kolvrt_execution: "NOT_RUN",
    publication: "LOCAL_REVIEW_REQUIRED",
  };
  if (!sanitize && JSON.stringify(input) !== JSON.stringify(result)) {
    // Require semantic equality while allowing JSON object key ordering.
    const canonical = (value) =>
      Array.isArray(value)
        ? value.map(canonical)
        : value && typeof value === "object"
          ? Object.fromEntries(
              Object.keys(value)
                .sort()
                .map((key) => [key, canonical(value[key])]),
            )
          : value;
    if (JSON.stringify(canonical(input)) !== JSON.stringify(canonical(result)))
      fail("inconsistent_report");
  }
  return result;
}

function render(report) {
  const text = JSON.stringify(report, null, 2) + "\n";
  if (Buffer.byteLength(text, "utf8") > MAX_OUTPUT) fail("output_budget");
  return text;
}
function readBounded(file) {
  let fd = 0;
  if (file !== "-") {
    if (!fs.lstatSync(file).isFile()) fail("input_not_regular");
    fd = fs.openSync(file, "r");
  }
  const chunks = [];
  let length = 0;
  try {
    for (;;) {
      const chunk = Buffer.alloc(16384),
        count = fs.readSync(fd, chunk, 0, chunk.length, null);
      if (!count) break;
      length += count;
      if (length > MAX_INPUT) fail("input_budget");
      chunks.push(chunk.subarray(0, count));
    }
    return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(
      Buffer.concat(chunks),
    );
  } finally {
    if (fd !== 0) fs.closeSync(fd);
  }
}
function cli(args) {
  if (args.length === 0 || (args.length === 1 && args[0] === "--help")) {
    process.stdout.write(
      "Usage: node scripts/host-survey.cjs sanitize INPUT.json|- | validate REPORT.json|-\nLocal output only. Review before separately authorizing publication.\n",
    );
    return;
  }
  if (args.length !== 2 || !["sanitize", "validate"].includes(args[0]))
    fail("invalid_arguments");
  const input = parse(readBounded(args[1]));
  if (args[0] === "sanitize") process.stdout.write(render(project(input)));
  else {
    render(project(input, false));
    process.stdout.write(
      '{"status":"valid","publication":"LOCAL_REVIEW_REQUIRED"}\n',
    );
  }
}
module.exports = {
  parse,
  project,
  render,
  MAX_INPUT,
  MAX_OUTPUT,
  MAX_DEVICES,
  CLASSES,
  PROVIDERS,
  UNKNOWN,
};
if (require.main === module) {
  try {
    cli(process.argv.slice(2));
  } catch (error) {
    const codes = new Set([
      "input_budget",
      "output_budget",
      "structure_budget",
      "invalid_json",
      "duplicate_key",
      "invalid_shape",
      "unknown_or_missing_field",
      "invalid_enum",
      "invalid_observation",
      "invalid_observation_value",
      "unsupported_scope_or_version",
      "unsupported_collector",
      "provider_coverage",
      "duplicate_provider",
      "cpu_budget",
      "unsupported_iommu_claim",
      "device_budget",
      "unsupported_bus_ids",
      "unsupported_subsystem_ids",
      "partial_subsystem_identity",
      "partial_bus_identity",
      "invalid_alias",
      "inconsistent_provider",
      "inconsistent_report",
      "input_not_regular",
      "invalid_arguments",
    ]);
    // Never print exception text, paths, unknown keys or raw input values.
    process.stderr.write(
      `host-survey: ${codes.has(error.message) ? error.message : "invalid_or_unavailable_input"}\n`,
    );
    process.exitCode = 1;
  }
}
