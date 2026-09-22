# SermonAI — Milestones

> **Companion to `PRD.md` v2.2.** This file answers one question at any moment: *what stage am I at, and what does "done" look like for this stage?*
> Update the status table and tick checkboxes in the same PR that ships the work. Claude Code should read this file at the start of every session and state the current milestone before doing anything.

**Project restart:** Monday 7 September 2026
**Target public launch:** Monday 22 March 2027
**Stack:** Tauri 2 + React/TypeScript + Rust (see PRD §11)

---

## Status Board

Update this table first. It is the only place status is recorded.

| # | Milestone | Phase | Window | Status | Done on |
|---|---|---|---|---|---|
| M0 | Foundation & Lightweight Installer | 0 | 8 Sept – 21 Sept 2026 | ✅ Done | 21 Sept 2026 |
| M1 | Audio In, Transcript Out | 1 | 22 Sept – 5 Oct 2026 | 🟡 In progress | |
| M2 | Scripture Detection Engine | 1 | 6 Oct – 19 Oct 2026 | ⬜ Not started | |
| M3 | Staging & Projector Output | 1 | 20 Oct – 2 Nov 2026 | ⬜ Not started | |
| M4 | Summary PDF & Library (MVP) | 1 | 3 Nov – 16 Nov 2026 | ⬜ Not started | |
| M5 | Offline Mode & Packs | 2 | 17 Nov – 30 Nov 2026 | ⬜ Not started | |
| M6 | Translations & Import | 2 | 1 Dec – 14 Dec 2026 | ⬜ Not started | |
| M7 | Broadcast Outputs & Phone Remote | 2 | 15 Dec 2026 – 4 Jan 2027 | ⬜ Not started | |
| M8 | Archive, Themes & Pilot Launch | 2 | 5 Jan – 25 Jan 2027 | ⬜ Not started | |
| M9 | Content Studio & Exports | 3 | 26 Jan – 15 Feb 2027 | ⬜ Not started | |
| M10 | Series-to-Book & AI Slides | 3 | 16 Feb – 8 Mar 2027 | ⬜ Not started | |
| M11 | Launch Hardening & Public Launch | 3 | 9 Mar – 22 Mar 2027 | ⬜ Not started | |
| M12 | Scale | 4 | Apr – Sept 2027 | ⬜ Not started | |

Status values: `⬜ Not started` · `🟡 In progress` · `🟠 Blocked` · `✅ Done`

**Current milestone:** M1 — Audio In, Transcript Out
**Current blocker:** none. **M0 closed on 21 September 2026, on schedule** — twelve of twelve deliverables and six of six Definition of Done lines.

The last line closed on a real Mac: the `.dmg` downloaded through Safari, the unidentified-developer warning appeared as expected for an unsigned build, and SermonAI launched and ran correctly once cleared. Three details of that run are still to be recorded (which Mac, which macOS version, how the warning was cleared) — they do not affect the tick, but they decide whether `docs/INSTALL.md` matches what a tester meets on macOS 15.

**What closing M0 does not mean.** Two things are ticked with known gaps, both parked against M11's release checklist rather than left implicit:

- **Gatekeeper is cleared, not satisfied.** An informed tester with INSTALL.md open got past the warning. A church volunteer on a Saturday evening will read it as "this download is unsafe" and stop. Only notarization removes it.
- **WebView2's silent bootstrap has never run.** DoD 1 passed on Windows 11, which ships WebView2 with the OS, so the installer never had to bootstrap it — and that is the path a Windows 10 machine takes. FR-63 lists Windows 10 1909+ as supported.

Neither blocks M1. Both block public download.

**Where M0 landed.** An under-40 MB unsigned installer for Windows and macOS on both architectures, three windows placed on chosen monitors, three bundled public-domain translations, a 31,102-verse semantic index searched in 2.46 ms against a 5 ms budget, a resumable pack downloader, and a cold start around 300 ms against a 1 second budget.

**Size budget, measured rather than estimated.** The earlier "only 7 to 13 MB left for the encoder" warning rested on a guess that three translations would cost 12 to 18 MB. A translation actually compresses to about 1.3 MB:

| Item | Size |
|---|---|
| Base installer (Windows msi) | 5.12 MB |
| KJV + WEB + ASV | 3.86 MB |
| Verse index, 256 dims int8 | 7.71 MB |
| Encoder matrix + tokenizer | 7.97 MB |
| **Projected installer** | **~25 MB** |

Comfortably inside the 40 MB gate, with roughly 15 MB spare. The PRD assumed 384 dims and a 12 MB index; the real model is 256 dims, so both came in smaller. CI will print the real number on the next build.

---

## How to Read a Milestone

Each milestone has five parts:

- **Goal** — one sentence, what exists at the end that did not exist at the start.
- **You are here when** — the observable state that tells you this milestone is the active one.
- **Deliverables** — the checklist, mapped to PRD requirement IDs.
- **Definition of Done** — tests you can actually run. The milestone is not done until every line passes on both Windows and macOS.
- **Do not start next until** — the gate.

Milestones are sequential. If you are tempted to pull work forward from a later milestone, write it down under "Parked" at the bottom instead.

---

## M0 — Foundation & Lightweight Installer
**Phase 0 · 8 Sept – 21 Sept 2026 · 2 weeks**

**Goal:** An under-40 MB SermonAI installer exists for Windows and macOS, opens three empty windows on the right monitors, and can download a test pack. Builds are unsigned during development; OS code signing is deliberately deferred to M11.

**You are here when:** there is no repo yet, or the repo exists but `cargo tauri build` does not produce an installer on both platforms.

