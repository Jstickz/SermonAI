import { AudioSettings } from "./AudioSettings";
import { DisplaySettings } from "./DisplaySettings";
import { PackSettings } from "./PackSettings";
import { ServiceSettings } from "./ServiceSettings";
import { useNavStore, type SettingsSection } from "@/stores/navStore";

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
  { id: "services", label: "Services & keys", milestone: null },
  { id: "translations", label: "Translations", milestone: "M6" },
  { id: "summary", label: "Summary template", milestone: "M4" },
  { id: "remote", label: "Phone remote", milestone: "M7" },
  { id: "broadcast", label: "Broadcast (NDI · OBS)", milestone: "M7" },
  { id: "packs", label: "Packs", milestone: null },
  { id: "license", label: "License & devices", milestone: "M8" },
  { id: "diagnostics", label: "Diagnostics", milestone: "M8" },
] as const;

type SectionId = (typeof SECTIONS)[number]["id"];

/** The sections with a screen behind them. Listing them once keeps the
 *  fallback from shadowing a panel that was added but not added here — a
 *  mistake that renders as "not built yet" over working code. */
const BUILT: readonly string[] = ["audio", "displays", "packs", "services"];

export function SettingsScreen() {
  // In the nav store rather than local state so the Live tab can send the
  // operator straight here when a service has no key. See navStore.
  const section = useNavStore((s) => s.settingsSection);
  const setSection = useNavStore((s) => s.setSettingsSection);

  return (
    <div className="settings-layout">
      <nav className="settings-nav" aria-label="Settings sections">
        {SECTIONS.map((s) => (
          <button
            key={s.id}
            className={section === s.id ? "on" : undefined}
            onClick={() => setSection(s.id as SettingsSection)}
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
        {section === "services" && <ServiceSettings />}
        {!BUILT.includes(section) && <NotBuiltYet section={section} />}
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
