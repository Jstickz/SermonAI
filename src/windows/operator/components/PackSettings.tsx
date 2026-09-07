import { useCallback, useEffect, useState } from "react";
import { on, packs as packsApi } from "@/lib/ipc";
import type { Pack, PackStatus } from "@/lib/types";

/**
 * Settings → Packs (FR-60): every optional pack with a clear size label, a
 * progress bar, pause/resume, and removal that frees the disk immediately.
 */
export function PackSettings() {
  const [packs, setPacks] = useState<Pack[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (fromCdn: boolean) => {
    setBusy(true);
    try {
      setPacks(fromCdn ? await packsApi.refresh() : await packsApi.list());
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void load(false);
  }, [load]);

  // Live progress from the Rust downloader.
  useEffect(() => {
    const unlisten = on("pack:progress", (event) => {
      setPacks((current) =>
        current.map((pack) =>
          pack.id === event.packId
            ? { ...pack, status: event.status, progress: event.progress, bytesOnDisk: event.bytesOnDisk }
            : pack,
        ),
      );
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  async function run(action: Promise<unknown>) {
    try {
      await action;
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      void load(false);
    }
  }

  return (
    <section className="card">
      <div className="mb-5 flex items-start justify-between gap-4">
        <div>
          <h2 className="text-[22px] font-semibold">Packs</h2>
          <p className="mt-1 text-xs text-content-muted">
            Offline speech, extra translations and themes download only when you ask for them.
          </p>
        </div>
        <button className="btn-secondary" disabled={busy} onClick={() => void load(true)}>
          {busy ? "Checking for packs…" : "Check for packs"}
        </button>
      </div>

      {error && (
        <p className="mb-4 rounded-md bg-status-danger-bg px-3 py-2 text-status-danger">{error}</p>
      )}

      {packs.length === 0 ? (
        <div className="text-content-muted">
          <p className="text-content-secondary">No packs listed yet.</p>
          <p className="mt-1">
            The catalog lives on the SermonAI pack server. Check for packs when you are online.
          </p>
        </div>
      ) : (
        <ul className="flex flex-col gap-3">
          {packs.map((pack) => (
            <li key={pack.id} className="rounded-md bg-bg-sunken p-4">
              <div className="flex flex-wrap items-start justify-between gap-4">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-[14px] font-semibold">{pack.name}</span>
                    <span className="chip">{formatBytes(pack.sizeBytes)}</span>
                    {pack.optional && <span className="chip">Optional</span>}
                  </div>
                  <p className="mt-1 text-xs text-content-muted">{pack.description}</p>
                </div>
                <div className="flex shrink-0 gap-2">{actionsFor(pack, run)}</div>
              </div>

              {pack.status !== "available" && pack.status !== "installed" && (
                <div className="mt-3">
                  <div className="h-1 overflow-hidden rounded-pill bg-bg-canvas">
                    <div
                      className="h-full bg-accent-500 transition-[width] duration-base ease-brand-out"
                      style={{ width: `${Math.round(pack.progress * 100)}%` }}
                    />
                  </div>
                  <p className="mono mt-1.5 text-[11px] text-content-muted">
                    {statusLabel(pack)} · {formatBytes(pack.bytesOnDisk)} of {formatBytes(pack.sizeBytes)}
                  </p>
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function actionsFor(pack: Pack, run: (action: Promise<unknown>) => Promise<void>) {
  switch (pack.status) {
    case "installed":
      return (
        <button className="btn-destructive !px-4 !py-2" onClick={() => void run(packsApi.remove(pack.id))}>
          Remove
        </button>
      );
    case "downloading":
    case "verifying":
      return (
        <button className="btn-secondary !px-4 !py-2" onClick={() => void run(packsApi.pause(pack.id))}>
          Pause
        </button>
      );
    case "paused":
    case "failed":
      return (
        <button className="btn-primary !px-4 !py-2" onClick={() => void run(packsApi.download(pack.id))}>
          Resume
        </button>
      );
    default:
      return (
        <button className="btn-primary !px-4 !py-2" onClick={() => void run(packsApi.download(pack.id))}>
          Download
        </button>
      );
  }
}

function statusLabel(pack: Pack): string {
  const labels: Record<PackStatus, string> = {
    available: "Not downloaded",
    downloading: "Downloading",
    paused: "Paused",
    verifying: "Checking the download",
    installed: "Installed",
    failed: "Download failed",
  };
  return labels[pack.status];
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${Math.round(bytes / 1024 ** 2)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} bytes`;
}
