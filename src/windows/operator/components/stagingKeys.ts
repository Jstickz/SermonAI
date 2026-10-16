/**
 * What Enter does on the Live tab (FR-50, PRD §13.5), as a pure rule so it
 * can be tested without a keyboard.
 *
 * Two documents describe Enter. The PRD (§13.5) says Enter promotes staged to
 * live; the wireframe's card says "Stage · ↵" and its footer "⇧↵ Go live".
 * Both are right about the moment they describe, and the rule that satisfies
 * both is: Enter does the next thing in the path. Nothing staged → Enter
 * stages the top card. Something staged → Enter sends it live. Shift+Enter
 * is the whole path in one keystroke: stage the top card and go live, for
 * the operator who has already read the card.
 *
 * Escape clears the staged slot and is handled where the slot is drawn.
 * **B** blanks and unblanks the projector (FR-33, wireframe §projector),
 * and is handled by the top bar's blank button. **→** and **←** step the
 * passage on the projector a verse forward or back (FR-51), handled by the
 * staging panel beside the Live slot; bare arrows, outside text fields,
 * where they would otherwise move a caret.
 */
export type EnterAction = "stage" | "go-live" | "stage-and-go-live" | "none";

export function enterAction(hasStaged: boolean, hasTopCard: boolean, shift: boolean): EnterAction {
  if (shift) {
    if (hasTopCard) return "stage-and-go-live";
    return hasStaged ? "go-live" : "none";
  }
  if (hasStaged) return "go-live";
  return hasTopCard ? "stage" : "none";
}

/** → is one verse on, ← one back (FR-51); 0 means the key is not ours. */
export function stepDelta(
  e: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey">,
  target: EventTarget | null,
): 1 | -1 | 0 {
  if (e.ctrlKey || e.metaKey || e.altKey || isTypingTarget(target)) return 0;
  if (e.key === "ArrowRight") return 1;
  if (e.key === "ArrowLeft") return -1;
  return 0;
}

/** B, bare — no modifier, so Ctrl+B in a browser-like field is not a
 *  blank — and outside a text field. */
export function isBlankHotkey(
  e: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey">,
  target: EventTarget | null,
): boolean {
  if (e.key !== "b" && e.key !== "B") return false;
  if (e.ctrlKey || e.metaKey || e.altKey) return false;
  return !isTypingTarget(target);
}

/** A keystroke inside a text field belongs to the field, not to staging. */
export function isTypingTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el) return false;
  return el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable === true;
}
