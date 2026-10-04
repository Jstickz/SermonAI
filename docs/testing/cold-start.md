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

## The platform's share, measured on its own

Two probe apps under `scripts/coldstart-probe/` open one window shaped exactly
like the operator window — same size, theme, background, `tray-icon` feature,
same `Cargo.lock` — and log the same four lines. `probe-bare` has no plugins;
`probe-plugins` registers the three SermonAI registers (dialog, fs, opener).
Both built into SermonAI's target directory and timed with the same script,
seven launches each, same afternoon, SermonAI idle in the background:

| | handle | setup | ready (median) | ready (cold) |
|---|---|---|---|---|
| probe-bare | 590 ms | 576 ms | **634 ms** | 798 ms |
| probe-plugins | 537 ms | 581 ms | **645 ms** | 714 ms |
| SermonAI (same day) | 70 ms | 1362 ms | **1651 ms** | 2275 ms |

So on this machine **Tauri 2 plus WebView2 costs about 640 ms to a painted
window, and our three plugins about 10 ms more.** The earlier attribution of
1275 ms to "the framework, before our code runs" was wrong by roughly 700 ms:
the probe reaches the same point in 576 ms. Something in how SermonAI is
built or configured, not in Tauri itself, accounts for the rest.

Ruled out the same afternoon, each by measurement:

| Hypothesis | Test | Result |
|---|---|---|
| The three plugins | `probe-plugins` vs `probe-bare` | +11 ms. Not it. |
| SermonAI's accumulated WebView2 profile (36.8 MB with browser components, vs 12.5 MB fresh) | copied SermonAI's profile into `probe-plugins`'s profile folder, timed again | 647 ms, unchanged. Not it. |
| The frontend: SermonAI's built `dist`, its CSP, capabilities for three windows, no console (`windows_subsystem`) | `probe-sermonui` | 683 ms — the real page costs about 40 ms more than a blank one. Not it. |
| The linked dependency set (cpal, rusqlite, reqwest, tokenizers, axum, keyring, …), linked but never called | `probe-deps` | 646 ms. Not it. |

Side finding: the `handle` column is an artefact of the console. Probes built
without `windows_subsystem = "windows"` report the window handle at ~550 ms,
the one built with it at ~45 ms, like SermonAI. It measures when Windows
considers the process to have a main window, not when anything is drawn, and
is kept only as a sanity check from outside the process.

What remains between `probe-sermonui` (683 ms) and SermonAI (1651 ms) is
inside SermonAI's `run()` before and after setup, and the marks added on 4 Oct
(`context generated`, `builder assembled`, `plugins initialised`) split it.

## The missing 700 ms was the environment, not the code

The build with the three pre-setup marks, measured 17:40–17:50 the same day,
ten launches:

```
logging ready        0 ms
context generated    2 ms
builder assembled    2 ms
plugins initialised  7 ms
setup entered      561 ms   <- Tauri + WebView2 creating the operator window: the platform's 550 ms
database ready     579 ms
translations ok    597 ms
startup complete   598 ms   <- our setup: ~37 ms
operator ready     702–1382 ms, median 716 ms
```

| | setup median | ready median | ready range |
|---|---|---|---|
| SermonAI, 16:40 build | 1362 ms | 1651 ms | 1573–2275 |
| SermonAI, 17:38 build, same pre-setup code | **592 ms** | **716 ms** | 702–1382 |
| probe-sermonui | 562 ms | 683 ms | 669–1007 |

Nothing in the code path before setup changed between the two SermonAI builds
except the marks themselves, and the marks show 7 ms of our code before Tauri
takes over. What did change: 5 GB of stale debug symbols were deleted (disk
went from 2% to 4% free), and **OneDrive was found not running** at the later
measurement. `src-tauri/target` — 26 GB of build output — sits inside the
OneDrive sync root with the pinned attribute, and every earlier measurement
followed a 15-minute build that wrote gigabytes into it. The hypothesis that
fits every number is that OneDrive was uploading build output during the
earlier runs. It cannot be proven after the fact; it can be prevented, by
moving the target directory out of OneDrive (`.cargo/config.toml`
`build.target-dir`, or excluding the folder in OneDrive's settings). That is a
machine decision for the operator. *(Taken on 4 October 2026: the
whole project, build output included, now lives at `C:\Projects\SermonAI`,
outside any sync root.)*

The tail to 1.4 s in four launches of ten was then traced and removed. Tauri
runs synchronous commands on the main thread, and the Live tab asks for the
audio device list the moment it mounts: a WASAPI endpoint walk of 300–700 ms
sat on the main thread between the first paint and the event loop, delaying
the ready call (and everything else) by that much. Two changes, 17:50 build:

- `list_audio_devices` is async on a blocking worker, off the main thread.
- The window passes its own `Date.now()` at the painted frame; the log line
  carries both that (`elapsed_ms`) and when the call arrived
  (`reported_after_ms`). The gap is the IPC queue, now 10–13 ms.

Final build, ten launches, OneDrive not running:

| | handle | setup | ready | ready − setup |
|---|---|---|---|---|
| cold | 66 ms | 666 ms | 790 ms | 124 ms |
| warm median (9) | 33 ms | 616 ms | **716 ms** | ~105 ms |
| range | 25–66 | 584–666 | **688–802** | 100–160 |

Stage split of a typical launch: logging 0 → context 2 → builder 2 →
plugins initialised 7 → **setup entered 561** → database 579 → translations
597 → setup complete 598 → **first paint 716**.

**Result: 716 ms median to the operator's first painted frame. Of that,
554 ms is Tauri creating the operator WebView2 window (platform) and 162 ms is
ours (cold: 236 ms).**

## What this means for §9.1 (amended 4 October 2026)

The single number hid that three quarters of it belongs to the platform, the
same way §18.1's single latency number hid Deepgram's share. §9.1 is now
budgeted in two shares, measured by the one script:

- **Ours, strict:** ≤ 300 ms median, ≤ 450 ms on the first launch after
  install. Measured 162 ms / 236 ms here. The headroom is deliberate: the
  reference machine is an 8 GB, 4-core church laptop, slower than this one.
- **The platform's, observed:** Tauri + WebView2 creating the operator window.
  554 ms here; a bare Tauri 2 window is 634 ms to paint on this machine. Nothing
  in this codebase shortens it, so it is recorded per machine, not gated.
- The one-second total stays the goal and is always reported; a miss from the
  platform share alone is recorded as such.

Two things the day also taught, kept here so they are not re-learned:

- **Build output inside OneDrive distorts timing.** `src-tauri/target` is 26 GB
  under the OneDrive sync root with the pinned attribute. The 700 ms that
  vanished between the 16:40 and 17:38 builds coincided with OneDrive not
  running; it is the only variable that moved. Moving the target directory out
  of OneDrive is recommended for every measurement, and for the disk.
  *(Done 4 October 2026 — the project moved to `C:\Projects\SermonAI`; later
  measurements are not subject to this.)*
- **Synchronous Tauri commands block the main thread.** Anything that walks
  devices, reads the credential store or touches the network must be `async`
  (or on `spawn_blocking`) if the window may call it while painting.

The first real Sunday in M4 takes the figure on church hardware.
