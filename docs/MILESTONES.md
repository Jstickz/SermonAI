# SermonAI — Milestones

> **Companion to `PRD.md` v2.1.** This file answers one question at any moment: *what stage am I at, and what does "done" look like for this stage?*
> Update the status table and tick checkboxes in the same PR that ships the work. Claude Code should read this file at the start of every session and state the current milestone before doing anything.

**Project restart:** Monday 7 September 2026
**Target public launch:** Monday 22 March 2027
**Stack:** Tauri 2 + React/TypeScript + Rust (see PRD §11)

---

## Status Board

Update this table first. It is the only place status is recorded.

| # | Milestone | Phase | Window | Status | Done on |
|---|---|---|---|---|---|
| M0 | Foundation & Lightweight Installer | 0 | 8 Sept – 21 Sept 2026 | 🟡 In progress | |
| M1 | Audio In, Transcript Out | 1 | 22 Sept – 5 Oct 2026 | ⬜ Not started | |
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

**Current milestone:** M0
**Current blocker:** none blocking code. CI is green on all three targets (run 34241507233, 8 Sept): Windows, macOS Intel and macOS Apple Silicon all build, lint, test and bundle inside the 40 MB gate. Remaining M0 work is gated on decisions and hardware, not engineering: (a) the encoder/embedding-space decision blocks deliverables 7, 8 and 9 — PRD §15.4 pairs an OpenAI-built index with a different on-device query encoder, and vectors from two models are not comparable (see Parked); (b) the base installer leaves only ~7 to 13 MB for the encoder once the 12 MB index and three translations are added, against the 25 MB deliverable 8 allows; (c) DoD lines 1, 2, 4 and 5 need clean Windows and macOS machines to test on.

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
- [ ] Three windows (operator, projector, alternate) created from Rust and placed on chosen monitors using the monitor API. Projector window is frameless and fullscreen. *(Built: all three windows created from Rust, outputs frameless and hidden until assigned; `list_monitors` / `set_projector_monitor` / `set_alternate_monitor` commands and a Settings → Displays picker. Not yet confirmed by eye on a second screen.)*
- [x] GitHub Actions matrix: `windows-latest`, `macos-15-intel` (Intel), `macos-latest` (Apple Silicon). Builds, runs `cargo test` and `vitest`, produces unsigned `.msi` / `.exe` / `.dmg`. *(8 Sept: all three jobs green on run 34241507233. First successful Rust compile, `cargo test` and bundle on macOS. OS code signing and notarization moved to M11; the Tauri updater key signs update bundles.)*
- [x] CI gate: build fails if any installer exceeds 40 MB. *(8 Sept: enforced per job. Measured sizes — msi 5.12 MB, nsis 4.16 MB, dmg x64 3.43 MB, dmg aarch64 3.21 MB.)*
- [ ] Vendor accounts and keys: Deepgram, API.Bible, Anthropic. Stored in CI secrets and local `.env`, never committed.
- [ ] `scripts/build-verse-index.py`: embeds 31,102 verses (OpenAI `text-embedding-3-small`), reduces to 384 dims, quantizes to int8, writes `src-tauri/assets/verse-index.bin` (~12 MB). Run once, output committed.
- [ ] Small on-device sentence encoder chosen and bundled for runtime query embedding (must be under 25 MB, must run on both platforms without GPU).
- [ ] KJV, WEB, ASV built into bundled translation assets by `scripts/build-translation-pack.py`.
- [x] Pack system: `packs-manifest.json` format, `packs/downloader.rs` with ranged resumable downloads and SHA-256 verification, Settings → Packs screen listing packs with sizes and progress. Tested against a manifest on a test bucket. *(7 Sept: manifest/downloader/registry modules, five commands, Packs screen with size labels, progress, pause/resume/remove. Integration tests run against a local range-capable server: resume-after-restart asserts the Range header continues from the halfway byte; checksum mismatch is rejected and the part file discarded. Not yet run against a real CDN bucket — pending deliverable 6.)*
- [x] SQLite schema from PRD §14.2 created via migrations on first launch. *(Verified 7 Sept: first launch logged `applying migration 0001_init` and created `%APPDATA%/app.sermonai.desktop/db/sermonai.sqlite` in WAL mode. Idempotency covered by `cargo test`.)*
- [x] ADRs written: Tauri over Electron; staging-first output; three-stage detection; local-only data; summary JSON schema; packs strategy. *(`docs/adr/0001`–`0006`.)*

