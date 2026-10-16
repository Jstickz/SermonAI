# 00 Start Here

This folder is the project's memory. It is an Obsidian vault: open `brain/` in Obsidian and the graph shows how the notes connect. Read this note and the newest note in `sessions/` before `docs/MILESTONES.md`. Then follow CLAUDE.md as usual.

**Last updated:** 16 October 2026 (vault created and backfilled).

## Where we are

- **Milestone:** M3 Staging & Projector Output (window 20 Oct to 2 Nov 2026; started early on 16 Oct). M0, M1 and M2 are done on Windows; M1 and M2 have **not** closed on macOS ([[macos-parked-knowingly]]).
- **Branch:** `m2-detection`, ahead of `main`, carries M2's close and M3's first work.
- **M3 state:** deliverables 1 (staging slot), 2 (auto-live), 4 (transitions) and 5 (blank hotkey) ticked; deliverable 3 (projector renderer, three layouts) built and waiting for the operator to run `docs/testing/m3-projector-checklist.md`, steps 1 to 28, on a second monitor. Next unticked: next/previous verse (6, FR-51), then the command palette (7, FR-31).
- **Merge plan:** `m2-detection` merges to `main` at M3 close, not before (operator, 16 Oct).
- **Blocker in code:** none. **Blockers outside code:** the gateway's Cloudflare account, a Mac, real church hardware ([[open-questions]]).
- **Newest session:** [[sessions/2026-10-16]].

## What SermonAI is, in four lines

A Tauri 2 desktop app (React + TypeScript front, Rust back, one binary, under 40 MB) for church media booths. It listens to the sermon through Deepgram, catches scripture references three ways (regex, on-device vector search, Claude for paraphrases), shows each as a card, and puts the verse on the projector only after the operator stages it and presses Enter. After the service it produces a validated summary PDF. All data stays on the machine.

## The rules that outlive milestones

Standing decisions, each with its reasoning in `decisions/`:

- [[two-stage-detection]]: a provisional card from interim text is never projectable; confirmed text makes it so; withdrawal is silent.
- [[two-latency-budgets]]: measure to the provisional candidate (vendor-dominated, p50 ≤ 500 / p95 ≤ 900 ms) and from confirmed text to projectable (ours, ~50 ms) separately.
- [[measure-the-thing-not-a-proxy]]: method beside every number; bound-check before trusting.
- [[close-leftover-instances]]: a stray `sermonai.exe` blocks cargo; close it unless a test is in progress.
- [[enter-does-the-next-thing]]: Enter stages, then goes live; Shift+Enter does both; never inside a text field.
- [[hybrid-key-model]]: managed gateway by default, BYOK optional, YouVersion managed-only, no real key ever ships.
- [[eu-deepgram-region]], [[projector-translation-label]], [[code-signing-deferred-to-m11]].
- The six ADRs: [[adr-0001-tauri-over-electron]], [[adr-0002-staging-first-output]], [[adr-0003-three-stage-detection]] (with its amendments), [[adr-0004-local-only-data]], [[adr-0005-summary-schema-and-validator]], [[adr-0006-on-demand-packs]].

## Numbers of record

| Thing | Figure | Method |
|---|---|---|
| Installer | msi 5.12 MB; projected ~25 MB with all bundled assets | CI size gate, 40 MB limit |
| Cold start | 716 ms median to first paint, 162 ms ours | app's own clock, release, 10 launches |
| Words on screen | p50 187 ms, p95 501 ms | 20-minute release run, EU endpoint, per-chunk anchor |
| Settled text | ~2.3 s, not tunable | Deepgram's decision |
| Memory under capture | 212 MB of 300 MB (before the index loaded) | private working set by PID, 60 minutes |
| Regex stage | 426/426 caught, 0 false hits on 459 cases; 242 µs per window | release profile |
| Vector stage | 79.2% of 168 paraphrases at top-3; 1.10 ms per query | hand-labelled set |
| With Claude as gated | 91.7%; ceiling 99.4%; $0.00047 a call | 191 live calls, 15 Oct |

## How to use this vault

- **Session start:** read this note, then the newest `sessions/` note, then MILESTONES. State the milestone and blocker.
- **Session end, milestone close, or decision made:** write or extend today's `sessions/YYYY-MM-DD.md` (what happened, decisions, parked, next), update "Where we are" above, add a `decisions/` note for any new standing call, and add to [[lessons]] when something was learned the hard way.
- **Never** put keys, tokens, emails or personal data here ([[people-and-accounts]] says where secrets live, not what they are).
- Sources of truth stay where they were: PRD for what and why, MILESTONES for status, BRANDING and the wireframe for how it looks, `docs/adr/` for the full architecture records, `docs/testing/` for measurement methods. This vault is the narrative that connects them.

## Map

- [[lessons]]: everything that bit us, with the day it happened.
- [[open-questions]]: unverified and undecided, grouped by when it must close.
- [[people-and-accounts]]: who decides what, and which services exist.
- `sessions/`: one note per working day since the restart on 7 September 2026. First: [[sessions/2026-09-07]].
- `decisions/`: one note per standing decision and ADR.
