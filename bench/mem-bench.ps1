# mem-bench.ps1 — picco di memoria dei walker durante l'elaborazione (metodo: docs/BENCHMARKS.md)
# Misura, per ogni variante di pubblicazione, il PICCO di memoria del processo durante
# il walk dell'albero target, con lo stesso protocollo anti-rumore di run-bench.ps1:
# warmup di TUTTE le varianti, poi round INTERLEAVED (mai in parallelo, ordine ruotato
# per round), mediana dei picchi.
#
# Metriche (contatori di picco tenuti dal kernel, letti con NtQueryInformationProcess
# / ProcessVmCounters su un handle trattenuto anche DOPO l'uscita del processo):
#   PeakWS     — picco di working set: RAM fisica residente realmente usata
#   PeakCommit — picco di commit privata (PeakPagefileUsage): memoria paginabile richiesta
# Nota tecnica: le proprieta' .PeakWorkingSet64 di System.Diagnostics.Process tornano 0
# su un processo gia' terminato — per questo si legge il contatore kernel via P/Invoke.
#
# Uso: powershell -File bench\mem-bench.ps1 [-Target <cartella>] [-Runs 5]
#      target piccolo (es. il repo)  = baseline del runtime, walk ~irrilevante
#      target grande (es. D:\SVILUPPO, 171k file) = elaborazione vera
param(
    [string]$Target = 'D:\SVILUPPO',
    [int]$Runs = 5
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$env:MFD_BENCH = '1'   # contratto output mfd-cli: righe RESULT machine-readable

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class ProcPeak {
    [StructLayout(LayoutKind.Sequential)]
    public struct VM_COUNTERS {
        public IntPtr PeakVirtualSize;
        public IntPtr VirtualSize;
        public UInt32 PageFaultCount;
        public IntPtr PeakWorkingSetSize;
        public IntPtr WorkingSetSize;
        public IntPtr QuotaPeakPagedPoolUsage;
        public IntPtr QuotaPagedPoolUsage;
        public IntPtr QuotaPeakNonPagedPoolUsage;
        public IntPtr QuotaNonPagedPoolUsage;
        public IntPtr PagefileUsage;
        public IntPtr PeakPagefileUsage;
    }
    [DllImport("ntdll.dll")]
    private static extern int NtQueryInformationProcess(IntPtr hProcess, int cls, ref VM_COUNTERS info, int len, out int retLen);
    public static long[] GetPeaks(IntPtr h) {
        var vc = new VM_COUNTERS();
        int retLen;
        int status = NtQueryInformationProcess(h, 3, ref vc, Marshal.SizeOf(typeof(VM_COUNTERS)), out retLen);
        if (status != 0) throw new InvalidOperationException("NtQueryInformationProcess status=0x" + status.ToString("X"));
        return new long[] { vc.PeakWorkingSetSize.ToInt64(), vc.PeakPagefileUsage.ToInt64() };
    }
}
"@

# percorsi delle varianti (identici a startup-bench.ps1)
$base    = Join-Path $root 'net\src\Manto.FreeDisk.Cli\bin\Release\net10.0'   # build JIT FDD (via driver)
$wx      = Join-Path $base 'win-x64'                               # publish con RID win-x64
$pub     = Join-Path $wx 'publish'         # FDD R2R via driver e via exe apphost
$pubself = Join-Path $wx 'publish-self'
$pubtrim = Join-Path $wx 'publish-selftrim'
$pubsing = Join-Path $wx 'publish-single'
$nodeJs  = Join-Path $root 'node\mfd-scan\walker-node.mjs'
$dotnet  = 'dotnet'
$rsExe   = Join-Path $root 'rust\target\release\mfd-cli.exe'

$variants = @(
    @{ name = 'c# JIT         (FDD driver)'; file = $dotnet; pre = (Join-Path $base 'Manto.FreeDisk.Cli.dll') }
    @{ name = 'c# R2R         (FDD driver)'; file = $dotnet; pre = (Join-Path $pub 'Manto.FreeDisk.Cli.dll') }
    @{ name = 'c# R2R exe     (apphost)';    file = (Join-Path $pub 'Manto.FreeDisk.Cli.exe');     pre = $null }
    @{ name = 'c# self-R2R';                 file = (Join-Path $pubself 'Manto.FreeDisk.Cli.exe'); pre = $null }
    @{ name = 'c# self-trim';                file = (Join-Path $pubtrim 'Manto.FreeDisk.Cli.exe'); pre = $null }
    @{ name = 'c# single-file';              file = (Join-Path $pubsing 'Manto.FreeDisk.Cli.exe'); pre = $null }
    @{ name = 'rust';                        file = $rsExe;                                pre = $null }
    @{ name = 'node';                        file = 'node';                                pre = $nodeJs }
)

function Test-Exe([string]$f) {
    if ($f -match '[\\/]') { return (Test-Path $f) }
    return [bool](Get-Command $f -ErrorAction SilentlyContinue)
}
$variants = @($variants | Where-Object { (Test-Exe $_.file) -and (-not $_.pre -or (Test-Path $_.pre)) })
if ($variants.Count -lt 2) { throw 'Servono almeno due varianti pubblicate (vedi commenti in testa allo script)' }

function Invoke-Mem([string]$file, [string]$argline) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $file
    $psi.Arguments = $argline
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $p = [System.Diagnostics.Process]::Start($psi)
    $h = $p.Handle   # handle trattenuto mentre il processo e' vivo: resta valido da spento
    $errTask = $p.StandardError.ReadToEndAsync()
    $out = $p.StandardOutput.ReadToEnd()
    $null = $errTask.Result
    $p.WaitForExit()
    if ($p.ExitCode -ne 0) { throw "$file uscito con code $($p.ExitCode): $($out.Trim())" }
    $line = ($out -split "`r?`n" | Where-Object { $_ -like 'RESULT *' } | Select-Object -First 1)
    if (-not $line) { throw "nessuna riga RESULT nell'output: $($out.Trim())" }
    $peaks = [ProcPeak]::GetPeaks($h)
    $m = [regex]::Match($line, 'files=(\d+)')
    return @{
        peak_ws     = $peaks[0]
        peak_commit = $peaks[1]
        files       = [int64]$(if ($m.Success) { $m.Groups[1].Value } else { -1 })
    }
}

