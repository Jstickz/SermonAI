import { LevelMeter } from "./components/LevelMeter";
import { DetectionStack } from "./components/DetectionStack";
import { LiveTranscript } from "./components/LiveTranscript";
import { SettingsScreen } from "./components/SettingsScreen";
import { StagingPanel } from "./components/StagingPanel";
import { BlankButton } from "./components/BlankButton";
import { CommandPalette } from "./components/CommandPalette";
import { TabPanel } from "./components/TabPanel";
import { TABS, useNavStore } from "@/stores/navStore";
import { useServiceStore } from "@/stores/serviceStore";
import { useEffect } from "react";
import { on, output } from "@/lib/ipc";

// M0 shell only. The panels below are filled in per milestone:
// transcript M1, detection cards M2, staging M3, library M4, packs M0/M5.

export function OperatorApp() {
  // In a store rather than local state so a message elsewhere can navigate —
  // the "no key set" error on the Live tab opens Settings at the right
  // section. See navStore.
  const tab = useNavStore((s) => s.tab);
  const setTab = useNavStore((s) => s.setTab);
  const autoLive = useServiceStore((s) => s.outputs.autoLive);
  const setOutputs = useServiceStore((s) => s.setOutputs);

  // The output state is mirrored here, at the root, rather than in the
  // staging panel: the auto-live indicator must show on every tab (§13.5,
  // "obvious"), and the panel only mounts once the Live tab has been seen.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void output.get().then((s) => {
      if (!cancelled) setOutputs(s);
    });
    void on("output:changed", setOutputs).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [setOutputs]);

  return (
    <div className="flex h-full flex-col bg-bg-canvas">
      <header className="flex items-center gap-4 border-b border-line-subtle bg-bg-surface px-4 py-2">
        <span className="font-display font-semibold tracking-[-0.01em]">SermonAI</span>

        <nav className="inline-flex gap-0.5 rounded-pill bg-bg-pill p-1">
          {TABS.map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`rounded-pill px-3 py-1 text-[13px] transition-colors duration-base ease-brand-out ${
                tab === t ? "bg-bg-raised text-content-primary" : "text-content-secondary hover:text-content-primary"
              }`}
            >
              {t}
            </button>
          ))}
        </nav>

        <div className="flex-1" />
        {/* Auto-live's indicator (PRD §13.5): on every tab, in the warning
            colour, because a verse can now reach the congregation without a
            keypress and the operator must never be surprised by that. */}
        {autoLive && (
          <span
            className="chip bg-status-warning-bg text-status-warning"
            role="status"
            title="Confirmed verses above 90% go straight to the projector. Turn off in Staging or Settings."
          >
            <span className="inline-block h-1.5 w-1.5 rounded-full bg-status-warning" aria-hidden="true" />
            Auto-live on
          </span>
        )}
        {/* Blank projector (FR-33): the wireframe's top-bar icon button, on
            every tab, with B as its hotkey. */}
        <BlankButton />
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
              right-column one (FR-50, wireframe §staging). */}
          <StagingPanel />
        </div>

        <div className="op-col">
          <DetectionStack />
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

      {/* Ctrl/Cmd+K from any tab (FR-31). */}
      <CommandPalette />
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
