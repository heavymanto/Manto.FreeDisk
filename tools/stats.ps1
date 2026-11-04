# stats.ps1 - il report d'uso del repository: download delle release,
# visite e cloni (il traffic e' del proprietario: serve il token di git).
# Uso: powershell -File tools\stats.ps1            (solo i download, pubblici)
#      powershell -File tools\stats.ps1 -Traffic   (anche visite e cloni)
param([switch]$Traffic)
$ErrorActionPreference='Stop'
$repo = 'heavymanto/Manto.FreeDisk'

# il token di git credential, se c'e' (serve per il traffic; i download
# si leggono anche senza, ma col token il rate limit e' piu' gentile)
$token = $null
try {
  $cred = "protocol=https`nhost=github.com`n" | git credential fill 2>$null
  $token = (@($cred -split "`n" | Where-Object { $_ -match '^password=' }) -replace '^password=','')[0]
} catch {}
$h = @{ Accept = 'application/vnd.github+json' }
if($token){ $h['Authorization'] = 'token ' + $token }

Write-Output ('=== ' + $repo + ' - report d''uso ===')
Write-Output ('data: ' + (Get-Date -Format 'yyyy-MM-dd HH:mm'))
Write-Output ''

# --- le release: i download per file ---
$rels = Invoke-RestMethod -Uri ('https://api.github.com/repos/' + $repo + '/releases?per_page=100') -Headers $h
$tot = 0
foreach($r in ($rels | Sort-Object -Property published_at -Descending)){
  Write-Output ('release ' + $r.tag_name + '  (' + (($r.published_at -split 'T')[0]) + ')')
  foreach($a in $r.assets){
    $tot += $a.download_count
    Write-Output ('  ' + ('{0,-46} {1,4} download' -f $a.name, $a.download_count))
  }
}
$stars = (Invoke-RestMethod -Uri ('https://api.github.com/repos/' + $repo) -Headers $h).stargazers_count
Write-Output ''
Write-Output ('download totali: ' + $tot)
Write-Output ('stelle:          ' + $stars + ' / 1000  (il gate del gemello C#)')

# --- il traffic (solo proprietario) ---
if($Traffic){
  Write-Output ''
  Write-Output '=== traffic (ultimi 14 giorni) ==='
  try {
    $v = Invoke-RestMethod -Uri ('https://api.github.com/repos/' + $repo + '/traffic/views') -Headers $h
    Write-Output ('visite: ' + $v.count + '  (unici: ' + $v.uniques + ')')
    $c = Invoke-RestMethod -Uri ('https://api.github.com/repos/' + $repo + '/traffic/clones') -Headers $h
    Write-Output ('cloni:  ' + $c.count + '  (unici: ' + $c.uniques + ')')
    $ref = Invoke-RestMethod -Uri ('https://api.github.com/repos/' + $repo + '/traffic/popular/referrers') -Headers $h
    Write-Output 'referrer:'
    foreach($x in $ref){ Write-Output ('  ' + $x.referrer.PadRight(30) + [string]$x.count + ' (' + $x.uniques + ' unici)') }
    $pp = Invoke-RestMethod -Uri ('https://api.github.com/repos/' + $repo + '/traffic/popular/paths') -Headers $h
    Write-Output 'path piu visti:'
    foreach($x in $pp){ Write-Output ('  ' + $x.path.PadRight(40) + [string]$x.count + ' (' + $x.uniques + ' unici)') }
  } catch {
    Write-Output 'traffic non disponibile (serve il token del proprietario)'
  }
}