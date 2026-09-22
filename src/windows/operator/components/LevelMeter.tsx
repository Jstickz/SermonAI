import { useEffect, useRef, useState } from "react";
import { on } from "@/lib/ipc";

/** Matches `FLOOR_DBFS` in `src-tauri/src/audio/meter.rs`. */
const FLOOR_DBFS = -60;

/**
 * Where the meter changes colour.
 *
 * Three tiers, reading the way a sound desk does: green is healthy, amber
 * means the input is getting hot and the gain wants easing, red means it is
 * close enough to full scale that the next loud moment will clip and distort.
 *
 * -12 and -3 rather than a single -6: a preacher's peaks land around -12 to -6
 * on a well-set desk, so warning at -6 would leave amber showing for most of a
 * sermon and stop meaning anything. Red at -3 gives the operator a little room
 * to react before 0 dBFS, where samples are lost outright.
 */
const WARN_DBFS = -12;
const LOUD_DBFS = -3;

export type LevelTier = "off" | "on" | "warn" | "loud";

/**
 * Which colour a level reads as. Exported for its test — the thresholds decide
 * whether an operator trusts the meter, so they are worth pinning down.
 */
export function tierFor(dbfs: number): LevelTier {
  if (dbfs <= FLOOR_DBFS) return "off";
  if (dbfs >= LOUD_DBFS) return "loud";
  if (dbfs >= WARN_DBFS) return "warn";
  return "on";
}

/**
 * Bars in the strip, matching `.meter` in `docs/wireframe.html`.
 *
 * The wireframe draws seven to eight; eight divides the scale evenly, so each
 * bar is 7.5 dB of the 60 dB range.
 */
const BAR_COUNT = 8;

/**
 * How long a bar keeps a peak before falling, in milliseconds.
 *
 * The bars are a scrolling history, not a spectrum: the newest level enters at
 * the right and older ones shift left, which is what the wireframe's varied bar
 * heights depict. A still image cannot show that it moves, so the behaviour is
 * written down here.
 */
const FRAME_MS = 1000 / 30;

/**
 * Input level, shown in the live header (FR-04, wireframe.html §live-header).
 *
 * The backend emits `transcript:level` thirty times a second. That is too
 * often for React state — every frame would re-render the header and its
 * siblings — so the bars are written straight to the DOM through refs, and
 * React is only told about changes worth re-rendering for: whether a signal is
 * arriving, and the dBFS readout.
 */
export function LevelMeter() {
  const barsRef = useRef<(HTMLDivElement | null)[]>([]);
  const historyRef = useRef<number[]>(new Array(BAR_COUNT).fill(FLOOR_DBFS));
  const lastFrameAt = useRef(0);
  const [live, setLive] = useState(false);
  const [readout, setReadout] = useState<number | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    void on("transcript:level", ({ peakDbfs }) => {
      const level = Math.max(FLOOR_DBFS, Math.min(0, peakDbfs));
      lastFrameAt.current = Date.now();

      // Oldest bar falls off the left, newest enters at the right.
      const history = historyRef.current;
      history.shift();
      history.push(level);

      for (let i = 0; i < BAR_COUNT; i += 1) {
        const bar = barsRef.current[i];
        if (!bar) continue;
        // ?? FLOOR_DBFS only satisfies noUncheckedIndexedAccess; the array is
        // fixed at BAR_COUNT and shift/push keep it that length.
        const dbfs = history[i] ?? FLOOR_DBFS;
        // A floor-level bar still shows a stub, so the meter reads as present
        // and idle rather than as missing.
        const fraction = (dbfs - FLOOR_DBFS) / -FLOOR_DBFS;
        bar.style.height = `${Math.max(8, fraction * 100)}%`;
        bar.dataset.state = tierFor(dbfs);
      }

      setLive(true);
      setReadout(level);
    }).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  /**
   * Fall back to "no signal" when frames stop.
   *
   * A bar frozen at its last height reads as a live input, which is the one
   * thing a meter must never do: an operator would see level on a dead
   * microphone and trust it.
   */
  useEffect(() => {
    if (!live) return;

    const timer = window.setInterval(() => {
      if (Date.now() - lastFrameAt.current < FRAME_MS * 20) return;

      historyRef.current.fill(FLOOR_DBFS);
      for (const bar of barsRef.current) {
        if (!bar) continue;
        bar.style.height = "8%";
        bar.dataset.state = "off";
      }
      setLive(false);
      setReadout(null);
    }, 250);

    return () => window.clearInterval(timer);
  }, [live]);

  return (
    <div className="flex items-center gap-3">
      <div
        className="inline-flex h-5 items-end gap-0.5"
        role="img"
        aria-label={live && readout !== null ? `Input level ${readout.toFixed(0)} dBFS` : "No input signal"}
      >
        {Array.from({ length: BAR_COUNT }, (_, i) => (
          <div
            key={i}
            ref={(el) => {
              barsRef.current[i] = el;
            }}
            data-state="off"
            style={{ height: "8%" }}
            className="w-[3px] rounded-sm bg-line-default transition-[height] duration-fast ease-brand-out data-[state=on]:bg-status-success data-[state=warn]:bg-status-warning data-[state=loud]:bg-status-danger"
          />
        ))}
      </div>
      <span className="mono text-[11px] tabular-nums text-content-muted">
        {live && readout !== null ? `${readout.toFixed(0)} dBFS` : "no signal"}
      </span>
    </div>
  );
}
