import { SHORTCUTS, SHORTCUT_RULE } from "@/lib/shortcuts";
import { Modal } from "./Modal";

/**
 * Keyboard shortcuts, as a modal from the bottom of Settings (M3 polish
 * item 7). Rendered from `src/lib/shortcuts.ts`, the one source the PRD's
 * §13.18 table is tested against, so this list cannot say something the
 * PRD does not.
 */
export function ShortcutsModal({ onDismiss }: { onDismiss: () => void }) {
  return (
    <Modal title="Keyboard shortcuts" description={SHORTCUT_RULE} onDismiss={onDismiss}>
      <table className="mt-3 w-full border-collapse text-[13px]">
        <tbody>
          {SHORTCUTS.map((s) => (
            <tr key={s.keys} className="border-t border-line-subtle">
              <td className="w-32 py-2 pr-3 align-top">
                <span className="kbd text-[11px]">{s.keys}</span>
              </td>
              <td className="py-2 pr-3 align-top text-content-primary">{s.does}</td>
              <td className="w-28 py-2 align-top text-[11px] text-content-muted">{s.where}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="mt-4 flex justify-end">
        <button type="button" className="btn-secondary" onClick={onDismiss}>
          Close
        </button>
      </div>
    </Modal>
  );
}
