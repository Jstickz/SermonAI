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
  setOutputs: (outputs: OutputState) => void;
  reset: () => void;
}

const EMPTY_OUTPUTS: OutputState = { live: null, staged: null, blanked: false };

export const useServiceStore = create<ServiceState>((set) => ({
  sermonId: null,
  isCapturing: false,
  peakDbfs: -60,
  segments: [],
  detections: [],
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
  addDetection: (detection) =>
    set((state) => {
      const index = state.detections.findIndex((d) => d.id === detection.id);
      if (index === -1) return { detections: [detection, ...state.detections] };
      const detections = [...state.detections];
      detections[index] = detection;
      return { detections };
    }),
  dismissDetection: (id) => set((state) => ({ detections: state.detections.filter((d) => d.id !== id) })),
  withdrawDetection: (id) => set((state) => ({ detections: state.detections.filter((d) => d.id !== id) })),
  setOutputs: (outputs) => set({ outputs }),

  reset: () => set({ sermonId: null, isCapturing: false, segments: [], detections: [], outputs: EMPTY_OUTPUTS }),
}));
