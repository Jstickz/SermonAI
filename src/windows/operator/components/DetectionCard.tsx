import { useEffect, useState } from "react";
import type { Detection, DetectionSource } from "@/lib/types";

/**
 * One detection card (PRD §13.3, wireframe.html §detection): source badge,
 * reference, verse text, confidence bar, and Stage / Reject / Edit.
 *
 * Two states the wireframe does not draw, both from the two-stage decision
 * (PRD §18.1, §13.5):
 *
 * - **Provisional.** Raised from interim text. Marked *Unconfirmed*, dimmed,
 *   and with no Stage button at all — not a disabled one. A disabled button
 *   invites the operator to wait for it; the point is that this card is a
 *   warning something is coming, not a thing to act on. It is withdrawn
 *   silently if the settled text does not support it.
 * - **No text yet.** A confirmed card whose verse is not in the cache. The
 *   reference is shown and the card says the text is not cached, rather than
 *   spinning: the operator can still stage the reference in M3, and the
 *   picker (deliverable 7) is where a translation gets fetched.
 *
 * Auto-dismiss after 30 s (§13.3) applies to confirmed cards nobody acted on.
 * A provisional card is never auto-dismissed here; its lifetime belongs to
 * the settled text.
 */

/** The wireframe's source badge: a coloured dot, a letter, a label. */
export function sourceBadge(source: DetectionSource): { letter: string; label: string; dot: string } {
  switch (source) {
    case "regex":
      return { letter: "R", label: "Direct reference", dot: "regex" };
    case "vector":
      return { letter: "V", label: "Semantic match", dot: "vector" };
    case "llm":
      return { letter: "A", label: "Paraphrase (Claude)", dot: "llm" };
    case "manual":
      return { letter: "M", label: "Edited by operator", dot: "manual" };
  }
}

/** Whether the operator may stage this card. Never while provisional. */
export function canStage(detection: Pick<Detection, "provisional">): boolean {
  return !detection.provisional;
}

/** 0.978 → 98. Rounded, never over 100. */
export function confidencePercent(confidence: number): number {
  return Math.min(100, Math.max(0, Math.round(confidence * 100)));
}

/** §13.3: a card nobody touched leaves after this long. */
export const AUTO_DISMISS_MS = 30_000;

export function DetectionCard({
  detection,
  onStage,
  onReject,
  onEdit,
  onExpire,
}: {
  detection: Detection;
  onStage: (d: Detection) => void;
  onReject: (d: Detection) => void;
  onEdit: (d: Detection, reference: string) => void;
  onExpire: (d: Detection) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(detection.reference);
  const badge = sourceBadge(detection.source);
  const percent = confidencePercent(detection.confidence);

  // Auto-dismiss, confirmed cards only, reset if the card changes state
  // (a provisional card firming up starts its 30 s then).
  useEffect(() => {
    if (detection.provisional || editing) return;
    const timer = window.setTimeout(() => onExpire(detection), AUTO_DISMISS_MS);
    return () => window.clearTimeout(timer);
  }, [detection, editing, onExpire]);

  return (
    <article
      className={`detection ${detection.provisional ? "provisional" : ""}`}
      aria-label={`${detection.reference}, ${badge.label}, ${percent}% confidence${
        detection.provisional ? ", unconfirmed" : ""
      }`}
    >
      <div className="top">
        <span className={`dot ${badge.dot}`} aria-hidden="true" />
        <span className="src-letter">{badge.letter}</span>
        <span>
          · {badge.label} · <span className="mono">{percent}%</span> confidence
        </span>
        {detection.provisional && <span className="chip ml-auto">Unconfirmed</span>}
      </div>

      <div className="ref">{detection.reference}</div>

      {detection.provisional ? (
        <p className="text-[13px] text-content-muted">Waiting for the words to settle.</p>
      ) : detection.verse ? (
        <div className="verse-text">{detection.verse.text}</div>
      ) : (
        <p className="text-[13px] text-content-muted">
          No cached text in this translation. The reference can still be staged.
        </p>
      )}

      {detection.evidence && !detection.provisional && (
        <p className="text-xs text-content-muted">
          Heard: <span className="italic">“{detection.evidence}”</span>
        </p>
      )}

      <div className="confidence-bar" aria-hidden="true">
        <div style={{ width: `${percent}%` }} />
      </div>

      {editing ? (
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            const value = draft.trim();
            if (value === "") return;
            setEditing(false);
            onEdit(detection, value);
          }}
        >
          <input
            className="mono min-w-0 flex-1 rounded-md bg-bg-canvas px-3 py-2 text-[13px]"
            autoFocus
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            aria-label="Corrected reference"
            placeholder="e.g. John 3:16"
          />
          <button className="btn-primary" type="submit">
            Use reference
          </button>
          <button className="btn-secondary" type="button" onClick={() => setEditing(false)}>
            Cancel
          </button>
        </form>
      ) : (
        !detection.provisional && (
          <div className="actions">
            {canStage(detection) && (
              <button className="accept" onClick={() => onStage(detection)}>
                Stage · ↵
              </button>
            )}
            <button className="reject" onClick={() => onReject(detection)}>
              Reject
            </button>
            <button className="edit" onClick={() => setEditing(true)}>
              Edit
            </button>
          </div>
        )
      )}
    </article>
  );
}
