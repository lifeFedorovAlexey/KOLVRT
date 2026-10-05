"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const {
  parse,
  project,
  render,
  MAX_INPUT,
  MAX_OUTPUT,
  CLASSES,
  UNKNOWN,
} = require("../host-survey.cjs");
const root = path.resolve(__dirname, "../..");
const cli = path.join(root, "scripts/host-survey.cjs");
const inputPath = path.join(
  root,
  "research/hardware/fixtures/synthetic-survey-input.json",
);
const fixture = () => JSON.parse(fs.readFileSync(inputPath, "utf8"));
const run = (mode, input) =>
  spawnSync(process.execPath, [cli, mode, "-"], {
    input:
      typeof input === "string" || Buffer.isBuffer(input)
        ? input
        : JSON.stringify(input),
    encoding: "utf8",
    maxBuffer: 2 * MAX_INPUT,
  });
const reject = (result, secret = "DO_NOT_EXPORT") => {
  assert.notEqual(result.status, 0);
  assert.equal(result.stdout, "");
  assert.ok(!result.stderr.includes(secret));
  assert.match(result.stderr, /^host-survey: [a-z_]+\n$/);
};

test("real CLI roundtrip distinguishes research host, synthetic provenance and missing classes", () => {
  const result = run("sanitize", fixture());
  assert.equal(result.status, 0, result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.scope, "research_host");
  assert.equal(report.kolvrt_execution, "NOT_RUN");
  assert.equal(report.provenance.input_kind, "SYNTHETIC_FIXTURE");
  assert.equal(report.system.architecture.value, "x86_64");
  assert.equal(report.system.iommu.status, "UNKNOWN");
  assert.deepEqual(
    report.devices.map((device) => device.alias),
    ["device-0001", "device-0002"],
  );
  assert.ok(report.coverage.unobserved_classes.includes("storage"));
  assert.equal(run("validate", report).status, 0);
  const reverse = fixture();
  reverse.devices.reverse();
  reverse.provenance.providers.reverse();
  assert.equal(render(project(reverse)), result.stdout);
});

test("unknown/private fields at every input boundary fail atomically without echo", () => {
  const targets = [
    (v) => v,
    (v) => v.system,
    (v) => v.system.cpus[0],
    (v) => v.system.cpus[0].model,
    (v) => v.devices[0],
    (v) => v.devices[0].ids,
    (v) => v.devices[0].driver,
    (v) => v.provenance,
    (v) => v.provenance.providers[0],
  ];
  for (const target of targets) {
    const value = fixture();
    target(value).private = {
      hostname: "DO_NOT_EXPORT",
      path: "C:\\Users\\DO_NOT_EXPORT",
      mac: "00:11:22:33:44:55",
      password: "DO_NOT_EXPORT",
    };
    reject(run("sanitize", value));
  }
});

test("unsafe strings, nested values and invalid counts become reasoned UNKNOWN, not zero", () => {
  for (const secret of [
    "C:\\Users\\DO_NOT_EXPORT",
    "/home/DO_NOT_EXPORT",
    "DO_NOT_EXPORT@example.invalid",
    "00:11:22:33:44:55",
    "192.0.2.4",
    "password DO_NOT_EXPORT",
    { serial: "DO_NOT_EXPORT" },
  ]) {
    const value = fixture();
    value.system.cpus[0].model.value = secret;
    value.devices[0].driver.inf_name.value = secret;
    const result = run("sanitize", value);
    assert.equal(result.status, 0, result.stderr);
    assert.ok(!result.stdout.includes("DO_NOT_EXPORT"));
    const report = JSON.parse(result.stdout);
    assert.deepEqual(report.system.cpus[0].model, UNKNOWN("malformed_value"));
    assert.deepEqual(
      report.devices[0].driver.inf_name,
      UNKNOWN("malformed_value"),
    );
  }
  const value = fixture();
  value.system.ram_bytes.value = 0;
  value.system.cpus[0].cores.value = -1;
  const report = project(value);
  assert.equal(report.system.ram_bytes.status, "UNKNOWN");
  assert.equal(report.system.cpus[0].cores.status, "UNKNOWN");
});

