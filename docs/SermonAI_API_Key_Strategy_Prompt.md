# SermonAI: API Key Strategy (Option 3, Hybrid)

**When to use this prompt:** after M1 deliverable 4 is committed, or at the latest before M4 starts (first real Sunday). Paste everything below the line into the code agent.

---

## Context

SermonAI calls three paid or licensed services: Deepgram (live speech to text), Anthropic Claude (paraphrase detection, summaries) and YouVersion Platform (Bible text), plus the Tyndale NLT API for NLT. Right now the app reads keys from `.env`, which only works on my dev machine. A church installing SermonAI will not have that file, and my own keys must never be shipped inside the installer, because anything bundled in a desktop app can be extracted.

**Decision: hybrid key model.**

1. **Managed mode (default).** A small SermonAI Gateway that I control holds my real keys. Each church installation gets its own install token at onboarding. The app never sees my real keys.
2. **Bring your own key (BYOK) mode (optional).** A church can paste its own Deepgram and Anthropic keys in Settings. These are stored in the operating system's secure keychain, never in SQLite or plain files.

Do not start building until you have read this whole prompt. Work in the phases below, one commit per phase, and stop at each marked **STOP** point for my input.

## Phase 0: Record the decision (do this first, no code)

1. Add an ADR at `docs/adr/` titled "API key handling: managed gateway plus BYOK". Include the options considered (BYOK only, gateway only, hybrid), why hybrid won, and the rule that release builds never contain real keys.
2. Update the PRD: add a subsection under the security or external services section describing both modes, and add functional requirements for them (continue from the next free FR number). Update anything that currently implies keys come from `.env` at runtime.
3. Update MILESTONES.md: add a "Key management" block as a prerequisite of M4, containing Phases 1 to 5 below as deliverables with their own Definition of Done. Remove or replace any parked item this supersedes.
4. Check the PRD for conflicts (for example, anything assuming a single developer key, or pricing tiers that affect the non-commercial status of the managed keys) and list them for me.

**STOP.** Show me the diff of the docs before continuing.

## Phase 1: Lock down dev keys

1. `.env` loading via `dotenvy` must only run in debug builds (`cfg(debug_assertions)` or an equivalent dev feature flag). Release builds must never call it.
2. Add a CI check that fails the release job if the built installer or binary contains any of my key values or obvious key patterns. Use the GitHub secrets to compare against, without ever printing them in logs.
3. Add a test proving a release build with no keychain entries and no install token starts cleanly and reports "not activated" instead of crashing.

## Phase 2: Credential abstraction in the desktop app

1. Create a single Rust module (suggested `src-tauri/src/credentials/`) that every service client goes through. No service client may read environment variables or config files for keys directly after this phase.
2. Model it per service, since each service can be in a different mode:

   | Service | Managed | BYOK | Notes |
   |---|---|---|---|
   | Deepgram | yes | yes | |
   | Anthropic | yes | yes | |
   | YouVersion | yes | no | Keys are tied to my app registration and my accepted publisher licenses. A church's own key would not carry those licenses. Managed only. |
   | Tyndale NLT | yes | no | Same reasoning. Managed only. |

3. Storage: use the `keyring` crate (Windows Credential Manager, macOS Keychain) for both the install token and any BYOK keys. Never write them to SQLite, logs, crash reports, IPC payloads sent to the frontend, or the settings export. The frontend may only ever see a masked form (last 4 characters) and a status.
4. Every credential lookup returns a clear status the UI can show: Managed active, BYOK active, Not activated, Token revoked, Quota reached, Gateway unreachable.

## Phase 3: The SermonAI Gateway

A Cloudflare Worker in a new folder `gateway/` in the same repo, deployed with `wrangler`. Real keys live only as Worker secrets (`wrangler secret put`), never in the repo.

**Endpoints** (all JSON, versioned under `/v1`):

