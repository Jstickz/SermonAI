import { useCallback, useEffect, useState } from "react";
import { output } from "@/lib/ipc";
import { useServiceStore } from "@/stores/serviceStore";
import type { Verse } from "@/lib/types";
import { isTypingTarget, stepDelta } from "./stagingKeys";
import { AutoLiveSwitch } from "./AutoLiveSwitch";

/**
 * The staging slot (FR-50, PRD §13.5): Staged and Live side by side, a Go
 * Live button, Escape to clear. Layout from wireframe.html §staging.
 *
 * The backend owns the state; `OperatorApp` mirrors `output:changed` into
 * the service store (it has to, for the auto-live indicator on every tab),
 * and this panel reads the store. Enter is handled by the detection stack,
 * which knows the top card; Escape is handled here, since the slot is what
 * it empties.
 *
 * Next and previous verse (FR-51, deliverable 6): the arrow keys and the
 * two buttons under the Live slot step the passage on the projector. The
 * backend finds the neighbour in the cache and replaces the live verse
 * directly; an end of the book, or an uncached verse, comes back as a
 * message under the buttons rather than a silent nothing.
 */
export function StagingPanel() {
  const outputs = useServiceStore((s) => s.outputs);
  const [stepError, setStepError] = useState<string | null>(null);

  const step = useCallback(async (delta: 1 | -1) => {
    try {
      await output.step(delta);
      setStepError(null);
    } catch (e) {
      setStepError(String(e));
    }
  }, []);

  // → and ← step the live passage (FR-51) — outside text fields, and only
  // while something is live, so the arrows keep their meaning elsewhere.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const delta = stepDelta(e, e.target);
      if (delta === 0 || !useServiceStore.getState().outputs.live) return;
      e.preventDefault();
      void step(delta);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [step]);

  // Escape clears the staged slot (§13.5) — outside text fields.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || isTypingTarget(e.target)) return;
      if (!useServiceStore.getState().outputs.staged) return;
      e.preventDefault();
      void output.clearStaged().catch(() => undefined);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const staged = outputs.staged;
  const live = outputs.live;

  return (
    <section className="rounded-lg bg-bg-raised p-5">
      <div className="mb-3 flex items-center justify-between gap-3">
        <h3 className="text-sm font-semibold">Staging</h3>
        <div className="flex items-center gap-3">
          {outputs.blanked && <span className="chip">Projector blanked</span>}
          {/* The wireframe puts the switch here as well as in Settings: the
              operator who turns it on mid-service should not have to leave
              the Live tab, and the one who sees the indicator should find
              the way off beside it. */}
          <AutoLiveSwitch compact />
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <Slot
          label={staged ? "Staged · press ↵ to go live" : "Staged"}
          verse={staged}
          empty="Nothing staged. Stage a card, or press ↵ on the top one."
          live={false}
        />
        <Slot
          label="Live on projector"
          verse={live}
          empty="Nothing live."
          live={true}
        />
      </div>

      <button
        type="button"
        className="btn-primary mt-3 w-full !py-3 disabled:opacity-40"
        disabled={!staged}
        onClick={() => void output.goLive().catch(() => undefined)}
        title={staged ? "Send the staged verse to the projector" : "Stage a verse first"}
      >
        Go Live · Enter
      </button>

      <div className="mt-3 flex items-center gap-2">
        <button
          type="button"
          className="btn-secondary min-h-10 flex-1 disabled:opacity-40"
          disabled={!live}
          onClick={() => void step(-1)}
          title={live ? "Show the verse before this one" : "Nothing is live"}
        >
          ← Previous Verse
        </button>
        <button
          type="button"
          className="btn-secondary min-h-10 flex-1 disabled:opacity-40"
          disabled={!live}
          onClick={() => void step(1)}
          title={live ? "Show the verse after this one" : "Nothing is live"}
        >
          Next Verse →
        </button>
      </div>
      {stepError && <p className="mt-2 text-xs text-status-warning">{stepError}</p>}
    </section>
  );
}

function Slot({
  label,
  verse,
  empty,
  live,
}: {
  label: string;
  verse: Verse | null;
  empty: string;
  live: boolean;
}) {
  const frame = live
    ? "border border-accent-500 bg-gradient-to-b from-accent-900/30 to-transparent"
    : "border border-dashed border-line-default bg-bg-sunken";
  return (
    <div className={`flex min-h-[130px] flex-col gap-1.5 rounded-md p-3.5 ${frame}`}>
      <div
        className={`flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-[0.08em] ${
          live ? "text-accent-500" : "text-content-muted"
        }`}
      >
        {live && verse && <span className="inline-block h-1.5 w-1.5 rounded-full bg-accent-500" />}
        {label}
      </div>
      {verse ? (
        <>
          <div className="text-[13px] font-semibold">
            {verse.reference}
            <span className="mono ml-2 text-content-muted">{verse.translation}</span>
          </div>
          <p className="line-clamp-4 font-serif text-[13px] leading-normal text-content-secondary">
            {verse.text || "No cached text in this translation."}
          </p>
        </>
      ) : (
        <p className="m-auto text-center text-xs text-content-muted">{empty}</p>
      )}
    </div>
  );
}
