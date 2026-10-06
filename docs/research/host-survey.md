# Local research-host surveys

Document status: CURRENT
Evidence scope: versioned Windows host-tool framework, synthetic provider experiments and schema/CLI checks; no actual host inventory collected or published.
Current reference: [Survey projection](../../scripts/host-survey.cjs)

<a name="kolvrt-host-survey"></a>

## Scope and commands

This is the host-tool delivery for #53, separate from the physical first-machine work in #58. Windows x86_64 is a valid `research_host`; it does not establish KOLVRT x86 boot support. Every report fixes `kolvrt_execution` to `NOT_RUN`. It cannot become a `boot_target` report or a driver/isolation acceptance result.

The implementation extends existing Node.js research scripts with a narrow PowerShell collector, rather than adding a kernel service or a second registry. Node.js 18+ and PowerShell 7 are required for Windows collection. No command uploads data, opens an issue, elevates privileges or changes device state.

```powershell
# Help only: no providers are queried.
pwsh -NoProfile -File scripts/collect-host-survey.ps1

# Synthetic input only.
node scripts/host-survey.cjs sanitize research/hardware/fixtures/synthetic-survey-input.json
node scripts/host-survey.cjs validate research/hardware/fixtures/synthetic-survey-report.json

# An explicit local collection, to be run separately by the operator.
pwsh -NoProfile -File scripts/collect-host-survey.ps1 -Collect > target/host-survey.local.json
node scripts/host-survey.cjs validate target/host-survey.local.json
```

Local workflow: explicitly collect → project through the allowlist → validate → review the exact report → separately authorize publication. Validation is not publication consent or proof of anonymity. Public model/ID combinations can still describe a distinctive configuration. Real collection was not run for this change.

## Provider allowlist

The [collector](../../scripts/collect-host-survey.ps1) queries only these CIM properties. It never serializes a complete provider object, exception, instance path or diagnostic command output.

| Provider                | Requested properties                                                                                  | Exported meaning                                                                                 |
| ----------------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `Win32_Processor`       | `Architecture`, `Name`, `NumberOfCores`, `NumberOfLogicalProcessors`, `VirtualizationFirmwareEnabled` | Host architecture, restricted model text, per-record counts and reported firmware virtualization |
| `Win32_ComputerSystem`  | `TotalPhysicalMemory`, `HypervisorPresent`                                                            | RAM bytes and reported hypervisor presence                                                       |
| `Win32_PnPEntity`       | `PNPDeviceID`, `PNPClass`, `HardwareID`, `CompatibleID`, filtered by `Present = TRUE`                 | PCI/USB public codes and bounded class labels; other buses keep IDs UNKNOWN                      |
| `Win32_PnPSignedDriver` | `DeviceID`, `InfName`, `DriverVersion`                                                                | Matching driver's INF basename and numeric version, not a binary path or loaded-module identity  |

Provider semantics come from Microsoft's [Processor](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-processor), [ComputerSystem](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-computersystem), [PnPEntity](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-pnpentity) and [PnPSignedDriver](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/legacy/aa394354%28v%3Dvs.85%29) documentation, reviewed 2026-10-05. These are live references, not immutable provider or driver support pins.

CIM wrappers and full PnP IDs are private, transient intermediates for the device-to-driver join. Only the hardware component before the instance suffix is parsed for public PCI/USB IDs; suffixes are never exported or hashed. Full provider objects and instance IDs stay in the collector process; selected model/driver text passes through a private in-memory pipe for validation, with no raw file/cache. Duplicate private instance rows are deduplicated only when projected facts agree; conflicting rows discard device results with a reasoned gap. Ambiguous driver joins return UNKNOWN. Equal public IDs remain separate PnP observations: they may represent multiple devices or interfaces.

The collector attempts current-user reads only. Failure or denied access produces a fixed reason code, never the exception message. Missing facts remain UNKNOWN; private-looking or out-of-format text becomes `malformed_value`. This is field minimization with restricted strings, not an infallible detector of secrets disguised as a legitimate model name. Review remains required.

## Closed export and limits

[Schema v1](../../schemas/host-survey.schema.json) is a closed JSON Schema Draft 2020-12 export. The CLI adds semantic checks for provider/fact consistency, paired vendor/device and PCI subsystem IDs, bus-specific class widths, report-local aliases and canonical ordering. Duplicate JSON keys, unknown fields, unsupported versions/scopes and inconsistent declarations fail before stdout.

CPU model/count, RAM, PCI vendor/device/subsystem/class codes, USB vendor/product/class codes and available driver metadata are the only machine facts. MAC/IP/SSID, account/host names, serials, credentials, personal paths, private instance identifiers, firmware dumps and free-form error text have no export fields. PCI class codes use six hex digits, USB compatible class codes two; an unreported programming interface is not filled with zero.

Storage, network, GPU, audio, Bluetooth, HID and other present PnP classes are classified without exporting unrestricted names. `coverage.inventory` is always `PARTIAL`; an unobserved class does not mean no such hardware exists. Entries count observations, not verified physical devices. Aliases such as `device-0001` are regenerated from sorted public facts inside each report; they are neither stable machine identifiers nor authority.

Limits: 1 MiB input, 128 KiB rendered output, 16 JSON nesting levels, 32,768 nodes, 64 processor records, 256 PnP records and 512 driver records. Provider exhaustion produces UNKNOWN without a truncated inventory. Export budget failure is atomic: nonzero exit and empty stdout. The CIM timeout request is ten seconds per query; it is not a guaranteed total deadline for an arbitrary provider.

