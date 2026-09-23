import { useEffect, useRef, useState } from "react";
import { audio, on } from "@/lib/ipc";
import { Modal } from "./Modal";
import { useDeviceStore } from "@/stores/deviceStore";
import { FONT_SIZES, useTranscriptViewStore } from "@/stores/transcriptViewStore";
import type { AudioDeviceKind, CaptureState, LatencySummary, SttStatus } from "@/lib/types";

/** Mirrors `PARAGRAPH_GAP_SECS` in `src-tauri/src/stt/transcript.rs`. A
 *  preacher pauses for breath in well under a second and for effect in two or
 *  three; 2.5 s catches the second without breaking on the first. */
const PARAGRAPH_GAP_SECONDS = 2.5;

/**
 * What the banner says while the connection is down.
 *
 * A pure function so it can be tested against the exact JSON the backend
 * emits — see `the_status_json_matches_what_the_banner_reads` in
 * `stt/reconnect.rs`. It was inline before, and rendered "Retrying in NaNs"
 * because a field name did not match across the boundary. Nothing threw:
 * `undefined / 1000` is a number in JavaScript, so a wrong name reached the
 * operator as a plausible-looking sentence.
 *
 * Guards against a missing number rather than trusting the type, because the
 * type is exactly what was wrong.
 */
