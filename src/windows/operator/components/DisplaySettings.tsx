import { useCallback, useEffect, useState } from "react";
import { display } from "@/lib/ipc";
import type { MonitorInfo, OutputAssignments } from "@/lib/types";

type Role = "projector" | "alternate";

const NO_OUTPUTS: OutputAssignments = { projector: null, alternate: null };

/**
 * Assign the projector and confidence-monitor outputs to displays
 * (M0 deliverable 3, FR-23, FR-24).
 *
 * Assignments live in the Rust backend, not in this component: the operator
 * switches tabs constantly during a service, which unmounts this panel. Local
 * state here would forget which screen is live the moment they looked at the
 * Library.
 */
export function DisplaySettings() {
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [outputs, setOutputs] = useState<OutputAssignments>(NO_OUTPUTS);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [attached, assigned] = await Promise.all([
        display.listMonitors(),
        display.getAssignments(),
      ]);
      setMonitors(attached);
      setOutputs(assigned);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function assign(monitor: MonitorInfo, role: Role) {
    try {
      // A display carries one output, so clear the other role from it first.
      if (role === "projector" && outputs.alternate === monitor.name) {
        setOutputs(await display.setAlternate(null));
      } else if (role === "alternate" && outputs.projector === monitor.name) {
        setOutputs(await display.setProjector(null));
      }

      const current = role === "projector" ? outputs.projector : outputs.alternate;
      const next = current === monitor.name ? null : monitor.name;

      setOutputs(role === "projector" ? await display.setProjector(next) : await display.setAlternate(next));
      setError(null);
    } catch (e) {
      setError(String(e));
      void refresh();
    }
  }

  /**
   * A role only counts if that display is still attached. After an unplug the
   * backend still holds the old name, and showing it as live would tell the
   * operator a screen is projecting when nothing is.
   */
  function roleOf(monitor: MonitorInfo): Role | null {
    if (outputs.projector === monitor.name) return "projector";
    if (outputs.alternate === monitor.name) return "alternate";
    return null;
  }

  const attachedNames = new Set(monitors.map((m) => m.name));
  const missingProjector = outputs.projector !== null && !attachedNames.has(outputs.projector);
  const missingAlternate = outputs.alternate !== null && !attachedNames.has(outputs.alternate);

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

      {(missingProjector || missingAlternate) && (
        <p className="mb-4 rounded-md bg-status-warning-bg px-3 py-2 text-status-warning">
          {missingProjector && missingAlternate
            ? "The projector and stage displays are no longer connected."
            : missingProjector
              ? "The projector display is no longer connected."
              : "The stage display is no longer connected."}{" "}
          Reconnect it, or pick another screen below.
        </p>
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
