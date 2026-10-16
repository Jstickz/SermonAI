# ADR 0002: Staging-first output

**Decided:** 7 September 2026 · **Status:** Accepted · **Full record:** `docs/adr/0002-staging-first-output.md`

Nothing reaches an output without passing through a staging slot. Accepting a detection puts the verse in Staged; a second deliberate action (Enter, Go Live, a remote tap) promotes it to Live. Auto-live is opt-in with a visible indicator. Staged and Live sit side by side so position carries the distinction as well as colour.

Why: a wrong verse on the projector in front of a congregation is the worst failure the product can produce, worse than no verse and worse than a slow one. With the operator as the last check, confidence thresholds can be tuned generously and the phone remote is safe to hand to a volunteer.

Built 16 October 2026 as `output/staging.rs` and `StagingPanel.tsx` ([[sessions/2026-10-16]]); the key rule is in [[enter-does-the-next-thing]]. [[two-stage-detection]] adds a stricter tier below Staged.
