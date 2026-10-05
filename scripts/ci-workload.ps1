param([Parameter(Mandatory)][ValidateSet('baseline', 'static', 'host', 'kernel-dev', 'kernel-prod', 'matrix', 'routing', 'asid', 'evidence')][string]$Workload)
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
    'baseline' {
        # Retain the original ordered checks, including its duplicate exception subset.
        Invoke-Check 'host' 'original-npm-check' 'npm.cmd' @('run', 'check:dependencies')
        Invoke-Check 'static' 'original-format' 'npm.cmd' @('run', 'format:check')
        Invoke-Check 'host' 'original-lint' 'npm.cmd' @('run', 'lint')
        Invoke-Check 'host' 'original-research' 'npm.cmd' @('run', 'test:research')
        Invoke-Check 'cargo-test' 'original-workspace-tests' 'cargo' @('test', '--locked')
        Invoke-Check 'host' 'original-models' 'cargo' @('run', '--locked', '-p', 'native-state-models', '--', '--check')
        Invoke-Check 'static' 'original-repository' 'cargo' @('run', '--locked', '-p', 'repository-checks', '--', 'check')
        if ($env:KNOWLEDGE_BASE) { Invoke-Check 'static' 'original-impact' 'cargo' @('run', '--locked', '-p', 'repository-checks', '--', 'docs', 'check-change', $env:KNOWLEDGE_BASE) }
        Invoke-Check 'cargo-test' 'original-exceptions' 'cargo' @('test', '--locked', '-p', 'repository-checks', 'exceptions::tests')
        Invoke-Check 'cargo-test' 'routing-all-features' 'cargo' @('test', '--locked', '-p', 'routing', '--all-features')
        Invoke-Check 'cargo-test' 'routing-no-default' 'cargo' @('test', '--locked', '-p', 'routing', '--no-default-features')
        & "$PSScriptRoot/ci-workload.ps1" kernel-dev
        & "$PSScriptRoot/ci-workload.ps1" kernel-prod
        & "$PSScriptRoot/ci-workload.ps1" matrix
        & "$PSScriptRoot/ci-workload.ps1" routing
        & "$PSScriptRoot/ci-workload.ps1" asid
    }
}
if ($Workload -eq 'static' -and $env:KNOWLEDGE_BASE) {
    Invoke-Check 'static' 'impact' 'cargo' @('run', '--locked', '-p', 'repository-checks', '--', 'docs', 'check-change', $env:KNOWLEDGE_BASE)
}
