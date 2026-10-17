import { create } from "zustand";
import type { Detection, OutputState, TranscriptSegment, Verse } from "@/lib/types";

/** How many recent verses the palette offers (FR-31, "recent verses on top"). */
export const RECENT_LIMIT = 8;

/** `verse` becomes the most recent; a verse already there moves up rather
 *  than appearing twice. Same reference in another translation is another
 *  entry: the operator may want either. */
export function rememberRecent(recent: Verse[], verse: Verse): Verse[] {
  const same = (v: Verse) => v.reference === verse.reference && v.translation === verse.translation;
  return [verse, ...recent.filter((v) => !same(v))].slice(0, RECENT_LIMIT);
}

/** A pinned verse (FR-64): kept until Unpin or Clear all. `detectionId` is
 *  the card it came from, for the row the PDF reads; null when pinned from
 *  a slot. */
export interface Pin {
  verse: Verse;
  detectionId: number | null;
  passageId: string | null;
}

/** Pins are keyed by reference and translation: the same verse in another
 *  translation is another pin, and the same verse twice is one. */
export function pinKey(verse: Pick<Verse, "reference" | "translation">): string {
  return `${verse.reference}·${verse.translation}`;
}

/** Live-service state for the operator window. Persisted state lives in SQLite;
 *  this store only holds what the UI needs between events. */
interface ServiceState {
  sermonId: number | null;
  isCapturing: boolean;
  peakDbfs: number;
  segments: TranscriptSegment[];
  detections: Detection[];
  /** Pinned verses in the order they were pinned (FR-64): the first pinned
   *  is 1 until it is unpinned, whatever else happens. */
  pins: Pin[];
  outputs: OutputState;
  /** Verses staged or sent live this service, newest first (FR-31). */
  recent: Verse[];

  setSermonId: (id: number | null) => void;
  setCapturing: (capturing: boolean) => void;
  setLevel: (peakDbfs: number) => void;
  /** Interim segments are replaced in place; finals append (FR-10). */
  upsertSegment: (segment: TranscriptSegment) => void;
  /** Add a card, or replace the card with the same id. A provisional
   *  candidate arrives once unconfirmed and again confirmed under the same id
   *  (PRD §18.1); the operator must see one card change state, not two. */
  addDetection: (detection: Detection) => void;
  dismissDetection: (id: number) => void;
  /** A provisional candidate the settled text did not support. Same removal
   *  as dismiss, named separately so a reader knows nobody clicked. */
  withdrawDetection: (id: number) => void;
  /** Pin a verse (FR-64). Already pinned: nothing changes, it keeps its place. */
  addPin: (verse: Verse, detectionId: number | null, passageId?: string | null) => void;
  removePin: (key: string) => void;
  /** Clear all: the only bulk removal (polish 5c). */
  clearPins: () => void;
  /** A new service starts with no cards. Pins stay (polish 5d): they go
   *  only by Unpin or Clear all. */
  clearDetections: () => void;
  setOutputs: (outputs: OutputState) => void;
  reset: () => void;
}

const EMPTY_OUTPUTS: OutputState = {
  live: null,
  staged: null,
  blanked: false,
  layout: "verse_reference",
  autoLive: false,
  transition: "fade",
  transitionMs: 400,
  liveSinceMs: null,
};

export const useServiceStore = create<ServiceState>((set) => ({
  sermonId: null,
  isCapturing: false,
  peakDbfs: -60,
  segments: [],
  detections: [],
  pins: [],
  outputs: EMPTY_OUTPUTS,
  recent: [],

  setSermonId: (sermonId) => set({ sermonId }),
  setCapturing: (isCapturing) => set({ isCapturing }),
  setLevel: (peakDbfs) => set({ peakDbfs }),

  upsertSegment: (segment) =>
    set((state) => {
      const index = state.segments.findIndex((s) => s.id === segment.id);
      if (index === -1) return { segments: [...state.segments, segment] };
      const segments = [...state.segments];
      segments[index] = segment;
      return { segments };
    }),

  // Newest first: the card stack grows downward from the top of the column.
  // A known id is replaced in place, keeping its position: a candidate firming
  // up from provisional to confirmed should not jump to the top as if new.
  addDetection: (detection) =>
    set((state) => {
      const index = state.detections.findIndex((d) => d.id === detection.id);
      if (index === -1) return { detections: [detection, ...state.detections] };
      const detections = [...state.detections];
      detections[index] = detection;
      return { detections };
    }),
  addPin: (verse, detectionId, passageId = null) =>
    set((state) => {
      const key = pinKey(verse);
      if (state.pins.some((p) => pinKey(p.verse) === key)) return {};
      return { pins: [...state.pins, { verse, detectionId, passageId }] };
    }),
  removePin: (key) => set((state) => ({ pins: state.pins.filter((p) => pinKey(p.verse) !== key) })),
  clearPins: () => set({ pins: [] }),
  clearDetections: () => set({ detections: [], recent: [] }),
  dismissDetection: (id) => set((state) => ({ detections: state.detections.filter((d) => d.id !== id) })),
  withdrawDetection: (id) => set((state) => ({ detections: state.detections.filter((d) => d.id !== id) })),
  // A verse arriving in either slot is remembered for the palette. Compared
  // by reference and translation, not identity: every broadcast is a new
  // object, and a blank toggle must not re-remember the live verse.
  setOutputs: (outputs) =>
    set((state) => {
      let recent = state.recent;
      const arrived = (next: Verse | null, prev: Verse | null) =>
        next !== null &&
        (prev === null || prev.reference !== next.reference || prev.translation !== next.translation);
      if (arrived(outputs.staged, state.outputs.staged)) recent = rememberRecent(recent, outputs.staged!);
      if (arrived(outputs.live, state.outputs.live)) recent = rememberRecent(recent, outputs.live!);
      return { outputs, recent };
    }),

  reset: () =>
    set({
      sermonId: null,
      isCapturing: false,
      segments: [],
      detections: [],
      pins: [],
      outputs: EMPTY_OUTPUTS,
      recent: [],
    }),
}));
