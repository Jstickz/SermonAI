# SermonAI — working agreement

## Start every session here

1. Read [docs/MILESTONES.md](docs/MILESTONES.md). State the current milestone
   and blocker before doing anything else.
2. Read that milestone's Deliverables. Pick the next unticked item.
3. Read the PRD section that item cites before writing code.
4. When the item is done, tick it, run the relevant Definition of Done line, and
   update the Status Board if the milestone is complete — in the same commit.
5. Anything out of scope goes in the **Parked** table at the bottom of
   MILESTONES.md, not into the code.

Milestones are sequential. Do not pull work forward from a later one.

## Sources of truth

| Question | Answer lives in |
|---|---|
| What are we building, and why | `docs/PRD.md` |
| What stage are we at | `docs/MILESTONES.md` |
| How should it look | `docs/BRANDING.md` |
| What does the screen look like | `docs/wireframe.html` |
| Why is it built this way | `docs/adr/` |

`docs/PRD.md` §10 and §11 are binding. A deviation means updating the PRD in the
same PR, not working around it.

## Standing decisions

Settled calls that outlive the milestone they were taken in. Do not relitigate;
if one turns out to be wrong, change it here and in the PRD in the same PR.

### Two-stage detection (23 Sept 2026, PRD §18.1, §13.5)

A scripture reference is caught twice, and the two stages have different rights.

- **Provisional.** The regex stage runs on **interim** transcript text. A match
  raises a candidate in staging marked unconfirmed. It is **never projectable**
  — not by Enter, not by Go Live, not by auto-live — and it is withdrawn
  **silently** if the confirmed text does not support it.
- **Confirmed.** When settled text supports the candidate it firms up and
  becomes projectable.

Why: confirmation is Deepgram's decision, taken only once it judges the speaker
to have stopped, and measured at about 2.3 s. That is not ours to shorten —
`endpointing` does not move it. Waiting for it before showing anything wastes a
second of the operator's warning; acting on interim text risks projecting a
verse that the next result revises away, and a verse on a screen cannot be
recalled. Showing it early without letting it out is the only option that costs
nothing.

Withdrawal is silent because interim results are *revisions*, not mistakes.
A panel that announced every retraction would teach the operator to ignore it.

### Latency has two budgets, not one (23 Sept 2026, PRD §18.1)

Measure to the **provisional candidate** (about 900 ms p95, mostly network and
Deepgram) and from **confirmed text to projectable** (about 50 ms p95, all ours).
A single end-to-end number hides the fact that most of the delay belongs to a
vendor. The previous single budget of 800 ms p99 was unreachable and went
unnoticed for exactly that reason.

### Measure the thing, not a proxy (23 Sept 2026)

M1 produced three figures that were wrong in ways that looked plausible:

- Summing `WorkingSet` across processes reported 583 MB where the private
  working set was 152 MB; resolving processes by performance-counter *name*
  reported 40 MB where the true figure was 193 MB, because several are called
  `msedgewebview2`.
- Timing lag from when capture started, rather than when the first audio was
  sent, charged the WebSocket handshake — over a second — to every sample.
- Measuring only confirmed text and comparing it to a budget written about
  visible text read as a fourfold failure of a passing pipeline.

So: state the method beside the number, and check a measurement against
something it must be bounded by before trusting it.

## Traceability

Every functional requirement is tagged `FR-XX`. Reference the tag in commits,
PRs, and in a comment on the code that implements it. Every module maps to a PRD
section; if a new module doesn't, either it is out of scope or the PRD needs an
update first.

## Code conventions

**Frontend**
- Colours, radii, spacing, type and motion come from `src/design/tokens.json`
  through Tailwind classes. Never a hard-coded hex value in a component.
- Components never call `invoke` directly — add a typed wrapper in
  `src/lib/ipc.ts` and use that. Command and wrapper land in the same commit.
- Types in `src/lib/types.ts` mirror the Rust structs in
  `src-tauri/src/db/models.rs`. Change both together.
- Booth constraints are real: 40px minimum hit targets, dark by default, tabular
  numerals on anything that ticks, no animation longer than 300 ms outside the
  projector.

**Backend**
- One module per architecture layer (PRD §10.2). Business logic lives in the
  domain module; `commands/` handlers only validate, delegate and map errors.
- All errors go through `crate::error::Error`. Messages are operator-facing:
  name what failed and offer one action.
- New dependencies must justify their weight against the 40 MB installer gate.
  If a feature needs a large asset, it becomes a pack.

## Voice in the product

Buttons are verbs (`Go Live`, `Download PDF`, `End Service`). Errors name the
thing that failed and offer one action. Loading verbs always take an object
("Preparing summary…", never "Loading…"). No emoji in operator UI text. Never
cute in error states. See Branding §9.

## Toolchain note

Building requires Node 20+ and Rust stable — see the README prerequisites. Do
not add a Python or Node runtime to the shipped app; the Tauri binary is the
whole backend (ADR 0001).
