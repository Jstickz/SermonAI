# Leftover SermonAI instances may be closed without asking

**Decided:** 24 September 2026 · **Status:** standing (CLAUDE.md)

## The decision

A `sermonai.exe` left running after one of the operator's tests holds `target/release/sermonai.exe`, and `cargo` then fails every build and every integration-test run with "failed to remove file … Access is denied (os error 5)". Close it and carry on, **unless the operator is in the middle of a test**, in which case ask.

## Why

The Live tab has no persistence yet (that is M4, FR-34), so closing the app discards whatever is on it. That is acceptable for a finished test and not for one in progress. The alternative, asking every time, blocks every build on a question whose answer is almost always yes.

Revisit when M4 persists services: the cost of closing falls to nothing, and the rule can be unconditional.

Related: [[sessions/2026-09-24]]
