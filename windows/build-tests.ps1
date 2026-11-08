# Builds gcad's test binaries with the game-assets feature and copies each to a stable path,
# so a debugger can name it. Cargo puts a content hash in test binary names.
# Usage:  $env:GCAD_REPO = "C:\src\gcad"; .\windows\build-tests.ps1
# Writes $GCAD_REPO/target/debug/tests/lib.exe (library unit tests), bin.exe (CLI unit tests) and
# <name>.exe for each tests/<name>.rs. The copies keep their embedded PDB paths, so symbols
# still resolve. GCAD_REPO defaults to ..\..\gcad next to this repo.

$ErrorActionPreference = "Stop"
. "$PSScriptRoot\dev-env.ps1"

$root = if ($env:GCAD_REPO) { Resolve-Path $env:GCAD_REPO } else { Resolve-Path "$PSScriptRoot\..\..\gcad" }
Push-Location $root
try {
    $lines = & cargo test --no-run --message-format=json --features game-assets --lib --bins --tests
    if ($LASTEXITCODE -ne 0) { throw "cargo test --no-run failed" }
} finally {
    Pop-Location
}

$dest = Join-Path $root "target\debug\tests"
New-Item -ItemType Directory -Force $dest | Out-Null
$count = 0
foreach ($line in $lines) {
    if (-not $line.StartsWith("{")) { continue }
    $m = $line | ConvertFrom-Json
    if ($m.reason -ne "compiler-artifact" -or -not $m.executable -or -not $m.profile.test) { continue }
    $name = switch ($m.target.kind[0]) {
        "lib" { "lib" }
        "bin" { "bin" }
        default { $m.target.name }
    }
    Copy-Item -LiteralPath $m.executable -Destination (Join-Path $dest "$name.exe") -Force
    $count++
}
Write-Host "copied $count test binaries to $dest"
