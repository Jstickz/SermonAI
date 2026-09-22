import { useState } from "react";
import { AudioSettings } from "./AudioSettings";
import { DisplaySettings } from "./DisplaySettings";
import { PackSettings } from "./PackSettings";

/**
 * Settings: a section list beside one section's content
 * (`docs/wireframe.html` §settings-layout).
 *
 * The sections and their order are the wireframe's, including the ones with
 * nothing behind them yet. Showing the whole shape and marking what has not
 * been built beats a short list that silently grows, because an operator can
 * see where a setting will live rather than wondering whether it exists.
 */
const SECTIONS = [
  { id: "general", label: "General", milestone: "M8" },
  { id: "audio", label: "Audio & speech", milestone: null },
  { id: "displays", label: "Displays & themes", milestone: null },
  { id: "translations", label: "Translations", milestone: "M6" },
  { id: "summary", label: "Summary template", milestone: "M4" },
  { id: "remote", label: "Phone remote", milestone: "M7" },
  { id: "broadcast", label: "Broadcast (NDI · OBS)", milestone: "M7" },
  { id: "packs", label: "Packs", milestone: null },
  { id: "license", label: "License & devices", milestone: "M8" },
  { id: "diagnostics", label: "Diagnostics", milestone: "M8" },
] as const;

type SectionId = (typeof SECTIONS)[number]["id"];

export function SettingsScreen() {
  const [section, setSection] = useState<SectionId>("audio");

  return (
    <div className="settings-layout">
      <nav className="settings-nav" aria-label="Settings sections">
        {SECTIONS.map((s) => (
          <button
            key={s.id}
            className={section === s.id ? "on" : undefined}
            onClick={() => setSection(s.id)}
            aria-current={section === s.id ? "page" : undefined}
          >
            {s.label}
          </button>
        ))}
      </nav>

      <div className="settings-content">
        {section === "audio" && <AudioSettings />}
        {section === "displays" && <DisplaySettings />}
        {section === "packs" && <PackSettings />}
        {!["audio", "displays", "packs"].includes(section) && <NotBuiltYet section={section} />}
      </div>
    </div>
  );
}

function NotBuiltYet({ section }: { section: SectionId }) {
  const entry = SECTIONS.find((s) => s.id === section);
  if (!entry) return null;

  return (
    <section className="card">
      <div className="settings-section-title">{entry.label}</div>
      <p className="text-[13px] text-content-muted">
        Not built yet. These settings arrive in {entry.milestone}; the section is listed so the shape
        of Settings stays the same as it fills in.
      </p>
    </section>
  );
}
