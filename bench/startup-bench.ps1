# Micro-benchmark di AVVIO dei walker MFD - tempo ESTERNO dell'intero processo
# (avvio del runtime + walk), complementare a run-bench.ps1 che misura il solo walk
# con stopwatch interno ai programmi (avvio escluso per design).
# Serve a osservare il costo di avvio delle modalita' di pubblicazione C#:
#   FDD JIT        : runtime cercato nel sistema, codice JIT-tato all'avvio
#   FDD R2R        : runtime cercato nel sistema, codice nativo precompilato
#   FDD R2R apphost: come sopra ma avviata dal suo exe launcher invece del driver "dotnet"
#   self-cont R2R  : runtime incluso nell'app (nessuna ricerca del framework), precompilato
#   self-cont trim : come sopra, runtime tagliato (meno assembly da caricare)
#   single-file    : self-contained impacchettato in UN SOLO exe (tutto il runtime dentro)
#   rust / node    : riferimenti (exe nativo / runtime JS)
# Metodo: stesso protocollo di run-bench.ps1 - warmup di TUTTE le varianti, poi N round
# INTERLEAVED (round-robin, mai in parallelo) con ordine ruotato a ogni round.
# Il tempo esterno si cronometra con System.Diagnostics.Stopwatch (performance counter
# QPC di Windows, precisione sub-ms) attorno all'intero processo: spawn -> uscita.
# Il tempo di AVVIO PURO si stima per differenza: esterno - ms interni del walk che il
# walker stesso riporta (RESULT ms=...), cosi' il walk non inquina la stima.
# Il target dev'essere una cartella PICCOLA (default: il repo): cosi' il tempo esterno
# e' dominato dall'avvio, che e' la grandezza in esame.
# Nota: il tempo esterno include anche ~3-5 ms di spawn del processo da PowerShell,
# costante e uguale per tutte le varianti (visibile nel totale di rust).
# Uso:
#   powershell -ExecutionPolicy Bypass -File bench\startup-bench.ps1 [-Target <cartella>] [-Runs 7]
param(
    [string]$Target = '',
    [int]$Runs = 7
)
$ErrorActionPreference = 'Stop'
$benchDir = $PSScriptRoot
$root = Split-Path $benchDir -Parent   # radice del repo
if (-not $Target) { $Target = $root }   # default: il repo stesso (cartella piccola)
$env:MFD_BENCH = '1'   # contratto output mfd-cli: righe RESULT machine-readable

$csJit    = Join-Path $root 'net\src\Manto.FreeDisk.Cli\bin\Release\net10.0\Manto.FreeDisk.Cli.dll'
$csR2r    = Join-Path $root 'net\src\Manto.FreeDisk.Cli\bin\Release\net10.0\win-x64\publish\Manto.FreeDisk.Cli.dll'
$csApp    = Join-Path $root 'net\src\Manto.FreeDisk.Cli\bin\Release\net10.0\win-x64\publish\Manto.FreeDisk.Cli.exe'
$csSelf   = Join-Path $root 'net\src\Manto.FreeDisk.Cli\bin\Release\net10.0\win-x64\publish-self\Manto.FreeDisk.Cli.exe'
$csTrim   = Join-Path $root 'net\src\Manto.FreeDisk.Cli\bin\Release\net10.0\win-x64\publish-selftrim\Manto.FreeDisk.Cli.exe'
$csSingle = Join-Path $root 'net\src\Manto.FreeDisk.Cli\bin\Release\net10.0\win-x64\publish-single\Manto.FreeDisk.Cli.exe'
$rustExe  = Join-Path $root 'rust\target\release\mfd-cli.exe'
$nodeJs   = Join-Path $root 'node\mfd-scan\walker-node.mjs'

