// Typed wrappers over the Tauri command and event bridge. Components never call
// `invoke` directly — every backend call goes through a function here so the
// argument and return types stay checked against the Rust side.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AudioDevice,
  Detection,
  MonitorInfo,
  OutputAssignments,
  OutputState,
  Pack,
  PackProgress,
  PackStatus,
  Sermon,
  TranscriptSegment,
  Verse,
} from "./types";

/* ---------- commands (frontend → Rust) ---------- */

export const audio = {
  listDevices: () => invoke<AudioDevice[]>("list_audio_devices"),
  select: (deviceId: string) => invoke<void>("select_audio_device", { deviceId }),
  start: () => invoke<void>("start_capture"),
  stop: () => invoke<void>("stop_capture"),
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

export const bible = {
  lookup: (reference: string, translation: string) => invoke<Verse>("lookup_verse", { reference, translation }),
  search: (query: string, translation: string) => invoke<Verse[]>("search_verses", { query, translation }),
};

export const library = {
  list: () => invoke<Sermon[]>("list_sermons"),
  downloadSummaryPdf: (sermonId: number) => invoke<string>("download_summary_pdf", { sermonId }),
  regenerateSummary: (sermonId: number) => invoke<void>("regenerate_summary", { sermonId }),
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
  "transcript:segment": TranscriptSegment;
  "transcript:level": { peakDbfs: number };
  "detection:new": Detection;
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
