param(
    [Parameter(Position = 0)]
    [ValidateSet('check', 'registry', 'profile', 'assess', 'compare')]
    [string] $Command = 'check',
    [Parameter(Position = 1, ValueFromRemainingArguments = $true)]
    [string[]] $InputPath = @(),
    [switch] $Help
)

$ErrorActionPreference = 'Stop'
$arenaRoot = Split-Path -Parent $PSScriptRoot

if ($Help) {
    Write-Output 'Usage: pwsh -File scripts/run-arena.ps1 [check|registry|profile|assess|compare] [PATH ...]'
    Write-Output 'Default: check. profile defaults to the proposed user-copy-range profile.'
    Write-Output 'assess requires one run JSON; compare requires two. Relative paths use the project root.'
    Write-Output 'Offline contract validation only; this script does not start a graphical Arena or publish records.'
    exit 0
}

if ($Command -eq 'profile' -and $InputPath.Count -eq 0) {
    $InputPath = @('research/arena/profiles/user-copy-range.json')
}
$arenaExpectedPaths = switch ($Command) {
    'profile' { 1 }
    'assess' { 1 }
    'compare' { 2 }
    default { 0 }
}
if ($InputPath.Count -ne $arenaExpectedPaths) {
    throw "$Command requires $arenaExpectedPaths input path(s). Use -Help for examples."
}
$arenaPaths = @($InputPath | ForEach-Object {
    $arenaCandidate = if ([IO.Path]::IsPathRooted($_)) { $_ } else { Join-Path $arenaRoot $_ }
    if (-not (Test-Path -LiteralPath $arenaCandidate -PathType Leaf)) { throw 'Arena input file does not exist.' }
    (Resolve-Path -LiteralPath $arenaCandidate).Path
})

$arenaCargo = Get-Command cargo -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
$arenaToolRoot = $null
if ($null -eq $arenaCargo) {
    $arenaRoots = @($arenaRoot)
    if (Get-Command git -CommandType Application -ErrorAction SilentlyContinue) {
        $arenaCommonDir = & git -C $arenaRoot rev-parse --path-format=absolute --git-common-dir 2>$null
        if ($LASTEXITCODE -eq 0) { $arenaRoots += Split-Path -Parent $arenaCommonDir }
    }
    foreach ($arenaCandidateRoot in $arenaRoots | Select-Object -Unique) {
        $arenaCandidateCargo = Join-Path $arenaCandidateRoot '.toolchains/cargo/bin/cargo.exe'
        if (Test-Path -LiteralPath $arenaCandidateCargo -PathType Leaf) {
            $arenaCargo = Get-Item -LiteralPath $arenaCandidateCargo
            $arenaToolRoot = $arenaCandidateRoot
            break
        }
    }
}
if ($null -eq $arenaCargo) {
    throw 'Cargo was not found. Configure the Rust toolchain pinned by rust-toolchain.toml, or the project .toolchains installation.'
}

$arenaSavedEnvironment = @{}
foreach ($arenaName in @('PATH', 'CARGO_HOME', 'RUSTUP_HOME')) {
    $arenaSavedEnvironment[$arenaName] = [Environment]::GetEnvironmentVariable($arenaName, 'Process')
}
try {
    if ($null -ne $arenaToolRoot) {
        $env:CARGO_HOME = Join-Path $arenaToolRoot '.toolchains/cargo'
        $env:RUSTUP_HOME = Join-Path $arenaToolRoot '.toolchains/rustup'
        $env:PATH = (Join-Path $env:CARGO_HOME 'bin') + [IO.Path]::PathSeparator + $env:PATH
    }
    $arenaExecutable = if ($arenaCargo -is [IO.FileInfo]) { $arenaCargo.FullName } else { $arenaCargo.Source }
    Push-Location -LiteralPath $arenaRoot
    try {
        & $arenaExecutable run --locked -p xtask -- arena $Command @arenaPaths
        $arenaExitCode = $LASTEXITCODE
    } finally {
        Pop-Location
    }
} finally {
    foreach ($arenaName in $arenaSavedEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($arenaName, $arenaSavedEnvironment[$arenaName], 'Process')
    }
}
exit $arenaExitCode
