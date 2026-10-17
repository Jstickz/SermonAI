import { useCallback, useEffect, useRef, useState } from "react";
import { detections as detectionsApi, on, output } from "@/lib/ipc";
import type { Detection, Verse } from "@/lib/types";
import { pinKey, useServiceStore, type Pin } from "@/stores/serviceStore";
import { useTranslationStore } from "@/stores/translationStore";
import { DetectionCard } from "./DetectionCard";
import { enterAction, isPinHotkey, isTypingTarget, pinnedIndex } from "./stagingKeys";

/** A card the operator can still act on: confirmed, and not already put on
 *  the projector by auto-live. Enter and the "waiting" count use the same
 *  rule, so the count never promises a card Enter will not take. */
export function isWaiting(d: Pick<Detection, "provisional" | "wentLive">): boolean {
  return !d.provisional && !d.wentLive;
}

/** The verse a card stages. A card whose text was not cached still carries
 *  its reference (§13.3), so the operator can stage it and read it later. */
export function verseOf(d: Detection): Verse {
  return (
    d.verse ?? {
      reference: d.reference,
      translation: useTranslationStore.getState().current,
      text: "",
    }
  );
}

/**
 * The right-hand column of the Live tab: the pinned strip, then detection
 * cards newest first (wireframe.html §op-col "Detections").
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
 * Pins (FR-64; 17 Oct polish item 5): a pin is a verse kept for the
 * service, not a card. Pinning a card moves it out of the waiting cards and
 * into the strip; a staged or live verse can be pinned from its slot with no
 * card at all. The strip is a fixed, compact height and scrolls sideways,
 * so the waiting cards below keep the column's height. A pin offers Go Live
 * — straight to the projector, the operator has read it — and Unpin; P pins
 * the top waiting card; 1 to 9 send the Nth pin live. Pins outlive
 * Transcribe being switched off and on, and go only by Unpin or Clear all.
 */
export function DetectionStack() {
  const detections = useServiceStore((s) => s.detections);
  const pins = useServiceStore((s) => s.pins);
  const addDetection = useServiceStore((s) => s.addDetection);
  const dismissDetection = useServiceStore((s) => s.dismissDetection);
  const withdrawDetection = useServiceStore((s) => s.withdrawDetection);
  const addPin = useServiceStore((s) => s.addPin);
  const removePin = useServiceStore((s) => s.removePin);
  const clearPins = useServiceStore((s) => s.clearPins);

  useEffect(() => {
    let cancelled = false;
    let unlistenNew: (() => void) | undefined;
    let unlistenWithdraw: (() => void) | undefined;

    void on("detection:new", (card) => {
      addDetection(card);
      // Two frames after the store update: the first is React's commit,
      // the second the paint of it. The same method as the projector's
      // paint report, so the two legs add up.
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          void detectionsApi.rendered(card.id, card.provisional, card.detectedAtMs, Date.now()).catch(() => undefined);
        });
      });
    }).then((fn) => {
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

  // Pin a card: it leaves the waiting cards for the strip, and its row is
  // marked for the PDF (M4).
  const pinCard = useCallback(
    (d: Detection) => {
      addPin(verseOf(d), d.id);
      void detectionsApi.pin(d.id, d.passageId, true).catch(() => undefined);
      dismissDetection(d.id);
    },
    [addPin, dismissDetection],
  );

  // A pin straight to the projector: stage then go live, sequenced on the
  // backend's one lock. The pin stays.
  const goLivePin = useCallback((p: Pin) => {
    void output
      .stage(p.verse)
      .then(() => output.goLive())
      .catch(() => undefined);
    if (p.detectionId !== null) {
      void detectionsApi.accept(p.detectionId, p.passageId ?? "").catch(() => undefined);
    }
  }, []);

  const unpin = useCallback(
    (p: Pin) => {
      removePin(pinKey(p.verse));
      if (p.detectionId !== null) {
        void detectionsApi.pin(p.detectionId, p.passageId ?? "", false).catch(() => undefined);
      }
    },
    [removePin],
  );

  // Clear all: the only bulk removal, behind a quick confirm (FR-64 5c).
  const [confirmClear, setConfirmClear] = useState(false);
  useEffect(() => {
    if (!confirmClear) return;
    const id = window.setTimeout(() => setConfirmClear(false), 5_000);
    return () => window.clearTimeout(id);
  }, [confirmClear]);
  const clearAll = useCallback(() => {
    for (const p of useServiceStore.getState().pins) {
      if (p.detectionId !== null) {
        void detectionsApi.pin(p.detectionId, p.passageId ?? "", false).catch(() => undefined);
      }
    }
    clearPins();
    setConfirmClear(false);
  }, [clearPins]);

  const reject = useCallback(
    (d: Detection) => {
      void detectionsApi.reject(d.id, d.passageId).catch(() => undefined);
      dismissDetection(d.id);
    },
    [dismissDetection],
  );

  const [editError, setEditError] = useState<string | null>(null);
  const edit = useCallback((d: Detection, reference: string, translation: string | null) => {
    // The corrected card comes back through detection:new under the same id;
    // nothing to do here but ask, and show why if it was refused.
    detectionsApi
      .edit(d.id, reference, translation)
      .then(() => setEditError(null))
      .catch((e) => setEditError(String(e)));
  }, []);

  const expire = useCallback((d: Detection) => dismissDetection(d.id), [dismissDetection]);

  // P → pin the top waiting card; 1–9 → the Nth pin live (FR-64).
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const { detections, pins } = useServiceStore.getState();
      if (isPinHotkey(e, e.target)) {
        const top = detections.find(isWaiting);
        if (!top) return;
        e.preventDefault();
        pinCard(top);
        return;
      }
      const index = pinnedIndex(e, e.target);
      if (index < 0) return;
      const target = pins[index];
      if (!target) return;
      e.preventDefault();
      goLivePin(target);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [pinCard, goLivePin]);

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

  return (
    <>
      {pins.length > 0 && (
        <>
          <div className="stack-header">
            <h3>
              Pinned <span className="mono text-xs font-normal text-content-muted">{pins.length}</span>
            </h3>
            {confirmClear ? (
              <span className="flex items-center gap-1.5 text-xs">
                <span className="text-content-secondary">Clear {pins.length}?</span>
                <button type="button" className="btn-compact-danger" onClick={clearAll} autoFocus>
                  Clear
                </button>
                <button type="button" className="btn-compact" onClick={() => setConfirmClear(false)}>
                  Keep
                </button>
              </span>
            ) : (
              <button
                type="button"
                className="btn-compact"
                onClick={() => setConfirmClear(true)}
                title="Remove every pin"
              >
                Clear all
              </button>
            )}
          </div>
          <PinStrip pins={pins} onGoLive={goLivePin} onUnpin={unpin} />
        </>
      )}
      <div className="stack-header">
        <h3>Detections</h3>
        <span className="count mono text-xs text-content-muted">
          {waiting === 0 ? "none waiting" : `${waiting} waiting`}
        </span>
      </div>
      {editError && (
        <p className="mb-2 rounded-md bg-status-warning-bg px-3 py-2 text-xs text-status-warning">{editError}</p>
      )}
      <section className="scroll-hidden flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto">
        {detections.length === 0 ? (
          <p className="rounded-lg bg-bg-surface p-4 text-[13px] text-content-muted">
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
              onPin={pinCard}
            />
          ))
        )}
      </section>
    </>
  );
}