test("ID pairing, bus scope, provider truth, aliases and invented protection claims reject", () => {
  const mutations = [
    (v) => {
      v.scope = "boot_target";
    },
    (v) => {
      v.schema_version = 2;
    },
    (v) => {
      v.devices[0].ids.vendor_id.value = "808G";
    },
    (v) => {
      v.devices[0].ids.subsystem_vendor_id = UNKNOWN();
    },
    (v) => {
      v.devices[1].ids.subsystem_vendor_id = {
        status: "OBSERVED",
        value: "1234",
      };
    },
    (v) => {
      v.devices[0].bus = "OTHER";
    },
    (v) => {
      v.system.iommu = { status: "OBSERVED", value: true };
    },
    (v) => {
      v.provenance.providers[0].outcome = "permission_denied";
    },
    (v) => {
      v.provenance.providers[1] = v.provenance.providers[0];
    },
    (v) => {
      v.system.iommu.reason = "DO_NOT_EXPORT exception text";
    },
  ];
  for (const mutate of mutations) {
    const value = fixture();
    mutate(value);
    reject(run("sanitize", value));
  }
  const report = project(fixture());
  report.devices[0].alias = "DO_NOT_EXPORT";
  reject(run("validate", report));
});

test("empty/missing inventory and repeated public IDs cannot invent absence or stable identity", () => {
  const empty = fixture();
  empty.devices = [];
  assert.deepEqual(project(empty).coverage.unobserved_classes, CLASSES);
  const repeated = fixture();
  repeated.devices.push(structuredClone(repeated.devices[0]));
  const report = project(repeated);
  assert.equal(report.devices.length, 3);
  assert.equal(new Set(report.devices.map((device) => device.alias)).size, 3);
  assert.equal(report.coverage.inventory, "PARTIAL");
  const unavailable = fixture();
  unavailable.system.cpus = [];
  unavailable.devices = [];
  for (const provider of unavailable.provenance.providers)
    provider.outcome = "unsupported_platform";
  unavailable.provenance.input_kind = "UNSUPPORTED_PLATFORM";
  for (const field of ["architecture", "ram_bytes", "hypervisor_present"])
    unavailable.system[field] = UNKNOWN("unsupported_platform");
  assert.equal(run("sanitize", unavailable).status, 0);
});

test("bounded JSON rejects duplicate keys, invalid UTF-8, depth, nodes and invalid trailing text", () => {
  assert.throws(() => parse('{"a":1,"a":2}'), /duplicate_key/);
  assert.throws(() => parse('{"a":1,"\\u0061":2}'), /duplicate_key/);
  for (const input of [
    '{"a":[{"x":1,"x":2}]}',
    "[".repeat(18) + "0" + "]".repeat(18),
    "[0,]",
    "{} x",
    "\u00a0{}",
    "\uFEFF{}",
    "[" + "0,".repeat(32768) + "0]",
  ])
    reject(run("sanitize", input));
  reject(run("sanitize", Buffer.from([0xff, 0xfe])));
});

test("real input, device and rendered-output budgets reject before stdout", () => {
  reject(run("sanitize", " ".repeat(MAX_INPUT + 1)));
  const overDevices = fixture();
  overDevices.devices = Array.from({ length: 257 }, () =>
    structuredClone(overDevices.devices[0]),
  );
  reject(run("sanitize", overDevices));
  const large = fixture();
  large.devices = Array.from({ length: 256 }, () =>
    structuredClone(large.devices[0]),
  );
  const report = project(large);
  assert.ok(Buffer.byteLength(JSON.stringify(report, null, 2)) > MAX_OUTPUT);
  reject(run("sanitize", large));
  reject(run("validate", report));
  large.devices = large.devices.slice(0, 64);
  assert.equal(run("sanitize", large).status, 0);
});