1. `POST /v1/activate`: takes a one-time onboarding code I issue, returns a long-lived install token. The code is consumed on use. Store only a hash of the install token.
2. `POST /v1/deepgram/token`: authenticated with the install token. Returns a short-lived Deepgram access token so the app streams audio **directly to Deepgram**. Audio must never pass through the gateway. Verify against the current Deepgram docs which mechanism to use (their temporary token or grant endpoint, or a short-expiry scoped key via the management API) by making a real call, the same way you verified YouVersion. Report what you found.
3. `POST /v1/claude/messages`: authenticated. Forwards to Anthropic with my key added. Enforce an allowlist of models and a max token cap so a stolen install token cannot run arbitrary expensive jobs.
4. `GET /v1/bible/...`: authenticated. Proxies the YouVersion and Tyndale calls the app needs, adding the right key. Cache responses at the edge where the licenses allow it.
5. `GET /v1/status`: authenticated. Returns the install's mode, remaining quota for the month, and whether it is active.

**Storage:** Cloudflare D1 or KV for installs (hashed token, church name, created date, status, notes) and monthly usage counters (Deepgram minutes, Claude calls, Bible calls).

**Limits:** per-install monthly quotas and a per-minute rate limit. When a quota is reached, return a specific error the app can show as a friendly banner.

**Admin:** a small CLI script (in `gateway/scripts/`) protected by an admin secret to: issue onboarding codes, list installs with usage, revoke an install, adjust a quota. No web admin panel yet.

**Privacy:** never log request or response bodies. Transcript text going to Claude passes through the gateway but must not be stored. Log only install ID, endpoint, status code, and counts.

**STOP before deploying.** I need to create the Cloudflare account and give you the account details myself. Do not create accounts on my behalf. Tell me exactly what to set up and which secrets to add.

## Phase 4: Wire the app to the gateway

1. Deepgram client: before opening the WebSocket, get a fresh short-lived token from the gateway (managed) or use the BYOK key. On every reconnect (deliverable 10 backoff logic), fetch a new token rather than reusing an expired one.
2. Claude client and Bible client: route through the gateway in managed mode; call Anthropic directly with the stored key in BYOK mode.
3. Failure behaviour, which must be tested:
   1. Gateway unreachable or quota reached during a service: show a banner, and fall back to on-device whisper (FR-08) if the Offline Speech Pack is installed. The transcript must keep going.
   2. Token revoked: stop cleanly with a message telling the operator to contact me.
   3. Bible lookups: serve from the local cache and bundled packs first. The gateway is only hit on a cache miss.
4. Gateway base URL comes from a build-time config, not user input, with a debug override for local testing against `wrangler dev`.

## Phase 5: Settings and onboarding UI

1. Onboarding wizard gets an activation step: "Enter your SermonAI activation code" (managed), with a secondary link "I have my own Deepgram and Anthropic keys" (BYOK).
2. Settings gets a "Services and keys" panel following branding.md: per service, show the current mode, masked key or install status, remaining monthly quota in managed mode, and buttons to Test, Replace and Remove.
3. The Test button makes the cheapest possible real call to confirm the key or token works and shows a clear pass or fail.
4. Switching modes never loses the other mode's stored credentials unless the user clicks Remove.

## Definition of Done

1. A release installer built by CI contains none of my keys (CI check passes and I can verify by searching the binary).
2. On a fresh machine, entering a valid activation code makes live transcription, detection and Bible lookups work with no `.env` and no manual key entry.
3. Revoking that install from the admin script stops it within one request, with a clear message in the app.
4. Setting a tiny quota and exceeding it shows the banner and switches to offline whisper without the transcript stopping.
5. BYOK: pasting my own Deepgram and Anthropic keys works with the gateway switched off entirely.
6. Deepgram audio is confirmed to go directly to Deepgram, never through the gateway.
7. `cargo clippy -D warnings`, `cargo test`, `tsc`, `eslint`, `vitest` and the gateway's tests all pass. Report the full test counts per binary, not a filtered run.

## Decisions I still need to make (ask me, do not guess)

1. Monthly quota per church in managed mode (Deepgram minutes, Claude calls).
2. Whether to use a custom domain for the gateway or the default `workers.dev` address.
3. Whether BYOK churches should still register with the gateway for Bible access (they must, since YouVersion and NLT are managed only), and how that looks in onboarding.

Keep the standing rules: milestones are sequential, park anything outside scope in MILESTONES.md rather than half building it, flag anything you guessed at, and verify external APIs with real calls before trusting documentation.
