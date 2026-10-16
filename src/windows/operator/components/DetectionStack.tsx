import { useCallback, useEffect } from "react";
import { detections as detectionsApi, on, output } from "@/lib/ipc";
import type { Detection, Verse } from "@/lib/types";
import { useServiceStore } from "@/stores/serviceStore";
import { useTranslationStore } from "@/stores/translationStore";
import { DetectionCard } from "./DetectionCard";
import { enterAction, isPinHotkey, isTypingTarget, pinnedIndex } from "./stagingKeys";

/** A card the operator can still act on: confirmed, not already put on the
 *  projector by auto-live, and not pinned. Enter and the "waiting" count use
 *  the same rule, so the count never promises a card Enter will not take; a
 *  pinned card goes live only by its own controls (FR-64). */
export function isWaiting(d: Pick<Detection, "provisional" | "wentLive" | "pinned">): boolean {
  return !d.provisional && !d.wentLive && !d.pinned;
}

/** Pinned cards in the order they were pinned, so the numbers 1–9 stay
 *  stable as more are pinned: the first verse pinned is 1 for the whole
 *  service, whatever arrived when. The store keeps that order. */
export function pinnedInOrder<T extends Pick<Detection, "id" | "pinned">>(
  detections: T[],
  pinOrder: number[],
): T[] {
  const byId = new Map(detections.filter((d) => d.pinned).map((d) => [d.id, d]));
  return pinOrder.map((id) => byId.get(id)).filter((d): d is T => d !== undefined);
}

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
 *
 * Pinned cards (FR-64, deliverable 10) sit in their own section above the
 * waiting ones. P pins the top waiting card; 1 to 9 send the Nth pinned
 * card live. Staging or sending a pinned card never dismisses or unpins
 * it, and Enter never reaches it.
 */
export function DetectionStack() {
  const detections = useServiceStore((s) => s.detections);
  const addDetection = useServiceStore((s) => s.addDetection);
  const dismissDetection = useServiceStore((s) => s.dismissDetection);
  const withdrawDetection = useServiceStore((s) => s.withdrawDetection);
  const pinDetection = useServiceStore((s) => s.pinDetection);
  const pinOrder = useServiceStore((s) => s.pinOrder);

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
  // reads it again before it goes live. A pinned card stays: it is kept for
  // the next time.
  const stage = useCallback(
    (d: Detection) => {
      void output.stage(verseOf(d)).catch(() => undefined);
      void detectionsApi.accept(d.id, d.passageId).catch(() => undefined);
      if (!d.pinned) dismissDetection(d.id);
    },
    [dismissDetection],
  );

  // A pinned card straight to the projector: stage then go live, sequenced
  // on the backend's one lock. The operator has read this card already.
  const goLivePinned = useCallback((d: Detection) => {
    void output
      .stage(verseOf(d))
      .then(() => output.goLive())
      .catch(() => undefined);
    void detectionsApi.accept(d.id, d.passageId).catch(() => undefined);
  }, []);

  const pin = useCallback(
    (d: Detection, pinned: boolean) => {
      pinDetection(d.id, pinned);
      void detectionsApi.pin(d.id, d.passageId, pinned).catch(() => undefined);
    },
    [pinDetection],
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

  // P → pin the top waiting card; 1–9 → the Nth pinned card live (FR-64).
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const { detections, pinOrder } = useServiceStore.getState();
      if (isPinHotkey(e, e.target)) {
        const top = detections.find(isWaiting);
        if (!top) return;
        e.preventDefault();
        pin(top, true);
        return;
      }
      const index = pinnedIndex(e, e.target);
      if (index < 0) return;
      const target = pinnedInOrder(detections, pinOrder)[index];
      if (!target) return;
      e.preventDefault();
      goLivePinned(target);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [pin, goLivePinned]);

  // Enter → stage the top card, or go live with what is staged; Shift+Enter
  // → both (stagingKeys.ts has the rule and its reasons).
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key !== "Enter" || isTypingTarget(e.target)) return;
      const { detections, outputs } = useServiceStore.getState();
      const top = detections.find(isWaiting);
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

  const waiting = detections.filter(isWaiting).length;
  const pinned = pinnedInOrder(detections, pinOrder);
  const unpinned = detections.filter((d) => !d.pinned);

  return (
    <>
      {pinned.length > 0 && (
        <>
          <div className="stack-header">
            <h3>Pinned</h3>
            <span className="count mono text-xs text-content-muted">
              {pinned.length <= 9 ? "1–9 sends live" : `${pinned.length} pinned`}
            </span>
          </div>
          <section className="mb-3 flex flex-col gap-3 rounded-lg border border-accent-900/60 bg-bg-sunken p-2">
            {pinned.map((d, i) => (
              <DetectionCard
                key={d.id}
                detection={d}
                onStage={stage}
                onReject={reject}
                onEdit={edit}
                onExpire={expire}
                onPin={pin}
                onGoLive={goLivePinned}
                pinnedNumber={i < 9 ? i + 1 : null}
              />
            ))}
          </section>
        </>
      )}
      <div className="stack-header">
        <h3>Detections</h3>
        <span className="count mono text-xs text-content-muted">
          {waiting === 0 ? "none waiting" : `${waiting} waiting`}
        </span>
      </div>
      <section className="scroll-hidden flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto">
        {unpinned.length === 0 ? (
          <p className="rounded-lg bg-bg-surface p-5 text-[13px] text-content-muted">
            Scripture detected in the transcript will appear here as cards.
          </p>
        ) : (
          unpinned.map((d) => (
            <DetectionCard
              key={d.id}
              detection={d}
              onStage={stage}
              onReject={reject}
              onEdit={edit}
              onExpire={expire}
              onPin={pin}
              onGoLive={goLivePinned}
            />
          ))
        )}
      </section>
    </>
  );
}
