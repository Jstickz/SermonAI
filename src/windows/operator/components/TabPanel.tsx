import { useEffect, useLayoutEffect, useRef, useState } from "react";

/**
 * One tab's view, kept alive when the operator switches away.
 *
 * ## Why kept mounted rather than saved to a store
 *
 * Unmounting loses everything a view holds: scroll position, search text,
 * filter selections, half-typed form input, and the uncontrolled DOM state
 * React never sees. Saving all of that per view means writing — and
 * remembering to write — save/restore code in every screen, and the failure is
 * silent: a filter that quietly resets is not an error anybody reports, it is
 * just an app that feels careless. Library and Series will each have several
 * such pieces of state.
 *
 * Keeping the tree mounted preserves all of it for free, including the parts
 * nobody thought to save.
 *
 * ## First mount is still lazy
 *
 * Mounting every tab at startup would put the Library's work in the cold-start
 * path, which PRD §9.1 budgets at one second. So a tab is mounted the first
 * time it is opened and kept from then on: the cost is paid once, when the
 * operator has asked for it.
 *
 * It also avoids measuring-on-mount bugs. A virtualised list or a chart that
 * reads its own height while hidden measures zero; mounting on first display
 * means it is always measured visible.
 *
 * ## Scroll needs restoring by hand
 *
 * `display: none` drops the element from layout and browsers reset `scrollTop`
 * with it, so keeping the tree alive is not enough on its own. The position is
 * recorded as the operator scrolls and put back when the tab returns — during
 * layout, so it never paints at the top first.
 */
export function TabPanel({
  active,
  className,
  children,
}: {
  active: boolean;
  className?: string;
  children: React.ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const scrollTop = useRef(0);
  const [mounted, setMounted] = useState(active);

  useEffect(() => {
    if (active) setMounted(true);
  }, [active]);

  useLayoutEffect(() => {
    if (!active || !ref.current) return;
    // Layout, not effect: by the time a passive effect runs the browser has
    // already painted, and the operator sees the view jump from the top to
    // where they left it.
    ref.current.scrollTop = scrollTop.current;
  }, [active]);

  return (
    <div
      ref={ref}
      // hidden rather than conditional rendering: the subtree stays in the
      // React tree and keeps its state.
      hidden={!active}
      // Recorded continuously rather than when the tab is left. React applies
      // the DOM change before effects run, so by then the element is already
      // display:none and its scrollTop is zero.
      onScroll={(e) => {
        scrollTop.current = e.currentTarget.scrollTop;
      }}
      className={className}
    >
      {mounted ? children : null}
    </div>
  );
}
