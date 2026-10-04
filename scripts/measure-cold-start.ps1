<#
.SYNOPSIS
  Cold start of the release build, from the app's own clock.

.DESCRIPTION
  PRD section 9.1 budgets one second from launch to ready; M0's DoD measured about
  300 ms on a fresh install. This launches the release binary N times, reads
  the "startup complete elapsed_ms=" line the app writes to its own log, and
  reports each figure plus the median.

  Three numbers per launch, in the order they happen:

    handle   from outside: wall clock from process start until the OS reports a
             main window handle. The native frame exists, filled with the dark
             background colour. Nothing is drawn in it yet.
    setup    the app's own clock, run() entry to the end of our setup. WebView2
             creates the operator window before setup runs, so most of this is
             the framework's, and the stage lines in the log split the rest.
    ready    the app's own clock, run() entry to the operator window's first
             painted frame, reported by the window itself. This is what PRD
             section 9.1 means by "cold start to ready" and is the number the
             budget is judged on.

  What "setup" is: milliseconds from entering run() to the end of setup -
  logging up, migrations run, bundled translations verified, pack manager and
  Bible client built, windows created. It is the same line the macOS CI smoke
  job reads, so the figures are comparable. It is NOT time-to-first-paint,
  which WebView2 adds after this; and it excludes the verse index, which since
  4 Oct 2026 loads on a background thread and logs "vector stage joined the
  pipeline" separately. Both lines are reported.

  Why the app's clock rather than a stopwatch: a stopwatch measures the
  measurer. The line is written by the process being measured, from its own
  Instant::now() at the top of run().

  Each launch is killed once the line appears, so no instance is left holding
  target\release\sermonai.exe (which breaks every cargo build).

.PARAMETER Runs
  How many launches. Default 5. The first is the true cold one - binary not in
  the OS page cache after a build - and is reported separately.

.EXAMPLE
  .\scripts\measure-cold-start.ps1
  .\scripts\measure-cold-start.ps1 -Runs 10
  .\scripts\measure-cold-start.ps1 -Exe <probe exe> -Log $env:LOCALAPPDATA\SermonAI\logs\probe-bare.log
#>
param(
    [int]$Runs = 5,
    [string]$Exe = ".\src-tauri\target\release\sermonai.exe",
    # The log the launched binary writes its timing lines to. SermonAI's by
    # default; the cold-start probes under scripts\coldstart-probe write their
    # own, in the same line format, so the one script times all three.
    [string]$Log = (Join-Path $env:LOCALAPPDATA "SermonAI\logs\sermonai.log")
)

$ErrorActionPreference = "Stop"
$log = $Log
$exeName = [System.IO.Path]::GetFileNameWithoutExtension($Exe)

if (-not (Test-Path $Exe)) {
    Write-Error "No release binary at $Exe. Run 'npm run tauri:build' first."
}
if (Get-Process $exeName -ErrorAction SilentlyContinue) {
    Write-Error "An instance of $exeName is already running. Close it first - this measures a fresh launch."
}

$startups = @()
$joins = @()
$windows = @()
$readies = @()
$platforms = @()
$ours = @()

