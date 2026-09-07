# ADR 0002 — Staging-first output

- **Status:** Accepted
- **Date:** 7 September 2026
- **PRD:** §8.6 (FR-50, FR-51), §10.2, §13.5

## Context

Detection is probabilistic. The regex stage is close to certain, but the vector
and LLM stages will sometimes propose a verse that is wrong, or right but not
the one the preacher means. A wrong verse on the projector in front of the
congregation is the single worst failure this product can produce — worse than
no verse at all, and worse than a slow one.

## Decision

Nothing reaches an output without passing through a staging slot. Accepting a
detection puts the verse in **Staged**; a second, deliberate action (Enter, the
Go Live button, or a remote tap) promotes it to **Live**. Auto-live exists as an
opt-in setting with a visible on-screen indicator while it is on.

The operator screen shows Staged and Live side by side at all times, so the
distinction is carried by position as well as by colour.

## Consequences

**Good**
- The operator is the last check on every AI decision. False positives cost a
  glance, not a service.
- Confidence thresholds can be tuned generously: a marginal proposal is cheap
  when it lands in staging rather than on the wall.
- The phone remote becomes safe to hand to a volunteer — Go Live is a decision
  someone made, not a side effect.

**Costs**
- One extra keypress per verse. Accepted: the M3 definition of done requires a
  non-developer to run a 15-minute mock service with only a keyboard and a
  printed cheat sheet.
- Auto-live has to be built anyway for churches that want hands-off operation,
  so both paths need testing.
