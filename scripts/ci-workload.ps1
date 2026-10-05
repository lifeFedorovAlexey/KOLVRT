param([Parameter(Mandatory)][ValidateSet('static', 'host', 'kernel-dev', 'kernel-prod', 'matrix', 'routing', 'asid', 'evidence')][string]$Workload)
$ErrorActionPreference = 'Stop'
function Invoke-Check([string]$Kind, [string]$Label, [string]$Command, [string[]]$CommandArguments) {
    & "$PSScriptRoot/run-timed.ps1" -Kind $Kind -Label $Label -Command $Command -CommandArguments $CommandArguments
    if ($LASTEXITCODE -ne 0) { throw "Required check failed: $Label (exit $LASTEXITCODE)" }
}
switch ($Workload) {
    'static' { Invoke-Check 'static' 'check:static' 'npm.cmd' @('run', 'check:static') }
    'host' { Invoke-Check 'host' 'check:host' 'npm.cmd' @('run', 'check:host') }
    'kernel-dev' { Invoke-Check 'cargo-clippy' 'kernel-dev' 'npm.cmd' @('run', 'check:kernel-dev') }
    'kernel-prod' { Invoke-Check 'cargo-clippy' 'kernel-prod' 'npm.cmd' @('run', 'check:kernel-prod') }
    'matrix' {
        if ($env:CI_MATRIX_SHARD -ne '') { Invoke-Check 'matrix' "kernel-shard-$env:CI_MATRIX_SHARD" 'cargo' @('xtask', 'matrix-shard', $env:CI_MATRIX_SHARD, '4') }
        else { Invoke-Check 'matrix' 'kernel-matrix' 'cargo' @('xtask', 'test') }
    }
    'evidence' {
        cargo xtask matrix-plan | Set-Content target/kernel/expected-plan.json -Encoding utf8
        if ($LASTEXITCODE -ne 0) { throw 'Matrix plan generation failed' }
        node scripts/ci-evidence.cjs target/kernel/expected-plan.json target/shards target/kernel/ci-matrix.json
        if ($LASTEXITCODE -ne 0) { throw 'Incomplete or incompatible matrix evidence' }
    }
    'routing' { Invoke-Check 'matrix' 'routing-matrix' 'cargo' @('xtask', 'routing', 'test') }
    'asid' {
        $env:QEMU_CPU = 'max'
        Invoke-Check 'matrix' 'asid-paired' 'cargo' @('xtask', 'asid-bench')
        $receipt = Get-Content research/results/issue18-asid-measurements.json -Raw | ConvertFrom-Json
        Copy-Item -LiteralPath research/results/issue18-asid-measurements.json -Destination target/kernel/asid-measurements.json
        foreach ($pair in $receipt.pairs) {
            if ($pair.results.tagged.hardware_asid_bits -ne 16 -or $pair.results.baseline.hardware_asid_bits -ne 16) { throw 'Expected actual 16-bit ASID hardware profile' }
        }
    }
}
if ($Workload -eq 'static' -and $env:KNOWLEDGE_BASE) {
    Invoke-Check 'static' 'impact' 'cargo' @('run', '--locked', '-p', 'repository-checks', '--', 'docs', 'check-change', $env:KNOWLEDGE_BASE)
}
