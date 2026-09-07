# ADR 0004 — Local-only data, no account required

- **Status:** Accepted
- **Date:** 7 September 2026
- **PRD:** §14, §17

## Context

Sermons are sensitive. A transcript can contain pastoral confidences,
congregational names, and teaching a church may not want indexed anywhere.
Churches in the target markets also have unreliable internet, and the competitor
that has won 1,000+ installs is fully local. Requiring an account to run a
Sunday service is a failure mode waiting for a bad connection.

## Decision

All sermon content lives in one SQLite file (WAL, FTS5) in the app data
directory. No account is required to use the app. Outbound traffic is limited to
Deepgram, API.Bible and Anthropic, only in online mode, and only carries what
those calls need. Telemetry is opt-in and content-free: no transcript text ever
leaves the machine in telemetry.

Cloud backup is deliberately deferred to Phase 4 and will be opt-in.

## Consequences

**Good**
- GDPR and CCPA are satisfied largely by design, with export and delete.
- A service never fails because a login expired or a server was down.
- "Your sermons stay in your building" is a claim we can make plainly.

**Costs**
- No cross-device sync, and no recovery if a church loses its laptop. The
  optional auto-save folder for summary PDFs (a synced Drive or OneDrive folder
  the church already has) is the pragmatic mitigation.
- Licensing still needs a periodic online check, 3 devices per license with a
  30-day offline grace — the one place an account-like concept exists.
- Support cannot inspect a customer's data. Diagnostics must be good enough to
  debug from logs the operator can send.
