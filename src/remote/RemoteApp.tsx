/**
 * The phone remote (M7, FR-52 to FR-55). Unlike the three desktop windows this
 * app runs in a phone browser over the LAN, so it has no Tauri bridge: it talks
 * to the axum server in the Rust binary over a WebSocket, authenticated by the
 * pairing token in the URL (`/r/<token>`).
 *
 * Thumb-reachable buttons; Go Live is the largest; Blank is red and needs no
 * confirmation (PRD §16.4).
 */
export function RemoteApp() {
  return (
    <div className="flex h-full flex-col bg-bg-canvas p-5">
      <header className="pb-4">
        <p className="text-xs text-content-muted">SermonAI Remote</p>
        <h1 className="text-lg font-semibold">Not paired</h1>
      </header>
      <p className="text-content-secondary">
        Scan the QR code in the operator window to pair this device.
      </p>
    </div>
  );
}
