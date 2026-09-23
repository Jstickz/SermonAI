# M1 Definition of Done — how to run each line

Four lines. Two need only this machine; two need hardware or a Mac.

The app instruments its own lag and a script measures its own memory, so three
of the four produce a **number** rather than an impression.

---

## Line 1 — Ten-minute sermon, lag under 700 ms (p99)

> *Play a 10-minute sermon recording through a virtual audio device on each
> platform: transcript appears with under 700 ms lag (p99), no dropped words
> visible on inspection.*

### No virtual audio device needed on Windows

The DoD says "virtual audio device", which usually means installing VB-Audio
Cable. **Skip it.** SermonAI already supports WASAPI loopback (FR-05), so
playing the file through the speakers and capturing the output *is* the routed
signal path — and it exercises the loopback code a church will actually use for
a sound-desk feed, which a virtual cable would not.

macOS has no equivalent, so the Mac run does need BlackHole or similar. That is
the platform difference described in `audio/devices.rs`.

### Steps

1. Build the test audio if you have not already:

   ```powershell
   .\scripts\make-test-sermon.ps1 -Minutes 10
   ```

   Writes `%TEMP%\sermonai-test-sermon.wav`, about 12 minutes of synthesised
   preaching with real references in it.

2. Start SermonAI. In **Settings → Audio & speech**, pick the **System audio**
   entry for whatever your speakers are (for example *Speakers (Realtek(R)
   Audio)*). That is the loopback device.

3. Go to the **Live** tab and turn **Transcribe** on.

4. Play the file at a normal volume:

   ```powershell
   Start-Process "$env:TEMP\sermonai-test-sermon.wav"
   ```

   Volume matters. Aim for the meter sitting in **green with occasional
   amber** — red means clipping and will hurt recognition, and a level near the
   floor will too.

5. Watch the footer. Two figures appear after a few utterances and refresh
   every three seconds:

   ```
   appear 452/681/698 ms      settle 811/2289/2289 ms
   ```

   `appear` is p50/p95/p99 for when words land on screen; it turns amber above
   700 ms. `settle` is when Deepgram confirms them. Hover either for the sample
   count and maximum. A reconnect count appears if the connection dropped.

6. When the file finishes, turn **Transcribe** off. Both summaries are logged:

   ```
   INFO lag until words appear (interim) ... p50_ms=… p95_ms=… p99_ms=… budget_ms=700
   INFO lag until an utterance is confirmed (settled) ... p50_ms=… p95_ms=… p99_ms=…
   ```

### Which number the line is asking for

**`appear`.** The line says "transcript *appears*", and words appear as interim
results. That is the figure to compare against 700 ms.

`settle` is when Deepgram stops revising an utterance, which it decides only
once it judges the speaker to have stopped — measured at roughly 2.3 s and not
movable through the `endpointing` parameter. It is recorded because M2's
detection runs on confirmed text, so it bounds how quickly a spoken reference
can reach the projector. It is **not** what this line budgets.

Reporting `settle` against the 700 ms budget is exactly the mistake that made a
passing pipeline read as a fourfold failure; see
`docs/testing/m1-latency-baseline.md`.

### What the numbers measure

Per result: **now, minus when those words were spoken** — wall clock since the
first audio was *sent*, less the audio timestamp of the last word. That covers
chunking, the socket, Deepgram's processing and the event arriving.

The anchor matters. Timing from when *capture* started charges the WebSocket
handshake, over a second against the live service, to every sample.

Of the total, 250 ms is our own chunk accumulation and is fixed by FR-03. The
rest is network and Deepgram.

### If a reconnect happened

The footer shows a reconnect count, and those samples are excluded from the
percentiles: replayed audio is sent faster than real time, so its results are
late by construction and would let a network outage read as a slow pipeline. A
run with reconnects covers less than the whole period, and the log says how many
samples were set aside.

### Also check, by reading

- No missing sentences where the audio was continuous
- Paragraph breaks fall at pauses, not mid-thought
- Book names are spelled correctly — "Thessalonians", "Habakkuk", not "the
  salonians"

---

## Line 2 — Device lost mid-stream

> *Unplug the audio device mid-stream: app shows "device lost", lets you pick
> another, resumes.*

Needs a USB microphone or interface — the built-in array cannot be unplugged.

