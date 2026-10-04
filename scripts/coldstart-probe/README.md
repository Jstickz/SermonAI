# Cold-start probes

Two Tauri 2 apps that do nothing but open one window shaped like SermonAI's
operator window and log the same four timing lines SermonAI logs:

- `probe-bare` — Tauri 2 with the `tray-icon` feature, no plugins.
- `probe-plugins` — the same plus the three plugins SermonAI registers
  (dialog, fs, opener).

Both copy SermonAI's `Cargo.lock`, so they build the same crate versions.
Build each with SermonAI's target directory so the compiled dependencies are
reused:

    $env:CARGO_TARGET_DIR = (Resolve-Path .\src-tauri	arget)
    cargo build --release --manifest-path scripts\coldstart-probe\probe-bare\Cargo.toml
    cargo build --release --manifest-path scripts\coldstart-probe\probe-plugins\Cargo.toml

Then time them with the same script that times SermonAI:

    .\scripts\measure-cold-start.ps1 -Runs 7 -Exe .\src-tauri	argetelease\probe-bare.exe    -Log $env:LOCALAPPDATA\SermonAI\logs\probe-bare.log
    .\scripts\measure-cold-start.ps1 -Runs 7 -Exe .\src-tauri	argetelease\probe-plugins.exe -Log $env:LOCALAPPDATA\SermonAI\logs\probe-plugins.log

The difference between SermonAI and `probe-plugins` is ours; between
`probe-plugins` and `probe-bare` is the plugin choice; `probe-bare` is the
platform. Results: `docs/testing/cold-start.md`.
