import { useEffect, useRef } from "react";

/**
 * A modal dialog (`docs/wireframe.html` §modal-backdrop).
 *
 * The wireframe shows the appearance; the behaviour below is what makes it a
 * dialog rather than a div that floats on top.
 *
 * **Focus moves in and comes back.** Without that, a keyboard operator stays
 * on whatever was behind the dialog: Tab walks the transcript they cannot see,
 * and Enter presses a button that is no longer on screen. Focus returns to
 * whatever opened it on close, so the booth does not lose its place.
 *
 * **Tab is trapped.** A dialog that lets focus wander behind itself is worse
 * than none — the operator is one Tab from acting on a control the dialog is
 * covering.
 *
 * **Escape and the backdrop dismiss, unless `required`.** Some dialogs have no
 * safe default: a lost microphone mid-sermon cannot be waved away without
 * either choosing another input or stopping, and offering a silent dismissal
 * would leave the app recording nothing.
 */
export function Modal({
  title,
  description,
  icon,
  required = false,
  onDismiss,
  children,
}: {
  title: string;
  description?: string;
  /** Sits left of the title. A warning colour belongs on the caller's node. */
  icon?: React.ReactNode;
  /** When true, Escape and the backdrop do nothing: the operator must choose. */
  required?: boolean;
  onDismiss?: () => void;
  children?: React.ReactNode;
}) {
  const cardRef = useRef<HTMLDivElement>(null);
  const restoreTo = useRef<HTMLElement | null>(null);

  useEffect(() => {
    restoreTo.current = document.activeElement as HTMLElement | null;

    // The first control, or the dialog itself when it has none — announcing
    // the title beats leaving focus behind the backdrop.
    const focusable = cardRef.current?.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    );
    (focusable?.[0] ?? cardRef.current)?.focus();

    return () => restoreTo.current?.focus();
  }, []);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape" && !required) {
        onDismiss?.();
        return;
      }

      if (e.key !== "Tab") return;

      const items = cardRef.current?.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
      );
      if (!items || items.length === 0) return;

      const first = items[0];
      const last = items[items.length - 1];
      if (!first || !last) return;

      // Wrap at both ends, so Tab cycles inside the dialog instead of walking
      // out into the page behind it.
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }

    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [required, onDismiss]);

  return (
    <div
      className="modal-backdrop"
      // Only a click that starts and ends on the backdrop dismisses: dragging
      // a selection out of the dialog and releasing outside should not close
      // it, which is how a naive handler loses someone's work.
      onMouseDown={(e) => {
        if (!required && e.target === e.currentTarget) onDismiss?.();
      }}
    >
      <div
        ref={cardRef}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        className="modal-card outline-none"
      >
        <div className="flex items-start gap-4">
          {icon}
          <div className="min-w-0 flex-1">
            <h2 className="text-[18px] font-semibold">{title}</h2>
            {description && (
              <p className="mt-2 text-[13px] leading-relaxed text-content-secondary">
                {description}
              </p>
            )}
          </div>
        </div>
        {children}
      </div>
    </div>
  );
}
