import type { CSSProperties } from "react";
import type { ProjectorTransition, Verse } from "@/lib/types";

/**
 * How one verse gives way to the next on the projector (FR-25, PRD §13.6,
 * Branding §7.1), as pure functions so the choreography can be tested
 * without a screen.
 *
 * The projector keeps a short stack of **layers**: the verse arriving (`in`)
 * and any verse leaving (`out`). A go-live turns every `in` layer `out` and
 * pushes a new `in`; an `out` layer is dropped once its exit has run. The
 * key for "a go-live happened" is `liveSinceMs`, not the verse: the same
 * verse sent twice is two go-lives and should animate twice.
 *
 * Timing, from Branding §7.2: the verse enters over the configured length
 * (`motion.verse-in`, 400 ms by default) and leaves over `motion.verse-out`,
 * 250 ms, or faster if the entry is shorter. **Fade** runs them in turn so
 * the screen rests on canvas between verses; **slide** and **dissolve**
 * overlap them. Zero means instant, and so does the viewer's
 * reduced-motion preference.
 */

export type Phase = "in" | "out";

export interface Layer {
  key: number;
  verse: Verse;
  phase: Phase;
  /** The go-live this layer belongs to; `onLive` uses it to tell a new
   *  go-live from a re-render. */
  since: number;
}

export interface LayerState {
  layers: Layer[];
  nextKey: number;
}

export const EMPTY_LAYERS: LayerState = { layers: [], nextKey: 1 };

/** Branding `motion.verse-out`. */
export const VERSE_OUT_MS = 250;

/** The state's response to the backend's live slot. Unchanged input gives
 *  the same object back, so React does not re-render for nothing. */
export function onLive(state: LayerState, live: Verse | null, liveSinceMs: number | null): LayerState {
  const top = state.layers.find((l) => l.phase === "in");
  if (live === null || liveSinceMs === null) {
    if (!top) return state;
    return { ...state, layers: state.layers.map((l) => ({ ...l, phase: "out" as const })) };
  }
  if (top && top.since === liveSinceMs) return state;
  const leaving = state.layers.map((l) => ({ ...l, phase: "out" as const }));
  return {
    layers: [...leaving, { key: state.nextKey, verse: live, phase: "in", since: liveSinceMs }],
    nextKey: state.nextKey + 1,
  };
}

/** An exiting layer has finished; drop it. */
export function onSettled(state: LayerState, key: number): LayerState {
  if (!state.layers.some((l) => l.key === key && l.phase === "out")) return state;
  return { ...state, layers: state.layers.filter((l) => l.key !== key) };
}

/** How long a layer's exit takes under this transition and entry length. */
export function exitMs(transition: ProjectorTransition, durationMs: number): number {
  if (durationMs === 0) return 0;
  if (transition === "dissolve") return durationMs;
  return Math.min(VERSE_OUT_MS, durationMs);
}

/** When a layer's entry starts: after the exit for a fade, at once otherwise. */
export function entryDelayMs(transition: ProjectorTransition, durationMs: number): number {
  return transition === "fade" ? exitMs(transition, durationMs) : 0;
}

/** The total time from go-live to a settled screen; the log notes it beside
 *  the paint time, which is measured to the first frame. */
export function settleMs(transition: ProjectorTransition, durationMs: number): number {
  return entryDelayMs(transition, durationMs) + durationMs;
}

const EASE_IN = "cubic-bezier(0.16, 1, 0.3, 1)"; // Branding §7.1 entries
const EASE_OUT = "cubic-bezier(0.4, 0, 0.2, 1)"; // exits

/** The CSS animation for a layer. Keyframe names are in global.css. */
export function layerStyle(
  transition: ProjectorTransition,
  phase: Phase,
  durationMs: number,
  reducedMotion: boolean,
): CSSProperties {
  const instant = reducedMotion || durationMs === 0;
  if (instant) {
    return phase === "out" ? { opacity: 0 } : {};
  }
  const name =
    transition === "slide" ? (phase === "in" ? "verseSlideIn" : "verseSlideOut") : phase === "in" ? "verseFadeIn" : "verseFadeOut";
  const ms = phase === "in" ? durationMs : exitMs(transition, durationMs);
  const delay = phase === "in" ? entryDelayMs(transition, durationMs) : 0;
  const ease = phase === "in" ? EASE_IN : EASE_OUT;
  return { animation: `${name} ${ms}ms ${ease} ${delay}ms both` };
}
