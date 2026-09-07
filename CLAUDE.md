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
