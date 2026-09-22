// Shared frontend types. These mirror the Rust structs in src-tauri/src/db/models.rs
// and the data model in PRD §14.2. Keep both sides in step.

export type SermonStatus = "live" | "processing" | "ready" | "needs_attention";

export type DetectionSource = "regex" | "vector" | "llm" | "manual";

export type SummaryTemplate = "sermon_brief" | "study_guide" | "devotional" | "minimal";

export type SummaryGenerator = "claude" | "local" | "template";

export interface ScriptureRef {
  book: string;
  chapter: number;
  verse: number;
  endVerse?: number;
}

export interface Verse {
  reference: string; // "John 3:16"
  translation: string; // "KJV"
  text: string;
}

/** A candidate scripture surfaced by the detection pipeline (PRD §13.3). */
export interface Detection {
  id: string;
  reference: string;
  verse: Verse;
  confidence: number; // 0..1
  source: DetectionSource;
  detectedAtMs: number;
}

/**
 * A persisted transcript row (PRD §14.2 `transcript_segments`, FR-34).
 *
 * Distinct from `TranscriptEvent`, which is what arrives from the
 * transcription stream. This is what M4 writes to SQLite and the Library
 * reads back: it has an id and millisecond offsets into the service, where an
 * event has neither and is gone once handled.
 */
export interface TranscriptSegment {
  id: number;
  startTimeMs: number;
  endTimeMs: number;
  text: string;
  confidence: number | null;
  /** Interim rows are replaced when the final arrives. */
  isFinal: boolean;
}

/** Everything transcribed so far (FR-10). Mirrors `TranscriptSnapshot` in
 *  `src-tauri/src/stt/transcript.rs`. */
export interface TranscriptSnapshot {
  /** Settled utterances grouped into paragraphs by the pauses between them.
   *  A 45-minute sermon as one unbroken block is unreadable. */
  paragraphs: string[];
  finals: string[];
  /** The utterance in progress; the next interim replaces it. */
  interim: string;
  wordCount: number;
}

/** One word with where it falls in the service (FR-10).
 *  Mirrors `Word` in `src-tauri/src/stt/deepgram.rs`. */
export interface TranscriptWord {
  /** Punctuated and capitalised where Deepgram supplies that form. */
  word: string;
  /** Seconds from the start of the stream. */
  start: number;
  end: number;
  confidence: number;
}

/**
 * What the transcription stream produced (FR-07, FR-10).
 *
 * Mirrors `TranscriptEvent` in `src-tauri/src/stt/deepgram.rs`, and is a
 * tagged union rather than a flag for a reason worth repeating here:
 * an `interim` **replaces** the previous interim, it does not follow it.
 * Deepgram revises its guess as it hears more — in testing
 * "...if you will to join" became "...to John chapter three" one result
 * later. Appending each one prints the same growing half-sentence over and
 * over, and acting on one can put a verse on the projector that the next
 * result takes back.
 */
export type TranscriptEvent =
  | { kind: "interim"; text: string; words: TranscriptWord[] }
  | { kind: "final"; text: string; words: TranscriptWord[]; speechFinal: boolean }
  | { kind: "closed"; reason: string | null };

/** What the projector and alternate windows render. Nothing reaches an
 *  output without passing through staging first (PRD §10.2, FR-50). */
export interface OutputState {
  live: Verse | null;
  staged: Verse | null;
  blanked: boolean;
}

/** Which display each output window is currently on. Null means hidden. */
export interface OutputAssignments {
  projector: string | null;
  alternate: string | null;
}

/** A display the operator can send the projector or alternate output to. */
export interface MonitorInfo {
  name: string;
  width: number;
  height: number;
  x: number;
  y: number;
  scaleFactor: number;
  isPrimary: boolean;
}

/** How a source is captured, which is not always what it is called.
 *  - `input` — a microphone, USB interface or HDMI capture card
 *  - `loopback` — a system output captured back. Windows only: CoreAudio
 *    cannot record a render endpoint, so macOS never reports one.
 *  - `virtual_input` — a virtual cable (BlackHole, VB-Audio) presenting as an
 *    ordinary input. Recognised by name, so the label is best-effort; an
 *    unrecognised cable still captures, it is just shown as `input`. */
/** Mirrors `CaptureState` in `src-tauri/src/audio/capture.rs` (FR-06).
 *  `paused` holds the device open — see `audio.pause` in `ipc.ts`. */
export type CaptureState = "stopped" | "running" | "paused";

export type AudioDeviceKind = "input" | "loopback" | "virtual_input";

export interface AudioDevice {
  /** The OS name, and the handle used to select the device again (FR-02).
   *  There is no id: cpal exposes no stable device identifier, and index
   *  order shifts as devices come and go, so the name is what survives a
   *  restart. Two devices can therefore share a name — paired capture cards
   *  do — and the first match wins. */
  name: string;
  kind: AudioDeviceKind;
  /** The host default, pre-selected on first run. At most one is true. */
  isDefault: boolean;
  /** The rate the device prefers, usually 44100 or 48000. Capture at 16 kHz
   *  (FR-03) resamples from this. Zero when `warning` is set. */
  defaultSampleRate: number;
  channels: number;
  /** Why the device could not be interrogated. It is still listed and still
   *  selectable — a device that will not describe itself is usually busy or
   *  asleep, and both resolve by the time someone picks it. */
  warning: string | null;
}

export interface Sermon {
  id: number;
  title: string | null;
  preacher: string | null;
  date: string;
  durationSeconds: number | null;
  defaultTranslation: string;
  seriesId: number | null;
  status: SermonStatus;
  scriptureCount: number;
}

/** The structured summary produced by the pipeline (PRD §26.3). The PDF
 *  renderer and Content Studio both read this shape. */
export interface SermonSummary {
  title: string;
  theme: string;
  overview: string;
  takeaways: string[];
  outline: {
    introduction: string;
    points: { heading: string; subpoints: string[]; scriptures: string[] }[];
    conclusion: string;
  };
  scriptures: { reference: string; translation: string; text: string; time_ms: number }[];
  quotes: { text: string; time_ms: number }[];
  applications: string[];
  discussion_questions: string[];
  prayer_points: string[];
  closing_summary: string;
}

export type PackKind = "speech" | "translation" | "intelligence" | "theme";

export type PackStatus =
  | "available"
  /** In the catalog, but YouVersion has not approved our app key for it. */
  | "license_required"
  | "downloading"
  | "paused"
  | "verifying"
  | "installed"
  | "failed";

/** A Bible version as YouVersion Platform reports it. */
export interface BibleVersion {
  id: number;
  name: string;
  shortName: string;
  language: string;
  /** Copyright string. Must be shown wherever the text is shown. */
  attribution: string;
  licenseStatus: "pending" | "approved" | "revoked";
}

export interface Pack {
  id: string;
  kind: PackKind;
  name: string;
  description: string;
  version: string;
  sizeBytes: number;
  sha256: string;
  status: PackStatus;
  /** 0..1 while downloading; resumable across restarts (FR-61). */
  progress: number;
  /** Bytes fetched so far, including a partial download from an earlier run. */
  bytesOnDisk: number;
  optional: boolean;
  tier: string | null;
}

/** Payload of the `pack:progress` event. */
export interface PackProgress {
  packId: string;
  status: PackStatus;
  progress: number;
  bytesOnDisk: number;
  sizeBytes: number;
}
