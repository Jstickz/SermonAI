import { useEffect, useState } from "react";
import { on, output } from "@/lib/ipc";
import type { ProjectorLayout } from "@/lib/types";

/**
 * What the projector shows beside the verse (PRD §13.6, FR-26 basic). Three
 * choices, persisted, applied to the verse that is live now so the operator
 * sees the change on the projector as they make it.
 */
const LAYOUTS: { id: ProjectorLayout; label: string; detail: string }[] = [
  { id: "verse_only", label: "Verse only", detail: "Just the words." },
  {
    id: "verse_reference",
    label: "Verse and reference",
    detail: "The reference under the verse. The default.",
  },
  {
    id: "verse_reference_translation",
    label: "Verse, reference and translation",
    detail: "Adds the translation's short name, small, in the corner.",
  },
];

export function ProjectorLayoutSettings() {
  const [layout, setLayout] = useState<ProjectorLayout>("verse_reference");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void output.get().then((s) => {
      if (!cancelled) setLayout(s.layout);
    });
    void on("output:changed", (s) => setLayout(s.layout)).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  async function choose(id: ProjectorLayout) {
    try {
      const next = await output.setLayout(id);
      setLayout(next.layout);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="card">
      <div className="mb-5">
        <h2 className="text-[22px] font-semibold">Projector layout</h2>
        <p className="mt-1 text-xs text-content-muted">
          What the congregation sees beside the verse. Changes apply to the verse on screen now.
        </p>
      </div>
      {error && (
        <p className="mb-4 rounded-md bg-status-danger-bg px-3 py-2 text-status-danger">{error}</p>
      )}
      <div className="grid gap-3 sm:grid-cols-3" role="radiogroup" aria-label="Projector layout">
        {LAYOUTS.map((l) => {
          const selected = l.id === layout;
          return (
            <button
              key={l.id}
              type="button"
              role="radio"
              aria-checked={selected}
              onClick={() => void choose(l.id)}
              className={`min-h-10 rounded-md border-2 bg-bg-sunken p-4 text-left transition-colors duration-base ease-brand-out ${
                selected ? "border-accent-500" : "border-line-default"
              }`}
            >
              <div className="text-[13px] font-medium">{l.label}</div>
              <p className="mt-1 text-[11px] text-content-muted">{l.detail}</p>
            </button>
          );
        })}
      </div>
    </section>
  );
}
