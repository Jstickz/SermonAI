import { useState } from "react";
import { LevelMeter } from "./components/LevelMeter";
import { LiveTranscript } from "./components/LiveTranscript";
import { SettingsScreen } from "./components/SettingsScreen";

// M0 shell only. The panels below are filled in per milestone:
// transcript M1, detection cards M2, staging M3, library M4, packs M0/M5.
const TABS = ["Live", "Library", "Series", "Studio", "Settings"] as const;
type Tab = (typeof TABS)[number];

export function OperatorApp() {
  const [tab, setTab] = useState<Tab>("Live");

  return (
    <div className="flex h-full flex-col bg-bg-canvas">
      <header className="flex items-center gap-4 border-b border-line-subtle bg-bg-surface px-5 py-3">
        <span className="font-display font-semibold tracking-[-0.01em]">SermonAI</span>

        <nav className="inline-flex gap-0.5 rounded-pill bg-bg-pill p-1">
          {TABS.map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`rounded-pill px-3.5 py-1.5 text-[13px] transition-colors duration-base ease-brand-out ${
                tab === t ? "bg-bg-raised text-content-primary" : "text-content-secondary hover:text-content-primary"
              }`}
            >
              {t}
            </button>
          ))}
        </nav>

        <div className="flex-1" />
        <span className="chip">Not in service</span>
      </header>

      {/* Status strip (wireframe.html §live-header). The recording state and
          service health that belong here arrive with the service lifecycle in
          M1 and the STT and detection stages in M1/M2; the input meter is
          real now, so the strip carries it and says what is not yet wired. */}
      <div className="live-header">
        <span className="shrink-0 text-content-muted">Not recording</span>
        <div className="status-cluster">
          <span>Deepgram — M1</span>
          <span>Scripture cache — M2</span>
          <span>Claude — M4</span>
        </div>
        <div className="flex-1" />
        <LevelMeter />
      </div>

      {tab === "Settings" ? (
        <main className="min-h-0 flex-1 overflow-auto p-5">
          <SettingsScreen />
        </main>
      ) : (
        /* Two columns: transcript 55%, cards + staging + preview 45% (PRD §16.2). */
        <main className="grid min-h-0 flex-1 grid-cols-1 gap-5 p-5 xl:grid-cols-[55fr_45fr]">
          <LiveTranscript />

          <section className="flex min-h-0 flex-col gap-4">
            <div className="card flex-1">
              <h2 className="text-[13px] text-content-muted">Detections</h2>
            </div>
            <div className="card">
              <h2 className="text-[13px] text-content-muted">Staging</h2>
            </div>
          </section>
        </main>
      )}
    </div>
  );
}
