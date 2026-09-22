import { useState } from "react";
import { LevelMeter } from "./components/LevelMeter";
import { LiveTranscript } from "./components/LiveTranscript";
import { SettingsScreen } from "./components/SettingsScreen";
import { TabPanel } from "./components/TabPanel";

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

      {/* Every tab is a TabPanel: mounted on first visit and kept alive from
          then on, so leaving and returning preserves scroll, search text,
          filters and form input. See TabPanel for why that beats saving each
          view's state by hand. */}
      <TabPanel active={tab === "Live"} className="op-shell scroll-hidden min-h-0 flex-1 overflow-y-auto">
        {/* Operator shell (wireframe.html §op-shell): transcript and staging
            on the left, the detection stack on the right. */}
        <div className="op-col">
          <LiveTranscript />

          {/* Staging is a left-column panel under the transcript, not a
              right-column one — M3 (FR-50). */}
          <section className="rounded-lg bg-bg-surface p-5">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-semibold">Staging</h3>
              <span className="text-xs text-content-muted">M3</span>
            </div>
            <p className="mt-2 text-[13px] text-content-muted">
              Accepted verses wait here before they reach the projector.
            </p>
          </section>
        </div>

        <div className="op-col">
          <div className="stack-header">
            <h3>Detections</h3>
            <span className="text-xs text-content-muted">M2</span>
          </div>
          <section className="scroll-hidden min-h-0 flex-1 overflow-y-auto rounded-lg bg-bg-surface p-5">
            <p className="text-[13px] text-content-muted">
              Scripture detected in the transcript will appear here as cards.
            </p>
          </section>
        </div>
      </TabPanel>

      <TabPanel active={tab === "Library"} className="min-h-0 flex-1 overflow-auto p-5">
        <NotBuiltYet
          title="Sermon Library"
          milestone="M4"
          detail="Past services, with their transcripts and summary PDFs."
        />
      </TabPanel>

      <TabPanel active={tab === "Series"} className="min-h-0 flex-1 overflow-auto p-5">
        <NotBuiltYet
          title="Series"
          milestone="M10"
          detail="Group sermons into a series, and compile one into a book."
        />
      </TabPanel>

      <TabPanel active={tab === "Studio"} className="min-h-0 flex-1 overflow-auto p-5">
        <NotBuiltYet
          title="Content Studio"
          milestone="M9"
          detail="Edit a summary and spin off social captions, guides and devotionals."
        />
      </TabPanel>

      <TabPanel active={tab === "Settings"} className="min-h-0 flex-1 overflow-auto p-5">
        <SettingsScreen />
      </TabPanel>
    </div>
  );
}

/** A tab whose screen belongs to a later milestone. Named rather than blank,
 *  so the operator can see where something will live rather than wondering
 *  whether it is missing or broken. */
function NotBuiltYet({
  title,
  milestone,
  detail,
}: {
  title: string;
  milestone: string;
  detail: string;
}) {
  return (
    <section className="card">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-[22px] font-semibold">{title}</h2>
        <span className="chip">{milestone}</span>
      </div>
      <p className="mt-2 text-[13px] text-content-muted">{detail}</p>
      <p className="mt-1 text-[13px] text-content-muted">
        Not built yet. The tab is here so the shape of the app stays the same as it fills in.
      </p>
    </section>
  );
}