$variants = @(
    @{ name = 'c# JIT         (FDD: runtime dal sistema)';   exe = $csJit;    cmd = { param($t) dotnet $csJit $t } }
    @{ name = 'c# R2R         (FDD precompilato)';           exe = $csR2r;    cmd = { param($t) dotnet $csR2r $t } }
    @{ name = 'c# R2R exe     (apphost launcher)';           exe = $csApp;    cmd = { param($t) & $csApp $t } }
    @{ name = 'c# self-R2R    (runtime incluso nell exe)';   exe = $csSelf;   cmd = { param($t) & $csSelf $t } }
    @{ name = 'c# self-trim   (runtime incluso e tagliato)'; exe = $csTrim;   cmd = { param($t) & $csTrim $t } }
    @{ name = 'c# single-file (tutto in un solo exe)';       exe = $csSingle; cmd = { param($t) & $csSingle $t } }
    @{ name = 'rust           (exe nativo)';                 exe = $rustExe;  cmd = { param($t) & $rustExe $t } }
    @{ name = 'node           (runtime JS)';                 exe = $nodeJs;   cmd = { param($t) node $nodeJs $t } }
)
$variants = @($variants | Where-Object { Test-Path $_.exe })
if ($variants.Count -lt 2) { throw 'Servono almeno due varianti pubblicate (vedi commenti in testa allo script)' }

function Invoke-Timed([scriptblock]$cmd, [string]$target) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $out = & $cmd $target
    $sw.Stop()
    $line = ($out | Where-Object { $_ -like 'RESULT *' } | Select-Object -First 1)
    if (-not $line) { throw "nessuna riga RESULT nell'output: $($out -join ' | ')" }
    $m  = [regex]::Match($line, 'files=(\d+)')
    $mm = [regex]::Match($line, 'ms=(\d+)')
    return @{
        ext_ms = [double]$sw.Elapsed.TotalMilliseconds
        int_ms = [double]$(if ($mm.Success) { $mm.Groups[1].Value } else { -1 })
        files  = [int64]$(if ($m.Success) { $m.Groups[1].Value } else { -1 })
    }
}

# warmup di TUTTE le varianti prima di misurare
foreach ($v in $variants) { Invoke-Timed $v.cmd $Target | Out-Null }

# round misurati INTERLEAVED (round-robin, mai in parallelo), ordine ruotato per round
$times = @{}
$intimes = @{}
foreach ($v in $variants) {
    $times[$v.name] = [System.Collections.Generic.List[double]]::new()
    $intimes[$v.name] = [System.Collections.Generic.List[double]]::new()
}
$files = @{}
for ($r = 0; $r -lt $Runs; $r++) {
    $cpu = $null
    try { $cpu = (Get-CimInstance Win32_Processor -Property LoadPercentage).LoadPercentage } catch { }
    Write-Host ("-- round {0}/{1} · carico CPU {2}% --" -f ($r + 1), $Runs, ($(if ($null -ne $cpu) { $cpu } else { '?' }))) -ForegroundColor DarkGray
    for ($i = 0; $i -lt $variants.Count; $i++) {
        $v = $variants[($i + $r) % $variants.Count]
        $res = Invoke-Timed $v.cmd $Target
        $times[$v.name].Add($res.ext_ms)
        $intimes[$v.name].Add($res.int_ms)
        $files[$v.name] = $res.files
    }
}

$rows = foreach ($v in $variants) {
    $sorted = @($times[$v.name] | Sort-Object)
    $median = $sorted[[int](($sorted.Count - 1) / 2)]
    $sortedInt = @($intimes[$v.name] | Sort-Object)
    $medInt = $sortedInt[[int](($sortedInt.Count - 1) / 2)]
    [pscustomobject]@{
        Variante   = $v.name
        Best_ms    = [math]::Round($sorted[0], 1)
        Median_ms  = [math]::Round($median, 1)
        Walk_ms    = [math]::Round($medInt, 1)
        Startup_ms = [math]::Round($median - $medInt, 1)
        Runs_ms    = (($times[$v.name] | ForEach-Object { [math]::Round($_, 1) }) -join ',')
        Files      = $files[$v.name]
    }
}
$rows | Format-Table Variante, Best_ms, Median_ms, Walk_ms, Startup_ms, Runs_ms, Files -AutoSize | Out-String -Width 240

# controllo di coerenza del conteggio file fra varianti
$distinct = @($rows | ForEach-Object { $_.Files } | Select-Object -Unique).Count
if ($distinct -ne 1) {
    Write-Host 'CONTEGGIO FILE DIVERSO fra varianti - indagare!' -ForegroundColor Red
} else {
    Write-Host 'Conteggio file coerente fra varianti [OK]' -ForegroundColor Green
}
