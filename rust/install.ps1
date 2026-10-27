# install.ps1 - pubblica mfd-cli e mfd-tui come comandi utente
# Copia rust\target\release\mfd-cli.exe e mfd-tui.exe (workspace Cargo) in
# %USERPROFILE%\.local\bin (directory gia' presente nel PATH utente, quindi
# nessuna modifica al PATH) e crates\mfd-core\lang\*.json (le sette lingue)
# in %APPDATA%\mfd\lang, il registro che CLI e TUI scoprono a runtime
# (l'inglese e' incorporato negli exe: la cartella serve per le altre lingue
# e per l'override di en.json, che nel profilo sopravanza quello di fabbrica).
# La copia installata e' uno snapshot: rieseguire questo script dopo ogni
# rebuild o dopo ogni cambio alle lingue.
# Uso dei comandi: mfd-cli <path>   -> scansiona il path indicato (griglia)
#                  mfd-cli          -> scansiona la cartella corrente (naked = .)
#                  mfd-tui <path>   -> apre il TUI interattivo sulla cartella
#                  mfd-tui          -> apre il TUI sulla cartella corrente
# In entrambi i casi output leggibile (file/cartelle/dimensione/tempo, numeri
# formattati); il formato machine-readable RESULT serve solo ai bench (MFD_BENCH=1).
$ErrorActionPreference = 'Stop'
$binDir = Join-Path $env:USERPROFILE '.local\bin'
New-Item -ItemType Directory -Force -Path $binDir | Out-Null
# pulizia: vecchio nome pre-rename (mfd-scan.exe) ed eventuali residui .old
Remove-Item (Join-Path $binDir 'mfd-scan.exe') -Force -ErrorAction SilentlyContinue
foreach ($name in 'mfd-cli', 'mfd-tui') {
    $src = Join-Path $PSScriptRoot "target\release\$name.exe"
    if (-not (Test-Path $src)) { throw "build mancante: $src (prima: cargo build --release in rust\)" }
    $dst = Join-Path $binDir "$name.exe"
    Remove-Item "$dst.old" -Force -ErrorAction SilentlyContinue
    try {
        Copy-Item $src $dst -Force
    } catch {
        # exe in esecuzione (es. uno scan interattivo dell'utente): Windows non consente
        # la sovrascrittura ma CONSENTE il rename di un exe in uso; spostiamo il vecchio
        # e mettiamo il nuovo al suo posto.
        Move-Item $dst "$dst.old" -Force
        Copy-Item $src $dst -Force
        # se il vecchio processo e' ancora attivo la Delete fallisce: ripulita al giro dopo
        Remove-Item "$dst.old" -Force -ErrorAction SilentlyContinue
    }
    "installato: $dst"
}
# registro lingue: i JSON sorgente (lang/ del core — il gate di parita' li ha
# gia' verificati nei test) finiscono nel profilo utente, dove CLI e TUI li
# scoprono a runtime. Su una macchina nuova la cartella non esiste: la creiamo.
$langSrc = Join-Path $PSScriptRoot 'crates\mfd-core\lang'
if (-not (Test-Path $langSrc)) { throw "cartella lingue mancante: $langSrc" }
$langDir = Join-Path $env:APPDATA 'mfd\lang'
New-Item -ItemType Directory -Force -Path $langDir | Out-Null
Copy-Item (Join-Path $langSrc '*.json') $langDir -Force
"lingue installate: $((Get-ChildItem $langDir -Filter *.json).Count) JSON in $langDir"
"auto-test (scan della cartella rust):"
& (Join-Path $binDir 'mfd-cli.exe') "$PSScriptRoot"
"auto-test mfd-tui:"
& (Join-Path $binDir 'mfd-tui.exe') -V
"auto-test registro lingue (mfd-cli -lang de sulla cartella delle lingue):"
& (Join-Path $binDir 'mfd-cli.exe') -lang de "$langSrc"