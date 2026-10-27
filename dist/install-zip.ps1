# install.ps1 (versione ZIP) - pubblica mfd-cli e mfd-tui come comandi utente.
# Stessa semantica dell'originale del repo (rust\install.ps1), con i percorsi
# del pacchetto: gli exe stanno accanto a questo script, le lingue in lang\.
# La copia installata e' uno snapshot: rieseguire dopo ogni nuovo ZIP.
$ErrorActionPreference = 'Stop'
$binDir = Join-Path $env:USERPROFILE '.local\bin'
New-Item -ItemType Directory -Force -Path $binDir | Out-Null
foreach ($name in 'mfd-cli', 'mfd-tui') {
    $src = Join-Path $PSScriptRoot "$name.exe"
    if (-not (Test-Path $src)) { throw "manca $src — l'archivio e' incompleto" }
    $dst = Join-Path $binDir "$name.exe"
    Remove-Item "$dst.old" -Force -ErrorAction SilentlyContinue
    try {
        Copy-Item $src $dst -Force
    } catch {
        # exe in esecuzione: Windows non consente la sovrascrittura ma il
        # rename si' — si sposta il vecchio e si mette il nuovo al suo posto.
        Move-Item $dst "$dst.old" -Force
        Copy-Item $src $dst -Force
        Remove-Item "$dst.old" -Force -ErrorAction SilentlyContinue
    }
    "installato: $dst"
}
# registro lingue: i JSON di lang\ finiscono nel profilo utente, dove CLI e
# TUI li scoprono a runtime (l'inglese e' gia' incorporato negli exe).
$langSrc = Join-Path $PSScriptRoot 'lang'
if (-not (Test-Path $langSrc)) { throw "cartella lingue mancante: $langSrc" }
$langDir = Join-Path $env:APPDATA 'mfd\lang'
New-Item -ItemType Directory -Force -Path $langDir | Out-Null
Copy-Item (Join-Path $langSrc '*.json') $langDir -Force
"lingue installate: $((Get-ChildItem $langDir -Filter *.json).Count) JSON in $langDir"
"auto-test:"
& (Join-Path $binDir 'mfd-cli.exe') -V