1. Plug in the USB device, select it in **Settings → Audio & speech**
2. Turn **Transcribe** on and speak
3. Unplug it mid-sentence
4. Expected: a message naming the device as disconnected, and the ability to
   choose another and carry on without restarting

**This is the least tested path in M1.** `capture.rs` has an error callback
that emits `audio:error` on `DeviceNotAvailable`, but no device has ever
actually been pulled while running. Treat a failure here as likely rather than
surprising.

---

## Line 3 — Internet drops for 30 seconds

> *Disconnect internet for 30 seconds mid-stream: app shows a banner, reconnects
> automatically, transcript resumes.*

1. Turn **Transcribe** on and speak
2. Disable wifi, or pull the ethernet cable
3. **Keep speaking for 30 seconds** — this is the part that matters
4. Reconnect

Expected:

- An amber banner: *"Deepgram is unreachable. Retrying in Ns (attempt N). Still
  recording — Ns of speech held and will be transcribed when it returns."*
- The held seconds climb while you speak
- On reconnect the banner clears and **the words spoken during the outage
  appear**, in order, before anything said after it

Audio is buffered for 60 seconds. Past that, a red banner reports how many
seconds were dropped rather than closing the gap silently.

**The reconnect itself has never been exercised against a real drop.** The
backoff, the buffer eviction and the clock arithmetic are unit-tested, and the
happy path was verified against the live service, but forcing a genuine
disconnection needs the network cut. This line *is* that test.

Worth checking afterwards: the `lag p99` figure. A reconnect flushes buffered
audio faster than real time, so those utterances settle late by design and will
pull the tail up. That is expected, not a regression.

---

## Line 4 — RAM under 300 MB during capture

> *RAM during capture under 300 MB.*

```powershell
# with the app running and transcribing
.\scripts\measure-memory.ps1 -Seconds 600 -Label "10-minute sermon"
```

Run it alongside line 1 and both lines are covered in one pass.

### Measure the private working set, not the sum of working sets

A Tauri app is one Rust process plus a WebView2 process per window and several
shared Chromium helpers — **nine processes** on this machine. `WorkingSet`
counts pages shared between them once *per process*, so summing it triple-counts
Chromium's shared code.

Measured at idle on a dev build:

| Method | Resolved | Reported |
|---|---|---|
| Sum of working sets | 9 of 9 | **583 MB** |
| Private working set, resolved by counter *name* | **1 of 9** | **6 MB** |
| Private working set, resolved by **PID** | 9 of 9 | **193 MB** |

Both wrong methods look plausible. Summing working sets triple-counts Chromium's
shared pages and would fail a budget the app is inside. Resolving by
performance-counter name silently loses most of the tree, because several
processes are all called `msedgewebview2` — that produced a 40 MB reading on a
DoD run, which is `sermonai.exe` on its own.

The script now matches by PID, prints how many processes it resolved on every
sample, and declares a run invalid if it only finds one.

### The number of record is a release build

A `tauri dev` run carries an unoptimized Rust binary and a live Vite server with
hot-reload state. For the figure that goes in MILESTONES:

```powershell
npm run tauri:build:local
```

Use `tauri:build:local`, not `tauri:build`. The latter also signs the updater
artifacts and needs `TAURI_SIGNING_PRIVATE_KEY`, which lives in GitHub Actions
secrets rather than on a development machine. Without it the build produces
every bundle correctly and *then* fails on the signature, which reads as a build
failure when nothing is actually wrong. `tauri:build:local` turns that step off;
CI keeps it on, because release downloads do need signed update bundles.

Then run the built app rather than the dev server:

```powershell
& ".\src-tauri\target\release\sermonai.exe"
```

---

## Status

| Line | State on Windows |
|---|---|
| 1 — words appear under 700 ms p99 | **failing on the first figure, needs re-running** after two measurement faults were fixed |
| 2 — device lost | **passed** 23 Sept, Bluetooth headset |
| 3 — internet drops | **passed** 23 Sept, real network, including a two-minute outage |
| 4 — RAM under 300 MB | **unverified** — the 40 MB figure was an undercount; re-run needed |

The 60-minute stability run passed on Windows.

**macOS needs all four repeated and none has run there**, and line 1 needs
BlackHole because CoreAudio cannot capture a render endpoint. Parked as a known
risk rather than done.