/** The strip and its arrows. The scrollbar is hidden (operator, 17 Oct);
 *  ‹ › under the strip at the right move it a chip at a time, and are shown
 *  only when there is more than fits. */
function PinStrip({
  pins,
  onGoLive,
  onUnpin,
}: {
  pins: Pin[];
  onGoLive: (p: Pin) => void;
  onUnpin: (p: Pin) => void;
}) {
  const strip = useRef<HTMLElement>(null);
  const [overflow, setOverflow] = useState(false);
  useEffect(() => {
    const el = strip.current;
    if (!el) return;
    const check = () => setOverflow(el.scrollWidth > el.clientWidth + 2);
    check();
    const ro = new ResizeObserver(check);
    ro.observe(el);
    return () => ro.disconnect();
  }, [pins.length]);
  const by = (dir: 1 | -1) => strip.current?.scrollBy({ left: dir * 156, behavior: "smooth" });
  return (
    <div className="mb-2">
      <section ref={strip} className="pin-strip" aria-label="Pinned verses">
        {pins.map((p, i) => (
          <PinChip key={pinKey(p.verse)} pin={p} number={i < 9 ? i + 1 : null} onGoLive={onGoLive} onUnpin={onUnpin} />
        ))}
      </section>
      {overflow && (
        <div className="mt-1 flex justify-end gap-1">
          <button type="button" className="pin-nav" onClick={() => by(-1)} aria-label="Scroll pins left" title="Earlier pins">
            ‹
          </button>
          <button type="button" className="pin-nav" onClick={() => by(1)} aria-label="Scroll pins right" title="Later pins">
            ›
          </button>
        </div>
      )}
    </div>
  );
}

/** One pin in the strip: number, reference, a small Go Live pill carrying
 *  the number (the key that sends it live), and a × to unpin. Sized to its
 *  reference and clipped, so nothing hangs outside the chip. The verse text
 *  is a tooltip: the point of a pin is that the operator knows the verse. */
function PinChip({
  pin,
  number,
  onGoLive,
  onUnpin,
}: {
  pin: Pin;
  number: number | null;
  onGoLive: (p: Pin) => void;
  onUnpin: (p: Pin) => void;
}) {
  return (
    <div className="pin-chip" title={pin.verse.text || "No cached text in this translation."}>
      <div className="flex min-w-0 items-center gap-1.5">
        {number !== null && <span className="pin-number shrink-0">{number}</span>}
        <span className="min-w-0 truncate text-[12px] font-semibold">{pin.verse.reference}</span>
        <span className="mono shrink-0 text-[10px] text-content-muted">{pin.verse.translation}</span>
      </div>
      <div className="mt-1 flex items-center justify-between gap-2">
        <button
          type="button"
          className="pin-go"
          onClick={() => onGoLive(pin)}
          title={number !== null ? `Send to the projector (key ${number})` : "Send to the projector"}
        >
          Go Live{number !== null ? ` · ${number}` : ""}
        </button>
        <button type="button" className="pin-off" onClick={() => onUnpin(pin)} title="Unpin" aria-label="Unpin">
          ×
        </button>
      </div>
    </div>
  );
}
