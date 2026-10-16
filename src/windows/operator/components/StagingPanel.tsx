import { useEffect } from "react";
import { on, output } from "@/lib/ipc";
import { useServiceStore } from "@/stores/serviceStore";
import type { Verse } from "@/lib/types";
import { isTypingTarget } from "./stagingKeys";

/**
 * The staging slot (FR-50, PRD §13.5): Staged and Live side by side, a Go
 * Live button, Escape to clear. Layout from wireframe.html §staging.
 *
 * The backend owns the state; this panel mirrors `output:changed` into the
 * service store and asks for the current state on mount, because the Live
 * tab unmounts when the operator switches away and must come back showing
 * what is actually on the projector. Enter is handled by the detection
 * stack, which knows the top card; Escape is handled here, since the slot
 * is what it empties.
 */
export function StagingPanel() {
  const outputs = useServiceStore((s) => s.outputs);
  const setOutputs = useServiceStore((s) => s.setOutputs);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void output.get().then((s) => {
      if (!cancelled) setOutputs(s);
    });
    void on("output:changed", setOutputs).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [setOutputs]);

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
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-sm font-semibold">Staging</h3>
        {outputs.blanked && <span className="chip">Projector blanked</span>}
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
