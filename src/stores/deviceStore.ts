import { create } from "zustand";
import { audio } from "@/lib/ipc";
import type { AudioDevice } from "@/lib/types";

/**
 * The audio device list, cached across mounts.
 *
 * Enumeration is a CoreAudio or WASAPI endpoint walk, which is not free: on
 * this machine it takes long enough that Settings visibly flickered through an
 * empty state every time the tab was opened. The list changes only when
 * hardware does, so it is fetched once and kept.
 *
 * Zustand rather than component state because the Settings panel unmounts
 * whenever the operator switches tabs — the same reason the transcript lives in
 * Rust. The difference is that this is a *cache* of something the backend can
 * always re-derive, so a stale copy costs a refresh and not a recording.
 *
 * Refreshing stays manual (FR-01's "on demand"). Polling would spend the cost
 * this cache exists to avoid, and an operator who has just plugged something in
 * knows they have.
 */
interface DeviceState {
  devices: AudioDevice[];
  /**
   * The input capture uses, shared by every panel that touches it.
   *
   * It lives here because it did not, and that was a bug: the Settings panel
   * and the Live tab each kept their own choice, so picking a device in
   * Settings changed a value the running capture never read. With a lost
   * Bluetooth headset that meant choosing another input appeared to do
   * nothing while the app kept trying the device that had gone.
   */
  selected: string | null;
  /** Null until the first load, which is how "not loaded yet" is told apart
   *  from "loaded and genuinely empty" — the second needs a message about
   *  connecting a microphone, the first does not. */
  loadedAt: number | null;
  loading: boolean;
  error: string | null;

  /** Load once. Repeat calls are free while a list is held. */
  ensure: () => Promise<void>;
  /** Choose an input. Switches a running capture over to it without stopping
   *  transcription, so a lost device can be replaced mid-sermon. */
  select: (name: string) => Promise<void>;
  /** Re-enumerate, for when hardware has been plugged in. */
  refresh: () => Promise<void>;
}

export const useDeviceStore = create<DeviceState>((set, get) => ({
  devices: [],
  selected: null,
  loadedAt: null,
  loading: false,
  error: null,

  ensure: async () => {
    if (get().loadedAt !== null || get().loading) return;
    await get().refresh();
  },

  select: async (name: string) => {
    set({ selected: name, error: null });
    try {
      // Only meaningful while something is capturing; stopped, the choice is
      // simply remembered for the next start.
      if ((await audio.state()) !== "stopped") {
        await audio.switchDevice(name);
      }
    } catch (e) {
      set({ error: String(e) });
    }
  },

  refresh: async () => {
    set({ loading: true });
    try {
      const devices = await audio.listDevices();
      set((state) => ({
        devices,
        loadedAt: Date.now(),
        error: null,
        // Default to the OS default on the first load, and drop a selection
        // whose device has gone so the UI cannot show a disconnected input as
        // the chosen one.
        selected:
          state.selected && devices.some((d) => d.name === state.selected)
            ? state.selected
            : (devices.find((d) => d.isDefault)?.name ?? null),
      }));
    } catch (e) {
      // The previous list is kept on failure: a transient enumeration error
      // should not empty a picker the operator is looking at.
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }
  },
}));
