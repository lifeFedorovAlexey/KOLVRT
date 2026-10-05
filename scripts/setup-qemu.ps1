$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$cache = Join-Path $repoRoot '.toolchains/qemu'
$archive = Join-Path $cache 'setup.exe'
$expected = '5e6b88318ab1233e6d9cea187f657a8d69c5007fbaaee18c57383abbadf080c2731ccdcfe5c12b021b344b285b4bf2d69f6c80124714a2ac7d5f6a46b7297ccc'
New-Item -ItemType Directory -Force -Path $cache | Out-Null
if (!(Test-Path -LiteralPath $archive) -or (Get-FileHash -LiteralPath $archive -Algorithm SHA512).Hash.ToLowerInvariant() -ne $expected) {
    & curl.exe --ssl-revoke-best-effort --retry 3 --retry-all-errors --fail --location --silent --show-error --max-time 600 --output $archive 'https://qemu.weilnetz.de/w64/2025/qemu-w64-setup-20250826.exe'
    if ($LASTEXITCODE -ne 0) { throw 'QEMU download failed' }
}
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA512).Hash.ToLowerInvariant() -ne $expected) { throw 'QEMU SHA-512 mismatch' }
$extractor = (Get-Command 7z.exe -ErrorAction SilentlyContinue).Source
if (!$extractor) { $extractor = 'C:\Program Files\7-Zip\7z.exe' }
if (!(Test-Path -LiteralPath $extractor)) { throw 'Install 7-Zip or put 7z.exe on PATH' }
$destination = Join-Path $cache 'bin'
& $extractor x $archive "-o$destination" -y | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'QEMU extraction failed' }
$binary = Join-Path $destination 'qemu-system-aarch64.exe'
$version = & $binary --version
if ($LASTEXITCODE -ne 0 -or $version[0] -notlike 'QEMU emulator version 10.1.0 *') { throw 'QEMU version mismatch' }
@{ url = 'https://qemu.weilnetz.de/w64/2025/qemu-w64-setup-20250826.exe'; archive_sha512 = $expected; binary_sha256 = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant(); version = $version[0] } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $cache 'provenance.json') -Encoding utf8
Write-Output $version[0]
