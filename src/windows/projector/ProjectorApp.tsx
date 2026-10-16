import { useEffect, useState } from "react";
import { on, output } from "@/lib/ipc";
import type { OutputState, ProjectorLayout } from "@/lib/types";

/**
 * The congregation's screen (FR-24; M3 deliverable 3).
 *
 * Rules, all Branding's: scripture in Crimson Pro on bg.canvas; no violet
 * except the 2px rule under the reference (§2.2); no logo (§8.4); nothing
 * on screen but the Word. Three layouts (PRD §13.6): verse only, verse and
 * reference, verse and reference and translation — the last adds the
 * translation's short name, small, in the bottom-right corner, the
 * operator's call of 16 Oct 2026; full attribution belongs on the operator
 * screen and in the PDF (§15.2), not in front of the congregation.
 *
 * Blanked, or nothing live: a canvas-coloured screen, not black, so the
 * projector's own "no signal" is never mistaken for a blank.
 *
 * Reports its own paint of each new live verse, two animation frames after
 * the state arrives, against the time the verse went live — the M3 DoD's
 * staged → live budget is measured to this, not to the backend's broadcast.
 */
export const EMPTY: OutputState = {
  live: null,
  staged: null,
  blanked: false,
  layout: "verse_reference",
  liveSinceMs: null,
};

/**
 * A size tier for the verse by its length (Branding type.scripture.projector:
 * fluid 64–140, auto-fit). Four tiers so a 60-word verse fits the screen
 * without the 12-word one shrinking to match; deliverable 9 replaces this
 * with a measured fit.
 */
export function verseSizeClass(text: string): string {
  const n = text.length;
  if (n <= 120) return "text-[clamp(64px,7vw,140px)]";
  if (n <= 240) return "text-[clamp(48px,5.2vw,104px)]";
  if (n <= 400) return "text-[clamp(36px,3.9vw,78px)]";
  return "text-[clamp(28px,3vw,60px)]";
}

export function showsReference(layout: ProjectorLayout): boolean {
  return layout !== "verse_only";
}

export function showsTranslation(layout: ProjectorLayout): boolean {
  return layout === "verse_reference_translation";
}

export function ProjectorApp() {
  const [state, setState] = useState<OutputState>(EMPTY);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    // Ask on mount: the window may be created after a verse is already live
    // (outputs are created on first display assignment), and must not wait
    // for the next change to show it.
    void output.get().then((s) => {
      if (!cancelled) setState(s);
    });
    void on("output:changed", setState).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // Paint report, once per go-live. Two frames: the first is when React has
  // committed, the second is after the browser has painted that commit.
  const liveSince = state.liveSinceMs;
  const liveRef = state.live?.reference;
  useEffect(() => {
    if (liveSince === null || !liveRef || state.blanked) return;
    let inner = 0;
    const outer = requestAnimationFrame(() => {
      inner = requestAnimationFrame(() => {
        void output.painted(liveSince, Date.now()).catch(() => undefined);
      });
    });
    return () => {
      cancelAnimationFrame(outer);
      cancelAnimationFrame(inner);
    };
  }, [liveSince, liveRef, state.blanked]);

  if (state.blanked || !state.live) {
    return <div className="h-full bg-bg-canvas" />;
  }

  const { live, layout } = state;

  return (
    <div className="relative flex h-full items-center justify-center bg-bg-canvas px-[8%]">
      <div className="max-w-[90%] text-center">
        <p
          className={`font-serif leading-[1.2] tracking-[-0.005em] text-content-primary ${verseSizeClass(live.text)}`}
        >
          {live.text || live.reference}
        </p>
        {showsReference(layout) && (
          <>
            <p className="mt-8 text-[22px] font-medium uppercase tracking-[0.06em] text-content-secondary">
              {live.reference}
            </p>
            <span className="mx-auto mt-3 block h-0.5 w-20 bg-accent-500" />
          </>
        )}
      </div>

      {showsTranslation(layout) && (
        <span
          className="mono absolute bottom-[3%] right-[3%] text-[clamp(14px,1.2vw,24px)] uppercase tracking-[0.12em] text-content-muted"
          aria-label="Translation"
        >
          {live.translation}
        </span>
      )}
    </div>
  );
}