**Definition of Done**
- Fresh Windows 10 VM with no WebView2: run installer, dismiss the SmartScreen prompt via More info → Run anyway (expected: builds are unsigned until M11), no admin prompt, app opens in under 60 seconds total, WebView2 bootstrapped silently.
- Fresh macOS 12 machine: open `.dmg`, drag to Applications, right-click → Open, confirm the unidentified-developer dialog (expected: builds are unsigned until M11), app launches. Both prompts are documented in `docs/INSTALL.md`.
- [x] Installer sizes printed in CI logs: both under 40 MB (unsigned builds). *(8 Sept: Windows 5.12 MB, macOS Intel 3.43 MB, macOS ARM 3.21 MB.)*
- App cold start under 1 second on both platforms.
- Plug in a second monitor: projector window appears on it fullscreen; unplug: app does not crash.
- Download a 30 MB test pack, kill the app at 50%, relaunch, download resumes and verifies.

**Do not start M1 until:** all six DoD lines pass and the status board says ✅.

---

## M1 — Audio In, Transcript Out
**Phase 1 · 22 Sept – 5 Oct 2026 · 2 weeks**

**Goal:** Speak into any input device and watch your words appear live in the operator window.

**You are here when:** M0 is ✅ and the operator window is still empty.

**Deliverables**
- [ ] Audio device enumeration with `cpal`, including HDMI capture cards, USB interfaces, and loopback devices where the OS exposes them (FR-01, FR-02, FR-05).
- [ ] Capture at 16 kHz mono 16-bit PCM in 250 ms chunks (FR-03).
- [ ] Live level meter at 30 fps in the operator top bar (FR-04).
- [ ] Start / stop / pause / resume without restart (FR-06).
- [ ] Deepgram Nova-3 streaming over WebSocket with interim and final results (FR-07).
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
- [ ] Bible cache in `rusqlite` seeded from bundled KJV/WEB/ASV; API.Bible client caching two more translations on demand (FR-18, FR-19, FR-22).
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
- [ ] Commercial license agreements in place with API.Bible for the paid translations; tier gating recorded in `settings`.
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
| **Decision needed, not an idea:** build the verse index with the *same* encoder used at runtime. PRD §15.4 embeds verses with OpenAI `text-embedding-3-small` but embeds spoken phrases with a bundled on-device model; cosine similarity across two different embedding spaces is meaningless. Proposal: use `all-MiniLM-L6-v2` for both (natively 384-dim, CPU-only, ~23 MB quantized), which also drops the OpenAI dependency. Requires a PRD §15.4 amendment. | M0 (deliverables 7, 8) | 7 Sept 2026 |
| Pack **archive extraction** (`.tar.zst`): packs install as one verified file today, which suits whisper models and theme JSON. Translation packs shipped as archives will need a decompress step. | M5 / M6 | 7 Sept 2026 |
| **Manifest signature verification** is not implemented — packs are checksum-verified against the manifest, but the manifest itself is trusted on TLS alone. Needs the signing key from deliverable 6. | M0 (deliverable 6) | 7 Sept 2026 |
| Wordmark/lockup art in `assets/logo-assets/` is not yet used anywhere in the UI — the operator top bar renders "SermonAI" as text, not the lockup. | M8 (design pass) | 7 Sept 2026 |

---

## Session Ritual for Claude Code

At the start of every session:
1. Read the Status Board. State the current milestone and blocker aloud.
2. Read that milestone's Deliverables. Pick the next unticked item.
3. Reference the PRD section for that item before writing code.
4. When the item is done, tick it, run the relevant DoD line, and update the Status Board if the milestone is complete.
5. Anything out of scope goes in Parked, not in the code.
