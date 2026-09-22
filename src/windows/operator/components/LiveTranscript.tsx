import { useEffect, useRef, useState } from "react";
import { audio, on } from "@/lib/ipc";
import { useDeviceStore } from "@/stores/deviceStore";
import { FONT_SIZES, useTranscriptViewStore } from "@/stores/transcriptViewStore";
import type { CaptureState } from "@/lib/types";

/** Mirrors `PARAGRAPH_GAP_SECS` in `src-tauri/src/stt/transcript.rs`. A
 *  preacher pauses for breath in well under a second and for effect in two or
 *  three; 2.5 s catches the second without breaking on the first. */
const PARAGRAPH_GAP_SECONDS = 2.5;

/**
 * The live transcript (FR-07, FR-10).
 *
 * Settled text accumulates; the sentence in progress is held separately and
 * **replaced** on each interim result rather than appended. Deepgram revises
 * its guess as it hears more — "...if you will to join" became "...to John
 * chapter three" one result later in testing — so appending would print the
 * same growing half-sentence five times over.
 *
 * The transcript itself is **not held here**. It lives in the Rust backend
 * (`stt::transcript`), because this component unmounts whenever the operator
 * switches tabs while capture keeps running. This hydrates from the backend on
 * mount, so returning mid-sermon shows the whole service rather than an empty
 * panel that reads as a crash.
 *
 * Auto-scroll with manual override, word-by-word rendering and the font size
 * setting are M1 deliverable 9.
 */
