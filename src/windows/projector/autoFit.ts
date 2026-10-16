import { useLayoutEffect, useState, type RefObject } from "react";

/**
 * Measured auto-fit for the projector verse (PRD §13.6 "auto-fit font size",
 * Branding type.scripture.projector: fluid 64–140; M3 deliverable 9).
 *
 * Deliverable 3 sized the verse by its character count in four tiers. That
 * fits Esther 8:9 on a 1080p screen and nothing else is promised: a wide
 * screen wastes the margin, a portrait one clips, and a verse of long words
 * wraps differently from one of short. This measures instead. The largest
 * size at which the whole block — verse, reference, rule — fits inside the
 * layer is found by bisection over the pixel range; each probe is one
 * synchronous layout, and seven probes cover 24 to 140 px to the pixel.
 *
 * Runs in a layout effect, so the first paint of a new verse is already at
 * the fitted size: the paint report (`projector_painted`) measures a frame
 * the congregation can read, not one that is about to shrink.
 */

/** Branding's bounds. 140 is the display ceiling; 24 is the floor below
 *  which a hall screen is unreadable from the back, and a verse that needs
 *  less than that is a passage, not a verse. */
export const MAX_PX = 140;
export const MIN_PX = 24;

/**
 * The largest integer size in [min, max] for which `fits(size)` holds,
 * assuming `fits` is monotone (true at small sizes, false at large). `min`
 * when nothing fits: better a clipped verse than no verse.
 */
export function fitSize(fits: (px: number) => boolean, min = MIN_PX, max = MAX_PX): number {
  if (fits(max)) return max;
  let lo = min;
  let hi = max; // invariant: hi does not fit (or is the ceiling just tested)
  while (hi - lo > 1) {
    const mid = Math.floor((lo + hi) / 2);
    if (fits(mid)) lo = mid;
    else hi = mid;
  }
  return lo;
}

/**
 * The fitted font size for a verse block. `layer` is the full-screen layer,
 * `block` the centred content inside it, `text` the verse element whose
 * font size is being chosen. Re-measured when `key` changes (the verse or
 * layout) and when the window resizes (the projector moved to another
 * screen).
 */
export function useAutoFit(
  layer: RefObject<HTMLElement | null>,
  block: RefObject<HTMLElement | null>,
  text: RefObject<HTMLElement | null>,
  key: string,
): number {
  const [size, setSize] = useState(MAX_PX);

  useLayoutEffect(() => {
    const measure = () => {
      const l = layer.current;
      const b = block.current;
      const t = text.current;
      if (!l || !b || !t) return;
      // The layer's own padding is the margin the verse must stay inside.
      const style = getComputedStyle(l);
      const availableH = l.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
      const availableW = l.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
      if (availableH <= 0 || availableW <= 0) return;
      const fits = (px: number) => {
        t.style.fontSize = `${px}px`;
        return b.scrollHeight <= availableH && b.scrollWidth <= availableW;
      };
      const fitted = fitSize(fits);
      t.style.fontSize = `${fitted}px`;
      setSize(fitted);
    };
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, [layer, block, text, key]);

  return size;
}
