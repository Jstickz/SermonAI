# ADR 0001 — Tauri 2 + Rust over Electron + Python

- **Status:** Accepted
- **Date:** 7 September 2026
- **PRD:** §0 (v2.1 changes), §9.7, §11

## Context

v2.0 specified Electron with a bundled Python sidecar. Two founder requirements
arrived afterwards: Windows and macOS must be equals, and the app must be
lightweight and fast to download and install. Electron ships a full Chromium
(150 to 220 MB) and a bundled Python runtime adds 90 to 180 MB more — a 350 to
450 MB base installer, 2 to 3 GB with offline models. That is incompatible with
the second requirement, and it is worse than the free competitor churches are
already using on slow connections in Lagos and Nairobi.

## Decision

Build on Tauri 2. The frontend stays React + TypeScript. The backend is Rust
compiled into the same binary — no sidecar process, no bundled runtime, no
loopback HTTP hop for the operator UI. Tauri uses the webview the OS already
has: WebView2 on Windows, WKWebView on macOS.

## Consequences

**Good**
- 15 to 30 MB base install, which makes the 40 MB gate (FR-59) achievable.
- Sub-second cold start and roughly a quarter of Electron's memory.
- One codebase covers both platforms with real parity.
- The compiler catches most mistakes before they reach a Sunday service.

**Costs**
- Rust is a learning curve for a solo founder. Mitigated by keeping business
  logic simple and tested, and strict module boundaries (PRD §22.1).
- Two webview engines instead of one means rendering differences must be tested
  on both platforms, not assumed.
- WebView2 may be missing on older Windows machines. The installer bootstraps
  the evergreen runtime silently (FR-63).
- Python is still used, but only for build-time scripts that never ship.

**Displaced choices**
- FAISS replaced by an in-binary brute-force cosine search (see ADR 0003).
- Vosk replaced by whisper.cpp via `whisper-rs`.
- The offline summary defaults to a template renderer rather than a bundled LLM.
