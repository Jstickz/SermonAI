import { create } from "zustand";

/**
 * Which tab is showing, and which Settings section.
 *
 * Tab state used to be `useState` inside `OperatorApp`, which was fine while
 * nothing but the tab strip could change it. It stopped being fine when an
 * error on the Live tab needed to send the operator to a specific settings
 * section: an error message that names a place is only half an answer if
 * getting there is still four clicks, and prop-drilling a setter through the
 * transcript panel to reach it would tie an unrelated component to the shell.
 *
 * A store instead, so anything can say "take them here" without the components
 * in between knowing about navigation.
 */
export const TABS = ["Live", "Library", "Series", "Studio", "Settings"] as const;
export type Tab = (typeof TABS)[number];

/** The settings sections that exist. Kept here rather than in the panel so a
 *  caller elsewhere cannot name one that is not there. */
export type SettingsSection =
  | "general"
  | "audio"
  | "displays"
  | "services"
  | "translations"
  | "summary"
  | "remote"
  | "broadcast"
  | "packs"
  | "license"
  | "diagnostics";

interface NavState {
  tab: Tab;
  settingsSection: SettingsSection;
  setTab: (tab: Tab) => void;
  setSettingsSection: (section: SettingsSection) => void;
  /** Open Settings at a named section, in one action. */
  goToSettings: (section: SettingsSection) => void;
}

export const useNavStore = create<NavState>((set) => ({
  tab: "Live",
  settingsSection: "audio",
  setTab: (tab) => set({ tab }),
  setSettingsSection: (settingsSection) => set({ settingsSection }),
  goToSettings: (settingsSection) => set({ tab: "Settings", settingsSection }),
}));
