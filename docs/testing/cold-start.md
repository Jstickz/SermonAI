# Cold start on Windows, release build (4 October 2026)

PRD §9.1 budgets **one second from launch to ready** on an 8 GB, 4-core laptop.
M0's DoD recorded "approximately 300 ms" on 21 September from a stopwatch on a
Windows 11 VM, before the app had a clock of its own. This note re-measures with
the app's clock and states the method beside every number.

## Method

`scripts/measure-cold-start.ps1` launches `target/release/sermonai.exe` N times
and reads three figures per launch, in the order they happen:

| Column | Measured by | Meaning |
|---|---|---|
| handle | the script, from outside | process start → the OS reports a main window handle. The native frame exists, filled with the dark background. Nothing drawn. |
| setup | the app, `startup complete elapsed_ms` | `run()` entry → end of our setup hook. WebView2 creates the operator window *before* setup runs, so most of this is the framework's; stage lines in the log split the rest. |
| ready | the app, `operator window ready elapsed_ms` | `run()` entry → the operator window's first painted frame, reported by the window itself through `operator_window_ready`. **This is §9.1's "ready".** |

All app-side figures share one `Instant` taken at the top of `run()`. The log
is `%LOCALAPPDATA%\SermonAI\logs\sermonai.log`. Machine: the development laptop
(not the VM), binary under the OneDrive folder; a copy outside OneDrive measured
the same to within noise (1265 vs 1255 ms median on the setup figure).

## What startup cost, and why, in three builds

| Build (same day, same machine) | setup median | of which ours | of which output windows | before our code runs |
|---|---|---|---|---|
| `6f59633` — index moved off the startup path, output windows at startup | 2746 ms | 62 ms | 1341 ms | 1156 ms |
| output windows deferred to first display assignment | 1255 ms | 62 ms | 0 | 1095 ms |
| final, with the `ready` line (7 launches) | 1362 ms | 87 ms | 0 | 1275 ms |

Stage lines from the final build's last launch:

```
logging ready                  elapsed_ms=1
setup entered                  elapsed_ms=1275   <- Tauri plugins + operator WebView2 window
database ready                 elapsed_ms=1323
bundled translations verified  elapsed_ms=1361
startup complete               elapsed_ms=1362
vector stage joined (own thread, own clock)  elapsed_ms=117
operator window ready          elapsed_ms=1630   <- first painted frame
```

Final build, seven launches, first one cold after the build:

| | handle | setup | ready |
|---|---|---|---|
| cold | 105 ms | 1905 ms | **2275 ms** |
| warm median (6) | ~45 ms | 1362 ms | **1651 ms** |

**Result: FAIL against 1000 ms, by 650 ms at the median.**

## Where the time is

1. **1.28 s before our code runs.** Between `logging ready` (1 ms) and `setup
   entered` is Tauri's plugin init and WebView2 creating the operator window
   from `tauri.conf.json`. Nothing of ours executes in that window. The WebView2
   runtime is the OS's; its browser process has to start.
2. **~90 ms of ours in setup.** Database open and migrations 48 ms, translation
   integrity checks 38 ms, the rest under 5 ms. The verse index (7.7 MB) and
   encoder (8 MB) load on a thread and join at about 115 ms, off the path.
3. **~270 ms from end of setup to first paint.** WebView2 loading the operator
   bundle and React mounting. The bundle is small already — 140 KB shared JS,
   40 KB operator JS, 32 KB CSS, uncompressed — so most of this is the webview
   starting its page, not our code size. Little to trim.
4. **0 ms for the output windows**, which used to be 1.34 s: two further
   WebView2 instances. They are now created the first time a display is
   assigned (`windows::ensure_output_window`), logged with their own cost.

## What this means for §9.1

Of the 1.65 s, about 0.36 s is ours and about 1.28 s is the framework and OS
bringing up one WebView2 window. Even with every line of ours removed the
figure would be ~1.3 s on this machine. This is the §18.1 situation again: a
single number hides that most of the delay belongs to someone else. The
recommendation is to budget it the same way — a figure for our share and a
separately observed figure for the platform's — rather than to keep a target the
platform alone exceeds. That is a PRD decision; the number of record stays FAIL
until it is taken.

Everything here was measured on the development laptop. The budget is written
for an 8 GB, 4-core church laptop, which will be slower; the first real Sunday
in M4 is where that figure gets taken.
