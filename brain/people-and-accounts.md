# People and accounts

**Rule:** no keys, tokens, passwords, emails or personal data in this folder, ever. This note says *which* accounts exist and *where* their secrets live, never what they contain.

## People

- **The operator** (git author on this repo, solo founder). Owns every decision marked "operator's call" in the notes, runs the hardware tests, hand-labels evaluation sets, and is the only person who creates vendor accounts. Based in Abuja, Nigeria, which is why every latency figure is from a Nigerian connection and why that matters for the target market (PRD §4.2 also names the UK).
- **Claude Code** (the agent). Writes code, docs and these notes. Must not create accounts on the operator's behalf; stops at the STOP points in the operator's briefs.
- **Future testers:** a private tester group receives unsigned builds with `docs/INSTALL.md`; the pilot (M8) targets 10 to 15 churches, half in Nigeria and Kenya, at least three on macOS.

## Accounts and services

| Service | Used for | Where the secret lives | Status |
|---|---|---|---|
| GitHub `Jstickz/SermonAI` | source, Actions CI on three runners, secrets | repository secrets | live; PR #1 was the YouVersion migration |
| Deepgram (Nova-3) | live speech to text, EU endpoint default | CI secrets, local `.env` (debug builds only), or a church's own key in the OS keychain | live |
| Anthropic | Claude paraphrase stage (Haiku 4.5), summaries (Sonnet 5) | same as Deepgram | live since 15 Oct, after a workspace-scoped key and account credit were sorted |
| YouVersion Platform | Bible text and licences; 20 English versions licensed to the app key | CI secrets and `.env`; managed-only in the key model | live for builds; unavailable in the app until the gateway exists |
| Tyndale (NLT) | NLT text | managed-only | not wired |
| Cloudflare (Workers, D1/KV) | the SermonAI Gateway | **account does not exist; the operator must create it** | blocked on the operator |
| Tauri updater key | signs update bundles | `tauri-updater.key*`, gitignored; CI secret | in place; local release builds can finish without it |
| Apple Developer ID, Azure Trusted Signing | notarization and Authenticode | not yet | M11 |
| ebible.org | public-domain KJV and ASV source text | none needed | used at build time |

Retired: API.Bible (replaced by YouVersion 21 Sept 2026; its key remains in GitHub secrets, unreferenced).

## Machines

- Development laptop, Windows 11, 4-channel 48 kHz mic array, three loopback endpoints including two HDMI display-audio devices. Project at `D:\Projects\SermonAI` (moved out of OneDrive on 4 October 2026, then from `C:\`), with `src-tauri/target` outside any sync root.
- A clean Windows 11 VM for installer tests. No Windows 10 ISO.
- No Mac to hand. One real Mac was borrowed once for M0 DoD 2 on 21 September 2026.

Related: [[hybrid-key-model]], [[adr-0004-local-only-data]], [[open-questions]]
