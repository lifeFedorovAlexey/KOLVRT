param([switch] $Collect)

# Only explicit local collection invokes CIM. No upload, elevation or raw files.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$WarningPreference = 'SilentlyContinue'
$VerbosePreference = 'SilentlyContinue'
$DebugPreference = 'SilentlyContinue'
$InformationPreference = 'SilentlyContinue'

if (-not $Collect) {
    Write-Output 'Usage: pwsh -File scripts/collect-host-survey.ps1 -Collect'
    Write-Output 'Local sanitized JSON only. Review it before separately authorizing publication.'
    exit 0
}

function Unknown([string] $Reason = 'not_reported') {
    return @{ status = 'UNKNOWN'; reason = $Reason }
}
function Observed($Value) {
    if ($null -eq $Value) { return (Unknown) }
    if ($Value -is [string] -and $Value.Length -gt 128) { return (Unknown 'malformed_value') }
    return @{ status = 'OBSERVED'; value = $Value }
}
function Read-Provider([string] $Name, [string[]] $Properties, [int] $Limit, [string] $Filter = '') {
    try {
        $providerArgs = @{ ClassName = $Name; Property = $Properties; OperationTimeoutSec = 10; ErrorAction = 'Stop' }
        if ($Filter) { $providerArgs.Filter = $Filter }
        # Property allowlist and a bounded stream. Provider/CIM wrapper metadata
        # stays private in memory; it is never formatted, serialized or logged.
        $rows = @(Get-CimInstance @providerArgs | Select-Object -First ($Limit + 1))
        if ($rows.Count -gt $Limit) { return @{ Outcome = 'budget_exceeded'; Rows = @() } }
        if ($rows.Count -eq 0) { return @{ Outcome = 'not_reported'; Rows = @() } }
        return @{ Outcome = 'OBSERVED'; Rows = $rows }
    } catch {
        $reason = 'provider_unavailable'
        if ($_.Exception -is [UnauthorizedAccessException] -or $_.Exception.HResult -eq -2147024891) { $reason = 'permission_denied' }
        # Exception messages may contain account names, paths and device IDs.
        return @{ Outcome = $reason; Rows = @() }
    }
}
function Class-Observation([string] $Bus, $HardwareIDs, $CompatibleIDs) {
    $candidates = @($HardwareIDs) + @($CompatibleIDs)
    if ($candidates.Count -gt 32) { return (Unknown 'budget_exceeded') }
    $values = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($candidate in $candidates) {
        if ($candidate -isnot [string] -or $candidate.Length -gt 512) { continue }
        $pattern = if ($Bus -eq 'PCI') { '^PCI\\(?:[^\\]{0,128}&)?CC_([0-9A-F]{6})(?:&|$)' } else { '^USB\\Class_([0-9A-F]{2})(?:&|$)' }
        $match = [regex]::Match($candidate, $pattern, [Text.RegularExpressions.RegexOptions]::IgnoreCase)
        if ($match.Success) { [void] $values.Add($match.Groups[1].Value.ToUpperInvariant()) }
    }
    if ($values.Count -eq 1) { return (Observed (@($values)[0])) }
    if ($values.Count -gt 1) { return (Unknown 'malformed_value') }
    return (Unknown)
}
function Device-Observation($Device, $Driver) {
    # Full instance IDs are needed only for a private transient driver join.
    # Public PCI/USB identity is parsed from the hardware component, never from
    # a serial-bearing instance suffix and never by hashing that suffix.
    $id = $Device.PNPDeviceID
    $bus = 'OTHER'
    $ids = @{ vendor_id = (Unknown); device_id = (Unknown); subsystem_vendor_id = (Unknown); subsystem_device_id = (Unknown); class_id = (Unknown) }
    if ($id -is [string] -and $id.Length -le 1024) {
        $parts = $id -split '\\', 3
        if ($parts.Count -ge 2 -and $parts[0] -in @('PCI', 'USB')) {
            $bus = $parts[0].ToUpperInvariant()
            $pattern = if ($bus -eq 'PCI') { '^VEN_([0-9A-F]{4})&DEV_([0-9A-F]{4})(?:&|$)' } else { '^VID_([0-9A-F]{4})&PID_([0-9A-F]{4})(?:&|$)' }
            $match = [regex]::Match($parts[1], $pattern, [Text.RegularExpressions.RegexOptions]::IgnoreCase)
            if ($match.Success) {
                $ids.vendor_id = Observed $match.Groups[1].Value.ToUpperInvariant()
                $ids.device_id = Observed $match.Groups[2].Value.ToUpperInvariant()
            } else {
                $ids.vendor_id = Unknown 'malformed_value'
                $ids.device_id = Unknown 'malformed_value'
            }
            if ($bus -eq 'PCI') {
                $subsystem = [regex]::Match($parts[1], '&SUBSYS_([0-9A-F]{4})([0-9A-F]{4})(?:&|$)', [Text.RegularExpressions.RegexOptions]::IgnoreCase)
                if ($subsystem.Success) {
                    $ids.subsystem_device_id = Observed $subsystem.Groups[1].Value.ToUpperInvariant()
                    $ids.subsystem_vendor_id = Observed $subsystem.Groups[2].Value.ToUpperInvariant()
                }
            }
            $ids.class_id = Class-Observation $bus $Device.HardwareID $Device.CompatibleID
        }
    }
    $classes = @{ DiskDrive = 'storage'; SCSIAdapter = 'storage'; HDC = 'storage'; Storage = 'storage'; Net = 'network'; Display = 'gpu'; MEDIA = 'audio'; AudioEndpoint = 'audio'; Bluetooth = 'bluetooth'; HIDClass = 'hid'; Keyboard = 'hid'; Mouse = 'hid' }
    $category = 'other'
    if ($Device.PNPClass -is [string] -and $classes.ContainsKey($Device.PNPClass)) { $category = $classes[$Device.PNPClass] }
    $driverInfo = @{ inf_name = (Unknown); version = (Unknown) }
    if ($null -ne $Driver) {
        $driverInfo.inf_name = Observed $Driver.InfName
        $driverInfo.version = Observed $Driver.DriverVersion
    }
    return @{ bus = $bus; category = $category; ids = $ids; driver = $driverInfo }
}

