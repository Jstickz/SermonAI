import { useCallback, useEffect, useRef, useState } from "react";
import { bible, output } from "@/lib/ipc";
import type { Verse } from "@/lib/types";
import { useNavStore } from "@/stores/navStore";
import { useServiceStore } from "@/stores/serviceStore";
import { useTranslationStore } from "@/stores/translationStore";

/**
 * The command palette (FR-31, PRD §13.9, wireframe §palette; M3
 * deliverable 7). Ctrl/Cmd+K from anywhere. Type "john 3 16"; the
 * reference resolves as you type, in the current translation; Enter stages
 * it, Shift+Enter sends it live; Escape closes. Recent verses — what went to
 * staging or the projector this service — sit under the typed result so
 * the pastor's return to a verse is two keys.
 *
 * Tab switches the translation the typed verse is looked up in. It cycles
 * the cached translations only: an uncached one would turn a two-key
 * action into a network wait, and the picker is where that choice is made
 * deliberately.
 *
 * The typed reference is resolved on the backend (`resolve_reference`),
 * which owns the parser and the cache; the palette shows what came back,
 * including "not cached", rather than guessing.
 */

/** Which row ↑↓ land on next, wrapping, over `count` rows. */
export function moveSelection(index: number, delta: 1 | -1, count: number): number {
  if (count === 0) return 0;
  return (index + delta + count) % count;
}

type Row = { kind: "typed"; verse: Verse } | { kind: "recent"; verse: Verse };

