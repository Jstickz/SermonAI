# ADR 0001: Tauri 2 + Rust over Electron + Python

**Decided:** 7 September 2026 · **Status:** Accepted · **Full record:** `docs/adr/0001-tauri-over-electron.md`

Electron plus a bundled Python sidecar meant a 350 to 450 MB base installer, 2 to 3 GB with offline models, against a free competitor churches in Lagos and Nairobi already download on slow connections. Tauri 2 uses the OS webview and compiles the Rust backend into one binary: no sidecar, no runtime, no loopback HTTP hop. Result today: a 5.12 MB msi, projected ~25 MB installer with all bundled assets, 716 ms cold start of which 162 ms is ours.

Costs accepted: Rust on a solo founder, two webview engines to test, WebView2 bootstrap on old Windows (still untested, see [[open-questions]]). Python survives only in build-time scripts that never ship.

Related: [[adr-0006-on-demand-packs]], [[sessions/2026-09-07]]
