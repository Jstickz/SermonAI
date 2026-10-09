import { useEffect, useRef, useState } from "react";
import { filterChoices, useTranslationStore } from "@/stores/translationStore";

/**
 * The translation picker (FR-32, PRD §13.4, M2 deliverable 7).
 *
 * A compact control in the Live header: the current code, and on click a
 * list to search as you type. Picking changes the translation the *next*
 * card is fetched in; the cards on screen keep theirs, so a switch mid-
 * service never disturbs what the operator is looking at (FR-32).
 *
 * A version that is not on disk is marked so the operator knows its text
 * comes from YouVersion on each lookup (§13.4's download mark). Per-verse
 * override and the 20+ imported translations are M6.
 */
export function TranslationPicker() {
  const choices = useTranslationStore((s) => s.choices);
  const current = useTranslationStore((s) => s.current);
  const loaded = useTranslationStore((s) => s.loaded);
  const error = useTranslationStore((s) => s.error);
  const load = useTranslationStore((s) => s.load);
  const select = useTranslationStore((s) => s.select);

  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const root = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!loaded) void load();
  }, [loaded, load]);

  // Close on a click elsewhere or Escape; focus the search on open.
  useEffect(() => {
    if (!open) return;
    input.current?.focus();
    const onDown = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const visible = filterChoices(choices, query);
  const currentChoice = choices.find((c) => c.code === current);

  async function pick(code: string) {
    await select(code);
    setOpen(false);
    setQuery("");
  }

  return (
    <div ref={root} className="relative shrink-0">
      <button
        type="button"
        className="btn-secondary !px-3 !py-2 text-[13px]"
        onClick={() => setOpen((o) => !o)}
        aria-haspopup="listbox"
        aria-expanded={open}
        title={currentChoice ? `${currentChoice.name} — click to change` : "Choose translation"}
      >
        <span className="mono">{current}</span>
      </button>

      {open && (
        <div
          className="absolute right-0 z-20 mt-2 w-72 rounded-md border border-line-default bg-bg-raised p-2 shadow-lg"
          role="listbox"
          aria-label="Translation"
        >
          <input
            ref={input}
            className="field mb-2 !h-9 text-[13px]"
            placeholder="Search translations"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              // Enter picks the first match: one keystroke after typing "niv".
              if (e.key === "Enter" && visible[0]) void pick(visible[0].code);
            }}
          />
          {error && (
            <p className="mb-2 rounded-md bg-status-danger-bg px-2 py-1 text-xs text-status-danger">
              {error}
            </p>
          )}
          <ul className="max-h-64 overflow-y-auto">
            {visible.length === 0 && (
              <li className="px-2 py-2 text-xs text-content-muted">No translation matches.</li>
            )}
            {visible.map((c) => (
              <li key={c.code}>
                <button
                  type="button"
                  role="option"
                  aria-selected={c.code === current}
                  className={`flex min-h-10 w-full items-center gap-3 rounded-md px-2 py-2 text-left hover:bg-bg-sunken ${
                    c.code === current ? "bg-bg-sunken" : ""
                  }`}
                  onClick={() => void pick(c.code)}
                >
                  <span className="mono w-10 shrink-0 text-[13px] font-medium">{c.code}</span>
                  <span className="min-w-0 flex-1 truncate text-[13px]">{c.name}</span>
                  {!c.cached && (
                    <span className="chip" title="Fetched from YouVersion on each lookup">
                      Online
                    </span>
                  )}
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
