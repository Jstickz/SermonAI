// Typed wrappers over the Tauri command and event bridge. Components never call
// `invoke` directly — every backend call goes through a function here so the
// argument and return types stay checked against the Rust side.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AudioDevice,
  CaptureState,
  BibleVersion,
  Detection,
  MonitorInfo,
  OutputAssignments,
  OutputState,
  Pack,
  PackProgress,
  PackStatus,
  Sermon,
  TranscriptEvent,
  LatencySummary,
  SttStatus,
  ServiceCredential,
  ServiceId,
  TestOutcome,
  TranscriptSnapshot,
  Verse,
} from "./types";

/* ---------- commands (frontend → Rust) ---------- */

export const audio = {
  /** Every source the operator can pick (FR-01, FR-05). Safe to call again
   *  on demand: devices appear and disappear while the app runs. */
  listDevices: () => invoke<AudioDevice[]>("list_audio_devices"),
  /** Confirm a remembered device is still present (FR-02). Worth calling
   *  before capture starts, so a stale choice is caught while there is still
   *  time to pick another rather than at the moment recording should begin. */
  checkDevice: (name: string) => invoke<void>("check_audio_device", { name }),

  /* Transport (FR-06). Starting again replaces any running capture, so two
     inputs are never open at once. */
  /** `transcribe` is explicit: a level check costs nothing, but streaming to
   *  Deepgram is billed by the minute, so a device test must not quietly open
   *  a paid connection. */
  start: (deviceName: string, transcribe: boolean) =>
    invoke<CaptureState>("start_capture", { deviceName, transcribe }),
  /** Releases the device, and delivers the final partial chunk rather than
   *  dropping up to 250 ms of the end of a service. */
  stop: () => invoke<CaptureState>("stop_capture"),
  /** Move a running capture to another device without stopping transcription:
   *  the transcript, the connection and the timeline all survive. */
  switchDevice: (deviceName: string) =>
    invoke<CaptureState>("switch_capture_device", { deviceName }),
  /** Stops audio without releasing the device: reopening risks losing the
   *  input to another application on interfaces that allow one client. */
  pause: () => invoke<CaptureState>("pause_capture"),
  resume: () => invoke<CaptureState>("resume_capture"),
  /** For a panel that has just mounted and cannot know what is running. */
  state: () => invoke<CaptureState>("capture_state"),
  /** Everything transcribed so far. The backend owns this, so it is whole
   *  even when the panel has been unmounted for half the sermon. */
  transcript: () => invoke<TranscriptSnapshot>("transcript_snapshot"),
  /** The last 60 seconds of settled speech (FR-11), for M2's paraphrase stage. */
  rollingTranscript: () => invoke<string>("transcript_rolling"),
  /** End-to-end lag so far. Null until something has settled. */
  latency: () => invoke<LatencySummary>("transcript_latency"),
};

export const display = {
  listMonitors: () => invoke<MonitorInfo[]>("list_monitors"),
  /** The backend owns this: the Settings panel unmounts on tab switch. */
  getAssignments: () => invoke<OutputAssignments>("get_output_assignments"),
  /** Pass null to hide the output again. Returns the updated assignments. */
  setProjector: (monitorName: string | null) =>
    invoke<OutputAssignments>("set_projector_monitor", { monitorName }),
  setAlternate: (monitorName: string | null) =>
    invoke<OutputAssignments>("set_alternate_monitor", { monitorName }),
  /**
   * The operator window has painted, at this wall-clock reading (Date.now()).
   * The backend logs the cold start from it (PRD §9.1).
   */
  operatorReady: (paintedAtMs: number) =>
    invoke<void>("operator_window_ready", { paintedAtMs }),
};

export const bible = {
  /** True when a YouVersion app key is configured. */
  isOnline: () => invoke<boolean>("is_bible_online"),
  /** Opens the YouVersion portal so the operator can accept licence terms. */
  openLicensePortal: () => invoke<void>("open_license_portal"),
  /** Re-reads which versions this app key may use, after a portal visit. */
  refreshLicenses: () => invoke<BibleVersion[]>("refresh_bible_licenses"),

  /**
   * Look a passage up by USFM ID, e.g. "JHN.3.16" or "PSA.139.13-16".
   *
   * Passages are addressed by USFM now rather than by free-text reference:
   * the detection stages convert what the preacher said before it gets here
   * (PRD v2.2 §15.2). Cache first, YouVersion only on a miss. (M2)
   */
  lookup: (passageId: string, versionId: number) =>
    invoke<Verse>("lookup_passage", { passageId, versionId }),
  search: (query: string, versionId: number) =>
    invoke<Verse[]>("search_verses", { query, versionId }),
};