test("default CLI is help only and missing private input paths never appear in errors", () => {
  const help = spawnSync(process.execPath, [cli], { encoding: "utf8" });
  assert.equal(help.status, 0);
  assert.match(help.stdout, /Usage:/);
  const missing = spawnSync(
    process.execPath,
    [cli, "sanitize", "C:\\DO_NOT_EXPORT\\missing.json"],
    { encoding: "utf8" },
  );
  reject(missing);
});

// Execute the real collector with a shadowed CIM provider. Never query the test
// machine. These are controlled host-process experiments, not hardware surveys.
function powershell(mockBody, collect = true) {
  const script = path
    .join(root, "scripts/collect-host-survey.ps1")
    .replace(/'/g, "''");
  const command = `function Get-CimInstance { param($ClassName,$Property,$OperationTimeoutSec,$Filter,$ErrorAction) ${mockBody} }\n& '${script}' ${collect ? "-Collect" : ""}\n`;
  return spawnSync(
    "pwsh",
    ["-NoProfile", "-NonInteractive", "-Command", command],
    { encoding: "utf8", timeout: 15000, maxBuffer: 2 * MAX_INPUT },
  );
}
test("collector defaults to help without touching providers", () => {
  const result = powershell(
    "[Console]::Error.WriteLine('DO_NOT_EXPORT'); throw 'DO_NOT_EXPORT'",
    false,
  );
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stderr, "");
  assert.match(result.stdout, /Usage:/);
});
test(
  "collector converts sensitive provider failures into typed gaps without logging text",
  { skip: process.platform !== "win32" },
  () => {
    const result = powershell(
      "throw [UnauthorizedAccessException]::new('DO_NOT_EXPORT C:\\Users\\DO_NOT_EXPORT')",
    );
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stderr, "");
    assert.ok(!result.stdout.includes("DO_NOT_EXPORT"));
    const report = JSON.parse(result.stdout);
    assert.ok(
      report.provenance.providers.every(
        (provider) => provider.outcome === "permission_denied",
      ),
    );
    assert.equal(report.system.ram_bytes.status, "UNKNOWN");
    assert.deepEqual(report.devices, []);
  },
);
test(
  "collector uses narrow properties, public IDs and private transient joins across device classes",
  { skip: process.platform !== "win32" },
  () => {
    const result = powershell(String.raw`
    $expected=@{Win32_Processor=@('Architecture','Name','NumberOfCores','NumberOfLogicalProcessors','VirtualizationFirmwareEnabled');Win32_ComputerSystem=@('TotalPhysicalMemory','HypervisorPresent');Win32_PnPEntity=@('PNPDeviceID','PNPClass','HardwareID','CompatibleID');Win32_PnPSignedDriver=@('DeviceID','InfName','DriverVersion')}
    if (($Property|Sort-Object) -join ',' -ne (($expected[$ClassName]|Sort-Object) -join ',')) { throw 'DO_NOT_EXPORT unexpected property' }
    if ($OperationTimeoutSec -ne 10) { throw 'DO_NOT_EXPORT timeout' }
    switch ($ClassName) {
      Win32_Processor { [pscustomobject]@{Architecture=9;Name='Synthetic Fixture CPU';NumberOfCores=4;NumberOfLogicalProcessors=8;VirtualizationFirmwareEnabled=$true} }
      Win32_ComputerSystem { [pscustomobject]@{TotalPhysicalMemory=[long]8589934592;HypervisorPresent=$true} }
      Win32_PnPSignedDriver { [pscustomobject]@{DeviceID='PCI\VEN_8086&DEV_100E&SUBSYS_56781234\DO_NOT_EXPORT';InfName='fixture-net.inf';DriverVersion='1.2.3.4'} }
      Win32_PnPEntity {
        if ($Filter -ne 'Present = TRUE') { throw 'DO_NOT_EXPORT missing filter' }
        [pscustomobject]@{PNPDeviceID='PCI\VEN_8086&DEV_100E&SUBSYS_56781234\DO_NOT_EXPORT';PNPClass='Net';HardwareID=@('PCI\VEN_8086&DEV_100E&CC_020000');CompatibleID=@()}
        [pscustomobject]@{PNPDeviceID='USB\VID_1234&PID_5678\SERIAL_DO_NOT_EXPORT';PNPClass='HIDClass';HardwareID=@();CompatibleID=@('USB\Class_03&SubClass_01&Prot_01')}
        [pscustomobject]@{PNPDeviceID='USB\VID_1234&PID_5678\ANOTHER_DO_NOT_EXPORT';PNPClass='HIDClass';HardwareID=@();CompatibleID=@('USB\Class_03')}
        foreach ($class in @('DiskDrive','Display','MEDIA','Bluetooth','System')) { [pscustomobject]@{PNPDeviceID=('ROOT\'+$class+'\DO_NOT_EXPORT');PNPClass=$class;HardwareID=@();CompatibleID=@()} }
      }
    }
  `);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stderr, "");
    assert.ok(!result.stdout.includes("DO_NOT_EXPORT"));
    const report = JSON.parse(result.stdout);
    assert.equal(report.devices.length, 8);
    assert.deepEqual(report.coverage.unobserved_classes, []);
    const pci = report.devices.find((device) => device.bus === "PCI");
    assert.equal(pci.ids.vendor_id.value, "8086");
    assert.equal(pci.ids.subsystem_vendor_id.value, "1234");
    assert.equal(pci.ids.subsystem_device_id.value, "5678");
    assert.equal(pci.ids.class_id.value, "020000");
    assert.equal(pci.driver.inf_name.value, "fixture-net.inf");
    assert.ok(
      report.devices
        .filter((device) => device.bus === "USB")
        .every((device) => device.ids.class_id.value === "03"),
    );
    assert.equal(run("validate", report).status, 0);
  },
);

