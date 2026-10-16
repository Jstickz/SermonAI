import { useState } from "react";
import { output } from "@/lib/ipc";
import { useServiceStore } from "@/stores/serviceStore";

/**
 * The auto-live switch (FR-50, PRD §13.5, M3 deliverable 2). Off by default:
 * the operator stays in control. On, a confirmed card whose confidence is
 * above 90% skips the staged slot and goes straight to the projector; the
 * backend holds that rule (`output/staging.rs`) and this only flips it.
 *
 * The state is the backend's, read from the service store, so the two
 * places the switch appears — the Staging panel and Settings → General —
 * and the header indicator never disagree. The label does not change with
 * the state: the switch says its own state (see the Transcribe switch).
 */
export function AutoLiveSwitch({ compact = false }: { compact?: boolean }) {
  const enabled = useServiceStore((s) => s.outputs.autoLive);
  const setOutputs = useServiceStore((s) => s.setOutputs);
  const [error, setError] = useState<string | null>(null);

  async function toggle() {
    try {
      setOutputs(await output.setAutoLive(!enabled));
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="flex flex-col items-end gap-1">
      <label className="flex min-h-10 cursor-pointer items-center gap-3">
        <span className={compact ? "text-xs text-content-muted" : "text-[13px] font-medium"}>
          Auto-live
        </span>
        <button
          type="button"
          role="switch"
          aria-checked={enabled}
          className="switch"
          onClick={() => void toggle()}
        />
      </label>
      {error && <p className="text-xs text-status-danger">{error}</p>}
    </div>
  );
}
