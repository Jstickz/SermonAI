import { useEffect, useState } from "react";
import { audio } from "@/lib/ipc";
import { useDeviceStore } from "@/stores/deviceStore";
import type { AudioDevice, AudioDeviceKind, CaptureState } from "@/lib/types";

/**
 * Pick the audio input for the service (M1 deliverable 1, FR-01, FR-02, FR-05).
 *
 * The chosen name is held here for now and handed to capture when it starts.
 * It is not yet remembered across restarts — see Parked in MILESTONES.md;
 * that lands with onboarding in M8, alongside display assignments.
 */
export function AudioSettings() {
  // Cached in a store, so switching tabs does not re-enumerate every device
  // and flash an empty picker on the way back.
  const devices = useDeviceStore((s) => s.devices);
  const loadedAt = useDeviceStore((s) => s.loadedAt);
  const loading = useDeviceStore((s) => s.loading);
  const listError = useDeviceStore((s) => s.error);
  const ensureDevices = useDeviceStore((s) => s.ensure);
  const refreshDevices = useDeviceStore((s) => s.refresh);

  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [checking, setChecking] = useState<string | null>(null);
  const [checked, setChecked] = useState<string | null>(null);
  const [capture, setCapture] = useState<CaptureState>("stopped");

  useEffect(() => {
    void ensureDevices();
  }, [ensureDevices]);

  // Pre-select the OS default once a list exists, so a church with one
  // microphone never has to open this panel.
  useEffect(() => {
    setSelected((current) => current ?? devices.find((d) => d.isDefault)?.name ?? null);
  }, [devices]);

  async function choose(device: AudioDevice) {
    setSelected(device.name);
    setChecked(null);
    setChecking(device.name);
    try {
      // Confirm it is still there before committing: the list can be stale
      // by the time someone clicks, and it is better to hear that now than
      // when capture is supposed to begin.
      await audio.checkDevice(device.name);
      setChecked(device.name);
      setError(null);
    } catch (e) {
      setError(String(e));
      void refreshDevices();
    } finally {
      setChecking(null);
    }
  }

  /**
   * Transport for the selected input (FR-06).
   *
   * Pause holds the device rather than releasing it: reopening risks the OS
   * handing the input to another application in the gap, and some interfaces
   * allow only one capture client.
   */
  async function run(action: "start" | "stop" | "pause" | "resume") {
    try {
      const next =
        action === "start" && selected
          ? await audio.start(selected, false)
          : action === "stop"
            ? await audio.stop()
            : action === "pause"
              ? await audio.pause()
              : await audio.resume();
      setCapture(next);
      setError(null);
    } catch (e) {
      setError(String(e));
      // The backend is the authority on what is actually open; a failed
      // transition must not leave the buttons claiming otherwise.
      setCapture(await audio.state().catch(() => "stopped" as CaptureState));
    }
  }

  // The panel unmounts whenever the operator switches tabs, so what capture is
  // doing has to be asked for rather than remembered.
  useEffect(() => {
    void audio.state().then(setCapture).catch(() => undefined);
  }, []);

  // Deliberately no stop-on-unmount: capture outlives this panel. An operator
  // switching to the Library mid-sermon must not silently end the recording.

  const selectedGone = selected !== null && !devices.some((d) => d.name === selected);

  return (
    <section className="card">
      <div className="mb-5 flex flex-wrap items-start justify-between gap-x-4 gap-y-3">
        <div className="min-w-0">
          <h2 className="text-[22px] font-semibold">Audio input</h2>
          <p className="mt-1 text-xs text-content-muted">
            Choose what SermonAI listens to: a microphone, a sound desk feed, or the system audio.
          </p>
          {capture !== "stopped" && (
            <p className="mt-1 text-xs text-content-secondary">
              {capture === "running"
                ? "Testing the input — speak and watch the meter above. Not transcribing."
                : "Paused. The device is still held, so resuming is immediate."}
            </p>
          )}
        </div>
        <div className="flex flex-wrap gap-2">
          {capture === "stopped" ? (
            <button
              className="btn-primary"
              disabled={selected === null || selectedGone}
              onClick={() => void run("start")}
            >
              Start Listening
            </button>
          ) : (
            <>
              <button
                className="btn-secondary"
                onClick={() => void run(capture === "paused" ? "resume" : "pause")}
              >
                {capture === "paused" ? "Resume" : "Pause"}
              </button>
              <button className="btn-secondary" onClick={() => void run("stop")}>
                Stop
              </button>
            </>
          )}
          <button
            className="btn-secondary"
            disabled={loading}
            onClick={() => void refreshDevices()}
          >
            {loading ? (loadedAt === null ? "Scanning…" : "Rescanning…") : "Rescan"}
          </button>
        </div>
      </div>

      {(error ?? listError) && (
        <p className="mb-4 rounded-md bg-status-danger-bg px-3 py-2 text-status-danger">
          {error ?? listError}
        </p>
      )}

      {selectedGone && (
        <p className="mb-4 rounded-md bg-status-warning-bg px-3 py-2 text-status-warning">
          The selected input is no longer connected. Reconnect it, or pick another below.
        </p>
      )}

      {devices.length === 0 ? (
        <p className="text-content-muted">
          {loadedAt === null
            ? "Looking for audio inputs…"
            : "No audio inputs detected. Connect a microphone or interface, then Rescan."}
        </p>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4">
          {devices.map((device) => {
            const isSelected = selected === device.name;
            return (
              <button
                key={`${device.kind}:${device.name}`}
                onClick={() => void choose(device)}
                className={`flex flex-col rounded-md border-2 bg-bg-sunken p-4 text-left transition-colors duration-base ease-brand-out ${
                  isSelected ? "border-accent-500" : "border-line-default hover:border-line-strong"
                }`}
              >
                <div className="flex items-start justify-between gap-2">
                  <span className="min-w-0 break-words text-[13px] font-medium leading-snug">
                    {device.name}
                  </span>
                  <span className="chip">{kindLabel(device.kind)}</span>
                </div>
                <p className="mono mt-1 break-words text-[11px] text-content-muted">
                  {device.warning
                    ? device.warning
                    : `${device.defaultSampleRate.toLocaleString()} Hz · ${channelLabel(device.channels)}`}
                </p>
                <p className="mt-auto pt-3 text-[12px]">
                  {checking === device.name
                    ? "Checking input…"
                    : isSelected && checked === device.name
                      ? "Selected and ready"
                      : isSelected
                        ? "Selected"
                        : device.isDefault
                          ? "System default"
                          : " "}
                </p>
              </button>
            );
          })}
        </div>
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

function channelLabel(channels: number): string {
  if (channels === 1) return "mono";
  if (channels === 2) return "stereo";
  return `${channels} channels`;
}
