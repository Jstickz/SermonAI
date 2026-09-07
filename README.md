# SermonAI

> Every verse. Every sermon. Kept.

SermonAI listens to a live sermon, detects scripture references (spoken or
paraphrased), and projects the verse in the operator's chosen translation. The
moment the service ends it produces a detailed sermon summary as a downloadable
PDF, and keeps the full transcript in a searchable local archive.

Windows and macOS as equals. Works offline. Base installer under 40 MB.

## Documentation

| Document | What it is |
|---|---|
| [docs/PRD.md](docs/PRD.md) | Product requirements, v2.1. The source of truth. Architecture decisions in §10 and §11 are binding. |
| [docs/MILESTONES.md](docs/MILESTONES.md) | Stage tracker. **Read the Status Board before doing anything.** |
| [docs/BRANDING.md](docs/BRANDING.md) | Design system. Reference tokens by name, never hard-code hex. |
| [docs/wireframe.html](docs/wireframe.html) | Every screen as a static mock. Open it in a browser. |
| [docs/adr/](docs/adr/) | Architecture decision records. |

## Stack

Tauri 2 — one compiled Rust binary per platform using the OS webview, no
sidecar process and no bundled runtime. React 18 + TypeScript + Tailwind +
Zustand on the frontend; `tokio`, `cpal`, `rusqlite`, `axum` and `reqwest` in
the backend. See PRD §11.

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Node.js | 20+ | Frontend build and the Tauri CLI |
| Rust | stable, 1.77+ | Install via [rustup](https://rustup.rs) |
| Git | any | |
| Windows | VS 2022 Build Tools with the C++ workload; WebView2 (present on Windows 11) | |
| macOS | Xcode Command Line Tools | |

## Getting started

```bash
npm install
cp .env.example .env      # then fill in the vendor keys
npm run tauri:dev
```

`npm run tauri:dev` starts Vite and the Rust backend together and opens the
operator window; the projector and alternate windows are created hidden and
shown when an output monitor is selected.

## Everyday commands

| Command | Does |
|---|---|
| `npm run tauri:dev` | Run the app with hot reload |
| `npm run tauri:build` | Build signed installers for the current platform |
| `npm test` | Frontend unit tests (Vitest) |
| `npm run lint` | ESLint over `src/` |
| `cd src-tauri && cargo test` | Backend unit tests |
| `cd src-tauri && cargo clippy -- -D warnings` | Backend lints, as CI runs them |

## Layout

```
src/            React frontend — one entry point per window plus the LAN remote
  windows/      operator, projector, alternate
  remote/       phone remote web app, served by the Rust axum server
  design/       tokens.json — the single source of colour, radius, type, motion
  lib/          typed Tauri command wrappers and event subscriptions
  stores/       Zustand stores
src-tauri/      Rust backend, one module per layer (PRD §10.2)
  migrations/   SQLite schema, applied on first launch
  assets/       bundled: KJV/WEB/ASV, the quantized verse index, default theme
scripts/        one-time and release-time build jobs (verse index, packs)
docs/           PRD, milestones, branding, ADRs, wireframe
```

## Non-negotiables

These are gates, not preferences:

- **Base installer under 40 MB** on both platforms. CI fails the build otherwise
  (FR-59). New heavy assets become on-demand packs.
- **Feature parity across Windows and macOS.** Nothing ships on one alone.
- **The summary never invents scripture.** The validator drops any reference not
  in the accepted log or the transcript, and any quote that is not a transcript
  substring (FR-46).
- **Nothing reaches an output without passing through staging**, unless the
  operator has explicitly enabled auto-live (FR-50).
- **All sermon content stays local.** No account, telemetry opt-in and
  content-free (PRD §17).
