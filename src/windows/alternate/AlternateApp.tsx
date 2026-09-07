import { useEffect, useState } from "react";
import { on } from "@/lib/ipc";
import type { OutputState } from "@/lib/types";

/** Stage confidence monitor: live verse large, staged verse small, clock (FR-48). */
export function AlternateApp() {
  const [state, setState] = useState<OutputState>({ live: null, staged: null, blanked: false });
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    const unlisten = on("output:changed", setState);
    const timer = setInterval(() => setNow(new Date()), 1000);
    return () => {
      void unlisten.then((fn) => fn());
      clearInterval(timer);
    };
  }, []);

  return (
    <div className="grid h-full grid-cols-[3fr_2fr] gap-5 bg-bg-canvas p-6">
      <section className="flex flex-col justify-between">
        <span className="text-[10px] font-semibold tracking-[0.16em] text-accent-500">LIVE</span>
        <div>
          <p className="font-serif text-[28px] leading-[1.35] text-content-primary">
            {state.live?.text ?? "—"}
          </p>
          <p className="mt-3 text-xs uppercase tracking-[0.12em] text-content-secondary">
            {state.live?.reference ?? ""}
          </p>
        </div>
      </section>

      <aside className="flex flex-col gap-4">
        <div className="rounded-md bg-bg-sunken p-4">
          <span className="text-[10px] uppercase tracking-[0.14em] text-content-muted">Clock</span>
          <p className="mono mt-1.5 text-2xl">{now.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</p>
        </div>
        <div className="rounded-md bg-bg-sunken p-4">
          <span className="text-[10px] uppercase tracking-[0.14em] text-content-muted">Staged</span>
          <p className="mt-1.5 font-serif text-sm leading-[1.5] text-content-secondary">
            {state.staged ? `${state.staged.reference} — ${state.staged.text}` : "Nothing staged"}
          </p>
        </div>
      </aside>
    </div>
  );
}
