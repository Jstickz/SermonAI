/**
 * Every keyboard shortcut, once (PRD §13.18; M3 polish item 7).
 *
 * This list is the source. The Keyboard shortcuts modal in Settings renders
 * it, and `shortcuts.test.ts` parses the table in docs/PRD.md §13.18 and
 * fails if the two disagree, so the PRD, the modal and the handlers in
 * `stagingKeys.ts` cannot drift apart without a red test. The printed
 * cheat sheet (docs/testing/cheat-sheet.html) is written for a reader, not
 * generated, and the same test checks that every key here appears on it.
 *
 * `keys` is what the PRD table prints in its first column, exactly.
 */
export interface Shortcut {
  keys: string;
  does: string;
  where: string;
  fr: string;
}

export const SHORTCUTS: readonly Shortcut[] = [
  {
    keys: "Enter",
    does: "The next thing in the path: nothing staged → stage the top waiting card; something staged → send it live",
    where: "Live tab",
    fr: "FR-50",
  },
  {
    keys: "Shift+Enter",
    does: "Stage the top waiting card and send it live in one",
    where: "Live tab",
    fr: "FR-50",
  },
  {
    keys: "Escape",
    does: "Clear the staged slot; live is untouched",
    where: "Live tab",
    fr: "FR-50",
  },
  {
    keys: "B",
    does: "Blank / unblank the projector, instantly",
    where: "every tab",
    fr: "FR-33",
  },
  {
    keys: "→ / ←",
    does: "Next / previous verse of the passage on the projector",
    where: "every tab, while something is live",
    fr: "FR-51",
  },
  {
    keys: "P",
    does: "Pin the top waiting card",
    where: "Live tab",
    fr: "FR-64",
  },
  {
    keys: "1 … 9",
    does: "Send the Nth pinned card live",
    where: "Live tab",
    fr: "FR-64",
  },
  {
    keys: "Ctrl/Cmd+K",
    does: "Command palette; inside it, Enter stages, Shift+Enter goes live, Tab switches translation, ↑↓ move",
    where: "every tab",
    fr: "FR-31",
  },
];

/** The rule every row shares, printed once above the table. */
export const SHORTCUT_RULE =
  "All bare keys unless marked, and none fires while a text field has focus: a keystroke inside a field belongs to the field.";
