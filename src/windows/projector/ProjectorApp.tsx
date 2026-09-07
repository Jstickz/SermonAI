import { useEffect, useState } from "react";
import { on } from "@/lib/ipc";
import type { OutputState } from "@/lib/types";

/**
 * The congregation's screen. Scripture is set in Crimson Pro on bg.canvas and
 * carries no violet except the 2px rule under the reference — nothing competes
 * with the Word (Branding §2.2, §15.2).
 */
export function ProjectorApp() {
  const [state, setState] = useState<OutputState>({ live: null, staged: null, blanked: false });

  useEffect(() => {
    const unlisten = on("output:changed", setState);
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  if (state.blanked || !state.live) {
    return <div className="h-full bg-bg-canvas" />;
  }

  return (
    <div className="flex h-full items-center justify-center bg-bg-canvas px-[8%]">
      <div className="max-w-[90%] text-center">
        <p className="font-serif text-[clamp(64px,7vw,140px)] leading-[1.2] tracking-[-0.005em] text-content-primary">
          {state.live.text}
        </p>
        <p className="mt-8 text-[22px] uppercase tracking-[0.16em] text-content-secondary">
          {state.live.reference}
        </p>
        <span className="mx-auto mt-3 block h-0.5 w-20 bg-accent-500" />
      </div>
    </div>
  );
}