test(
  "collector deduplicates private rows and refuses ambiguous driver metadata",
  { skip: process.platform !== "win32" },
  () => {
    const result = powershell(
      [
        "if ($ClassName -eq 'Win32_PnPEntity') {",
        "1..2 | ForEach-Object { [pscustomobject]@{PNPDeviceID='PCI\\VEN_8086&DEV_100E\\DO_NOT_EXPORT';PNPClass='Net';HardwareID=@();CompatibleID=@()} }",
        "} elseif ($ClassName -eq 'Win32_PnPSignedDriver') {",
        "1..2 | ForEach-Object { [pscustomobject]@{DeviceID='PCI\\VEN_8086&DEV_100E\\DO_NOT_EXPORT';InfName='fixture-net.inf';DriverVersion=($_.ToString()+'.0')} }",
        "}",
      ].join("\n"),
    );
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stderr, "");
    assert.ok(!result.stdout.includes("DO_NOT_EXPORT"));
    const report = JSON.parse(result.stdout);
    assert.equal(report.devices.length, 1);
    assert.deepEqual(
      report.devices[0].driver.version,
      UNKNOWN("malformed_value"),
    );
  },
);

test(
  "collector exhaustion publishes UNKNOWN instead of a truncated inventory",
  { skip: process.platform !== "win32" },
  () => {
    const result = powershell(
      [
        "if ($ClassName -eq 'Win32_PnPEntity') {",
        "1..257 | ForEach-Object { [pscustomobject]@{PNPDeviceID=('ROOT\\DO_NOT_EXPORT\\'+$_);PNPClass='System';HardwareID=@();CompatibleID=@()} }",
        "}",
      ].join("\n"),
    );
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stderr, "");
    assert.ok(!result.stdout.includes("DO_NOT_EXPORT"));
    const report = JSON.parse(result.stdout);
    assert.deepEqual(report.devices, []);
    assert.equal(
      report.provenance.providers.find(
        (provider) => provider.name === "Win32_PnPEntity",
      ).outcome,
      "budget_exceeded",
    );
    assert.deepEqual(report.coverage.unobserved_classes, CLASSES);
  },
);