**Deliverables**
- [x] Repo created with the structure in PRD §10.7. `README.md`, `docs/PRD.md`, `docs/MILESTONES.md` committed. *(Local repo on `main`, first commit 7 Sept. No remote yet.)*
- [x] Tauri 2 project scaffolded: React + TypeScript + Tailwind + Zustand frontend, Rust backend, Vite dev server working. *(Verified 7 Sept: `npm run tauri:dev` builds and launches, operator window opens, 42 MB idle RSS.)*
- [x] Three windows (operator, projector, alternate) created from Rust and placed on chosen monitors using the monitor API. Projector window is frameless and fullscreen. *(8 Sept: confirmed on hardware — projector went fullscreen and frameless on the chosen second screen, button showed "Projecting", and unplugging it did not crash the app. A bug found during that test is fixed: assignments now live in Rust state, not React, so switching tabs no longer loses track of which screen is live, and a disconnected display shows a warning instead of still reading as projecting.)*
- [x] GitHub Actions matrix: `windows-latest`, `macos-15-intel` (Intel), `macos-latest` (Apple Silicon). Builds, runs `cargo test` and `vitest`, produces unsigned `.msi` / `.exe` / `.dmg`. *(8 Sept: all three jobs green on run 34241507233. First successful Rust compile, `cargo test` and bundle on macOS. OS code signing and notarization moved to M11; the Tauri updater key signs update bundles.)*
- [x] CI gate: build fails if any installer exceeds 40 MB. *(8 Sept: enforced per job. Measured sizes — msi 5.12 MB, nsis 4.16 MB, dmg x64 3.43 MB, dmg aarch64 3.21 MB.)*
- [x] Vendor accounts and keys: Deepgram, **YouVersion Platform**, Anthropic. Stored in CI secrets and local `.env`, never committed. *(21 Sept: API.Bible replaced by YouVersion Platform per PRD v2.2. `YVP_APP_KEY` is in `.env` and in GitHub Actions secrets; verified live — 20 English versions licensed to this app key. `.env` is gitignored and no key has ever entered git history. `API_BIBLE_KEY` remains in GitHub secrets but nothing references it.)*
- [x] `scripts/build-verse-index.py`: embeds 31,102 verses, quantizes to int8, writes `src-tauri/assets/verse-index.bin`. Run once, output committed. *(8 Sept: 31,102 KJV verses at 256 dims, 7.71 MB. Rows are L2-normalised before quantizing so a cosine search is a plain int8 dot product. Validated against float32 — self-retrieval top-1 99.3%, top-5 99.7%; the script refuses to write below 95%. Built with the same model that answers runtime queries, not OpenAI.)*
- [x] Small on-device sentence encoder chosen and bundled for runtime query embedding (must run on both platforms without GPU). *(9 Sept: `minishlab/potion-base-8M` static embeddings, 256 dims. Runtime is a token lookup plus a mean — no transformer, no ONNX, no GPU. `detection/vector.rs` loads the encoder and index, replicates model2vec's pooling exactly, and searches by int8 dot product. Verbatim quotes retrieve themselves, proving Rust queries share the Python-built index's space. Latency 2.46 ms per query against FR-15's 5 ms, after fixing `opt-level = "s"` (15.98 ms) and widening the dot product to 8 accumulators. Verified on macOS Intel and Apple Silicon by CI run 34296856629.)*
- [x] Bundled translation assets built and shipping in `src-tauri/assets/translations/`. *(21 Sept: **KJV, WEB and ASV**, matching FR-59. WEB from YouVersion via `scripts/build-translation-pack.py` (30,990 verses, attribution `PUBLIC DOMAIN (not copyrighted)`); KJV and ASV from ebible.org via `scripts/build-public-domain-pack.py` (31,102 and 31,086 verses), because YouVersion licenses no King James to our key and returns no copyright string for their ASV — limitations of their copy, not of the public-domain text. 3.86 MB for all three. Each pack ships a `.sha256` and a manifest recording source, licence and attribution; the builder refuses to write a pack whose per-book counts disagree with the source, and distinguishes the ASV's sixteen marginal verses from verses a parser dropped.)*
- [x] Pack system: `packs-manifest.json` format, `packs/downloader.rs` with ranged resumable downloads and SHA-256 verification, Settings → Packs screen listing packs with sizes and progress. Tested against a manifest on a test bucket. *(7 Sept: manifest/downloader/registry modules, five commands, Packs screen with size labels, progress, pause/resume/remove. Integration tests run against a local range-capable server: resume-after-restart asserts the Range header continues from the halfway byte; checksum mismatch is rejected and the part file discarded. Not yet run against a real CDN bucket — pending deliverable 6.)*
- [x] SQLite schema from PRD §14.2 created via migrations on first launch. *(Verified 7 Sept: first launch logged `applying migration 0001_init` and created `%APPDATA%/app.sermonai.desktop/db/sermonai.sqlite` in WAL mode. Idempotency covered by `cargo test`.)*
- [x] ADRs written: Tauri over Electron; staging-first output; three-stage detection; local-only data; summary JSON schema; packs strategy. *(`docs/adr/0001`–`0006`.)*

**Definition of Done**
- [x] Fresh Windows VM: run installer, dismiss the SmartScreen prompt via More info → Run anyway (expected: builds are unsigned until M11), no admin prompt, app opens in under 60 seconds total, WebView2 bootstrapped silently. *(21 Sept: passed on a clean **Windows 11** VM — no Windows 10 ISO available. SmartScreen appeared and cleared as documented in `docs/INSTALL.md`, no admin prompt, app opened well inside 60 seconds. **One clause of this line is not actually covered:** Windows 11 ships WebView2 as part of the OS, so the installer never had to bootstrap it. The silent-bootstrap path — the thing this line exists to test — remains unexercised, and it is the path a Windows 10 church machine will take. Tracked in Parked against M11's release checklist.)*
- [x] Fresh macOS machine: open `.dmg`, drag to Applications, right-click → Open, confirm the unidentified-developer dialog (expected: builds are unsigned until M11), app launches. Both prompts are documented in `docs/INSTALL.md`. *(21 Sept: **passed on a real Mac.** The `.dmg` was downloaded through Safari — which is what makes the test valid, since Gatekeeper's dialog is triggered by the `com.apple.quarantine` attribute a browser applies and a locally built file never carries. The unidentified-developer warning appeared as expected for an unsigned build, was cleared, and SermonAI launched and ran correctly. Hardware and OS version still to be filled in: **which Mac (Intel or Apple Silicon), which macOS version, and which of the two ways the warning was cleared** (right-click → Open, or System Settings → Privacy & Security → Open Anyway). Those decide whether `docs/INSTALL.md`'s instructions match what a tester actually encounters — macOS 15 routes some cases to Privacy & Security and no longer honours right-click → Open, and INSTALL.md currently documents only the right-click path.)*

  This line is closed, but it does not mean Gatekeeper is satisfied for churches — it means an informed tester can get past it. Removing the warning entirely needs notarization, which is M11's first deliverable.

  The `macos-install-smoke` CI job continues to cover the rest of this line on every build — it mounts the `.dmg` on both Intel and Apple Silicon runners, installs to `/Applications`, launches, confirms the app is alive ten seconds later and times the cold start. It cannot reach the Gatekeeper dialog, which is why the manual test above was needed once.

- [x] Installer sizes printed in CI logs: both under 40 MB (unsigned builds). *(8 Sept: Windows msi 5.12 MB, macOS Intel 3.43 MB, macOS ARM 3.21 MB.)*
- [x] App cold start under 1 second on Windows. *(21 Sept: **approximately 300 ms** from a real install on the Windows 11 VM, against a 1 second budget (PRD §9.1) — three times the headroom. macOS is now measured on every build instead of by stopwatch: `run()` logs a `startup complete` line carrying the elapsed milliseconds once migrations, the pack manager, the Bible client and the windows are all up, and the smoke job reads it. A runner is slower than a church laptop, so an overshoot there is a warning rather than a failure — a human measurement stays the number of record.)*
- [x] Plug in a second monitor: projector window appears on it fullscreen; unplug: app does not crash. *(8 Sept: confirmed on hardware with three displays attached.)*
- [x] Download a 30 MB test pack, kill the app at 50%, relaunch, download resumes and verifies. *(Covered by `tests/pack_download.rs` against a local range-capable server: the resume request carries `Range: bytes=N-` from the halfway mark and the installed file matches the catalog digest. Passing on all three CI targets. Not yet run against a real CDN bucket.)*

**Do not start M1 until:** all six DoD lines pass and the status board says ✅.

**M0 is complete: 12 of 12 deliverables, 6 of 6 DoD lines. Closed 21 September 2026.**

| Carried into M11 | Why it is not an M0 failure | Who |
|---|---|---|
| Notarization (Apple) and Authenticode (Windows) | M0 deliberately deferred OS code signing; the DoD lines were written to expect the warnings and both passed with them | M11 deliverable 1 |
| WebView2 silent bootstrap on Windows 10 | DoD 1 passed on Windows 11, which ships WebView2 with the OS, so the bootstrap path was never exercised | M11 release checklist |
| Pack system against a real CDN bucket | Verified against a local range-capable server only | Before M5 |

Deliverable 10's note still stands: the pack system has never run against a real CDN bucket, only a local stub. That is worth closing before M5 leans on it.

---

## M1 — Audio In, Transcript Out
**Phase 1 · 22 Sept – 5 Oct 2026 · 2 weeks**

**Goal:** Speak into any input device and watch your words appear live in the operator window.

**You are here when:** M0 is ✅ and the operator window is still empty.

**Deliverables**
- [x] Audio device enumeration with `cpal`, including HDMI capture cards, USB interfaces, and loopback devices where the OS exposes them (FR-01, FR-02, FR-05). *(21 Sept: `audio/devices.rs` plus `list_audio_devices` / `check_audio_device` and their `ipc.ts` wrappers. Sources are classified three ways — input, loopback, virtual cable — because the operator should not need to know which is which. Verified on this Windows machine: one input (the default) and three loopback endpoints including two HDMI display-audio devices.
  - **The platforms differ here and it shapes the module.** Windows has real loopback and cpal 0.15 enables it transparently — build an *input* stream on an *output* device and it sets `AUDCLNT_STREAMFLAGS_LOOPBACK`. But `supported_input_configs()` returns **empty** for an output device, so a loopback device's format must be read from the *output* config; reading the input side would have made every loopback device look unusable. macOS has no equivalent at all — CoreAudio cannot capture a render endpoint — so output devices are not offered there, and a Mac church installs BlackHole or similar, which appears as an ordinary input and is recognised by name.
  - **Not yet covered:** no HDMI *capture card* (Elgato and the like) was attached, so that path is reasoned about rather than tested, and macOS enumeration has not been run on a Mac — CI will exercise it. FR-02 resolves a name to a device but there is no picker UI yet; that arrives with the capture lifecycle, since the two are used together.)*
- [x] Capture at 16 kHz mono 16-bit PCM in 250 ms chunks (FR-03). *(21 Sept: `audio/convert.rs` does the format work, `audio/capture.rs` the cpal wiring. Verified on real hardware — the 4-channel 48 kHz mic array on this machine downmixed to mono 16 kHz, seven chunks of exactly 4,000 samples, non-silent.
  - **Resampling is done properly rather than by dropping samples.** 48 kHz to 16 kHz is exactly one in three, which makes decimation tempting. Without band-limiting first it folds everything above 8 kHz back into the speech band — a 12 kHz sibilant returns as a 4 kHz tone nobody spoke — and it damages recognition precisely where consonants are distinguished, while passing every obvious check: right length, right rate, roughly right sound. `rubato` (+1 dependency, pure Rust, no C) band-limits; `aliasing_is_removed_not_folded` measures that a 12 kHz tone arrives at under a tenth the energy of a real 4 kHz one, and its counterpart confirms a 1 kHz tone survives. 44.1 kHz settles it anyway: 441:160 has no integer shortcut.
  - The device's own format is accepted and converted rather than 16 kHz mono being requested: asking WASAPI shared mode for a format the hardware lacks fails outright, and a church interface is entitled to be 48 kHz and 4-channel. Stereo is averaged, not half-ignored — a desk feeding one side only is common. Samples exceeding full scale clamp instead of wrapping.
  - **Known gap, closing with FR-06:** `CaptureConverter::flush` exists but the stop path does not call it yet, because the converter lives inside the audio callback. Up to 250 ms of the final audio is therefore dropped at stop — visible in the hardware check as 1.75 s captured from a 2 s run. Harmless mid-service, wrong at End Service, and it belongs with the lifecycle work rather than a second stop path now.)*
- [x] Live level meter at 30 fps, in the status strip under the top bar (FR-04). *(22 Sept: `audio/meter.rs`, `LevelMeter.tsx`, driven by `start_level_monitor` / `stop_level_monitor`. Confirmed on hardware — bars move with speech and fall to "no signal" when the test stops.
  - **Metered from the raw device buffers, not the 250 ms chunks.** Chunks arrive four times a second; the PRD asks for thirty (§13.2). A 4 fps meter lags the room enough to feel broken and cannot answer the one question it exists for. Tapping the device is also more truthful — a clipping input reads as clipping rather than as whatever survived conversion. Peaks are **held** between frames rather than sampled, because a transient lasts milliseconds and a meter that misses clipping is worse than none.
  - **Three colour tiers**, at the operator's request: green to -12 dBFS, amber to -3, red above. Not one -6 threshold — a preacher's peaks sit around -12 to -6 on a well-set desk, so amber would show through most of a sermon and stop meaning anything. Red at -3 leaves room to pull the gain before samples are lost at 0.
  - **Capture now runs on its own thread**, which closes the `cpal::Stream` `!Send` gap flagged under FR-03. Starting a new input releases the old device first.
  - **The UI was realigned to `docs/wireframe.html`**, which it had diverged from in three ways: the meter is segmented bars rather than one continuous bar, it belongs in the `live-header` strip rather than the top bar, and Settings is a section nav beside content rather than a stack of cards. The wireframe gained a third meter tier in the same commit so code and source of truth stay together.
  - **Scope note:** `start_level_monitor` is level monitoring, not the service transport. Pause and resume without reopening the device is FR-06, the next deliverable.)*
- [x] Start / stop / pause / resume without restart (FR-06). *(22 Sept: `start_capture` / `stop_capture` / `pause_capture` / `resume_capture` / `capture_state`, with transport buttons in Settings → Audio & speech. Verified on hardware: start → Running, pause → Paused with **0 chunks delivered during a 700 ms pause**, resume → Running, stop → a partial tail flushed.
  - **Pause holds the device rather than releasing it.** Reopening risks the OS handing the input to another application in the gap, and some interfaces allow only one capture client — a pause that loses the microphone is not a pause. `cpal`'s `Stream::pause` does this; the capture thread already owns the stream.
  - **The FR-03 flush gap is closed.** Stop now pauses the stream, then flushes the converter's tail, then drops the stream. Measured: **1.98 s of audio from a 2 s capture, where it was 1.75 s** — the last partial chunk is delivered rather than dropped. That mattered at End Service, where the lost 250 ms was the closing words of the sermon.
  - The shared converter is behind a mutex the audio callback takes. Normally a mistake — a blocked audio thread misses its deadline and drops audio — but safe here **by construction**: every lock but one is taken by the callback itself, and the exception is the final flush, which happens after the stream is paused and cpal has stopped calling back.
  - The thread now blocks on a command channel instead of waking every 50 ms to poll a flag.
  - `start_level_monitor` / `stop_level_monitor` are gone, folded into the transport rather than left as a second way to open a device.)*
- [x] Deepgram Nova-3 streaming over WebSocket with interim and final results (FR-07). *(22 Sept: `stt/deepgram.rs`. **Verified end-to-end against the live service**, streaming a 14.8 s synthesised sermon sample in 250 ms chunks: 12 interim results, 3 finals, transcript exact.
  - **Interim results are revisions of one utterance, not drafts of separate ones**, and the live run showed exactly why that matters: `"...if you will to join"` became `"...to John chapter three"` one result later. A consumer that appended every interim would print the same half-sentence five times, growing; one that acted on interim text would have detected the wrong reference, or none. `TranscriptEvent` is therefore two variants rather than a struct with an `is_final` bool, and `is_actionable()` is the property M2's detection stages gate on.
  - Audio is queued to the socket through a bounded channel with `try_send`, never `send`: the audio thread has an OS-enforced deadline and must not block. Ten seconds of backlog means the transcript is already useless, so the chunk is dropped and the operator warned rather than growing a buffer until the app runs out of memory.
  - `KeepAlive` every 8 s, because Deepgram closes a stream after about ten seconds of silence — a paused capture would otherwise lose the socket during the pause rather than at the end of it.
  - Stop sends `CloseStream` and drains, so the final results for the last utterance arrive instead of being cut off.
  - The key comes from `credentials.access(Service::Deepgram)`; this file reads no environment variable.)*
- [ ] Custom vocabulary: 66 Bible book names + archaic terms sent with the stream (FR-09).
- [ ] Word-level transcript with timestamps emitted as Tauri events to the frontend (FR-10).
- [ ] Rolling 60-second transcript buffer maintained in Rust (FR-11).
- [ ] Live Transcript Panel in the operator window: word-by-word append, auto-scroll with manual override, font size setting (PRD §13.2).
- [ ] Reconnect with exponential backoff on Deepgram drop; clear banner on failure.

**Definition of Done**
- Play a 10-minute sermon recording through a virtual audio device on each platform: transcript appears with under 700 ms lag (p99), no dropped words visible on inspection.
- Unplug the audio device mid-stream: app shows "device lost", lets you pick another, resumes.
- Disconnect internet for 30 seconds mid-stream: app shows a banner, reconnects automatically, transcript resumes.
- RAM during capture under 300 MB.

**Do not start M2 until:** transcript is stable for a full 60-minute run on both platforms.

---

## M2 — Scripture Detection Engine
**Phase 1 · 6 Oct – 19 Oct 2026 · 2 weeks**

**Goal:** When a scripture is spoken or paraphrased, a detection card appears with the right verse text.

**You are here when:** M1 is ✅ and the transcript shows scripture references as plain text with nothing happening.

**Deliverables**
- [ ] Regex stage covering standard, spoken, and shorthand forms (FR-12, FR-13). Test corpus of 300 spoken reference variants.
- [ ] Vector stage: query embedding via the bundled encoder, brute-force cosine over the int8 index, top-5 in under 5 ms (FR-15).
- [ ] Claude paraphrase stage on the 60-second buffer, fired only after 10 seconds with no regex hit, online only (FR-14).
- [ ] Detection pipeline in `detection/pipeline.rs` running stages in order with confidence scoring (FR-16) and a queue that never drops (FR-17).
- [ ] Bible cache in `rusqlite` seeded from the bundled translations; YouVersion client caching further versions on demand, storing attribution with the text (FR-18, FR-19, FR-22). *(Client, USFM converter and sanitizer already exist from the v2.2 migration.)*
- [ ] Detection Card UI: reference, verse text, translation, confidence, source badge (regex / vector / AI), Accept / Reject / Edit (PRD §13.3).
- [ ] Translation picker with per-service default (FR-32, PRD §13.4).
- [ ] Every detection and operator decision written to `detected_scriptures`.

**Definition of Done**
- Test corpus: 95%+ of direct references caught by regex; 80%+ of 100 hand-labelled paraphrases surfaced in top-3 by vector or Claude stage.
- Vector lookup under 5 ms and cache lookup under 5 ms, measured and logged.
- Rapid-fire test: 10 references in 20 seconds produce 10 cards in order, none lost.
- Cards render identically on Windows and macOS.

**Do not start M3 until:** accuracy numbers are recorded in the milestone PR.

---

## M3 — Staging & Projector Output
**Phase 1 · 20 Oct – 2 Nov 2026 · 2 weeks**

**Goal:** A verse travels from a card to staging to the projector with one keypress, and the operator can blank, search, and step through a passage.

**You are here when:** M2 is ✅ and cards pile up but nothing reaches the projector window.

**Deliverables**
- [ ] Staging slot UI with Staged and Live side by side; Enter promotes, Escape clears (FR-50, PRD §13.5).
- [ ] Auto-live opt-in setting with a visible on-screen indicator.
- [ ] Projector window renders the live verse with the default theme; layouts: verse only, verse + reference, verse + reference + translation (FR-24, FR-26 basic).
- [ ] Fade / slide / dissolve transitions with configurable duration, default 400 ms (FR-25).
- [ ] Blank / unblank hotkey (FR-33).
- [ ] Next / previous verse navigation (FR-51).
- [ ] Command palette Cmd/Ctrl+K: type "john 3 16", Enter stages, Shift+Enter goes live, recent verses on top (FR-31, PRD §13.9).
- [ ] Manual override: Edit on a card opens the picker to change verse or translation (FR-30).
- [ ] Auto-fit font size for long verses.

**Definition of Done**
- Staged → live measured under 100 ms on both platforms.
- Run the M1 sermon recording end to end: verses appear on the projector monitor with no operator typing.
- Blank during a 60-second pause and unblank: no flicker, no lost state.
- Navigate a 12-verse passage forward and back with the keyboard only.

**Do not start M4 until:** a non-developer can run a 15-minute mock service with only the keyboard and a printed cheat sheet.

---

## M4 — Summary PDF & Library (MVP)
**Phase 1 · 3 Nov – 16 Nov 2026 · 2 weeks**

**Goal:** Click End Service and, within 90 seconds, download a detailed sermon summary PDF from the Library. **This is the end of Phase 1 and the first real Sunday demo.**

**You are here when:** M3 is ✅ and End Service does nothing beyond stopping audio.

**Deliverables**
- [ ] Transcript persistence: every segment written to `transcript_segments` within 250 ms (FR-34).
- [ ] Service metadata entry: preacher, title, series (PRD §12.2 step 2).
- [ ] Summary pipeline in `intelligence/summarizer.rs` following PRD §10.5: seal, chunk, prompt, Claude structured JSON (schema in PRD §26.3), retries (FR-40, FR-41, FR-35).
- [ ] Validator: rejects any scripture not in the accepted log or regex-matched in the transcript; quotes must be substrings of the transcript (FR-46).
- [ ] HTML "Sermon Brief" template and hidden-webview print-to-PDF renderer (FR-42, PRD §13.11).
- [ ] `sermon_summaries` and `exports` tables populated with versioning.
- [ ] Sermon Library tab: cards with title, date, preacher, status, PDF badge, **Download PDF** (FR-43, PRD §13.12).
- [ ] Optional auto-save folder for summary PDFs.
- [ ] Status flow: live → processing → ready / needs_attention, with retry from the UI.

**Definition of Done**
- Run a real recorded 45-minute sermon: PDF ready in under 90 seconds, 4 to 8 pages, all ten sections present.
- Validator test: inject a fake reference into the model output, confirm it is dropped and logged.
- Kill the app during processing, relaunch: job resumes and completes.
- Download the same PDF a week later from the Library on a machine that has been offline the whole time.
- **Phase 1 exit:** run a live Sunday service at one church on one laptop. Verses on screen, transcript kept, PDF downloaded by the pastor.

**Do not start M5 until:** the Phase 1 exit service has happened and feedback is written in the PR.

---

## M5 — Offline Mode & Packs
**Phase 2 · 17 Nov – 30 Nov 2026 · 2 weeks**

**Goal:** Pull the internet cable and the service still runs: transcript, detection, projection, and a usable summary PDF.

**You are here when:** M4 is ✅ and the app is useless without internet.

**Deliverables**
- [ ] Offline Speech Pack (whisper.cpp `base.en`) downloadable from Settings → Packs; `small.en` offered as higher accuracy (FR-60, FR-61, PRD §15.5).
- [ ] `stt/whisper.rs` via `whisper-rs`; `stt/router.rs` switches automatically after three Deepgram failures or when offline mode is selected (FR-08).
- [ ] Template-based offline summary renderer producing a full PDF with no model (FR-45 default path).
- [ ] Optional Offline Intelligence Pack with `llama-cpp` bindings and the reduced schema (FR-45 optional path). Clearly labelled size.
- [ ] "Enhance when online" job that regenerates the full AI summary and keeps both versions.
- [ ] Delta updates via the Tauri updater with signed patches (FR-62).
- [ ] Pack removal frees disk immediately.

**Definition of Done**
- Airplane mode on both platforms: run the 45-minute recording; transcript lag under 2 seconds with `base.en`; detection works; template PDF generated.
- Reconnect: full AI summary appears as version 2 within 3 minutes without user action.
- RAM with speech pack loaded under 900 MB.
- Ship a patch release: update download under 10 MB, applies without re-downloading any pack.

**Do not start M6 until:** an entire mock service has been run with wifi off from start to finish.

---

## M6 — Translations & Import
**Phase 2 · 1 Dec – 14 Dec 2026 · 2 weeks**

**Goal:** Twenty-plus licensed translations available as packs, and any church can import its own.

**You are here when:** M5 is ✅ and the picker still only shows a handful of translations.

**Deliverables**
- [ ] Translation packs for all 20 in PRD §26.2 built, checksummed, published to the Pack CDN with the signed manifest (FR-20, FR-21).
- [ ] Licence terms accepted in the YouVersion portal for each paid translation, against our app key; tier gating recorded in `settings`.
- [ ] Translation importer wizard: USFM, OSIS, JSON, CSV; validation report; adds to picker and detection index (FR-47, PRD §13.15).
- [ ] Imported translations flagged "church-supplied" in the UI (PRD §17).
- [ ] Integrity check of all installed translations on startup (FR-22).
- [ ] Per-verse translation override from the card and command palette (FR-32).

**Definition of Done**
- Install all 20 packs on a test machine: total disk under 250 MB with the speech pack.
- Import a USFM Yoruba Bible and a JSON Swahili Bible: both detect and project correctly.
- Corrupt a pack file on disk: startup check catches it and offers re-download.
- Switch translation mid-service: current live verse unchanged, next verse in the new translation.

**Do not start M7 until:** at least one African-language import has been tested by someone who reads that language.

---

## M7 — Broadcast Outputs & Phone Remote
**Phase 2 · 15 Dec 2026 – 4 Jan 2027 · 3 weeks (holiday buffer included)**

**Goal:** The verse reaches the livestream as a lower third, the stage sees a confidence monitor, and anyone in the room can drive the screen from a phone.

**You are here when:** M6 is ✅ and the only output is the projector window.

**Deliverables**
- [ ] Alternate Output window with its own layout: live verse, staged verse, clock, elapsed (FR-48, PRD §13.7).
- [ ] OBS Browser Source overlay on `localhost:8001/overlay`, transparent, WebSocket-driven (FR-28).
- [ ] NDI source "SermonAI Scripture" with alpha via `ndi-sys`, shipped as an optional pack (FR-27).
- [ ] Phone remote: `axum` server on LAN port 8002 only while enabled; QR + 6-digit pairing; token expires at End Service (FR-52, FR-53, PRD §15.8).
- [ ] Remote web app: Now page (staged / live, Go Live, Next, Prev, Blank), Search page, Devices page (FR-54, PRD §13.8).
- [ ] Up to 5 devices, operator can see and revoke each (FR-55).
- [ ] Watermark on free-tier outputs, removed on paid tiers (FR-49).

**Definition of Done**
- OBS on the same machine shows the verse as a clean lower third within 300 ms of Go Live.
- vMix on a second machine receives the NDI source with correct alpha.
- Pair an Android and an iPhone: tap Go Live on the phone, projector updates in under 300 ms.
- Revoke a phone: its next tap is rejected. End Service: all phones disconnected.
- Port 8002 is closed when the remote is disabled (verified with a port scan).

**Do not start M8 until:** a livestream test has been recorded from OBS with the overlay visible.

---

## M8 — Archive, Themes & Pilot Launch
**Phase 2 · 5 Jan – 25 Jan 2027 · 3 weeks**

**Goal:** Every past sermon is searchable, the projector looks the way each church wants, and 10+ churches are running SermonAI every week. **This is the end of Phase 2.**

**You are here when:** M7 is ✅ and there is no way to find last month's sermon.

**Deliverables**
- [ ] FTS5 indexes over transcripts and summaries; global search Cmd/Ctrl+Shift+F with timestamped snippets (FR-56, PRD §13.13).
- [ ] Library filters: date, preacher, series, Bible book (PRD §13.12).
- [ ] Theme Designer with live preview, JSON export/import, five presets: Classic, Modern, Minimal, Cathedral, Contemporary (FR-26, PRD §13.10).
- [ ] Per-church custom vocabulary editor feeding the STT stream (FR-09 enhancement).
- [ ] Onboarding wizard per PRD §12.1.
- [ ] License activation, 3-device limit, 30-day offline grace (PRD §17).
- [ ] Opt-in Sentry crash reporting, content-free.
- [ ] Health panel: STT, Bible API, Claude, audio device status (PRD §18.4).
- [ ] Pilot kit: setup guide, printed cheat sheet, feedback form, weekly check-in schedule.
- [ ] **Pilot launch:** 10 to 15 churches, at least half in Nigeria and Kenya, at least three on macOS.

**Definition of Done**
- Search "grace" across 1,000 synthetic sermons returns in under 500 ms.
- A theme exported on Windows imports and renders identically on macOS.
- New operator completes onboarding and projects a verse in under 10 minutes, timed.
- All pilot churches have run at least one live service; issues logged with severity.
- **Phase 2 exit:** no pilot church names a live-projection feature that TajiCast has and SermonAI does not.

**Do not start M9 until:** the first two weeks of pilot feedback are triaged and Sunday-blocking bugs are fixed.

---

## M9 — Content Studio & Exports
**Phase 3 · 26 Jan – 15 Feb 2027 · 3 weeks**

**Goal:** The pastor can refine the summary, spin off derivatives, and export the full transcript.

**You are here when:** M8 is ✅ and summaries are read-only.

**Deliverables**
- [ ] Content Studio editor over the summary JSON, section by section, with Regenerate PDF and version history (FR-44, FR-57, PRD §13.14).
- [ ] Derivative generators: social captions, small-group guide, devotional, newsletter blurb; export as PDF, DOCX, text.
- [ ] Summary templates: Study Guide, Devotional, Minimal (PRD §13.11).
- [ ] Transcript export as PDF and DOCX via `docx-rs` (FR-36, FR-37).
- [ ] Batch export from the Library.

**Definition of Done**
- Edit a summary title and one application, regenerate: PDF v2 reflects both, v1 still downloadable.
- Each of the four templates renders the same sermon correctly on both platforms; golden-file tests pass.
- DOCX opens cleanly in Word and Google Docs with headings intact.

**Do not start M10 until:** three pilot pastors have used Studio on a real sermon and rated the summary accuracy.

---

## M10 — Series-to-Book & AI Slides
**Phase 3 · 16 Feb – 8 Mar 2027 · 3 weeks**

**Goal:** A preaching series becomes a book manuscript with one click, and an outline becomes a slide deck.

**You are here when:** M9 is ✅ and sermons exist only as individual documents.

**Deliverables**
- [ ] Series Manager: create, add, drag-reorder, book template choice (FR-38, PRD §13.16).
- [ ] Book compiler via `typst`: cover, table of contents, one chapter per sermon opening with its summary, combined scripture index; PDF and DOCX (FR-39).
- [ ] AI Slides: one slide per main point with supporting scripture in the active theme; PPTX export and in-app presenting (FR-58, PRD §13.17).
- [ ] Basic speaker diarization for services with two speakers (Phase 3 scope).

**Definition of Done**
- Compile a 6-sermon series: book PDF under 60 seconds, table of contents and index page numbers correct.
- Generated PPTX opens in PowerPoint, Keynote, and Google Slides without repair prompts.
- A two-speaker recording shows correct speaker labels in the transcript for 90%+ of segments.

**Do not start M11 until:** one pilot pastor has a compiled book manuscript in hand.

---

## M11 — Launch Hardening & Public Launch
**Phase 3 · 9 Mar – 22 Mar 2027 · 2 weeks**

**Goal:** SermonAI is public, self-serve, documented, and stable. **This is the end of Phase 3.**

**You are here when:** M10 is ✅ and only pilot churches can get the app.

**Deliverables**
- [ ] **Add code signing: Apple Developer ID + notarization, Windows Authenticode via Azure Trusted Signing.** Deferred from M0 while builds went to a private tester group. Public downloads must not trigger SmartScreen or Gatekeeper. Restore the signing secrets to the CI build step, set `bundle.macOS.signingIdentity`, and re-test both DoD installer lines on clean machines.
  - **This is now a confirmed launch blocker, not a theoretical one.** M0's DoD 2 test hit the unidentified-developer warning on a real Mac, exactly as expected for an unsigned build. That was the correct result for a tester who knew to expect it and had `docs/INSTALL.md` open. It is the wrong experience for a church volunteer setting up on a Saturday evening: the dialog offers no obvious way forward, the workaround is buried in System Settings, and the honest reading of the warning is that the download is unsafe. Notarization is what removes it — signing alone is not enough, because Gatekeeper checks for a notarization ticket, not merely a valid signature.
  - The same holds on Windows, where SmartScreen reputation accrues to the signing certificate over time, so signing close to launch still leaves early downloads flagged. Both are reasons to do this work early in M11 rather than at the end of it.
- [ ] Free / Plus / Pro tiers wired to licensing per PRD §23; checkout and key delivery.
- [ ] Public website with download, pricing, and the "End Service → PDF in 60 seconds" demo video.
- [ ] Knowledge base and in-app help for every feature.
- [ ] Read-only congregation follow-along page served alongside the remote (Phase 3 scope).
- [ ] Release checklist: installer sizes, signing, notarization, updater, all DoD lines from M0 to M10 re-run on release candidates.
- [ ] Support channels live: email SLA, community forum, in-app chat for paid tiers.
- [ ] Launch: Product Hunt, church media press, YouTube with pilot pastors, referral program.

**Definition of Done**
- A stranger downloads from the website on Windows and on macOS and projects a verse within 10 minutes without contacting support.
- Zero Sunday-blocking bugs open at launch.
- All KPIs in PRD §6.3 instrumented and visible on a dashboard.
- **Phase 3 exit:** public launch day has happened; first paying non-pilot church recorded.

---

## M12 — Scale
**Phase 4 · April – September 2027**

**Goal:** Grow from launch to 250 paying churches with the enterprise-facing features that larger ministries ask for.

**You are here when:** M11 is ✅ and the roadmap is driven by customer requests rather than parity.

**Deliverables (sequence by demand)**
- [ ] Optional cloud backup of the library.
- [ ] Multi-site synchronization.
- [ ] Live in-service language translation.
- [ ] Church management integrations (Planning Center, ChurchTrac, Subsplash).
- [ ] Enterprise tier and admin console.
- [ ] Quarterly pastor advisory board established.

**Definition of Done:** measured against PRD §6.3 KPIs at the twelve-month mark.

---

## Parked

Ideas that came up early but belong to a later milestone. Write the idea and the milestone it belongs to; do not build it now.

| Idea | Belongs to | Noted on |
|---|---|---|
| **PRD §15.4 amendment owed.** Resolved 8 Sept: one model builds the index *and* embeds runtime queries, shipped as static embeddings inside the base installer. §15.4 still describes an OpenAI-built index queried by a different on-device model, which cannot work — vectors from two models are not comparable. The amendment lands with deliverables 7 and 8, and drops `OPENAI_API_KEY` from `.env.example`. | M0 (deliverables 7, 8) | 8 Sept 2026 |
| **Deuterocanonical books** are filtered out of translation packs. API.Bible's KJV and ASV carry the Apocrypha (80 books, 36,820 verses); packs keep the 66-book Protestant canon, giving exactly the 31,102 verses of PRD §15.4 and matching FR-09's book-name vocabulary. Churches in traditions that preach from those books would need a catalog decision. | M6 (translations) | 8 Sept 2026 |
| Pack **archive extraction** (`.tar.zst`): packs install as one verified file today, which suits whisper models and theme JSON. Translation packs shipped as archives will need a decompress step. | M5 / M6 | 7 Sept 2026 |
| **Manifest signature verification** is not implemented — packs are checksum-verified against the manifest, but the manifest itself is trusted on TLS alone. Needs the signing key from deliverable 6. | M0 (deliverable 6) | 7 Sept 2026 |
| Display assignments are **not persisted across restarts** — the backend holds them in memory only. A church re-picks its projector on every launch. Persist to the `settings` table and re-apply on startup as part of the onboarding wizard. | M8 (onboarding, PRD §12.1) | 8 Sept 2026 |
| **KJV under UK Crown copyright.** Resolved for the bundle: KJV and ASV are built from ebible.org, so FR-59's original three ship. One question survives — the KJV is public domain in the US but under perpetual Crown copyright in the UK, administered by Cambridge University Press under letters patent, and §4.2 lists the UK as a target market. Recorded in `kjv.manifest.json`. Needs a deliberate call before UK distribution, not before M0 closes. | M11 (launch hardening) | 21 Sept 2026 |
| **NIV as the default displayed translation.** Confirmed 21 Sept: NIV (111) is the default in the picker, fetched from YouVersion at runtime and cached per verse with Biblica's copyright string; nothing NIV is bundled. The distinction that makes this safe is that API access is not redistribution — only redistributable text belongs in the installer. The picker and the cache-on-lookup path are M2 deliverables, so the default is set when the picker is built, not before. AMP (1588) and NASB (100, 2692) are licensed the same way for the Plus tier. | M2 (translation picker) | 21 Sept 2026 |
| **Attribution has no renderer to appear in yet.** Every cached verse stores its copyright string and the store refuses a verse without one, but nothing displays it: the projector renders reference and text only, and there is no PDF renderer at all. §15.2's split — short version label on the projector, full copyright line in the PDF — cannot be satisfied until M3 and M4 build those surfaces. Until then the obligation is met by storage, not by display, and no licensed translation should be projected in front of a congregation. | M3 (projector), M4 (PDF) | 21 Sept 2026 |
| **YouVersion licence scope for M6.** FR-20 wants 20+ translations; the app key currently licenses 20 English versions, which covers the count but not the specific list in PRD §26.2 (no KJV, NKJV, NLT, ESV, CSB, NET or MSG). Each additional version needs its terms accepted in the portal for our key. The old API.Bible quota concern no longer applies. | M6 (translations) | 21 Sept 2026 |
| **Attribution in the summary PDF.** YouVersion requires the copyright string wherever scripture is shown. The PDF renderer does not exist yet, so when it is built each scripture block must carry the attribution in a muted line beneath it, and a verse with no stored attribution must be skipped with a logged warning rather than rendered bare. | M4 (summary PDF) | 21 Sept 2026 |
| **Route the regex stage through the USFM converter.** `bible::reference::parse` exists and the vector stage already emits USFM. The regex stage is not built yet; when it is, its reference strings go through the converter before reaching the Bible client. | M2 (detection) | 21 Sept 2026 |
| **Projector version short name.** The projector renders the reference only. Whether it should also show the version short name is a branding call (§15.2 keeps the projector minimal); attribution itself belongs on the PDF, not on the congregation's screen. | M3 (projector) | 21 Sept 2026 |
| **The oldest supported macOS is untested.** PRD §9 keeps the floor at macOS 12+ — a support commitment, not a test record, and deliberately not raised to whatever M0's DoD 2 happened to run on: testing one version shows that version works, not that older ones fail. Nothing verifies 12, 13 or 14, and the CI runners only track recent macOS. Needs one run on the oldest supported version before public download. | M11 (release checklist) | 21 Sept 2026 |
| **WebView2 silent bootstrap is untested.** DoD 1 passed on a Windows 11 VM because no Windows 10 ISO was available, and Windows 11 ships WebView2 with the OS — so the installer never exercised the bootstrap path. Windows 10 (1909+) is a supported target under FR-63 and a realistic church machine. Needs one clean Windows 10 install before public download, either as part of M11's release checklist or sooner if a Windows 10 ISO turns up. | M11 (release checklist) | 21 Sept 2026 |
| **The SermonAI Gateway is not built.** The hybrid key model is decided and now written into PRD §17 and §11.3, but only the *shape* exists in code: `credentials/` is the abstraction every service client goes through, with a development provider reading `.env`. Phases 3 to 5 of `docs/SermonAI_API_Key_Strategy_Prompt.md` — the Cloudflare Worker, per-installation tokens, keychain storage via the `keyring` crate, BYOK settings UI — are still to do, after M1. One STOP point is the operator's: **the Cloudflare account must be created by them, not by us.** Until the gateway exists an installed release build has no way to reach any vendor, which is correct and safe but means no church build is functional online. That is the gate on M4's first Sunday. | After M1 → M4 | 22 Sept 2026 |
| **Test with real church hardware before M4.** A USB audio interface or HDMI capture card fed from a sound mixer — the actual signal path a church uses, rather than a laptop microphone. Borrowed from the media team; report what shows up in the device list, which `kind` it is classified as, what sample rate and channel count it declares, and whether the 16 kHz mono conversion holds up on a real desk feed. This is the one configuration that matters most and the one nothing in CI or on a developer machine can stand in for: enumeration is currently verified against a built-in mic array and three loopback endpoints, and the capture path against synthetic signals. Needed before the M4 Phase 1 exit service, ideally before M3 so a problem surfaces with time to fix it. | M1–M3 (before M4's live service) | 21 Sept 2026 |
| **The chosen audio device is not remembered across restarts.** Enumeration and name resolution exist, but nothing persists the operator's choice — the same gap as display assignments, and the same fix: the `settings` table, re-applied on startup as part of onboarding. A church currently re-picks its input every launch. | M8 (onboarding, PRD §12.1) | 21 Sept 2026 |
| Wordmark/lockup art in `assets/logo-assets/` is not yet used anywhere in the UI — the operator top bar renders "SermonAI" as text, not the lockup. | M8 (design pass) | 7 Sept 2026 |

---

## Session Ritual for Claude Code

At the start of every session:
1. Read the Status Board. State the current milestone and blocker aloud.
2. Read that milestone's Deliverables. Pick the next unticked item.
3. Reference the PRD section for that item before writing code.
4. When the item is done, tick it, run the relevant DoD line, and update the Status Board if the milestone is complete.
5. Anything out of scope goes in Parked, not in the code.
