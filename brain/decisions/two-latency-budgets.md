# Latency has two budgets, not one

**Decided:** 23 September 2026 · **Status:** standing (CLAUDE.md) · **PRD:** §18.1

## The decision

Measure two things and never one:

| Budget | Measures | Target | Whose |
|---|---|---|---|
| To the provisional candidate | spoken word → words on screen / provisional card | p50 ≤ 500 ms, p95 ≤ 900 ms, plus episode conduct | mostly network and Deepgram |
| Confirmed text → projectable | settled text → confirmed card | about 50 ms p95 | all ours |

Episode conduct: zero dropped audio, zero reconnects, transcript complete, automatic recovery, nothing past 6 s. Degraded minutes are recorded as observed; no threshold is set until M4's first real Sunday.

## Why

A single end-to-end number hides that most of the delay belongs to a vendor. The previous single budget, 800 ms p99 end to end (and the DoD's "700 ms p99"), was unreachable from a Nigerian connection at any Deepgram region: the round trip alone spends 715 ms at p99 on the nearest one before the 250 ms chunk is counted. It was unreachable and went unnoticed for exactly that reason.

The split also follows from [[two-stage-detection]]: the ~2.3 s settle time is Deepgram's, and the only part we can be judged on starts when settled text arrives.

## Numbers of record

20-minute release run, EU endpoint, 835 results: p50 187 ms, p95 501 ms, 0 dropped, 0 reconnects, worst 3,952 ms. See `docs/testing/m1-latency-baseline.md` and `docs/testing/network-latency.md`.

Related: [[eu-deepgram-region]], [[measure-the-thing-not-a-proxy]], [[sessions/2026-09-23]]
