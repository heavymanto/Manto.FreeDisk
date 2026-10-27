# install.ps1 (ZIP version) - publishes mfd-cli and mfd-tui as user commands.
# Same semantics as the repo's original (rust\install.ps1), with the ZIP's
# paths: the exes sit next to this script, languages in lang\.
# The installed copy is a snapshot: re-run after every new ZIP.
$ErrorActionPreference = 'Stop'
$binDir = Join-Path $env:USERPROFILE '.local\bin'
New-Item -ItemType Directory -Force -Path $binDir | Out-Null
foreach ($name in 'mfd-cli', 'mfd-tui') {
    $src = Join-Path $PSScriptRoot "$name.exe"
    if (-not (Test-Path $src)) { throw "missing $src - the archive is incomplete" }
    $dst = Join-Path $binDir "$name.exe"
    Remove-Item "$dst.old" -Force -ErrorAction SilentlyContinue
    try {
        Copy-Item $src $dst -Force
    } catch {
        # exe in use: Windows won't overwrite but allows a rename
        Move-Item $dst "$dst.old" -Force
        Copy-Item $src $dst -Force
        Remove-Item "$dst.old" -Force -ErrorAction SilentlyContinue
    }
    "installed: $dst"
}
# language registry: the JSON files from lang\ go to the user profile,
# where CLI and TUI discover them at runtime (English is already embedded).
$langSrc = Join-Path $PSScriptRoot 'lang'
if (-not (Test-Path $langSrc)) { throw "language folder missing: $langSrc" }
$langDir = Join-Path $env:APPDATA 'mfd\lang'
New-Item -ItemType Directory -Force -Path $langDir | Out-Null
Copy-Item (Join-Path $langSrc '*.json') $langDir -Force
"languages installed: $((Get-ChildItem $langDir -Filter *.json).Count) JSON in $langDir"
"self-test:"
& (Join-Path $binDir 'mfd-cli.exe') -V