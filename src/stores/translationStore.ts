import { create } from "zustand";
import { translations as ipc } from "@/lib/ipc";
import type { TranslationChoice } from "@/lib/types";

/**
 * The translation picker's state (FR-32, PRD §13.4).
 *
 * The backend owns the default — it is a row in `settings` and the value
 * every card's text is fetched in — so this store is a mirror: it loads the
 * list and the current code once, and `select` asks the backend first and
 * mirrors on success. Picking never touches cards already on screen; it
 * changes what the next card is fetched in.
 */
interface TranslationState {
  choices: TranslationChoice[];
  current: string;
  loaded: boolean;
  error: string | null;
  load: () => Promise<void>;
  select: (code: string) => Promise<void>;
}

/** Search-as-you-type over code and name, case-insensitively. Exported
 *  without the store so it can be tested as the pure function it is. */
export function filterChoices(choices: TranslationChoice[], query: string): TranslationChoice[] {
  const q = query.trim().toLowerCase();
  if (!q) return choices;
  return choices.filter(
    (c) => c.code.toLowerCase().includes(q) || c.name.toLowerCase().includes(q),
  );
}

export const useTranslationStore = create<TranslationState>((set) => ({
  choices: [],
  current: "KJV",
  loaded: false,
  error: null,

  load: async () => {
    try {
      const [choices, current] = await Promise.all([ipc.list(), ipc.getDefault()]);
      set({ choices, current, loaded: true, error: null });
    } catch (e) {
      set({ error: String(e), loaded: true });
    }
  },

  select: async (code) => {
    try {
      await ipc.setDefault(code);
      set({ current: code, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
