import { useCallback, useEffect } from "react";
import { detections as detectionsApi, on, output } from "@/lib/ipc";
import type { Detection, Verse } from "@/lib/types";
import { useServiceStore } from "@/stores/serviceStore";
import { useTranslationStore } from "@/stores/translationStore";
import { DetectionCard } from "./DetectionCard";
import { enterAction, isTypingTarget } from "./stagingKeys";

/** The verse a card stages. A card whose text was not cached still carries
 *  its reference (§13.3), so the operator can stage it and read it later. */
function verseOf(d: Detection): Verse {
  return (
    d.verse ?? {
      reference: d.reference,
      translation: useTranslationStore.getState().current,
      text: "",
    }
  );
}

/**
 * The right-hand column of the Live tab: detection cards, newest first
 * (wireframe.html §op-col "Detections").
 *
 * Subscribes to `detection:new` and `detection:withdraw` and hands both to
 * the service store, which upserts by id — so a provisional card firming up
 * changes in place rather than appearing twice. The store lives outside this
 * component so the stack survives a tab switch (see TabPanel).
 *
 * Enter does the next thing in the path — stage the top confirmed card, or
 * send the staged verse live (see `stagingKeys.ts`) — but only when no text
 * field has focus: an operator typing a corrected reference is not staging
 * anything.
 */
export function DetectionStack() {
  const detections = useServiceStore((s) => s.detections);
  const addDetection = useServiceStore((s) => s.addDetection);
  const dismissDetection = useServiceStore((s) => s.dismissDetection);
  const withdrawDetection = useServiceStore((s) => s.withdrawDetection);

  useEffect(() => {
    let cancelled = false;
    let unlistenNew: (() => void) | undefined;
    let unlistenWithdraw: (() => void) | undefined;

    void on("detection:new", (card) => addDetection(card)).then((fn) => {
      if (cancelled) fn();
      else unlistenNew = fn;
    });
    void on("detection:withdraw", ({ id }) => withdrawDetection(id)).then((fn) => {
      if (cancelled) fn();
      else unlistenWithdraw = fn;
    });

    return () => {
      cancelled = true;
      unlistenNew?.();
      unlistenWithdraw?.();
    };
  }, [addDetection, withdrawDetection]);

  // Stage puts the verse in the staged slot (FR-50) and records the Accept.
  // The card leaves the stack: it is now in staging, where the operator
  // reads it again before it goes live.
  const stage = useCallback(
    (d: Detection) => {
      void output.stage(verseOf(d)).catch(() => undefined);
      void detectionsApi.accept(d.id, d.passageId).catch(() => undefined);
      dismissDetection(d.id);
    },
    [dismissDetection],
  );

  const reject = useCallback(
    (d: Detection) => {
      void detectionsApi.reject(d.id, d.passageId).catch(() => undefined);
      dismissDetection(d.id);
    },
    [dismissDetection],
  );

  const edit = useCallback((d: Detection, reference: string) => {
    // The corrected card comes back through detection:new under the same id;
    // nothing to do here but ask.
    void detectionsApi.edit(d.id, reference).catch(() => undefined);
  }, []);

  const expire = useCallback((d: Detection) => dismissDetection(d.id), [dismissDetection]);

  // Enter → stage the top card, or go live with what is staged; Shift+Enter
  // → both (stagingKeys.ts has the rule and its reasons).
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key !== "Enter" || isTypingTarget(e.target)) return;
      const { detections, outputs } = useServiceStore.getState();
      const top = detections.find((d) => !d.provisional);
      const action = enterAction(outputs.staged !== null, top !== undefined, e.shiftKey);
      if (action === "none") return;
      e.preventDefault();
      if (action === "go-live") {
        void output.goLive().catch(() => undefined);
        return;
      }
      if (!top) return; // "stage" and "stage-and-go-live" both need a card
      if (action === "stage") {
        stage(top);
        return;
      }
      // Sequenced: the staged slot must hold the card before go_live
      // promotes it; the two commands share one lock on the backend.
      void output
        .stage(verseOf(top))
        .then(() => output.goLive())
        .catch(() => undefined);
      void detectionsApi.accept(top.id, top.passageId).catch(() => undefined);
      dismissDetection(top.id);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [stage, dismissDetection]);

  const waiting = detections.filter((d) => !d.provisional).length;

  return (
    <>
      <div className="stack-header">
        <h3>Detections</h3>
        <span className="count mono text-xs text-content-muted">
          {waiting === 0 ? "none waiting" : `${waiting} waiting`}
        </span>
      </div>
      <section className="scroll-hidden flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto">
        {detections.length === 0 ? (
          <p className="rounded-lg bg-bg-surface p-5 text-[13px] text-content-muted">
            Scripture detected in the transcript will appear here as cards.
          </p>
        ) : (
          detections.map((d) => (
            <DetectionCard
              key={d.id}
              detection={d}
              onStage={stage}
              onReject={reject}
              onEdit={edit}
              onExpire={expire}
            />
          ))
        )}
      </section>
    </>
  );
}
