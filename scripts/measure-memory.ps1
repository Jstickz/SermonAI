<#
.SYNOPSIS
  Sample SermonAI's memory while it runs, for M1's Definition of Done.

.DESCRIPTION
  Reports the PRIVATE working set of the whole process tree, not the sum of
  each process's working set. A Tauri app is one Rust process plus a WebView2
  process per window and several shared Chromium helpers -- nine on this
  machine -- and WorkingSet counts pages shared between them once per process.
  Summing it gave 583 MB where the private figure was 168 MB, which would have
  failed a 300 MB budget the app was comfortably inside.

  Private working set is what Task Manager's "Memory (private working set)"
  column shows, and it is the number that answers "how much RAM does this cost
  the machine".

.EXAMPLE
  .\scripts\measure-memory.ps1 -Seconds 600 -Label "10-minute sermon"
#>
param(
  [int]$Seconds = 120,
  [int]$IntervalSeconds = 2,
  [string]$Label = "run",
  [int]$BudgetMB = 300
)

$main = Get-Process sermonai -ErrorAction SilentlyContinue
if (-not $main) {
  Write-Error "SermonAI is not running. Start it first."
  exit 1
}

function Get-TreeIds([int]$rootId) {
  $ids = @($rootId)
  $added = $true
  while ($added) {
    $added = $false
    $kids = Get-CimInstance Win32_Process |
      Where-Object { $ids -contains $_.ParentProcessId -and $ids -notcontains $_.ProcessId }
    foreach ($k in $kids) { $ids += $k.ProcessId; $added = $true }
  }
  $ids
}

# Perf-counter instance names are resolved once per sample, because WebView2
# spawns and retires renderers while the app runs and a cached map goes stale.
function Get-PrivateMB([int[]]$ids) {
  $byPid = @{}
  foreach ($s in (Get-Counter "\Process(*)\ID Process" -ErrorAction SilentlyContinue).CounterSamples) {
    $byPid[[int]$s.CookedValue] = ($s.Path -split '\(|\)')[1]
  }
  $total = 0
  foreach ($id in $ids) {
    $name = $byPid[$id]
    if (-not $name) { continue }
    $c = Get-Counter "\Process($name)\Working Set - Private" -ErrorAction SilentlyContinue
    if ($c) { $total += $c.CounterSamples[0].CookedValue }
  }
  [math]::Round($total / 1MB, 1)
}

$samples = @()
$deadline = (Get-Date).AddSeconds($Seconds)
Write-Host "Sampling '$Label' for $Seconds s (budget $BudgetMB MB). Ctrl+C to stop early."

while ((Get-Date) -lt $deadline) {
  $ids = Get-TreeIds $main.Id
  $mb = Get-PrivateMB $ids
  $samples += $mb
  $peak = ($samples | Measure-Object -Maximum).Maximum
  Write-Host ("  {0}  {1,7:N1} MB   peak {2,7:N1} MB   ({3} processes)" -f (Get-Date -Format "HH:mm:ss"), $mb, $peak, $ids.Count)
  Start-Sleep -Seconds $IntervalSeconds

  if (-not (Get-Process -Id $main.Id -ErrorAction SilentlyContinue)) {
    Write-Warning "SermonAI exited during sampling."
    break
  }
}

if ($samples.Count -eq 0) { Write-Error "No samples taken."; exit 1 }

$stats = $samples | Measure-Object -Average -Maximum -Minimum
Write-Host ""
Write-Host "=== $Label ==="
Write-Host ("samples : {0}" -f $samples.Count)
Write-Host ("min     : {0:N1} MB" -f $stats.Minimum)
Write-Host ("mean    : {0:N1} MB" -f $stats.Average)
Write-Host ("PEAK    : {0:N1} MB" -f $stats.Maximum)
Write-Host ("budget  : {0} MB" -f $BudgetMB)
if ($stats.Maximum -le $BudgetMB) {
  Write-Host "RESULT  : within budget" -ForegroundColor Green
} else {
  Write-Host "RESULT  : OVER budget" -ForegroundColor Red
}
Write-Host ""
Write-Host "Note: a 'tauri dev' run carries an unoptimized Rust binary and a live"
Write-Host "Vite server. The number of record for the DoD is a release build."