function Get-Argline($v) {
    if ($v.pre) { return '"{0}" "{1}"' -f $v.pre, $Target }
    return '"{0}"' -f $Target
}

# warmup di TUTTE le varianti prima di misurare
foreach ($v in $variants) { Invoke-Mem $v.file (Get-Argline $v) | Out-Null }

$memWs = @{}; $memCommit = @{}; $files = @{}
foreach ($v in $variants) {
    $memWs[$v.name] = [System.Collections.Generic.List[double]]::new()
    $memCommit[$v.name] = [System.Collections.Generic.List[double]]::new()
}
for ($r = 0; $r -lt $Runs; $r++) {
    Write-Host ("-- round {0}/{1} --" -f ($r + 1), $Runs) -ForegroundColor DarkGray
    for ($i = 0; $i -lt $variants.Count; $i++) {
        $v = $variants[($i + $r) % $variants.Count]
        $res = Invoke-Mem $v.file (Get-Argline $v)
        $memWs[$v.name].Add($res.peak_ws / 1MB)
        $memCommit[$v.name].Add($res.peak_commit / 1MB)
        $files[$v.name] = $res.files
    }
}

$rows = foreach ($v in $variants) {
    $sW = @($memWs[$v.name] | Sort-Object); $medW = $sW[[int](($sW.Count - 1) / 2)]
    $sC = @($memCommit[$v.name] | Sort-Object); $medC = $sC[[int](($sC.Count - 1) / 2)]
    [pscustomobject]@{
        Variante      = $v.name
        PeakWS_MB     = [math]::Round($medW, 1)
        PeakCommit_MB = [math]::Round($medC, 1)
        RunsWS_MB     = (($memWs[$v.name] | ForEach-Object { [math]::Round($_, 1) }) -join ',')
        Files         = $files[$v.name]
    }
}
$rows | Format-Table Variante, PeakWS_MB, PeakCommit_MB, RunsWS_MB, Files -AutoSize | Out-String -Width 200

$uniq = @($files.Values | Sort-Object -Unique)
if ($uniq.Count -eq 1) { "Conteggio file coerente fra varianti [OK]" }
else { "ATTENZIONE: conteggi file diversi — " + (($files.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join '; ') }