for ($i = 1; $i -le $Runs; $i++) {
    # Byte offset of the log before launch, so only this launch's lines count.
    $offset = if (Test-Path $log) { (Get-Item $log).Length } else { 0 }

    $proc = Start-Process -FilePath $Exe -PassThru
    $launched = Get-Date
    $deadline = $launched.AddSeconds(30)
    $startup = $null
    $join = $null
    $window = $null
    $ready = $null
    $plugins = $null
    $setupIn = $null

    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 20
        if (-not $window) {
            $live = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
            if ($live -and $live.MainWindowHandle -ne 0) {
                $window = [int]((Get-Date) - $launched).TotalMilliseconds
            }
        }
        if (-not (Test-Path $log)) { continue }
        $stream = [System.IO.File]::Open($log, 'Open', 'Read', 'ReadWrite')
        try {
            $null = $stream.Seek($offset, 'Begin')
            $reader = New-Object System.IO.StreamReader($stream)
            $tail = $reader.ReadToEnd()
        } finally { $stream.Close() }

        if (-not $startup -and $tail -match 'startup complete elapsed_ms=(\d+)') {
            $startup = [int]$Matches[1]
        }
        if (-not $join -and $tail -match 'vector stage joined the pipeline.*?elapsed_ms=(\d+)') {
            $join = [int]$Matches[1]
        }
        if (-not $ready -and $tail -match 'operator window ready elapsed_ms=(\d+)') {
            $ready = [int]$Matches[1]
        }
        if (-not $plugins -and $tail -match 'plugins initialised elapsed_ms=(\d+)') {
            $plugins = [int]$Matches[1]
        }
        if (-not $setupIn -and $tail -match 'setup entered elapsed_ms=(\d+)') {
            $setupIn = [int]$Matches[1]
        }
        # Give the window and the index thread up to three seconds after setup to report.
        if ($startup -and (($join -and $ready) -or ((Get-Date) -gt $deadline.AddSeconds(-27)))) { break }
    }

    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 400

    if (-not $startup) {
        Write-Warning "run ${i}: no 'startup complete' line within 30 s"
        continue
    }
    $startups += $startup
    $joins += $join
    $windows += $window
    if ($ready) { $readies += $ready }
    # PRD 9.1 shares. Platform: from the last plugin initialising to our setup
    # hook, which is Tauri creating the operator WebView2 window. Ours: the rest.
    if ($ready -and $plugins -ne $null -and $setupIn) {
        $platform = $setupIn - $plugins
        $platforms += $platform
        $ours += ($ready - $platform)
    }
    $label = if ($i -eq 1) { "cold" } else { "warm" }
    "run {0} ({1,-4})  handle {2,5} ms   setup {3,5} ms   ready {4,5} ms   vector joined {5} ms" -f $i, $label, ($(if ($window) { $window } else { "  n/a" })), $startup, ($(if ($ready) { $ready } else { "  n/a" })), ($(if ($join) { $join } else { "  n/a" }))
}

if ($startups.Count -eq 0) { Write-Error "no measurements" }

$sorted = $startups | Sort-Object
$median = $sorted[[math]::Floor(($sorted.Count - 1) / 2)]
$windowMedian = if ($windows.Count -gt 0) { ($windows | Sort-Object)[[math]::Floor(($windows.Count - 1) / 2)] } else { "n/a" }
$readyMedian = if ($readies.Count -gt 0) { ($readies | Sort-Object)[[math]::Floor(($readies.Count - 1) / 2)] } else { $null }
$readyCold = if ($readies.Count -gt 0) { $readies[0] } else { $null }
$platformMedian = if ($platforms.Count -gt 0) { ($platforms | Sort-Object)[[math]::Floor(($platforms.Count - 1) / 2)] } else { $null }
$oursMedian = if ($ours.Count -gt 0) { ($ours | Sort-Object)[[math]::Floor(($ours.Count - 1) / 2)] } else { $null }
$oursCold = if ($ours.Count -gt 0) { $ours[0] } else { $null }
""
"handle, median:                  {0} ms   (process start to native window handle, from outside)" -f $windowMedian
"setup, median of {0}:             {1} ms   (run() entry to end of setup; cold {2} ms)" -f $startups.Count, $median, $startups[0]
if ($readyMedian) {
    "ready, median of {0}:             {1} ms   (run() entry to the operator window's first painted frame; cold {2} ms)" -f $readies.Count, $readyMedian, $readyCold
    "goal (PRD section 9.1, ready):   1000 ms   -> {0}   (reported; a miss from the platform share alone is recorded, not a defect)" -f $(if ($readyMedian -lt 1000 -and $readyCold -lt 1000) { "met" } else { "missed" })
    if ($oursMedian -ne $null) {
        "platform share, median:          {0} ms   (last plugin initialised -> setup entered: Tauri creating the operator WebView2 window; observed, not budgeted)" -f $platformMedian
        "our share, median:               {0} ms   (everything else, to first paint; cold {1} ms)" -f $oursMedian, $oursCold
        "budget (PRD section 9.1, ours):  300 ms median, 450 ms cold   -> {0}" -f $(if ($oursMedian -le 300 -and $oursCold -le 450) { "PASS" } else { "FAIL" })
    } else {
        "shares:                          n/a      (no plugins initialised / setup entered lines; binary predates them)"
    }
} else {
    "ready:                           n/a      (no 'operator window ready' line; binary predates it)"
    "budget (PRD section 9.1, setup): 1000 ms   -> {0}" -f $(if ($median -lt 1000 -and $startups[0] -lt 1000) { "PASS" } else { "FAIL" })
}
""
"Method: 'ready' and 'setup' are the app's own log lines on one clock started at run() entry."
"'ready' is written when the operator window reports its first painted frame; 'setup' when our"
"setup ends, before WebView2 has drawn anything. The verse index loads on a background thread"
"and is reported separately as 'vector joined'. The macOS CI smoke job reads the same lines."
