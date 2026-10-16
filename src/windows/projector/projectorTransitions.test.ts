import { describe, expect, it } from "vitest";
import type { Verse } from "@/lib/types";
import {
  EMPTY_LAYERS,
  entryDelayMs,
  exitMs,
  layerStyle,
  onLive,
  onSettled,
  settleMs,
} from "./projectorTransitions";

const verse = (reference: string): Verse => ({ reference, translation: "KJV", text: `text of ${reference}` });

/**
 * The projector's choreography, pinned (FR-25). What matters: a go-live is
 * a go-live even for the same verse; a layout change is not one; an exiting
 * layer leaves once; and zero or reduced motion means no animation at all.
 */
describe("projector transitions", () => {
  it("a go-live pushes the new verse in and turns the old one out", () => {
    let s = onLive(EMPTY_LAYERS, verse("John 3:16"), 1_000);
    expect(s.layers).toEqual([{ key: 1, verse: verse("John 3:16"), phase: "in", since: 1_000 }]);

    s = onLive(s, verse("Romans 8:28"), 2_000);
    expect(s.layers.map((l) => [l.key, l.phase])).toEqual([
      [1, "out"],
      [2, "in"],
    ]);
  });

  it("the same state again is the same object: a layout change is not a go-live", () => {
    const s = onLive(EMPTY_LAYERS, verse("John 3:16"), 1_000);
    expect(onLive(s, verse("John 3:16"), 1_000)).toBe(s);
  });

  it("the same verse sent twice animates twice", () => {
    let s = onLive(EMPTY_LAYERS, verse("John 3:16"), 1_000);
    s = onLive(s, verse("John 3:16"), 1_500);
    expect(s.layers).toHaveLength(2);
    expect(s.layers[1]).toMatchObject({ key: 2, phase: "in", since: 1_500 });
  });

  it("an exiting layer is dropped once settled, and only an exiting one", () => {
    let s = onLive(EMPTY_LAYERS, verse("John 3:16"), 1_000);
    s = onLive(s, verse("Romans 8:28"), 2_000);
    expect(onSettled(s, 2)).toBe(s);
    s = onSettled(s, 1);
    expect(s.layers.map((l) => l.key)).toEqual([2]);
    expect(onSettled(s, 1)).toBe(s);
  });

  it("nothing live sends every layer out", () => {
    let s = onLive(EMPTY_LAYERS, verse("John 3:16"), 1_000);
    s = onLive(s, null, null);
    expect(s.layers.map((l) => l.phase)).toEqual(["out"]);
    expect(onLive(s, null, null)).toBe(s);
  });

  it("fade runs exit then entry; slide and dissolve overlap them", () => {
    expect(exitMs("fade", 400)).toBe(250);
    expect(entryDelayMs("fade", 400)).toBe(250);
    expect(settleMs("fade", 400)).toBe(650);

    expect(exitMs("slide", 400)).toBe(250);
    expect(entryDelayMs("slide", 400)).toBe(0);

    expect(exitMs("dissolve", 400)).toBe(400);
    expect(entryDelayMs("dissolve", 400)).toBe(0);
    expect(settleMs("dissolve", 400)).toBe(400);

    // A short entry shortens the exit with it: the old verse must not
    // outstay the new one.
    expect(exitMs("fade", 100)).toBe(100);
    expect(exitMs("slide", 0)).toBe(0);
  });

  it("zero or reduced motion means no animation", () => {
    expect(layerStyle("fade", "in", 0, false)).toEqual({});
    expect(layerStyle("fade", "out", 0, false)).toEqual({ opacity: 0 });
    expect(layerStyle("slide", "in", 400, true)).toEqual({});
    expect(layerStyle("slide", "out", 400, true)).toEqual({ opacity: 0 });
  });

  it("names the keyframes by transition and phase, with the fade's delay", () => {
    expect(layerStyle("fade", "in", 400, false).animation).toMatch(/^verseFadeIn 400ms .* 250ms both$/);
    expect(layerStyle("fade", "out", 400, false).animation).toMatch(/^verseFadeOut 250ms .* 0ms both$/);
    expect(layerStyle("slide", "in", 400, false).animation).toMatch(/^verseSlideIn 400ms .* 0ms both$/);
    expect(layerStyle("slide", "out", 400, false).animation).toMatch(/^verseSlideOut 250ms/);
    expect(layerStyle("dissolve", "in", 600, false).animation).toMatch(/^verseFadeIn 600ms .* 0ms both$/);
    expect(layerStyle("dissolve", "out", 600, false).animation).toMatch(/^verseFadeOut 600ms/);
  });
});
