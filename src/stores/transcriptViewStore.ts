import { create } from "zustand";

/**
 * How the transcript is *displayed*. Not the transcript itself, which lives in
 * Rust (`stt::transcript`) because losing it costs a recording.
 *
 * This is a view preference: losing it costs the operator one click. It is
 * kept in a store rather than component state so it survives the panel
 * unmounting on a tab switch, and out of Rust because the backend has no
 * business knowing how large the operator likes their text.
 *
 * Not yet persisted across restarts — settings persistence is M8, alongside
 * the display assignments and the chosen audio device, which have the same
 * gap for the same reason.
 */
export type TranscriptFontSize = "small" | "medium" | "large";

/** Matches PRD §13.2's font size setting. Three steps, not a slider: a booth
 *  volunteer wants bigger or smaller, not a value to tune. */
export const FONT_SIZES: Record<TranscriptFontSize, { label: string; className: string }> = {
  small: { label: "Small", className: "text-[14px]" },
  medium: { label: "Medium", className: "text-[15px]" },
  large: { label: "Large", className: "text-[18px]" },
};

const ORDER: TranscriptFontSize[] = ["small", "medium", "large"];

interface TranscriptViewState {
  fontSize: TranscriptFontSize;
  /** Cycle through the three sizes, wrapping. One control, no menu. */
  cycleFontSize: () => void;
}

export const useTranscriptViewStore = create<TranscriptViewState>((set) => ({
  // 15px is the wireframe's transcript body size.
  fontSize: "medium",
  cycleFontSize: () =>
    set((state) => ({
      fontSize: ORDER[(ORDER.indexOf(state.fontSize) + 1) % ORDER.length] ?? "medium",
    })),
}));