try {
    $windows = [Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT
    $kind = if ($windows) { 'WINDOWS_CIM' } else { 'UNSUPPORTED_PLATFORM' }
    $cpus = @{ Outcome = 'unsupported_platform'; Rows = @() }
    $system = @{ Outcome = 'unsupported_platform'; Rows = @() }
    $pnp = @{ Outcome = 'unsupported_platform'; Rows = @() }
    $drivers = @{ Outcome = 'unsupported_platform'; Rows = @() }
    if ($windows) {
        $cpus = Read-Provider 'Win32_Processor' @('Architecture', 'Name', 'NumberOfCores', 'NumberOfLogicalProcessors', 'VirtualizationFirmwareEnabled') 64
        $system = Read-Provider 'Win32_ComputerSystem' @('TotalPhysicalMemory', 'HypervisorPresent') 1
        $pnp = Read-Provider 'Win32_PnPEntity' @('PNPDeviceID', 'PNPClass', 'HardwareID', 'CompatibleID') 256 'Present = TRUE'
        $drivers = Read-Provider 'Win32_PnPSignedDriver' @('DeviceID', 'InfName', 'DriverVersion') 512
    }
    $architecture = Unknown $(if ($cpus.Outcome -eq 'OBSERVED') { 'not_reported' } else { $cpus.Outcome })
    $architectureNames = @{ 0 = 'x86'; 5 = 'aarch32'; 9 = 'x86_64'; 12 = 'aarch64' }
    $architectureValues = @($cpus.Rows | ForEach-Object { $_.Architecture } | Select-Object -Unique)
    if ($architectureValues.Count -gt 1) { $architecture = Unknown 'malformed_value' }
    if ($architectureValues.Count -eq 1 -and $null -ne $architectureValues[0] -and $architectureNames.ContainsKey([int] $architectureValues[0])) {
        $architecture = Observed $architectureNames[[int] $architectureValues[0]]
    }
    $cpuFacts = @($cpus.Rows | ForEach-Object { @{ model = (Observed $_.Name); cores = (Observed $_.NumberOfCores); logical_processors = (Observed $_.NumberOfLogicalProcessors); firmware_virtualization = (Observed $_.VirtualizationFirmwareEnabled) } })
    $ram = Unknown $(if ($system.Outcome -eq 'OBSERVED') { 'not_reported' } else { $system.Outcome })
    $hypervisor = Unknown $(if ($system.Outcome -eq 'OBSERVED') { 'not_reported' } else { $system.Outcome })
    if ($system.Rows.Count -eq 1) {
        $ram = Observed $system.Rows[0].TotalPhysicalMemory
        $hypervisor = Observed $system.Rows[0].HypervisorPresent
    }
    $driverIndex = [Collections.Generic.Dictionary[string, object]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($driver in $drivers.Rows) {
        if ($driver.DeviceID -isnot [string] -or $driver.DeviceID.Length -gt 1024) { continue }
        if ($driverIndex.ContainsKey($driver.DeviceID)) { $driverIndex[$driver.DeviceID] = $null } else { $driverIndex.Add($driver.DeviceID, $driver) }
    }
    $deviceFacts = [Collections.Generic.List[object]]::new()
    $seen = [Collections.Generic.Dictionary[string, string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($device in $pnp.Rows) {
        $driver = $null
        $ambiguousDriver = $false
        if ($device.PNPDeviceID -is [string] -and $device.PNPDeviceID.Length -le 1024 -and $driverIndex.ContainsKey($device.PNPDeviceID)) { $driver = $driverIndex[$device.PNPDeviceID] }
        if ($device.PNPDeviceID -is [string] -and $device.PNPDeviceID.Length -le 1024 -and $driverIndex.ContainsKey($device.PNPDeviceID) -and $null -eq $driver) { $ambiguousDriver = $true }
        $fact = Device-Observation $device $driver
        if ($ambiguousDriver) { $fact.driver = @{ inf_name = (Unknown 'malformed_value'); version = (Unknown 'malformed_value') } }
        if ($drivers.Outcome -ne 'OBSERVED') { $fact.driver = @{ inf_name = (Unknown $drivers.Outcome); version = (Unknown $drivers.Outcome) } }
        if ($device.PNPDeviceID -is [string] -and $device.PNPDeviceID.Length -le 1024) {
            $signature = $fact | ConvertTo-Json -Depth 8 -Compress
            if ($seen.ContainsKey($device.PNPDeviceID)) {
                if ($seen[$device.PNPDeviceID] -ne $signature) { $pnp.Outcome = 'malformed_value'; $deviceFacts.Clear(); break }
                continue
            }
            $seen.Add($device.PNPDeviceID, $signature)
        }
        $deviceFacts.Add($fact)
    }
    $report = @{
        schema_version = 1; scope = 'research_host'
        provenance = @{ collector = 'kolvrt.windows-cim-survey'; version = 1; input_kind = $kind; providers = @(@{ name = 'Win32_Processor'; outcome = $cpus.Outcome }, @{ name = 'Win32_ComputerSystem'; outcome = $system.Outcome }, @{ name = 'Win32_PnPEntity'; outcome = $pnp.Outcome }, @{ name = 'Win32_PnPSignedDriver'; outcome = $drivers.Outcome }) }
        system = @{ architecture = $architecture; cpus = $cpuFacts; ram_bytes = $ram; hypervisor_present = $hypervisor; iommu = (Unknown 'unsupported_provider') }
        devices = @($deviceFacts.ToArray())
    }
    # Only the narrow DTO reaches the pure sanitizer. No raw intermediate file.
    $OutputEncoding = [Text.UTF8Encoding]::new($false)
    $report | ConvertTo-Json -Depth 12 -Compress | & node (Join-Path $PSScriptRoot 'host-survey.cjs') sanitize -
    if ($LASTEXITCODE -ne 0) { exit 1 }
} catch {
    [Console]::Error.WriteLine('host-survey: collection_failed')
    exit 1
}