export function LiveTranscript() {
  /**
   * Settled text, grouped into paragraphs by the pauses between utterances.
   *
   * A 45-minute sermon as one unbroken block is unreadable, and the timings
   * needed to break it are already on every word.
   */
  const [paragraphs, setParagraphs] = useState<string[]>([]);
  /** End of the newest utterance, for deciding where the next one belongs. */
  const lastEnd = useRef<number | null>(null);
  const [interim, setInterim] = useState("");
  const [capture, setCapture] = useState<CaptureState>("stopped");
  const [device, setDevice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  /**
   * Where the switch is shown while the backend catches up.
   *
   * Starting opens a WebSocket to Deepgram before it returns, and stopping
   * waits for Deepgram to drain the final results of the last utterance —
   * together a good fraction of a second. Driving the switch straight from
   * backend state meant it sat still through all of that, which on a switch
   * reads as a dead control rather than a busy one. So it moves at once and
   * snaps back if the operation fails.
   */
  const [pending, setPending] = useState<boolean | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    // Subscribed before hydrating, so an event arriving during the round trip
    // is not missed. The hydrate below only overwrites when it has at least as
    // much as is already on screen, which keeps such an event from being
    // clobbered by a snapshot taken a moment earlier.
    void on("transcript:segment", (event) => {
      switch (event.kind) {
        case "interim":
          setInterim(event.text);
          break;
        case "final": {
          // Mirrors PARAGRAPH_GAP_SECS in stt/transcript.rs, applied here so a
          // sentence lands in the right paragraph the moment it arrives rather
          // than after the next snapshot.
          const start = event.words[0]?.start ?? null;
          const breaks =
            lastEnd.current !== null &&
            start !== null &&
            start - lastEnd.current >= PARAGRAPH_GAP_SECONDS;

          setParagraphs((previous) => {
            if (previous.length === 0 || breaks) return [...previous, event.text];
            const next = [...previous];
            next[next.length - 1] = `${next[next.length - 1]} ${event.text}`;
            return next;
          });

          lastEnd.current = event.words[event.words.length - 1]?.end ?? lastEnd.current;
          // The interim is cleared here rather than left to the next one:
          // between a final and the next interim nothing is in progress, and
          // the old text would duplicate what just settled above it.
          setInterim("");
          break;
        }
        case "closed":
          setInterim("");
          break;
      }
    }).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });

    void audio
      .transcript()
      .then((snapshot) => {
        if (cancelled) return;
        setParagraphs((current) =>
          snapshot.paragraphs.length >= current.length ? snapshot.paragraphs : current,
        );
        setInterim((current) => (current === "" ? snapshot.interim : current));
      })
      .catch(() => undefined);

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // The device is chosen in Settings; this reads the same cached list rather
  // than offering a second picker that could disagree with the first.
  const devices = useDeviceStore((s) => s.devices);
  const ensureDevices = useDeviceStore((s) => s.ensure);

  useEffect(() => {
    void ensureDevices();
  }, [ensureDevices]);

  useEffect(() => {
    setDevice((current) => current ?? devices.find((d) => d.isDefault)?.name ?? devices[0]?.name ?? null);
  }, [devices]);

  useEffect(() => {
    void audio.state().then(setCapture).catch(() => undefined);
  }, []);

  /**
   * Auto-scroll, until the operator scrolls away (PRD §13.2).
   *
   * Following the newest words is right while the operator is watching the
   * service, and wrong the moment they scroll back to read something: yanking
   * them to the bottom every two seconds makes the transcript unreadable
   * exactly when they are trying to read it. So scrolling up switches
   * auto-scroll off, and returning to the bottom switches it back on — no
   * setting to find, and the way back is the gesture they already made.
   */
  const bodyRef = useRef<HTMLDivElement>(null);
  const [following, setFollowing] = useState(true);

  function onBodyScroll(e: React.UIEvent<HTMLDivElement>) {
    const el = e.currentTarget;
    // A few pixels of slack: sub-pixel layout and smooth scrolling rarely
    // land exactly on the bottom, and a meter that never quite re-latches
    // would be worse than one that latches slightly early.
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
    setFollowing(atBottom);
  }

  useEffect(() => {
    if (!following) return;
    bodyRef.current?.scrollTo({ top: bodyRef.current.scrollHeight });
  }, [paragraphs, interim, following]);

  async function toggle() {
    // Ignored rather than disabled: greying the switch out mid-operation makes
    // it look broken exactly when the operator is waiting on it.
    if (pending !== null) return;

    const next = capture === "stopped";
    setPending(next);

    try {
      if (next) {
        if (!device) throw new Error("No audio input available. Choose one in Settings.");
        setParagraphs([]);
        setInterim("");
        lastEnd.current = null;
        setCapture(await audio.start(device, true));
      } else {
        setCapture(await audio.stop());
      }
      setError(null);
    } catch (e) {
      setError(String(e));
      // The backend is the authority. Clearing the optimistic position lets
      // the switch snap back to what actually happened, rather than showing a
      // service that is not running.
      setCapture(await audio.state().catch(() => "stopped" as CaptureState));
    } finally {
      setPending(null);
    }
  }

  const running = capture !== "stopped";
  // The optimistic position wins while an operation is in flight.
  const listening = pending ?? running;

  const fontSize = useTranscriptViewStore((s) => s.fontSize);
  const cycleFontSize = useTranscriptViewStore((s) => s.cycleFontSize);
  const words = paragraphs.join(" ").trim().split(/\s+/).filter(Boolean).length;

  return (
    <section className="transcript-panel">
      <div className="transcript-header">
        <div className="min-w-0">
          <div className="text-sm font-semibold">Live transcript</div>
          {/* Preacher and translation join this line once service metadata
              exists (M4) and the translation picker lands (M2). */}
          <div className="mt-0.5 truncate text-xs text-content-muted">
            {/* Moving the switch optimistically hides the wait; saying what
                the wait is for explains it. Branding §9.3: a loading verb
                always takes an object, never a bare "Loading…". */}
            {pending === true
              ? "Connecting to Deepgram…"
              : pending === false
                ? "Finishing the last sentence…"
                : (device ?? "No input selected")}
          </div>
        </div>
        {/* A switch rather than a button: transcription is a state that is on
            or off, and the control should show which without the operator
            having to read a verb and work out whether it describes what is
            happening now or what pressing it would do. */}
        <div className="flex shrink-0 items-center gap-3">
          {/* The wireframe's "Aa" control (§transcript-header). Cycles three
              steps rather than opening a menu: a booth volunteer wants bigger
              or smaller, not a value to tune. */}
          <button
            className="btn-icon"
            onClick={cycleFontSize}
            title={`Text size: ${FONT_SIZES[fontSize].label}`}
            aria-label={`Text size: ${FONT_SIZES[fontSize].label}. Click to change.`}
          >
            <span className="font-semibold">Aa</span>
          </button>

          <label className="flex cursor-pointer items-center gap-3">
          {/* The label stays "Transcribe" whether it is on or off. A switch
              conveys its own state through position and colour; swapping the
              word to "Transcribing" would say it twice, and would make the
              control's accessible name change under a screen reader every
              time it was used. No aria-label here for the same reason — the
              visible text is the name. */}
          <span className="text-[13px] font-medium">Transcribe</span>
          <button
            type="button"
            role="switch"
            aria-checked={listening}
            className="switch"
            disabled={pending === null && !running && device === null}
            onClick={() => void toggle()}
          />
          </label>
        </div>
      </div>

      {error && (
        <p className="rounded-md bg-status-danger-bg px-3 py-2 text-status-danger">{error}</p>
      )}

      <div
        ref={bodyRef}
        onScroll={onBodyScroll}
        className={`transcript-body scroll-hidden ${FONT_SIZES[fontSize].className}`}
        // Focusable so the transcript can still be scrolled from the keyboard.
        // With the bar hidden this is the only way to reach it without a mouse,
        // and a booth is often driven by keyboard alone.
        tabIndex={0}
        aria-label="Live transcript"
      >
        {paragraphs.length === 0 && !interim ? (
          <p className="text-content-muted">
            {listening
              ? "Listening. Words appear as they are spoken."
              : "Nothing preached here yet. Turn on Transcribe to see the words."}
          </p>
        ) : (
          <>
            {paragraphs.map((paragraph, i) => (
              <p key={i} className={i === 0 ? undefined : "mt-3"}>
                {paragraph}
                {/* The in-progress sentence continues the last paragraph
                    rather than starting its own, so it does not jump down a
                    line and back up the moment it settles. */}
                {i === paragraphs.length - 1 && interim && (
                  <span className="interim"> {interim}</span>
                )}
              </p>
            ))}
            {paragraphs.length === 0 && interim && (
              <p>
                <span className="interim">{interim}</span>
              </p>
            )}
          </>
        )}
      </div>

      {!following && (
        <button
          className="btn-secondary self-center !py-2 text-[12px]"
          onClick={() => {
            setFollowing(true);
            bodyRef.current?.scrollTo({
              top: bodyRef.current.scrollHeight,
              behavior: "smooth",
            });
          }}
        >
          Jump to Latest
        </button>
      )}

      <div className="flex items-center justify-between border-t border-line-subtle pt-3 text-xs text-content-muted">
        <span className="flex items-center gap-2">
          <span
            className={`inline-block h-2 w-2 rounded-pill ${
              listening ? "bg-status-success" : "bg-line-default"
            }`}
          />
          {!listening ? "Not recording" : following ? "Autoscrolling" : "Scrolled back"} ·{" "}
          {words.toLocaleString()}{" "}
          {words === 1 ? "word" : "words"}
        </span>
        {/* Persistence is M4 (FR-34); until then nothing is saved, and saying
            so beats an empty space the operator reads as "saved". */}
        <span className="mono">not saved yet · M4</span>
      </div>
    </section>
  );
}