export function reconnectingMessage(
  status: { attempt: number; retryInMs: number; bufferedSeconds: number },
  droppedSeconds = 0,
): string {
  const seconds = Number.isFinite(status.retryInMs)
    ? Math.max(1, Math.round(status.retryInMs / 1000))
    : null;

  const retry = seconds === null ? "Retrying" : `Retrying in ${seconds}s`;
  const held = Number.isFinite(status.bufferedSeconds) ? Math.round(status.bufferedSeconds) : 0;
  const lost = Math.round(droppedSeconds);

  // Once the buffer is full the reassurance stops being true, and saying it
  // anyway contradicts the banner underneath reporting the loss. An operator
  // reading "it will be transcribed when the connection returns" directly
  // above "25s was not transcribed" cannot tell which to believe.
  const kept =
    lost > 0
      ? `The buffer is full: about ${lost}s of speech has already been lost, and the oldest is dropped as you keep talking. The most recent ${held}s is still held.`
      : held > 0
        ? `Still recording — ${held}s of speech held, and it will be transcribed when the connection returns.`
        : "Still recording. Nothing spoken is being lost.";

  return `Deepgram is unreachable. ${retry} (attempt ${status.attempt}). ${kept}`;
}

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
  /** Null while connected: a banner only exists when something is wrong. */
  const [stt, setStt] = useState<SttStatus | null>(null);
  /** Total speech dropped because an outage outlasted the buffer. Accumulated
   *  rather than shown per chunk, since a 90-second outage would otherwise
   *  emit a banner a hundred and twenty times. */
  const [droppedSeconds, setDroppedSeconds] = useState(0);
  /** Shown in the footer during a test run, so the DoD number is visible as it
   *  is being measured rather than only in the log afterwards. */
  const [latency, setLatency] = useState<LatencySummary | null>(null);
  /** Set when the device stops sending. Cleared by choosing another. */
  const [deviceLost, setDeviceLost] = useState<string | null>(null);
  const [capture, setCapture] = useState<CaptureState>("stopped");
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

    let unlistenAudio: (() => void) | undefined;
    void on("audio:error", ({ message }) => {
      setDeviceLost(message);
      // The list is stale the moment a device goes, so the picker in the
      // banner must not offer the one that just disappeared.
      void useDeviceStore.getState().refresh();
    }).then((fn) => {
      if (cancelled) fn();
      else unlistenAudio = fn;
    });

    let unlistenStatus: (() => void) | undefined;
    void on("stt:status", (status) => {
      if (status.kind === "audio_dropped") {
        setDroppedSeconds((previous) => previous + status.seconds);
        return;
      }
      setStt(status.kind === "connected" ? null : status);
    }).then((fn) => {
      if (cancelled) fn();
      else unlistenStatus = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
      unlistenAudio?.();
      unlistenStatus?.();
    };
  }, []);

  // The device is chosen in Settings; this reads the same cached list rather
  // than offering a second picker that could disagree with the first.
  const devices = useDeviceStore((s) => s.devices);
  const device = useDeviceStore((s) => s.selected);
  const ensureDevices = useDeviceStore((s) => s.ensure);
  const refreshDevices = useDeviceStore((s) => s.refresh);
  const selectDevice = useDeviceStore((s) => s.select);

  useEffect(() => {
    void ensureDevices();
  }, [ensureDevices]);

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
        setStt(null);
        setDroppedSeconds(0);
        setLatency(null);
        setDeviceLost(null);
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

  // Polled rather than pushed: a lag figure updated on every utterance would
  // re-render the panel as often as the transcript itself, for a number nobody
  // watches that closely.
  useEffect(() => {
    if (!listening) return;
    const timer = window.setInterval(() => {
      void audio.latency().then(setLatency).catch(() => undefined);
    }, 3000);
    return () => window.clearInterval(timer);
  }, [listening]);


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

      {/* Branding §9.3: name the thing that failed, then say what happens
          next. "Reconnecting…" alone leaves the operator's real question —
          am I losing the sermon — unanswered. */}
      {stt?.kind === "reconnecting" && (
        <p className="rounded-md bg-status-warning-bg px-3 py-2 text-[13px] text-status-warning">
          {reconnectingMessage(stt, droppedSeconds)}
        </p>
      )}

      {/* Hidden while reconnecting, because the amber banner above already
          reports the loss. Shown afterwards as the standing record that this
          transcript has a hole in it. */}
      {droppedSeconds > 0 && stt?.kind !== "reconnecting" && (
        <p className="rounded-md bg-status-danger-bg px-3 py-2 text-[13px] text-status-danger">
          The outage outlasted the buffer: about {Math.round(droppedSeconds)}s of speech was not
          transcribed. The transcript has a gap.
        </p>
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
        <span className="mono flex items-center gap-3">
          {/* Both numbers, because they mean different things: when words
              appear, and when Deepgram stops revising them. The first is what
              the DoD budgets at 700 ms; the second is what M2's detection
              will run on. */}
          {latency?.interim && (
            <span
              className={latency.interim.p99Ms > 700 ? "text-status-warning" : undefined}
              title={`Words appear: p50 ${latency.interim.p50Ms} ms · p95 ${latency.interim.p95Ms} ms · p99 ${latency.interim.p99Ms} ms · max ${latency.interim.maxMs} ms over ${latency.interim.samples} results`}
            >
              appear {latency.interim.p50Ms}/{latency.interim.p95Ms}/{latency.interim.p99Ms} ms
            </span>
          )}
          {latency?.settled && (
            <span
              title={`Confirmed: p50 ${latency.settled.p50Ms} ms · p95 ${latency.settled.p95Ms} ms · p99 ${latency.settled.p99Ms} ms · max ${latency.settled.maxMs} ms over ${latency.settled.samples} utterances. Gated by Deepgram's endpointing.`}
            >
              settle {latency.settled.p50Ms}/{latency.settled.p95Ms}/{latency.settled.p99Ms} ms
            </span>
          )}
          {latency && latency.reconnects > 0 && (
            <span
              className="text-status-warning"
              title="A reconnect replays held audio faster than real time, so its results are late by construction. Those samples are excluded."
            >
              {latency.reconnects} reconnect{latency.reconnects === 1 ? "" : "s"}
            </span>
          )}
          <span>not saved yet · M4</span>
        </span>
      </div>

      {/* A lost microphone interrupts the service, so it interrupts the
          screen. Inline in the transcript it competed with the words for
          attention and could be scrolled away from. */}
      {deviceLost && (
        <Modal
          title="The audio input stopped"
          description={deviceLost}
          // No safe default: dismissing would leave the app recording nothing
          // while looking like it was recording.
          required
          icon={
            <span className="icon-circle bg-status-warning-bg text-status-warning">
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.6"
                strokeLinecap="round"
                className="h-5 w-5"
                aria-hidden="true"
              >
                <path d="M12 8v5M12 17h.01" />
                <circle cx="12" cy="12" r="9" />
              </svg>
            </span>
          }
        >
          <div className="mt-5">
            {devices.length === 0 ? (
              <p className="text-[13px] text-content-muted">
                No other input is available. Reconnect a device and rescan.
              </p>
            ) : (
              <>
                <p className="mb-2 text-[12px] text-content-muted">
                  Choose another input to carry on. The transcript so far is kept.
                </p>
                <div className="flex flex-col gap-2">
                  {devices.map((d) => (
                    <button
                      key={`${d.kind}:${d.name}`}
                      className="flex items-center justify-between gap-3 rounded-md border-2 border-line-default bg-bg-sunken px-4 py-3 text-left transition-colors duration-base ease-brand-out hover:border-line-strong"
                      onClick={() => {
                        void selectDevice(d.name);
                        setDeviceLost(null);
                      }}
                    >
                      <span className="min-w-0 break-words text-[13px] font-medium">{d.name}</span>
                      <span className="chip">{kindLabel(d.kind)}</span>
                    </button>
                  ))}
                </div>
              </>
            )}

            <div className="mt-5 flex flex-wrap justify-end gap-2">
              <button className="btn-secondary" onClick={() => void refreshDevices()}>
                Rescan
              </button>
              {/* Stopping is the other honest way out, and it keeps the
                  transcript rather than discarding it. */}
              <button
                className="btn-secondary"
                onClick={() => {
                  setDeviceLost(null);
                  void toggle();
                }}
              >
                Stop Transcribing
              </button>
            </div>
          </div>
        </Modal>
      )}
    </section>
  );
}

function kindLabel(kind: AudioDeviceKind): string {
  switch (kind) {
    case "input":
      return "Input";
    case "loopback":
      return "System audio";
    case "virtual_input":
      return "Virtual cable";
  }
}
