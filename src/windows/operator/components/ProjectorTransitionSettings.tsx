import { useEffect, useState } from "react";
import { on, output } from "@/lib/ipc";
import type { ProjectorTransition } from "@/lib/types";

/**
 * How verses change on the projector (FR-25, PRD §13.6): fade, slide or
 * dissolve, and how long it takes. Default 400 ms fade (Branding §7.2
 * `motion.verse-in`). Persisted; the next go-live uses it.
 *
 * The length is a number field committed on blur or Enter rather than a
 * slider: a church sets this once, to a value it can write down, and a
 * slider invites the kind of fiddling that ends in a 1,400 ms verse.
 */
const TRANSITIONS: { id: ProjectorTransition; label: string; detail: string }[] = [
  { id: "fade", label: "Fade", detail: "The old verse fades out, then the new one fades in. The default." },
  { id: "slide", label: "Slide", detail: "The old verse lifts away as the new one rises into place." },
  { id: "dissolve", label: "Dissolve", detail: "The two cross-fade at once." },
];

export const DEFAULT_TRANSITION_MS = 400;
export const MAX_TRANSITION_MS = 1500;

/** The field's text as a length the backend accepts, or null. */
export function parseDurationMs(text: string): number | null {
  if (!/^\d+$/.test(text.trim())) return null;
  const n = Number(text.trim());
  return n >= 0 && n <= MAX_TRANSITION_MS ? n : null;
}

export function ProjectorTransitionSettings() {
  const [transition, setTransition] = useState<ProjectorTransition>("fade");
  const [durationMs, setDurationMs] = useState(DEFAULT_TRANSITION_MS);
  const [draft, setDraft] = useState(String(DEFAULT_TRANSITION_MS));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    const apply = (t: ProjectorTransition, ms: number) => {
      setTransition(t);
      setDurationMs(ms);
      setDraft(String(ms));
    };
    void output.get().then((s) => {
      if (!cancelled) apply(s.transition, s.transitionMs);
    });
    void on("output:changed", (s) => apply(s.transition, s.transitionMs)).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  async function choose(t: ProjectorTransition, ms: number) {
    try {
      const next = await output.setTransition(t, ms);
      setTransition(next.transition);
      setDurationMs(next.transitionMs);
      setDraft(String(next.transitionMs));
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  function commitDraft() {
    const ms = parseDurationMs(draft);
    if (ms === null) {
      setError(`Enter a length from 0 to ${MAX_TRANSITION_MS} ms. 0 is instant.`);
      setDraft(String(durationMs));
      return;
    }
    if (ms !== durationMs) void choose(transition, ms);
  }

  return (
    <section className="card">
      <div className="mb-5">
        <h2 className="text-[22px] font-semibold">Transition</h2>
        <p className="mt-1 text-xs text-content-muted">
          How one verse gives way to the next. Blanking is always instant.
        </p>
      </div>
      {error && (
        <p className="mb-4 rounded-md bg-status-danger-bg px-3 py-2 text-status-danger">{error}</p>
      )}
      <div className="grid gap-3 sm:grid-cols-3" role="radiogroup" aria-label="Projector transition">
        {TRANSITIONS.map((t) => {
          const selected = t.id === transition;
          return (
            <button
              key={t.id}
              type="button"
              role="radio"
              aria-checked={selected}
              onClick={() => void choose(t.id, durationMs)}
              className={`min-h-10 rounded-md border-2 bg-bg-sunken p-4 text-left transition-colors duration-base ease-brand-out ${
                selected ? "border-accent-500" : "border-line-default"
              }`}
            >
              <div className="text-[13px] font-medium">{t.label}</div>
              <p className="mt-1 text-[11px] text-content-muted">{t.detail}</p>
            </button>
          );
        })}
      </div>

      <div className="mt-5 flex flex-wrap items-center gap-3">
        <label className="flex items-center gap-3 text-[13px] font-medium">
          Length
          <input
            className="mono w-24 rounded-md bg-bg-canvas px-3 py-2 text-right text-[13px] tabular-nums"
            inputMode="numeric"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commitDraft}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                commitDraft();
              }
            }}
            aria-label="Transition length in milliseconds"
          />
          <span className="text-xs text-content-muted">ms</span>
        </label>
        <button
          type="button"
          className="btn-secondary text-xs"
          disabled={durationMs === DEFAULT_TRANSITION_MS}
          onClick={() => void choose(transition, DEFAULT_TRANSITION_MS)}
        >
          Use Default ({DEFAULT_TRANSITION_MS} ms)
        </button>
      </div>
    </section>
  );
}
