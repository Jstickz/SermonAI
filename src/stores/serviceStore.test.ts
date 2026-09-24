import { beforeEach, describe, expect, it } from "vitest";
import { useServiceStore } from "./serviceStore";
import type { Detection, TranscriptSegment } from "@/lib/types";

function segment(id: number, text: string, isFinal: boolean): TranscriptSegment {
  return { id, startTimeMs: id * 1000, endTimeMs: id * 1000 + 900, text, confidence: 0.9, isFinal };
}

function detection(id: number, provisional = false): Detection {
  return {
    id,
    passageId: "JHN.3.16",
    reference: "John 3:16",
    source: "regex",
    confidence: 0.98,
    provisional,
    evidence: null,
    detectedAtMs: 0,
    // Null until the Bible cache has the text (M2 deliverable 5).
    verse: null,
  };
}

describe("serviceStore", () => {
  beforeEach(() => {
    useServiceStore.getState().reset();
  });

  it("replaces an interim segment in place when the final result arrives", () => {
    const { upsertSegment } = useServiceStore.getState();

    upsertSegment(segment(1, "for I know the plans", false));
    upsertSegment(segment(1, "for I know the plans I have for you", true));

    const { segments } = useServiceStore.getState();
    expect(segments).toHaveLength(1);
    expect(segments[0]?.text).toBe("for I know the plans I have for you");
    expect(segments[0]?.isFinal).toBe(true);
  });

  it("appends distinct segments in arrival order", () => {
    const { upsertSegment } = useServiceStore.getState();

    upsertSegment(segment(1, "first", true));
    upsertSegment(segment(2, "second", true));

    expect(useServiceStore.getState().segments.map((s) => s.text)).toEqual(["first", "second"]);
  });

  // Rapid-fire references must never be dropped (FR-17), and the newest card
  // sits at the top of the operator's stack.
  it("stacks detections newest first and never drops one", () => {
    const { addDetection } = useServiceStore.getState();

    for (let i = 0; i < 10; i += 1) addDetection(detection(i));

    const { detections } = useServiceStore.getState();
    expect(detections).toHaveLength(10);
    expect(detections[0]?.id).toBe(9);
  });

  it("dismisses only the named detection", () => {
    const { addDetection, dismissDetection } = useServiceStore.getState();

    addDetection(detection(1));
    addDetection(detection(2));
    dismissDetection(1);

    expect(useServiceStore.getState().detections.map((d) => d.id)).toEqual([2]);
  });

  // Two-stage detection (PRD §18.1): the same id arrives twice, first
  // provisional then confirmed. One card, changing state, in place.
  it("firms up a provisional card under its id without moving it", () => {
    const { addDetection } = useServiceStore.getState();

    addDetection(detection(1, true));
    addDetection(detection(2, false)); // a newer card above it
    addDetection(detection(1, false)); // the first one confirms

    const { detections } = useServiceStore.getState();
    expect(detections).toHaveLength(2);
    expect(detections.map((d) => d.id)).toEqual([2, 1]);
    expect(detections[1]?.provisional).toBe(false);
  });

  it("withdraws a provisional card the settled text dropped", () => {
    const { addDetection, withdrawDetection } = useServiceStore.getState();

    addDetection(detection(1, true));
    addDetection(detection(2));
    withdrawDetection(1);

    expect(useServiceStore.getState().detections.map((d) => d.id)).toEqual([2]);
  });

  it("clears live state on reset so the next service starts clean", () => {
    const store = useServiceStore.getState();
    store.setSermonId(7);
    store.setCapturing(true);
    store.upsertSegment(segment(1, "text", true));
    store.addDetection(detection(99));

    store.reset();

    const after = useServiceStore.getState();
    expect(after.sermonId).toBeNull();
    expect(after.isCapturing).toBe(false);
    expect(after.segments).toHaveLength(0);
    expect(after.detections).toHaveLength(0);
    expect(after.outputs).toEqual({ live: null, staged: null, blanked: false });
  });
});