Virtualization facts do not prove IOMMU presence or enabled per-device DMA containment. This collector has no reviewed provider for that guarantee, so IOMMU remains UNKNOWN with `unsupported_provider`. Non-Windows collection produces an unsupported-platform report rather than guessing.

## Authority and evidence

The design follows the [native contracts](../architecture/native-model.md), [security boundary](../architecture-decisions/0013-security-boundaries.md) and [placement decision](../architecture-decisions/0008-placement.md). Observations do not grant MMIO/IRQ/DMA rights, confirm upstream Linux support or permit compatibility exceptions. COST-L metadata and device support declarations remain distinct. Later device/driver architecture must be derived independently; this host format cannot freeze native ABI or driver binding.

[Tests](../../scripts/tests/host-survey.test.cjs) execute the real CLI and collector with synthetic, shadowed CIM providers. They cover nested sensitive fields, paths/messages, denied/missing providers, malformed IDs, duplicate private/public identities, ambiguous drivers, empty inventory, unsupported collection, strict JSON and actual input/output/record exhaustion. The [schema test](../../crates/repository-checks/tests/host_survey_schema.rs) checks real CLI output against the closed schema and retained synthetic report, with forbidden-field and fake-IOMMU negatives. Both suites run in `npm run check`.

[Exact-source host evidence](../../research/results/issue53-host-survey.json) covers only synthetic experiments. Actual Windows-provider collection, real ARM64 inventory, upstream-driver mapping (#54) and physical KOLVRT execution remain unverified. Use the [issue-submission privacy rules](issue-submissions.md) for any separately authorized publication.

[Russian translation](../../translations/ru/docs/research/host-survey.md)

<!-- knowledge -->

```json
{
  "kind": "subsystem-contract",
  "schema_version": 1,
  "units": [
    {
      "id": "kolvrt.research.host-survey",
      "anchor": "kolvrt-host-survey",
      "depends_on": [
        "law.001",
        "law.009",
        "law.013",
        "law.031",
        "law.036",
        "law.040",
        "adr.0008",
        "adr.0013"
      ],
      "kind": "feature",
      "tags": ["hardware", "survey", "privacy", "research_host"],
      "feature": {
        "verification": [
          {
            "receipt_sha256": "cc07b61d771e202ca0ffc7025fd3070a3c27d179c40704618b3dccb51b896beb",
            "state": "STALE",
            "reason": "Phase 3.7 extends package.json with native QEMU correctness commands. The immutable issue53 synthetic receipt retains its earlier exact-source package integration; framework semantics and hardware-collection boundaries are unchanged, and current-source synthetic acceptance is not inferred from that old receipt.",
            "scope": "Synthetic fixtures and shadowed CIM providers only; no actual hardware collection or kernel execution.",
            "receipt": "research/results/issue53-host-survey.json",
            "environment": "host-process"
          },
          {
            "state": "UNKNOWN",
            "environment": "windows-cim",
            "reason": "Phase 3.7 separates ordinary implementation from isolated mutation builds and standalone ELF selftests. Prior receipts retain their original source scope; the changed source set requires fresh reviewed exact-source acceptance. Local partial passes do not establish the full declared gate."
          },
          {
            "state": "NOT_APPLICABLE",
            "environment": "physical-arm64",
            "reason": "This host research framework cannot establish physical KOLVRT kernel/device support."
          }
        ],
        "adrs": ["adr.0008", "adr.0013"],
        "implementation": "BOUNDED_IMPLEMENTED",
        "readiness": "NOT_READY",
        "sources": [
          "scripts/host-survey.cjs",
          "scripts/collect-host-survey.ps1",
          "scripts/tests/host-survey.test.cjs",
          "crates/repository-checks/tests/host_survey_schema.rs",
          "schemas/host-survey.schema.json",
          "research/hardware/fixtures/synthetic-survey-input.json",
          "research/hardware/fixtures/synthetic-survey-report.json",
          "package.json"
        ],
        "next_gate": "Review the exact local report and separately authorize publication before #58 first-machine research; rederive device/binding authority independently before runtime use.",
        "transitions": [
          {
            "acceptance": ["research/results/issue53-host-survey.json"],
            "to": "BOUNDED_IMPLEMENTED",
            "reason": "First local allowlisted framework with exact-source synthetic host acceptance; physical collection and publication remain separate decisions.",
            "from": "UNRECORDED"
          }
        ],
        "roadmap_gate": "COST-L host survey #53",
        "issues": [53],
        "limitations": [
          "Actual Windows-provider collection and real ARM64 inventory have not run. Reports are unauthenticated research observations, not boot_target/driver/DMA guarantees or publication consent. #54 driver mapping and #58 physical first-machine review remain separate."
        ],
        "implementation_scope": "Versioned local Windows research-host collection with a closed allowlist, Node projection/validation, explicit gaps, bounded output and synthetic integration evidence.",
        "acceptance": ["research/results/issue53-host-survey.json"]
      },
      "summary": "Local allowlisted research-host observations with synthetic verification and no publication or device authority."
    }
  ],
  "id": "doc.kolvrt.research.host-survey",
  "summary": "Local allowlisted research-host observations with synthetic verification and no publication or device authority."
}
```
