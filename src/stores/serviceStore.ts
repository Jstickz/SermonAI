import { create } from "zustand";
import type { Detection, OutputState, TranscriptSegment } from "@/lib/types";

/** Live-service state for the operator window. Persisted state lives in SQLite;
 *  this store only holds what the UI needs between events. */
interface ServiceState {
  sermonId: number | null;
  isCapturing: boolean;
  peakDbfs: number;
  segments: TranscriptSegment[];
  detections: Detection[];
  /** Ids of pinned cards in the order they were pinned (FR-64). The first
   *  pinned is 1 for the whole service, whatever arrived when. */
  pinOrder: number[];
  outputs: OutputState;

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
  /** Pin or unpin a card (FR-64). Unpinning puts it back among the waiting
   *  cards, where its 30 s starts again. */
  pinDetection: (id: number, pinned: boolean) => void;
  /** A new service starts with no cards: pins last for the service. */
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
  pinOrder: [],
  outputs: EMPTY_OUTPUTS,

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
  // The pin is the operator's, not the backend's, so a replacement (an Edit
  // coming back under the same id) keeps it.
  addDetection: (detection) =>
    set((state) => {
      const index = state.detections.findIndex((d) => d.id === detection.id);
      if (index === -1) return { detections: [detection, ...state.detections] };
      const detections = [...state.detections];
      const pinned = detections[index]?.pinned ?? false;
      detections[index] = { ...detection, pinned };
      return { detections };
    }),
  pinDetection: (id, pinned) =>
    set((state) => ({
      detections: state.detections.map((d) => (d.id === id ? { ...d, pinned } : d)),
      pinOrder: pinned
        ? state.pinOrder.includes(id)
          ? state.pinOrder
          : [...state.pinOrder, id]
        : state.pinOrder.filter((p) => p !== id),
    })),
  clearDetections: () => set({ detections: [], pinOrder: [] }),
  dismissDetection: (id) => set((state) => ({ detections: state.detections.filter((d) => d.id !== id) })),
  withdrawDetection: (id) => set((state) => ({ detections: state.detections.filter((d) => d.id !== id) })),
  setOutputs: (outputs) => set({ outputs }),

  reset: () =>
    set({ sermonId: null, isCapturing: false, segments: [], detections: [], pinOrder: [], outputs: EMPTY_OUTPUTS }),
}));
