import { useEffect } from "react";
import { output } from "@/lib/ipc";
import { useServiceStore } from "@/stores/serviceStore";
import { isBlankHotkey } from "./stagingKeys";

/**
 * Blank and unblank the projector (FR-33, M3 deliverable 5): the top bar's
 * icon button (wireframe §topbar "Blank projector") and the **B** hotkey
 * (wireframe §projector "Blanked state"). Instant, no fade, no
 * confirmation: a blank is the operator's fastest way out of a wrong verse,
 * and a dialog in front of it would cost the seconds it exists to save.
 *
 * The live verse is kept underneath (`output/staging.rs`), so unblank shows
 * it again with nothing re-staged. The hotkey works on every tab and never
 * inside a text field, like Enter and Escape.
 */
export function BlankButton() {
  const blanked = useServiceStore((s) => s.outputs.blanked);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (!isBlankHotkey(e, e.target)) return;
      e.preventDefault();
      const { blanked } = useServiceStore.getState().outputs;
      void output.setBlanked(!blanked).catch(() => undefined);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  return (
    <button
      type="button"
      aria-pressed={blanked}
      title={blanked ? "Unblank projector · B" : "Blank projector · B"}
      onClick={() => void output.setBlanked(!blanked).catch(() => undefined)}
      className={`inline-flex min-h-10 min-w-10 items-center justify-center gap-2 rounded-md px-3 text-[13px] transition-colors duration-base ease-brand-out ${
        blanked
          ? "bg-status-warning-bg text-status-warning"
          : "text-content-secondary hover:bg-bg-pill hover:text-content-primary"
      }`}
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" className="h-[18px] w-[18px]" aria-hidden="true">
        <rect x="3" y="5" width="18" height="14" rx="2" />
        <path d="M3 12h18" />
      </svg>
      {blanked ? "Blanked" : "Blank"}
    </button>
  );
}
