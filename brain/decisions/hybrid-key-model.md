# Hybrid key model: managed gateway plus bring-your-own-key

**Decided:** 22 September 2026 (PRD §17, §11.3 amended) · **Refined:** 16 October 2026 · **Spec:** `docs/SermonAI_API_Key_Strategy_Prompt.md`

## The decision

The app calls three paid or licensed services (Deepgram, Anthropic, YouVersion Platform, plus Tyndale for NLT). Developer keys must never ship inside the installer, because anything bundled in a desktop app can be extracted.

1. **Managed mode (default).** A SermonAI Gateway (Cloudflare Worker, `gateway/` in this repo) holds the real keys. Each installation gets an install token at onboarding. Audio streams directly to Deepgram on a short-lived token; Claude and Bible calls proxy through the gateway. Never logs bodies.
2. **BYOK mode (optional).** A church pastes its own Deepgram and Anthropic keys in Settings → Services & keys. Stored in the OS keychain only, never SQLite, logs, IPC payloads or exports; the frontend sees a masked form and a status.

YouVersion and Tyndale are managed-only: their licences are SermonAI's, and a church's own key cannot carry them.

## Operator's decisions of 16 October 2026

- Managed quota: **10 hours of transcription and 1,500 Claude calls per church per month**.
- The pilot runs on the `workers.dev` address; a custom domain later.
- **BYOK churches still register with the gateway** for Bible access, using the same activation code, so onboarding has one path with an optional "use my own keys" step.

## State

Phases 0 to 2 done 22 to 23 September: `credentials/` abstraction, `keychain.rs`, `.env` loaded in debug builds only, BYOK working in Settings. Phases 3 to 5 (the Worker, app wiring, onboarding activation) are not built. **Hard STOP that is the operator's:** the Cloudflare account must be created by them, never by the agent. Until the gateway exists, NIV cannot be the default translation and YouVersion/Tyndale report "not activated" on every machine. Due before M4's live service.

Related: [[adr-0004-local-only-data]], [[people-and-accounts]], [[open-questions]], [[sessions/2026-09-22]], [[sessions/2026-09-23]]
