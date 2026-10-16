import { useEffect, useRef, useState } from "react";
import { on, output } from "@/lib/ipc";
import type { OutputState, ProjectorLayout, Verse } from "@/lib/types";
import { useAutoFit } from "./autoFit";
import {
  EMPTY_LAYERS,
  exitMs,
  layerStyle,
  onLive,
  onSettled,
  settleMs,
  type LayerState,
} from "./projectorTransitions";

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
 * Transitions (FR-25, deliverable 4): verses are layers, and a go-live
 * animates the old one out and the new one in per the configured
 * transition and length (`projectorTransitions.ts`). The paint report is
 * still the first frame of the new verse — that is when the congregation
 * can begin to read it — and the log notes how long the transition takes
 * to settle beside it.
 *
 * The verse is sized by measurement (deliverable 9, `autoFit.ts`): the
 * largest size at which verse, reference and rule fit the layer, found
 * before the first paint, re-found on resize.
 *
 * Blanked (FR-33, deliverable 5): a canvas-coloured cover over the layers,
 * instantly, with no fade (wireframe §projector "Blanked state"). The
 * layers stay mounted underneath, so unblanking uncovers the same verse
 * with no re-layout and no re-animation: no flicker, no lost state. Blank
 * and empty are canvas-coloured rather than black, so the projector's own
 * "no signal" is never mistaken for a blank.
 */
export const EMPTY: OutputState = {
  live: null,
  staged: null,
  blanked: false,
  layout: "verse_reference",
  autoLive: false,
  transition: "fade",
  transitionMs: 400,
  liveSinceMs: null,
};

export function showsReference(layout: ProjectorLayout): boolean {
  return layout !== "verse_only";
}

export function showsTranslation(layout: ProjectorLayout): boolean {
  return layout === "verse_reference_translation";
}

function usePrefersReducedMotion(): boolean {
  const [reduced, setReduced] = useState(
    () => typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true,
  );
  useEffect(() => {
    const mq = window.matchMedia?.("(prefers-reduced-motion: reduce)");
    if (!mq) return;
    const handler = (e: MediaQueryListEvent) => setReduced(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);
  return reduced;
}

export function ProjectorApp() {
  const [state, setState] = useState<OutputState>(EMPTY);
  const [layers, setLayers] = useState<LayerState>(EMPTY_LAYERS);
  const reducedMotion = usePrefersReducedMotion();

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

  // The live slot drives the layer stack; a layout change touches no layer.
  const live = state.live;
  const liveSince = state.liveSinceMs;
  useEffect(() => {
    setLayers((prev) => onLive(prev, live, liveSince));
  }, [live, liveSince]);

  // Drop each exiting layer once its exit has run. A timer rather than
  // animationend: an instant exit fires no animation event, and a layer
  // covered by a blank may be in a window that is not painting.
  const { transition, transitionMs } = state;
  const outKeys = layers.layers
    .filter((l) => l.phase === "out")
    .map((l) => l.key)
    .join(",");
  useEffect(() => {
    if (outKeys === "") return;
    const ms = reducedMotion ? 0 : exitMs(transition, transitionMs);
    const timers = outKeys.split(",").map((k) => {
      const key = Number(k);
      return window.setTimeout(() => setLayers((prev) => onSettled(prev, key)), ms + 20);
    });
    return () => timers.forEach((t) => window.clearTimeout(t));
  }, [outKeys, transition, transitionMs, reducedMotion]);

  // Paint report, once per go-live. Two frames: the first is when React has
  // committed, the second is after the browser has painted that commit.
  // Keyed on the go-live, not on blank: uncovering a verse is not a paint
  // of a new one, and would report a stale figure.
  const reported = useRef<number | null>(null);
  useEffect(() => {
    if (liveSince === null || !live || reported.current === liveSince) return;
    const settle = reducedMotion ? 0 : settleMs(transition, transitionMs);
    let inner = 0;
    const outer = requestAnimationFrame(() => {
      inner = requestAnimationFrame(() => {
        reported.current = liveSince;
        void output.painted(liveSince, Date.now(), settle).catch(() => undefined);
      });
    });
    return () => {
      cancelAnimationFrame(outer);
      cancelAnimationFrame(inner);
    };
  }, [liveSince, live, transition, transitionMs, reducedMotion]);

  const { layout } = state;

  return (
    <div className="relative h-full overflow-hidden bg-bg-canvas">
      {layers.layers.map((layer) => (
        <VerseLayer
          key={layer.key}
          verse={layer.verse}
          layout={layout}
          style={layerStyle(transition, layer.phase, transitionMs, reducedMotion)}
          leaving={layer.phase === "out"}
        />
      ))}
      {state.blanked && <div className="absolute inset-0 bg-bg-canvas" aria-label="Blanked" />}
    </div>
  );
}

function VerseLayer({
  verse,
  layout,
  style,
  leaving,
}: {
  verse: Verse;
  layout: ProjectorLayout;
  style: React.CSSProperties;
  leaving: boolean;
}) {
  const layerRef = useRef<HTMLDivElement>(null);
  const blockRef = useRef<HTMLDivElement>(null);
  const textRef = useRef<HTMLParagraphElement>(null);
  const fontSize = useAutoFit(
    layerRef,
    blockRef,
    textRef,
    `${verse.reference}|${verse.translation}|${layout}|${verse.text.length}`,
  );
  return (
    <div
      ref={layerRef}
      className="absolute inset-0 flex items-center justify-center px-[8%] py-[6%]"
      style={style}
      aria-hidden={leaving || undefined}
    >
      <div ref={blockRef} className="w-full text-center">
        <p
          ref={textRef}
          className="font-serif leading-[1.2] tracking-[-0.005em] text-content-primary"
          style={{ fontSize: `${fontSize}px` }}
        >
          {verse.text || verse.reference}
        </p>
        {showsReference(layout) && (
          <>
            <p className="mt-8 text-[22px] font-medium uppercase tracking-[0.06em] text-content-secondary">
              {verse.reference}
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
          {verse.translation}
        </span>
      )}
    </div>
  );
}
