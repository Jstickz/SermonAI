import { useCallback, useEffect, useState } from "react";
import { audio } from "@/lib/ipc";
import type { AudioDevice, AudioDeviceKind } from "@/lib/types";

/**
 * Pick the audio input for the service (M1 deliverable 1, FR-01, FR-02, FR-05).
 *
 * The chosen name is held here for now and handed to capture when it starts.
 * It is not yet remembered across restarts — see Parked in MILESTONES.md;
 * that lands with onboarding in M8, alongside display assignments.
 */
export function AudioSettings() {
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [checking, setChecking] = useState<string | null>(null);
  const [checked, setChecked] = useState<string | null>(null);
  const [monitoring, setMonitoring] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const found = await audio.listDevices();
      setDevices(found);
      // First run: pre-select whatever the OS calls the default input, so a
      // church with one microphone never has to touch this panel.
      setSelected((current) => current ?? found.find((d) => d.isDefault)?.name ?? null);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

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
      void refresh();
    } finally {
      setChecking(null);
    }
  }

  /**
   * Open the selected input and drive the top-bar meter, so the operator can
   * see the signal arriving rather than trust the list (FR-04).
   */
  async function toggleMonitor() {
    try {
      if (monitoring) {
        await audio.stopLevelMonitor();
        setMonitoring(false);
      } else if (selected) {
        await audio.startLevelMonitor(selected);
        setMonitoring(true);
      }
      setError(null);
    } catch (e) {
      setError(String(e));
      setMonitoring(false);
    }
  }

  // Releasing the device on unmount: the operator switching tabs should not
  // leave a microphone open with a meter nobody is looking at.
  useEffect(() => {
    return () => {
      void audio.stopLevelMonitor();
    };
  }, []);

  const selectedGone = selected !== null && !devices.some((d) => d.name === selected);

  return (
    <section className="card">
      <div className="mb-5 flex flex-wrap items-start justify-between gap-x-4 gap-y-3">
        <div className="min-w-0">
          <h2 className="text-[22px] font-semibold">Audio input</h2>
          <p className="mt-1 text-xs text-content-muted">
            Choose what SermonAI listens to: a microphone, a sound desk feed, or the system audio.
          </p>
          {monitoring && (
            <p className="mt-1 text-xs text-content-secondary">
              Testing — speak or play audio and watch the meter in the top bar.
            </p>
          )}
        </div>
        <div className="flex gap-2">
          <button
            className={monitoring ? "btn-primary" : "btn-secondary"}
            disabled={selected === null || selectedGone}
            onClick={() => void toggleMonitor()}
          >
            {monitoring ? "Stop test" : "Test input"}
          </button>
          <button className="btn-secondary" onClick={() => void refresh()}>
            Rescan
          </button>
        </div>
      </div>

      {error && (
        <p className="mb-4 rounded-md bg-status-danger-bg px-3 py-2 text-status-danger">{error}</p>
      )}

      {selectedGone && (
        <p className="mb-4 rounded-md bg-status-warning-bg px-3 py-2 text-status-warning">
          The selected input is no longer connected. Reconnect it, or pick another below.
        </p>
      )}

      {devices.length === 0 ? (
        <p className="text-content-muted">
          No audio inputs detected. Connect a microphone or interface, then Rescan.
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
