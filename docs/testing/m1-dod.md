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

5. Watch the footer: **`lag p99 … ms`** appears after a few utterances and
   updates every three seconds. It turns amber above 700 ms. Hover it for p50,
   p95, max and the sample count.

6. When the file finishes, turn **Transcribe** off. The full summary is logged:

   ```
   INFO transcription lag for this run samples=… p50_ms=… p95_ms=… p99_ms=… max_ms=… budget_ms=700
   ```

### What the number means

Lag is measured per settled utterance as **now, minus when those words were
spoken** — wall clock since capture began, less the audio timestamp of the last
word. That covers the whole path: capture, conversion, the socket, Deepgram's
own processing, and the event arriving. It is not one hop timed and called
latency.

Expect p50 well under the budget and p99 close to it. Deepgram only settles an
utterance when it is confident the speaker has finished, so the tail is
dominated by how long it waits, not by anything on this machine.

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

| | |
|---|---|
| Sum of working sets | **583 MB** |
| Private working set | **168 MB** |

The first would have failed a 300 MB budget the app is comfortably inside. The
script reports the private figure, which is what Task Manager's "Memory (private
working set)" column shows.

### The number of record is a release build

A `tauri dev` run carries an unoptimized Rust binary and a live Vite server with
hot-reload state. For the figure that goes in MILESTONES:

```powershell
npm run tauri:build
# then run the installed app, not the dev server
```

---

## Status

| Line | Can be run here | State |
|---|---|---|
| 1 — lag p99 < 700 ms | yes, via loopback | instrumented, not yet run |
| 2 — device lost | needs a USB device | code path never exercised |
| 3 — internet drops | yes | reconnect never exercised against a real drop |
| 4 — RAM < 300 MB | yes | 168 MB at idle on a dev build; not yet under load |

macOS needs all four repeated, and line 1 needs BlackHole there. The gate on M2
is a full 60-minute run on both platforms.
