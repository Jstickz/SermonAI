# Open questions

Things not yet decided or not yet verified. When one closes, move its answer into a decision note or a session note and strike it here with the date. The authoritative Parked table is in `docs/MILESTONES.md`; this is the short list a new session should hold in mind.

## Blocking before M4's live service

- [ ] **The SermonAI Gateway does not exist.** Managed mode, and with it YouVersion and Tyndale, is unavailable on every machine. Hard STOP: the operator must create the Cloudflare account themselves. Quota, address and BYOK registration were decided 16 Oct ([[hybrid-key-model]]). Remaining: the account, then Phases 3 to 5.
- [ ] **Nothing has run on macOS** beyond a CI compile and install smoke: M1's four DoD lines, the 60-minute run, M2's cards. The watchdog premise is the sharpest risk ([[macos-parked-knowingly]]). Needs a Mac.
- [ ] **Real church hardware.** A USB interface or HDMI capture card fed from a mixer has never been attached. Enumeration is verified against a laptop mic array and three loopback endpoints only.
- [ ] **Church wifi during a service** is unmeasured; every latency figure is from one home connection in Abuja. §18.1 says measure at the first real Sunday, and the chunk-size question (125 ms) waits on it.
- [ ] **Did the two-minute outage report its dropped audio?** The backlog holds 60 s, so a red "about 60 s lost" banner should have appeared. It was not mentioned. If that path does not reach the UI, a transcript closes a gap silently.

## M3, right now

- [x] ~~Deliverable 3 awaits the operator's eye~~ Run 16 Oct on f261934: steps 1–27 passed, staged → paint median 10 ms. Step 28 (the 60-second blank) was not reported either way; confirm before M3 closes.
- [ ] **The picker cannot tell which loopback Windows is playing through.** Parked in MILESTONES; the Windows default output moved to a DELL monitor's HDMI audio when the projector went fullscreen there.
- [ ] **Manual Go Live does not mark `went_live`** on the detection row; auto-live does. Carry the detection id through `stage_verse` with deliverables 7–8 (parked in MILESTONES).
- [ ] The length-tier verse fit is a stopgap; deliverable 9 replaces it with a measured fit.
- [ ] Branch `m2-detection` carries M2's close and M3's first five deliverables; merge to `main` at M3 close (decided 16 Oct).

## Measurement debts

- [ ] **Memory with the index loaded** has not been re-measured since the vector stage and cache landed. Headroom was 88 MB (212 of 300 MB) before them.
- [ ] **Keyterm benefit is unmeasured**; the A/B on synthesised speech was null. Needs real preaching audio.
- [ ] **FR-14 gate edges** (0.55 / 0.80) were set on n=8/15 and the band is hotter than expected (the cooldown, not the edges, sets the call rate at ~280/hour). Re-read from the first real service's score log.
- [ ] **Claude cost per service** is extrapolated from $0.00047 a call on short inputs; a live 60-second buffer is ~10× the input.

## Design and licensing calls not yet made

- [ ] **Encoder upgrade Phase 2** (`docs/SermonAI_Qwen3_Embedding_Prompt.md`): compare BGE-small and Qwen3-Embedding-0.6B against potion-base-8M on the same data; adopt the smallest that meets the target; anything over the gate ships as a pack. STOP after the comparison plan. Phase 0 and 1 are done (70% → 79%).
- [ ] **KJV under UK Crown copyright** needs a deliberate call before UK distribution (M11).
- [ ] **NIV as default translation** waits on the gateway; KJV until then.
- [ ] **Licensed translations on the projector:** none should be projected until the PDF carries the full attribution line (M4). The corner label decision is made ([[projector-translation-label]]).
- [ ] **§13.2 says amber for the scripture highlight, the wireframe says violet.** Code follows the wireframe; the PRD should be amended.
- [ ] **Button capitalisation:** Branding Title Case vs wireframe sentence case; and Branding defines no disabled state (one exists at 40% opacity).
- [ ] **Place names in keyterms:** "Zion" became "Ireland". Only ~4 slots remain under Deepgram's 100-term cap; the proper fix is M8's per-church vocabulary editor.
- [ ] **Deuterocanonical books** are filtered out; churches that preach from them need a catalog decision (M6).

## Release checklist debts (M11)

- [ ] Notarization and Authenticode ([[code-signing-deferred-to-m11]]).
- [ ] WebView2 silent bootstrap on a clean Windows 10.
- [ ] The oldest supported macOS (12) has never been run.
- [ ] Pack system against a real CDN bucket; manifest signature verification; archive extraction.
- [ ] Display assignments and the chosen audio device are not remembered across restarts (M8 onboarding).
