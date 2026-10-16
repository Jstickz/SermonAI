# Enter does the next thing in the path

**Decided:** 16 October 2026 · **Code:** `src/lib/stagingKeys.ts` (tested) · **PRD:** §13.5, FR-50

The PRD says Enter promotes staged to live. The wireframe's card says "Stage · ↵" and its footer "⇧↵ Go live". Both describe a moment, not a conflict, so the rule is:

| State | Enter | Shift+Enter |
|---|---|---|
| Nothing staged | stages the top confirmed card | stages the top card and goes live in one |
| Something staged | goes live | same |
| A text field has focus | acts on the field | acts on the field |

Escape clears Staged only; Live is untouched. Go Live moves staged to live and empties staged, so a second Enter cannot send the same verse twice. Go Live also unblanks, since it means "show this". No PRD change was needed: §13.5's sentence is the second row.

The operator's cheat-sheet consequence: one key, pressed twice, takes a verse from card to projector; held with Shift it takes one press.

Related: [[adr-0002-staging-first-output]], [[sessions/2026-10-16]]
