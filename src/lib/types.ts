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

export interface TranscriptSegment {
  id: number;
  startTimeMs: number;
  endTimeMs: number;
  text: string;
  confidence: number | null;
  /** Interim results are replaced when the final result arrives. */
  isFinal: boolean;
}

/** What the projector and alternate windows render. Nothing reaches an
 *  output without passing through staging first (PRD §10.2, FR-50). */
export interface OutputState {
  live: Verse | null;
  staged: Verse | null;
  blanked: boolean;
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

export interface AudioDevice {
  id: string;
  name: string;
  channels: number;
  isLoopback: boolean;
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

export type PackStatus = "available" | "downloading" | "paused" | "verifying" | "installed" | "failed";

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
