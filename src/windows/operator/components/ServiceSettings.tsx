import { useCallback, useEffect, useState } from "react";
import { credentials as credentialsApi } from "@/lib/ipc";
import type { CredentialStatus, ServiceCredential, ServiceId, TestOutcome } from "@/lib/types";

/**
 * Settings → Services and keys (PRD §17, key strategy Phase 5).
 *
 * Each service the app talks to, what stops working without it, and — where
 * the licence allows a church to use its own account — a box to paste a key,
 * with Test, Replace and Remove.
 *
 * ## Two things this panel is careful about
 *
 * It never holds a key longer than the operator is typing it. The draft is
 * cleared the moment a save succeeds, and nothing reads a stored key back:
 * the backend has no command that returns one, so there is nothing here to
 * leak into a screenshot or a devtools session.
 *
 * And it distinguishes "no key set" from "could not look". They render the
 * same way in most settings screens, and telling an operator they have no key
 * when the credential store is broken sends them to paste one they already
 * pasted.
 */
export function ServiceSettings() {
  const [services, setServices] = useState<ServiceCredential[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setServices(await credentialsApi.list());
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <section className="card">
      <div className="mb-5">
        <h2 className="text-[22px] font-semibold">Services and keys</h2>
        <p className="mt-1 text-xs text-content-muted">
          SermonAI uses these accounts for transcription, summaries and scripture. Keys are stored
          in this computer&rsquo;s credential manager, never in a file SermonAI writes.
        </p>
      </div>

      {error && (
        <p className="mb-4 rounded-md bg-status-danger-bg px-3 py-2 text-status-danger">{error}</p>
      )}

      {loading ? (
        <p className="text-[13px] text-content-muted">Checking stored keys…</p>
      ) : (
        <ul className="flex flex-col gap-3">
          {services.map((service) => (
            <ServiceRow
              key={service.service}
              service={service}
              onChanged={setServices}
              onError={setError}
            />
          ))}
        </ul>
      )}
    </section>
  );
}

function ServiceRow({
  service,
  onChanged,
  onError,
}: {
  service: ServiceCredential;
  onChanged: (services: ServiceCredential[]) => void;
  onError: (message: string | null) => void;
}) {
  const [draft, setDraft] = useState("");
  const [editing, setEditing] = useState(false);
  const [busy, setBusy] = useState<null | "saving" | "testing" | "removing">(null);
  const [outcome, setOutcome] = useState<TestOutcome | null>(null);

  const active = service.status.kind === "byok_active";

  async function run(action: "saving" | "removing", work: () => Promise<ServiceCredential[]>) {
    setBusy(action);
    setOutcome(null);
    try {
      onChanged(await work());
      onError(null);
      // The draft is dropped as soon as it is stored. Keeping it around would
      // leave a full key in component state for the rest of the session, for
      // no benefit — it is already saved.
      setDraft("");
      setEditing(false);
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(null);
    }
  }

  async function test() {
    setBusy("testing");
    setOutcome(null);
    try {
      setOutcome(await credentialsApi.test(service.service));
      onError(null);
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(null);
    }
  }

  return (
    <li className="rounded-md bg-bg-sunken p-4">
      <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="break-words text-[14px] font-semibold">{service.label}</span>
            <StatusChip status={service.status} />
          </div>
          <p className="mt-1 text-xs text-content-muted">{service.purpose}</p>
        </div>

        {service.allowsByok && !editing && (
          <div className="flex shrink-0 flex-wrap gap-2">
            {active && (
              <button className="btn-secondary" disabled={busy !== null} onClick={() => void test()}>
                {busy === "testing" ? "Testing key…" : "Test"}
              </button>
            )}
            <button className="btn-secondary" disabled={busy !== null} onClick={() => setEditing(true)}>
              {active ? "Replace" : "Add key"}
            </button>
            {active && service.status.kind === "byok_active" && service.status.source === "keychain" && (
              <button
                className="btn-secondary"
                disabled={busy !== null}
                onClick={() => void run("removing", () => credentialsApi.remove(service.service))}
              >
                {busy === "removing" ? "Removing key…" : "Remove"}
              </button>
            )}
          </div>
        )}
      </div>

      {editing && (
        <form
          className="mt-3 flex flex-wrap items-center gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            void run("saving", () => credentialsApi.setKey(service.service, draft));
          }}
        >
          <label className="sr-only" htmlFor={`key-${service.service}`}>
            {service.label} key
          </label>
          <input
            id={`key-${service.service}`}
            className="mono min-w-0 flex-1 rounded-md bg-bg-canvas px-3 py-2 text-[13px]"
            type="password"
            autoComplete="off"
            spellCheck={false}
            autoFocus
            placeholder={`Paste your ${service.label} key`}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
          />
          <button className="btn-primary" type="submit" disabled={busy !== null || draft.trim() === ""}>
            {busy === "saving" ? "Saving key…" : "Save key"}
          </button>
          <button
            className="btn-secondary"
            type="button"
            disabled={busy !== null}
            onClick={() => {
              setDraft("");
              setEditing(false);
            }}
          >
            Cancel
          </button>
        </form>
      )}

      {service.status.kind === "store_unavailable" && (
        <p className="mt-2 text-xs text-status-danger">{service.status.detail}</p>
      )}

      {!service.allowsByok && (
        <p className="mt-2 text-xs text-content-muted">
          Comes with SermonAI activation. The publisher licence for these texts is SermonAI&rsquo;s,
          so your own key would not carry it.
        </p>
      )}

      {outcome && (
        <p
          className={`mt-2 rounded-md px-3 py-2 text-xs ${
            outcome.ok
              ? "bg-status-success-bg text-status-success"
              : "bg-status-danger-bg text-status-danger"
          }`}
        >
          {outcome.message}
        </p>
      )}
    </li>
  );
}

/** The state of one service, in as few words as carry the meaning. */
function StatusChip({ status }: { status: CredentialStatus }) {
  switch (status.kind) {
    case "byok_active":
      return (
        <span className="chip">
          {status.source === "dev_env" ? "From .env" : "Your key"} · {status.masked}
        </span>
      );
    case "managed_active":
      return <span className="chip">Managed by SermonAI</span>;
    case "store_unavailable":
      return <span className="chip text-status-danger">Cannot read stored key</span>;
    case "token_revoked":
      return <span className="chip text-status-danger">Revoked</span>;
    case "quota_reached":
      return <span className="chip text-status-warning">Out of allowance</span>;
    case "gateway_unreachable":
      return <span className="chip text-status-warning">Cannot reach SermonAI</span>;
    case "dev_key_missing":
    case "not_activated":
      return <span className="chip">No key set</span>;
  }
}

export type { ServiceId };