export function CommandPalette() {
  const open = useNavStore((s) => s.paletteOpen);
  const setOpen = useNavStore((s) => s.setPaletteOpen);
  const recent = useServiceStore((s) => s.recent);
  const translation = useTranslationStore((s) => s.current);
  const choices = useTranslationStore((s) => s.choices);
  const selectTranslation = useTranslationStore((s) => s.select);
  const loadTranslations = useTranslationStore((s) => s.load);

  const [query, setQuery] = useState("");
  const [typed, setTyped] = useState<Verse | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const restoreTo = useRef<HTMLElement | null>(null);

  // Ctrl/Cmd+K opens from anywhere, text fields included: it is a chord,
  // not a letter, and the operator's hands are on the keyboard already.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && (e.key === "k" || e.key === "K")) {
        e.preventDefault();
        setOpen(!useNavStore.getState().paletteOpen);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [setOpen]);

  // Open: remember where focus was, clear, focus the field, make sure the
  // translation list is loaded for Tab.
  useEffect(() => {
    if (!open) return;
    restoreTo.current = document.activeElement as HTMLElement | null;
    setQuery("");
    setTyped(null);
    setProblem(null);
    setSelected(0);
    void loadTranslations();
    const id = window.setTimeout(() => inputRef.current?.focus(), 0);
    return () => {
      window.clearTimeout(id);
      const target = restoreTo.current;
      if (target?.isConnected && target.matches("button, a[href], input, select, textarea")) {
        target.focus();
      }
    };
  }, [open, loadTranslations]);

  // Resolve as you type, a beat after the last keystroke. The last request
  // wins: a reply for "john 3 1" must not land after the one for "john 3 16".
  useEffect(() => {
    if (!open) return;
    const q = query.trim();
    if (q === "") {
      setTyped(null);
      setProblem(null);
      return;
    }
    let cancelled = false;
    const id = window.setTimeout(() => {
      bible
        .resolve(q)
        .then((verse) => {
          if (cancelled) return;
          setTyped(verse);
          setProblem(null);
          setSelected(0);
        })
        .catch((e) => {
          if (cancelled) return;
          setTyped(null);
          setProblem(String(e));
        });
    }, 150);
    return () => {
      cancelled = true;
      window.clearTimeout(id);
    };
  }, [open, query, translation]);

  const rows: Row[] = [
    ...(typed ? [{ kind: "typed" as const, verse: typed }] : []),
    ...recent.map((verse) => ({ kind: "recent" as const, verse })),
  ];

  const act = useCallback(
    async (row: Row | undefined, live: boolean) => {
      if (!row) return;
      setOpen(false);
      try {
        await output.stage(row.verse);
        if (live) await output.goLive();
      } catch {
        // The staging panel shows the state; a failed stage leaves it as it was.
      }
    },
    [setOpen],
  );

  // Tab: the next cached translation. The typed verse re-resolves in it.
  const cycleTranslation = useCallback(() => {
    const cached = choices.filter((c) => c.cached);
    if (cached.length < 2) return;
    const at = cached.findIndex((c) => c.code === translation);
    const next = cached[(at + 1 + cached.length) % cached.length];
    if (next) void selectTranslation(next.code);
  }, [choices, translation, selectTranslation]);

  if (!open) return null;

  return (
    <div
      className="modal-backdrop"
      onClick={(e) => {
        if (e.target === e.currentTarget) setOpen(false);
      }}
    >
      <div className="palette" role="dialog" aria-label="Search any verse">
        <div className="palette-input">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" className="h-[18px] w-[18px] text-content-muted" aria-hidden="true">
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-3.5-3.5" />
          </svg>
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search any verse, e.g. john 3 16"
            aria-label="Reference"
            autoComplete="off"
            spellCheck={false}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.preventDefault();
                setOpen(false);
              } else if (e.key === "ArrowDown") {
                e.preventDefault();
                setSelected((i) => moveSelection(i, 1, rows.length));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setSelected((i) => moveSelection(i, -1, rows.length));
              } else if (e.key === "Enter") {
                e.preventDefault();
                void act(rows[selected], e.shiftKey);
              } else if (e.key === "Tab") {
                e.preventDefault();
                cycleTranslation();
              }
            }}
          />
          <span className="kbd">{translation}</span>
          <span className="kbd">ESC</span>
        </div>

        <div className="palette-list">
          {problem && query.trim() !== "" && (
            <p className="px-3 py-2 text-[13px] text-content-muted">{problem}</p>
          )}
          {typed && (
            <>
              <div className="palette-group">Typed</div>
              <PaletteRow
                verse={typed}
                on={selected === 0}
                hint="↵ Stage · ⇧↵ Go Live"
                onClick={() => void act(rows[0], false)}
                onHover={() => setSelected(0)}
              />
            </>
          )}
          {recent.length > 0 && (
            <>
              <div className="palette-group">Recent</div>
              {recent.map((verse, i) => {
                const index = (typed ? 1 : 0) + i;
                return (
                  <PaletteRow
                    key={`${verse.reference}·${verse.translation}`}
                    verse={verse}
                    on={selected === index}
                    hint="↵ Stage"
                    onClick={() => void act(rows[index], false)}
                    onHover={() => setSelected(index)}
                  />
                );
              })}
            </>
          )}
          {!typed && recent.length === 0 && !problem && (
            <p className="px-3 py-2 text-[13px] text-content-muted">
              Type a reference. Verses you stage or send live this service appear here.
            </p>
          )}
        </div>

        <div className="palette-footer">
          <span>
            <span className="kbd">↑↓</span> Navigate
          </span>
          <span>
            <span className="kbd">↵</span> Stage
          </span>
          <span>
            <span className="kbd">⇧↵</span> Go Live
          </span>
          <span>
            <span className="kbd">Tab</span> Switch translation
          </span>
        </div>
      </div>
    </div>
  );
}

function PaletteRow({
  verse,
  on,
  hint,
  onClick,
  onHover,
}: {
  verse: Verse;
  on: boolean;
  hint: string;
  onClick: () => void;
  onHover: () => void;
}) {
  return (
    <div
      className={`palette-item ${on ? "on" : ""}`}
      role="option"
      aria-selected={on}
      onClick={onClick}
      onMouseEnter={onHover}
    >
      <span className="ref">{verse.reference}</span>
      <span className="snippet line-clamp-1">
        {verse.text || `Not cached in the ${verse.translation}.`}
      </span>
      <span className="hint">{hint}</span>
    </div>
  );
}
