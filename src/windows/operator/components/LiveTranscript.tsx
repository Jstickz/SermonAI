import { useEffect, useRef, useState } from "react";
import { audio, on } from "@/lib/ipc";
import { useDeviceStore } from "@/stores/deviceStore";
import type { CaptureState } from "@/lib/types";

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
  const [finals, setFinals] = useState<string[]>([]);
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
  const endRef = useRef<HTMLDivElement>(null);

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
        case "final":
          // The interim is cleared here rather than left to the next one:
          // between a final and the next interim nothing is in progress, and
          // the old text would duplicate what just settled above it.
          setFinals((previous) => [...previous, event.text]);
          setInterim("");
          break;
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
        setFinals((current) =>
          snapshot.finals.length >= current.length ? snapshot.finals : current,
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

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [finals, interim]);

  async function toggle() {
    // Ignored rather than disabled: greying the switch out mid-operation makes
    // it look broken exactly when the operator is waiting on it.
    if (pending !== null) return;

    const next = capture === "stopped";
    setPending(next);

    try {
      if (next) {
        if (!device) throw new Error("No audio input available. Choose one in Settings.");
        setFinals([]);
        setInterim("");
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

  const words = finals.join(" ").trim().split(/\s+/).filter(Boolean).length;

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
        <label className="flex shrink-0 cursor-pointer items-center gap-3">
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

      {error && (
        <p className="rounded-md bg-status-danger-bg px-3 py-2 text-status-danger">{error}</p>
      )}

      <div className="transcript-body">
        {finals.length === 0 && !interim ? (
          <p className="text-content-muted">
            {listening
              ? "Listening. Words appear as they are spoken."
              : "Nothing preached here yet. Start transcribing to see the words."}
          </p>
        ) : (
          <p>
            {finals.join(" ")}{" "}
            {interim && <span className="interim">{interim}</span>}
          </p>
        )}
        <div ref={endRef} />
      </div>

      <div className="flex items-center justify-between border-t border-line-subtle pt-3 text-xs text-content-muted">
        <span className="flex items-center gap-2">
          <span
            className={`inline-block h-2 w-2 rounded-pill ${
              listening ? "bg-status-success" : "bg-line-default"
            }`}
          />
          {listening ? "Autoscrolling" : "Not recording"} · {words.toLocaleString()}{" "}
          {words === 1 ? "word" : "words"}
        </span>
        {/* Persistence is M4 (FR-34); until then nothing is saved, and saying
            so beats an empty space the operator reads as "saved". */}
        <span className="mono">not saved yet · M4</span>
      </div>
    </section>
  );
}
