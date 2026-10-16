import { AutoLiveSwitch } from "./AutoLiveSwitch";

/**
 * Settings → General → Behaviour (wireframe.html §settings "Behaviour").
 *
 * Only the auto-live row is built (M3 deliverable 2). The other rows the
 * wireframe draws here — enhance summary when online, auto-save PDF to a
 * folder, crash reporting — belong to M4 and M8 and are named below so the
 * operator can see where they will live.
 */
export function BehaviourSettings() {
  return (
    <section className="card">
      <div className="mb-5">
        <h2 className="text-[22px] font-semibold">Behaviour</h2>
        <p className="mt-1 text-xs text-content-muted">
          How SermonAI acts during a service.
        </p>
      </div>

      <div className="flex items-start justify-between gap-6 border-b border-line-subtle py-4">
        <div>
          <div className="text-[13px] font-medium">Auto-live detected verses</div>
          <p className="mt-1 text-xs text-content-muted">
            Skip staging when confidence is above 90%. Off by default, so the operator stays in
            control. While on, &ldquo;Auto-live on&rdquo; shows in the top bar on every tab.
            Unconfirmed cards and verses with no cached text never go live on their own.
          </p>
        </div>
        <AutoLiveSwitch />
      </div>

      <p className="mt-4 text-xs text-content-muted">
        Enhance summary when online and auto-save PDF to a folder arrive with M4. Anonymous crash
        reporting arrives with M8.
      </p>
    </section>
  );
}