export const service = {
  start: (meta: { preacher: string; title?: string; seriesId?: number; translation: string }) =>
    invoke<number>("start_service", { meta }),
  end: (sermonId: number) => invoke<void>("end_service", { sermonId }),
};

export const output = {
  stage: (verse: Verse) => invoke<void>("stage_verse", { verse }),
  goLive: () => invoke<void>("go_live"),
  clearStaged: () => invoke<void>("clear_staged"),
  setBlanked: (blanked: boolean) => invoke<void>("set_blanked", { blanked }),
  step: (delta: number) => invoke<void>("step_verse", { delta }),
  get: () => invoke<OutputState>("get_output_state"),
};

export const library = {
  list: () => invoke<Sermon[]>("list_sermons"),
  downloadSummaryPdf: (sermonId: number) => invoke<string>("download_summary_pdf", { sermonId }),
  regenerateSummary: (sermonId: number) => invoke<void>("regenerate_summary", { sermonId }),
};

export const credentials = {
  /** Every service and its current state. Cheap; safe on every panel mount. */
  list: () => invoke<ServiceCredential[]>("list_service_credentials"),

  /** Store a key the operator pasted. Returns the refreshed listing, so the
   *  panel cannot show a stale row after a save — and a save that lands but
   *  reads back missing, which is what a failing credential store looks like,
   *  shows up at once rather than after a restart. */
  setKey: (service: ServiceId, key: string) =>
    invoke<ServiceCredential[]>("set_service_key", { service, key }),

  remove: (service: ServiceId) =>
    invoke<ServiceCredential[]>("remove_service_key", { service }),

  /** The cheapest real call this vendor offers. Resolves with ok:false for a
   *  key the vendor rejected, and rejects only when the test could not run —
   *  the operator's problem versus ours. */
  test: (service: ServiceId) => invoke<TestOutcome>("test_service_key", { service }),
};

export const detections = {
  /** The operator accepted a card. Staging is M3; today this is recorded. */
  accept: (id: number, passageId: string) => invoke<void>("accept_detection", { id, passageId }),
  reject: (id: number, passageId: string) => invoke<void>("reject_detection", { id, passageId }),
  /** Correct a card's reference. It comes back under the same id as a manual
   *  detection with the new verse text, and the store replaces it in place. */
  edit: (id: number, reference: string) => invoke<Detection>("edit_detection", { id, reference }),
};

export const packs = {
  list: () => invoke<Pack[]>("list_packs"),
  /** Re-read the catalog from the Pack CDN, then list. */
  refresh: () => invoke<Pack[]>("refresh_pack_catalog"),
  download: (packId: string) => invoke<PackStatus>("download_pack", { packId }),
  pause: (packId: string) => invoke<void>("pause_pack_download", { packId }),
  remove: (packId: string) => invoke<void>("remove_pack", { packId }),
};

/* ---------- events (Rust → frontend) ---------- */

interface EventMap {
  "transcript:segment": TranscriptEvent;
  /** Peak since the last frame, in dBFS: 0 is full scale, -60 the floor. */
  "transcript:level": { peakDbfs: number };
  "audio:error": { message: string };
  "stt:status": SttStatus;
  "detection:new": Detection;
  /** A provisional candidate the settled text did not support. Remove the
   *  card with this id, quietly. */
  "detection:withdraw": { id: number };
  "output:changed": OutputState;
  "summary:progress": { sermonId: number; step: string; percent: number };
  "pack:progress": PackProgress;
}

export function on<K extends keyof EventMap>(
  event: K,
  handler: (payload: EventMap[K]) => void,
): Promise<UnlistenFn> {
  return listen<EventMap[K]>(event, (e) => handler(e.payload));
}
