import { useCallback, useEffect, useState } from "react";
import { display } from "@/lib/ipc";
import type { MonitorInfo } from "@/lib/types";

type Assignment = "projector" | "alternate" | null;

/**
 * Assign the projector and confidence-monitor outputs to displays
 * (M0 deliverable 3, FR-23, FR-24).
 *
 * Displays are re-read whenever the operator asks, so a monitor plugged in
 * after launch shows up without restarting the app.
 */
export function DisplaySettings() {
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [projector, setProjector] = useState<string | null>(null);
  const [alternate, setAlternate] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setMonitors(await display.listMonitors());
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function assign(monitor: MonitorInfo, role: Exclude<Assignment, null>) {
    try {
      // A display can only carry one output, so clear the other role first.
      if (role === "projector") {
        if (alternate === monitor.name) {
          await display.setAlternate(null);
          setAlternate(null);
        }
        const next = projector === monitor.name ? null : monitor.name;
        await display.setProjector(next);
        setProjector(next);
      } else {
        if (projector === monitor.name) {
          await display.setProjector(null);
          setProjector(null);
        }
        const next = alternate === monitor.name ? null : monitor.name;
        await display.setAlternate(next);
        setAlternate(next);
      }
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  function roleOf(monitor: MonitorInfo): Assignment {
    if (projector === monitor.name) return "projector";
    if (alternate === monitor.name) return "alternate";
    return null;
  }

  return (
    <section className="card">
      <div className="mb-5 flex items-start justify-between gap-4">
        <div>
          <h2 className="text-[22px] font-semibold">Displays</h2>
          <p className="mt-1 text-xs text-content-muted">
            Choose which screen the congregation sees and which the stage sees.
          </p>
        </div>
        <button className="btn-secondary" onClick={() => void refresh()}>
          Rescan
        </button>
      </div>

      {error && (
        <p className="mb-4 rounded-md bg-status-danger-bg px-3 py-2 text-status-danger">{error}</p>
      )}

      {monitors.length === 0 ? (
        <p className="text-content-muted">
          No displays detected. Connect a projector or second screen, then Rescan.
        </p>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {monitors.map((monitor) => {
            const role = roleOf(monitor);
            return (
              <div
                key={monitor.name}
                className={`rounded-md border-2 bg-bg-sunken p-4 transition-colors duration-base ease-brand-out ${
                  role ? "border-accent-500" : "border-line-default"
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="text-[13px] font-medium">{monitor.name}</span>
                  {monitor.isPrimary && <span className="chip">Primary</span>}
                </div>
                <p className="mono mt-1 text-[11px] text-content-muted">
                  {monitor.width} × {monitor.height} @ {monitor.scaleFactor}x
                </p>

                <div className="mt-4 flex gap-2">
                  <button
                    className={role === "projector" ? "btn-primary !px-4 !py-2" : "btn-secondary !px-4 !py-2"}
                    onClick={() => void assign(monitor, "projector")}
                  >
                    {role === "projector" ? "Projecting" : "Use as projector"}
                  </button>
                  <button
                    className={role === "alternate" ? "btn-primary !px-4 !py-2" : "btn-secondary !px-4 !py-2"}
                    onClick={() => void assign(monitor, "alternate")}
                  >
                    {role === "alternate" ? "On stage" : "Use for stage"}
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
