# ADR 0004: Local-only data, no account required

**Decided:** 7 September 2026 · **Status:** Accepted · **Full record:** `docs/adr/0004-local-only-data.md`

All sermon content lives in one SQLite file (WAL, FTS5) in the app data directory. No account is needed to run a service. Outbound traffic is limited to Deepgram, the Bible API (now YouVersion Platform, not API.Bible as the ADR says) and Anthropic, only in online mode. Telemetry is opt-in and content-free.

Why: sermons carry pastoral confidences; target-market internet is unreliable; the incumbent competitor is fully local. "Your sermons stay in your building" is a claim we can make plainly.

Costs: no sync, no recovery if a laptop is lost (mitigated by an auto-save folder the church already syncs), and support cannot inspect data, so logs must be good. The one account-like concept is licensing (3 devices, 30-day offline grace). The [[hybrid-key-model]] adds a gateway for managed keys, but transcript text passing through it is never stored.

Related: [[people-and-accounts]]
