import { describe, expect, it } from "vitest";
import { tierFor } from "./LevelMeter";

/**
 * The colour an operator sees is the whole signal the meter carries, so the
 * boundaries are pinned rather than left to whoever edits the component next.
 */
describe("tierFor", () => {
  it("reads silence as off rather than as a quiet signal", () => {
    // The floor and below are "nothing arriving", not "very quiet": a bar that
    // lit green on room tone would say a dead microphone was working.
    expect(tierFor(-60)).toBe("off");
    expect(tierFor(-80)).toBe("off");
  });

  it("reads normal speech as green", () => {
    // Where a preacher on a well-set desk actually sits.
    expect(tierFor(-40)).toBe("on");
    expect(tierFor(-20)).toBe("on");
    expect(tierFor(-12.1)).toBe("on");
  });

  it("warns in amber once the input is getting hot", () => {
    expect(tierFor(-12)).toBe("warn");
    expect(tierFor(-6)).toBe("warn");
    expect(tierFor(-3.1)).toBe("warn");
  });

  it("goes red near full scale, while there is still room to react", () => {
    // Red has to arrive before 0 dBFS, not at it. At 0 the samples are already
    // clipped and the distortion is in the recording; the point of the tier is
    // to give the operator a few dB of warning.
    expect(tierFor(-3)).toBe("loud");
    expect(tierFor(-1)).toBe("loud");
    expect(tierFor(0)).toBe("loud");
  });

  it("keeps the tiers in order across the whole range", () => {
    // Guards against an edit that overlaps two thresholds: walking the scale
    // must only ever move up through the tiers, never back down.
    const order = { off: 0, on: 1, warn: 2, loud: 3 };
    let previous = 0;

    for (let dbfs = -70; dbfs <= 0; dbfs += 0.5) {
      const rank = order[tierFor(dbfs)];
      expect(rank).toBeGreaterThanOrEqual(previous);
      previous = rank;
    }

    expect(previous).toBe(order.loud);
  });
});
