import { describe, expect, it } from "vitest";
import { MAX_PX, MIN_PX, fitSize } from "./autoFit";

/**
 * The bisection, pinned (M3 deliverable 9). The measurement itself needs a
 * layout engine jsdom does not have; the operator's checklist step 39 is
 * where that is seen.
 */
describe("fitSize", () => {
  it("returns the ceiling when everything fits", () => {
    expect(fitSize(() => true)).toBe(MAX_PX);
  });

  it("returns the floor when nothing fits: a clipped verse beats no verse", () => {
    expect(fitSize(() => false)).toBe(MIN_PX);
  });

  it("finds the largest size that fits, to the pixel", () => {
    for (const limit of [25, 64, 97, 139]) {
      expect(fitSize((px) => px <= limit)).toBe(limit);
    }
  });

  it("probes a handful of sizes, not every one", () => {
    let probes = 0;
    fitSize((px) => {
      probes += 1;
      return px <= 71;
    });
    expect(probes).toBeLessThanOrEqual(9);
  });
});
