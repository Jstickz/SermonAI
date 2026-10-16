# Measure the thing, not a proxy

**Decided:** 23 September 2026 · **Status:** standing (CLAUDE.md)

## The decision

State the method beside the number, and check a measurement against something it must be bounded by before trusting it. A figure with no method next to it is not a figure of record.

## Why

M1 produced three figures that were wrong in ways that looked plausible:

1. **Memory.** Summing `WorkingSet` across processes reported 583 MB where the private working set was 152 MB. Resolving processes by performance-counter *name* reported 40 MB where the true figure was 193 MB, because several processes are all called `msedgewebview2` and the lookup resolved 1 of 9.
2. **Lag anchor.** Timing from when capture started, rather than when the first audio was sent, charged the WebSocket handshake (over a second) to every sample.
3. **Lag population.** Measuring only confirmed text and comparing it to a budget written about visible text read as a fourfold failure of a passing pipeline.

Then a fourth, on 23 September: lag as the gap between two clocks (wall clock vs Deepgram's audio timestamp) accumulated every dropped chunk and every tick of sound-card drift, so an hour read three times a 38-second figure for the same pipeline.

And a fifth, on 4 October: M0's "approximately 300 ms" cold start was a stopwatch on a VM before the app had a clock. Re-measured properly it was 716 ms, with 554 ms the platform's.

## How it is applied now

- Every measurement script prints what it resolved or sampled, and declares itself invalid rather than quietly halving an answer.
- Each lag result is timed against the moment its own audio was handed over: one clock measured against itself.
- Budget tests assert only in the release profile when a debug build would measure the wrong thing.
- Cold start is split into the platform's share (probe apps) and ours.

The full catalogue of faults is in [[lessons]]. Related: [[two-latency-budgets]], [[sessions/2026-09-23]], [[sessions/2026-10-04]]
