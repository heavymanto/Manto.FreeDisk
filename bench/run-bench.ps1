# Micro-benchmark dei walker MFD - stessa cartella, stessa logica, linguaggi diversi.
# Metodo (vedi docs/BENCHMARKS.md):
#   warmup di TUTTE le implementazioni (cache NTFS calda, analisi Defender esaurita), poi
#   N round misurati INTERLEAVED: ogni round esegue tutte le implementazioni in fila (mai
#   in parallelo) con ordine ruotato a ogni round, cosi' un carico esterno che va e viene
#   colpisce tutte le implementazioni allo stesso modo invece di contaminare un blocco.
#   Stopwatch interno ai programmi (avvio del runtime escluso). I totali (bytes/files/dirs)
#   devono coincidere fra implementazioni: e' il controllo di correttezza incrociato.
# Uso:
#   powershell -ExecutionPolicy Bypass -File bench\run-bench.ps1 -Target D:\SVILUPPO -Runs 5 [-Only node|csharp|csr2r|csself|rust]
# Rust (toolchain GNU via rustup): compilare con cargo build --release in rust\ (workspace).
# C# R2R (FDD): dotnet publish -c Release -r win-x64 --self-contained false -p:PublishReadyToRun=true in net\src\Manto.FreeDisk.Cli.
# C# self-contained R2R: dotnet publish -c Release -r win-x64 --self-contained true -p:PublishReadyToRun=true -o bin\Release\net10.0\win-x64\publish-self in net\src\Manto.FreeDisk.Cli.
param(
    [string]$Target = "D:\SVILUPPO",
    [int]$Runs = 3,
    [ValidateSet('all','node','csharp','csr2r','csself','rust')]
    [string]$Only = 'all'
)
$ErrorActionPreference = 'Stop'
$benchDir = $PSScriptRoot
$root = Split-Path $benchDir -Parent   # radice del repo
$env:MFD_BENCH = '1'   # contratto output mfd-cli: righe RESULT machine-readable

function Invoke-Walker([scriptblock]$cmd, [string]$target) {
    $out = & $cmd $target
    $line = ($out | Where-Object { $_ -like 'RESULT *' } | Select-Object -First 1)
    if (-not $line) { throw "nessuna riga RESULT nell'output: $out" }
    $kv = @{}
    foreach ($p in ($line -replace '^RESULT ','') -split ' ') {
        $a = $p -split '=', 2
        $kv[$a[0]] = $a[1]
    }
    return $kv
}

$impls = @()
if ($Only -in 'all','node') {
    $impls += @{ name = 'node  (readdir+stat per file, libuv)'; cmd = { param($t) node (Join-Path $root 'node\mfd-scan\walker-node.mjs') $t } }
}
if ($Only -in 'all','csharp') {
    $impls += @{ name = 'c#   (find-data: dimensioni gratis, no stat)'; cmd = { param($t) dotnet (Join-Path $root 'net\src\Manto.FreeDisk.Cli\bin\Release\net10.0\Manto.FreeDisk.Cli.dll') $t } }
}
if ($Only -in 'all','csr2r') {
    $r2rDll = Join-Path $root 'net\src\Manto.FreeDisk.Cli\bin\Release\net10.0\win-x64\publish\Manto.FreeDisk.Cli.dll'
    if (Test-Path $r2rDll) {
        $impls += @{ name = 'c#R2R (ReadyToRun: precompilato, zero JIT)'; cmd = { param($t) dotnet $r2rDll $t } }
    } else {
        Write-Warning "Manto.FreeDisk.Cli R2R non pubblicato: dotnet publish -c Release -r win-x64 --self-contained false -p:PublishReadyToRun=true in net\src\Manto.FreeDisk.Cli"
    }
}
if ($Only -in 'all','csself') {
    $selfExe = Join-Path $root 'net\src\Manto.FreeDisk.Cli\bin\Release\net10.0\win-x64\publish-self\Manto.FreeDisk.Cli.exe'
    if (Test-Path $selfExe) {
        $impls += @{ name = 'c#self (self-contained R2R: runtime incluso)'; cmd = { param($t) & $selfExe $t } }
    } else {
        Write-Warning "Manto.FreeDisk.Cli self-contained non pubblicato: dotnet publish -c Release -r win-x64 --self-contained true -p:PublishReadyToRun=true -o bin\Release\net10.0\win-x64\publish-self in net\src\Manto.FreeDisk.Cli"
    }
}
if ($Only -in 'all','rust') {
    $rustExe = Join-Path $root 'rust\target\release\mfd-cli.exe'
    if (Test-Path $rustExe) {
        $impls += @{ name = 'rust  (find-data via std::fs, no stat)'; cmd = { param($t) & $rustExe $t } }
    } else {
        Write-Warning "mfd-cli non compilato: esegui prima cargo build --release in rust\ (workspace)"
    }
}

# threadpool libuv piu' larga: da a Node la sua miglior configurazione (default 4)
$env:UV_THREADPOOL_SIZE = '16'

$all = @{}

# warmup di TUTTE le implementazioni PRIMA di misurare: stesse condizioni di partenza
foreach ($impl in $impls) { Invoke-Walker $impl.cmd $Target | Out-Null }

# round misurati INTERLEAVED (round-robin, mai in parallelo): ordine di partenza ruotato
# a ogni round per cancellare anche il bias di posizione (chi parte primo/primo-migliore)
$times = @{}
foreach ($impl in $impls) { $times[$impl.name] = [System.Collections.Generic.List[double]]::new() }
$lastKv = @{}
for ($r = 0; $r -lt $Runs; $r++) {
    $cpu = $null
    try { $cpu = (Get-CimInstance Win32_Processor -Property LoadPercentage).LoadPercentage } catch { }
    Write-Host ("-- round {0}/{1} · carico CPU {2}% --" -f ($r + 1), $Runs, ($(if ($null -ne $cpu) { $cpu } else { '?' }))) -ForegroundColor DarkGray
    for ($i = 0; $i -lt $impls.Count; $i++) {
        $impl = $impls[($i + $r) % $impls.Count]
        $kv = Invoke-Walker $impl.cmd $Target
        $times[$impl.name].Add([double]$kv['ms'])
        $lastKv[$impl.name] = $kv
    }
}

foreach ($impl in $impls) {
    $sorted = @($times[$impl.name] | Sort-Object)
    $median = $sorted[[int](($sorted.Count - 1) / 2)]
    $kv = $lastKv[$impl.name]
    $all[$impl.name] = [pscustomobject]@{
        Impl      = $impl.name
        Best_ms   = [int]$sorted[0]
        Median_ms = [int]$median
        Runs_ms   = (($times[$impl.name] | ForEach-Object { [int]$_ }) -join ',')
        Files     = $kv['files']
        Dirs      = $kv['dirs']
        Bytes     = $kv['bytes']
        Errors    = $kv['errors']
    }
}

$all.Values | Format-Table Impl, Best_ms, Median_ms, Runs_ms, Files, Dirs, Errors, Bytes -AutoSize | Out-String -Width 200

# controllo di coerenza dei totali fra implementazioni
$vals = @($all.Values)
if ($vals.Count -ge 2) {
    $ok = $true
    foreach ($f in 'Bytes','Files','Dirs') {
        $distinct = @($vals | ForEach-Object { $_.$f } | Select-Object -Unique).Count
        if ($distinct -ne 1) { $ok = $false; Write-Host "TOTALI DIVERSI su ${f} - indagare!" -ForegroundColor Red }
    }
    if ($ok) { Write-Host "Totali coerenti fra implementazioni (bytes/files/dirs identici) [OK]" -ForegroundColor Green }
}

