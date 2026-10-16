# OS code signing deferred to M11; the updater key signs update bundles

**Decided:** 8 September 2026 · **Confirmed a launch blocker:** 21 September 2026

Builds are unsigned through development. M0's two installer DoD lines were written to *expect* the SmartScreen and Gatekeeper warnings and both passed with them (clean Windows 11 VM, and a real Mac with the dmg downloaded through Safari so the quarantine attribute was present). The Tauri updater key signs update bundles; local release builds can finish without it (23 Sept).

Why defer: a private tester group can follow `docs/INSTALL.md`. Why it is a blocker anyway: the macOS dialog offers no obvious way forward, the workaround is buried in System Settings, and the honest reading of the warning is that the download is unsafe. Notarization, not signing alone, removes it (Gatekeeper checks for a ticket). On Windows, SmartScreen reputation accrues to the certificate over time, so signing close to launch still leaves early downloads flagged. Both are reasons to do this early in M11, not at the end.

Also carried to M11: WebView2 silent bootstrap on Windows 10 has never been exercised (the VM was Windows 11, which ships WebView2), and the oldest supported macOS (12) is untested.

Related: [[open-questions]], [[sessions/2026-09-08]], [[sessions/2026-09-21]]
