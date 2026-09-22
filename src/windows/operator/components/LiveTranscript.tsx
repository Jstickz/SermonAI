import { useCallback, useEffect, useRef, useState } from "react";
import { audio, on } from "@/lib/ipc";
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
 * Auto-scroll with manual override, word-by-word rendering and the font size
 * setting are M1 deliverable 9. This shows the stream working.
 */
export function LiveTranscript() {
  const [finals, setFinals] = useState<string[]>([]);
  const [interim, setInterim] = useState("");
  const [capture, setCapture] = useState<CaptureState>("stopped");
  const [device, setDevice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    void on("transcript:segment", (event) => {
      switch (event.kind) {
        case "interim":
          setInterim(event.text);
          break;
        case "final":
          // The interim is cleared here, not left to the next one: between a
          // final and the next interim there is nothing in progress, and
          // leaving the old text would show a sentence that is already part
          // of the settled transcript above it.
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

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // Which device to use is chosen in Settings; this reads it rather than
  // offering a second picker that could disagree with the first.
  const loadDevice = useCallback(async () => {
    try {
      const devices = await audio.listDevices();
      setDevice((current) => current ?? devices.find((d) => d.isDefault)?.name ?? devices[0]?.name ?? null);
      setCapture(await audio.state());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void loadDevice();
  }, [loadDevice]);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [finals, interim]);

  async function toggle() {
    setBusy(true);
    try {
      if (capture === "stopped") {
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
      setCapture(await audio.state().catch(() => "stopped" as CaptureState));
    } finally {
      setBusy(false);
    }
  }

  const listening = capture !== "stopped";

  return (
    <section className="card flex min-h-0 flex-col">
      <div className="mb-4 flex flex-wrap items-start justify-between gap-x-4 gap-y-3">
        <div className="min-w-0">
          <h2 className="text-[14px] font-semibold">Live transcript</h2>
          <p className="mt-0.5 text-xs text-content-muted">
            {device ?? "No input selected"}
          </p>
        </div>
        <button
          className={listening ? "btn-secondary" : "btn-primary"}
          disabled={busy || (!listening && device === null)}
          onClick={() => void toggle()}
        >
          {listening ? "Stop transcribing" : "Start transcribing"}
        </button>
      </div>

      {error && (
        <p className="mb-3 rounded-md bg-status-danger-bg px-3 py-2 text-status-danger">{error}</p>
      )}

      <div className="min-h-0 flex-1 overflow-auto text-[15px] leading-relaxed">
        {finals.length === 0 && !interim ? (
          <p className="text-content-muted">
            {listening
              ? "Listening. Words appear as they are spoken."
              : "Nothing preached here yet. Start transcribing to see the words."}
          </p>
        ) : (
          <p>
            {finals.join(" ")}{" "}
            {/* Muted so the operator can see at a glance which words are
                still provisional and may change. */}
            {interim && <span className="text-content-muted">{interim}</span>}
          </p>
        )}
        <div ref={endRef} />
      </div>
    </section>
  );
}
