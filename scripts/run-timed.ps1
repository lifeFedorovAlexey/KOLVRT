param(
    [Parameter(Mandatory)][string]$Kind,
    [Parameter(Mandatory)][string]$Label,
    [Parameter(Mandatory)][string]$Command,
    [string[]]$CommandArguments
)
$ErrorActionPreference = 'Stop'
$watch = [System.Diagnostics.Stopwatch]::StartNew()
$code = 1
try {
    $global:LASTEXITCODE = 0
    & $Command @CommandArguments
    $code = $LASTEXITCODE
} finally {
    $watch.Stop()
    if ($env:CI_TIMING_DIR) {
        New-Item -ItemType Directory -Force -Path $env:CI_TIMING_DIR | Out-Null
        @{ schema_version = 1; kind = $Kind; configuration = $Label; outcome = $(if ($code -eq 0) { 'success' } else { 'failure' }); wall_seconds = $watch.Elapsed.TotalSeconds; process_id = $PID; observation = 'executed' } |
            ConvertTo-Json -Compress | Add-Content -LiteralPath (Join-Path $env:CI_TIMING_DIR "steps-$PID.jsonl") -Encoding utf8
    }
}
exit $code
