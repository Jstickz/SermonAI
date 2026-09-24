import { useCallback, useEffect } from "react";
import { detections as detectionsApi, on } from "@/lib/ipc";
import type { Detection } from "@/lib/types";
import { useServiceStore } from "@/stores/serviceStore";
import { DetectionCard } from "./DetectionCard";

/**
 * The right-hand column of the Live tab: detection cards, newest first
 * (wireframe.html §op-col "Detections").
 *
 * Subscribes to `detection:new` and `detection:withdraw` and hands both to
 * the service store, which upserts by id — so a provisional card firming up
 * changes in place rather than appearing twice. The store lives outside this
 * component so the stack survives a tab switch (see TabPanel).
 *
 * Enter stages the top confirmed card (wireframe: "Stage · ↵"), but only when
 * no text field has focus: an operator typing a corrected reference is not
 * staging anything.
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

  const stage = useCallback(
    (d: Detection) => {
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

  // Enter → stage the top confirmed card.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key !== "Enter") return;
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) return;
      const top = useServiceStore.getState().detections.find((d) => !d.provisional);
      if (top) {
        e.preventDefault();
        stage(top);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [stage]);

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
