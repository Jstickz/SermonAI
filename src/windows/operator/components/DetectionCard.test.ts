import { describe, expect, it } from "vitest";
import { canPin, canStage, confidencePercent, sourceBadge } from "./DetectionCard";

/**
 * The card's pure decisions, pinned. The one that matters is `canStage`: a
 * provisional candidate must never reach staging (PRD §13.5), and the button
 * is absent rather than disabled so the operator is not invited to wait on it.
 */
describe("DetectionCard", () => {
  it("never lets a provisional card be staged", () => {
    expect(canStage({ provisional: true, wentLive: false })).toBe(false);
    expect(canStage({ provisional: false, wentLive: false })).toBe(true);
  });

  it("offers no Stage on a card auto-live already showed", () => {
    // It is on the projector; staging it again would send it twice.
    expect(canStage({ provisional: false, wentLive: true })).toBe(false);
  });

  it("only a waiting card can be pinned (FR-64)", () => {
    expect(canPin({ provisional: false, wentLive: false })).toBe(true);
    expect(canPin({ provisional: true, wentLive: false })).toBe(false);
    expect(canPin({ provisional: false, wentLive: true })).toBe(false);
  });

  it("badges every source the backend can send", () => {
    // Letters and labels are the wireframe's: R, V, A. Manual is ours.
    expect(sourceBadge("regex")).toEqual({ letter: "R", label: "Direct reference", dot: "regex" });
    expect(sourceBadge("vector")).toEqual({ letter: "V", label: "Semantic match", dot: "vector" });
    expect(sourceBadge("llm")).toEqual({ letter: "A", label: "Paraphrase (Claude)", dot: "llm" });
    expect(sourceBadge("manual").letter).toBe("M");
  });

  it("shows confidence as a whole percent inside 0..100", () => {
    expect(confidencePercent(0.978)).toBe(98);
    expect(confidencePercent(0.55)).toBe(55);
    expect(confidencePercent(1.2)).toBe(100);
    expect(confidencePercent(-0.1)).toBe(0);
  });
});
