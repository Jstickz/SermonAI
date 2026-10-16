# Two-stage detection: provisional, then confirmed

**Decided:** 23 September 2026 · **Status:** standing (CLAUDE.md) · **PRD:** §18.1, §13.5
**Implemented:** 24 September 2026 in `detection/pipeline.rs` ([[sessions/2026-09-24]])

## The decision

A scripture reference is caught twice, and the two stages have different rights.

- **Provisional.** The regex stage runs on *interim* transcript text. A match raises a candidate in staging marked unconfirmed. It is never projectable: not by Enter, not by Go Live, not by auto-live. It is withdrawn *silently* if the confirmed text does not support it.
- **Confirmed.** When settled text supports the candidate it firms up and becomes projectable.

## Why

Confirmation is Deepgram's decision, taken only once it judges the speaker to have stopped, measured at about 2.3 s. That is not ours to shorten: the `endpointing` parameter does not move it (tested at default, 100 ms and 300 ms; see [[lessons]]).

Waiting for confirmation before showing anything wastes a second of the operator's warning. Acting on interim text risks projecting a verse the next result revises away, and a verse on a screen cannot be recalled. A live run showed the risk concretely: "...if you will to join" became "...to John chapter three" one result later. Showing early without letting it out is the only option that costs nothing.

Withdrawal is silent because interim results are *revisions*, not mistakes. A panel that announced every retraction would teach the operator to ignore it.

## Consequences seen since

- A provisional card has no Stage button, not a disabled one (4 Oct). A disabled button invites waiting on it.
- The last interim of a run has no "next final" to withdraw it, so provisional entries now also withdraw on expiry and on stream close (bug found 4 Oct, [[sessions/2026-10-04]]).
- Only confirmed candidates are written to `detected_scriptures`; a withdrawn one was never a detection.

Related: [[two-latency-budgets]], [[adr-0002-staging-first-output]], [[adr-0003-three-stage-detection]]
