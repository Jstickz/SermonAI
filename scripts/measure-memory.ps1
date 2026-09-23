<#
.SYNOPSIS
  Sample SermonAI's memory while it runs, for M1's Definition of Done.

.DESCRIPTION
  Reports the PRIVATE working set of the whole process tree.

  Two mistakes are easy here and this script has made both.

  First, summing each process's WorkingSet triple-counts. A Tauri app is one
  Rust process plus a WebView2 renderer per window and several shared Chromium
  helpers -- nine on this machine -- and WorkingSet counts pages shared between
  them once per process. That read 583 MB where the private figure was 152 MB,
  and would have failed a 300 MB budget the app was comfortably inside.

  Second, and the reason for this rewrite: resolving processes through
  performance-counter *instance names* silently loses most of them. Several
  processes are all called "msedgewebview2", so the name is ambiguous and the
  lookup returns one of them. Measured against a live app it resolved 1 of 9
  processes and reported 6 MB where the true figure was 193 MB. A DoD run
  reported 40 MB -- which is sermonai.exe on its own, and looked plausible.

  So processes are matched by PID through Win32_PerfRawData_PerfProc_Process,
  which carries WorkingSetPrivate keyed by IDProcess and cannot be confused by
  duplicate names. Every sample prints how many processes it resolved, so an
  undercount is visible rather than quietly halving the answer.

.EXAMPLE
  .\scripts\measure-memory.ps1 -Seconds 600 -Label "10-minute sermon"
#>
param(
  [int]$Seconds = 120,
  [int]$IntervalSeconds = 2,
  [string]$Label = "run",
  [int]$BudgetMB = 300,
  [switch]$Breakdown
)

$all = @(Get-Process sermonai -ErrorAction SilentlyContinue)
if ($all.Count -eq 0) {
  Write-Error "SermonAI is not running. Start it first."
  exit 1
}
if ($all.Count -gt 1) {
  # Almost always a dev build and an installed build running together, which
  # would otherwise be measured as one app.
  Write-Warning "$($all.Count) sermonai processes are running; measuring pid $($all[0].Id) and its children only. Close the others for a clean figure."
}
$main = $all[0]

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

# Keyed by PID, never by name. See the note above.
function Get-PrivateBytes([int[]]$ids) {
  $rows = Get-CimInstance Win32_PerfRawData_PerfProc_Process |
    Where-Object { $ids -contains $_.IDProcess }
  [pscustomobject]@{
    Bytes    = ($rows | Measure-Object WorkingSetPrivate -Sum).Sum
    Resolved = @($rows).Count
    Rows     = $rows
  }
}

$samples = @()
$minResolved = [int]::MaxValue
$deadline = (Get-Date).AddSeconds($Seconds)
Write-Host "Sampling '$Label' for $Seconds s (budget $BudgetMB MB). Ctrl+C to stop early."

while ((Get-Date) -lt $deadline) {
  $ids = Get-TreeIds $main.Id
  $m = Get-PrivateBytes $ids
  $mb = [math]::Round($m.Bytes / 1MB, 1)
  $samples += $mb
  if ($m.Resolved -lt $minResolved) { $minResolved = $m.Resolved }
  $peak = ($samples | Measure-Object -Maximum).Maximum

  $flag = if ($m.Resolved -lt $ids.Count) { "  <-- UNDERCOUNT" } else { "" }
  Write-Host ("  {0}  {1,7:N1} MB   peak {2,7:N1} MB   {3}/{4} processes{5}" -f `
    (Get-Date -Format "HH:mm:ss"), $mb, $peak, $m.Resolved, $ids.Count, $flag)

  if ($Breakdown) {
    $m.Rows | Sort-Object WorkingSetPrivate -Descending |
      ForEach-Object { Write-Host ("      {0,-18} {1,7:N1} MB" -f $_.Name, ($_.WorkingSetPrivate / 1MB)) }
  }

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

# A figure that only counted the Rust process would look like a comfortable
# pass. Saying so beats letting it read as one.
if ($minResolved -lt 2) {
  Write-Host ""
  Write-Warning "Only $minResolved process was resolved at some point. A Tauri app is the Rust process PLUS its WebView2 children; a single-process figure is not the app's memory. Treat this run as invalid."
}

if ($stats.Maximum -le $BudgetMB) {
  Write-Host "RESULT  : within budget" -ForegroundColor Green
} else {
  Write-Host "RESULT  : OVER budget" -ForegroundColor Red
}
Write-Host ""
Write-Host "Note: a 'tauri dev' run carries an unoptimized Rust binary and a live"
Write-Host "Vite server. The number of record for the DoD is a release build."
